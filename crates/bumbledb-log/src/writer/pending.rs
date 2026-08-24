//! Pending recovery: the three forced arms, crash-idempotent at every
//! prefix. A lost slot takes the one path — re-open to tip and race once.

use std::sync::Arc;

use bumbledb::Admission;

use crate::apply::{Applied, apply};
use crate::braids::BraidId;
use crate::codec::{Op, OpKind};
use crate::manifest::log_key;
use crate::replica::Fault;

use super::{
    Core, Error, Inner, Live, ObjectStore, PublishEnd, Result, Settled, StepHook, Theory,
    WriterStep,
};

pub(crate) enum PendingArm {
    Clear,
    Backlog(BraidId),
    Discard,
}

impl<T, S, H> Inner<T, S, H>
where
    T: Theory + Clone + Send + Sync + 'static,
    S: ObjectStore + 'static,
    H: StepHook + 'static,
{
    /// Pending resolution — the three forced arms, idempotent by L10:
    /// apply the pending batch; `Rejected` clears and publishes nothing
    /// (a resurrected never-judged batch); an accepted no-op at the
    /// exact vector sum was born a no-op and clears; otherwise the
    /// commit is real and unpublished — publish create-or-compare, and
    /// a lost slot takes the one path: re-open to tip and race once.
    #[allow(clippy::too_many_lines)]
    pub(crate) fn resolve_backlog(
        self: &Arc<Self>,
        core: &mut Core<T>,
        segments: Option<&[Vec<Op>]>,
        live: &mut Live,
    ) -> Result<()> {
        let Some(pending) = core.chain.pending.clone() else {
            return Ok(());
        };
        let braid = pending.braid;
        if core.wedged.contains_key(&braid) {
            return Err(Error::Wedged { braid });
        }
        let Ok(batch) = self.codec.decode(&pending.bytes) else {
            // Our own pending bytes refusing to decode is a torn local
            // state; the disposable law answers.
            return self.re_establish(core, None);
        };
        let ops = batch.ops;

        let before = core.generation()?;
        let admission = core
            .db()
            .write(|tx| {
                for op in &ops {
                    match op.kind {
                        OpKind::Insert => {
                            tx.insert_dyn(op.relation, op.rows.iter())?;
                        }
                        OpKind::Delete => {
                            tx.delete_dyn(op.relation, op.rows.iter())?;
                        }
                    }
                }
                Ok(())
            })
            .map_err(|err| Error::Fault(Fault::Engine(err)))?;
        self.step(WriterStep::ApplyLocal)?;
        let after = match admission {
            Admission::Rejected(_) => {
                // Nothing was acked at `Published`, and a `Local` ack
                // spent its recorded loss window; a born-rejected batch
                // reaching the log is the publish law's cardinal sin,
                // structurally impossible here.
                self.clear_pending(core)?;
                return Ok(());
            }
            Admission::Accepted(committed) => committed.generation.value(),
        };
        let sum = core.chain.sum();
        if after == sum {
            // Born a net no-op: the crash landed between the no-op
            // verdict and its pending clear.
            self.clear_pending(core)?;
            return Ok(());
        }
        if after != sum + 1 || before > after {
            // A generation no pending term accounts for: phantom or
            // torn store.
            return self.re_establish(core, None);
        }

        // Real and unpublished. The slot is head+1 by construction;
        // whatever occupies it decides between the byte-equal
        // absorption and the one loss path.
        let key = log_key(&self.prefix, braid, pending.slot);
        let occupant = self
            .store
            .get(&key)
            .map_err(|err| Error::Fault(Fault::Store(err)))?;
        match occupant {
            Some(winner) if winner.bytes == pending.bytes => {
                // Already published: the crash was mid-publish, after
                // the create landed. The byte comparison proves the
                // slot is ours and the store survives in place.
                let applied = apply(
                    core.db
                        .as_deref()
                        .expect("an established writer holds a store"),
                    &mut core.chain,
                    &self.codec,
                    braid,
                    pending.slot,
                    &winner.bytes,
                    0,
                )
                .map_err(|err| Error::Fault(Fault::Engine(err)))?;
                match applied {
                    Applied::Absorbed { .. } | Applied::Advanced { .. } => {
                        self.clear_pending(core)?;
                        return Ok(());
                    }
                    Applied::Rejected(_) | Applied::Refused(_) => {
                        return self.re_establish(core, None);
                    }
                }
            }
            Some(winner) => {
                self.lose(core, braid, pending.slot, &winner.bytes, live)?;
            }
            None => {
                let hole = core
                    .floor
                    .as_ref()
                    .and_then(|(_, doc)| doc.braids.get(&braid))
                    .is_some_and(|head| pending.slot <= head.g);
                if hole {
                    // Retention passed our slot: the world moved beyond
                    // reach; re-open to tip and re-judge there.
                    self.re_establish(core, Some(pending))?;
                } else {
                    match self.publish(
                        core,
                        braid,
                        pending.slot,
                        batch.header.timestamp,
                        &pending.bytes,
                    )? {
                        PublishEnd::Done(_) => return Ok(()),
                        PublishEnd::Lost { winner } => {
                            self.lose(core, braid, pending.slot, &winner, live)?;
                        }
                    }
                }
            }
        }

        // The one path's re-judgment of the recorded ops at the tip.
        let settled = self.discipline(core, braid, &ops, live, None)?;
        if let Settled::Rejected(_) = settled
            && let Some(segments) = segments
            && segments.len() > 1
        {
            // The composite rejected at the re-judgment: one-by-one
            // fallback, each caller as its own transaction in queue
            // order.
            for segment in segments {
                let _ = self.discipline(core, braid, segment, &mut Live::default(), None)?;
            }
        }
        Ok(())
    }
    /// Applies the pending batch and reads the arm: the verdict plus
    /// the wholeness instrument decide everything.
    pub(crate) fn pending_arm(&self, core: &mut Core<T>) -> Result<PendingArm> {
        let pending = core
            .chain
            .pending
            .clone()
            .expect("caller checked pending presence");
        let Ok(batch) = self.codec.decode(&pending.bytes) else {
            return Ok(PendingArm::Discard);
        };
        let admission = core
            .db()
            .write(|tx| {
                for op in &batch.ops {
                    match op.kind {
                        OpKind::Insert => {
                            tx.insert_dyn(op.relation, op.rows.iter())?;
                        }
                        OpKind::Delete => {
                            tx.delete_dyn(op.relation, op.rows.iter())?;
                        }
                    }
                }
                Ok(())
            })
            .map_err(|err| Error::Fault(Fault::Engine(err)))?;
        self.step(WriterStep::ApplyLocal)?;
        let after = match admission {
            Admission::Rejected(_) => {
                self.clear_pending(core)?;
                return Ok(PendingArm::Clear);
            }
            Admission::Accepted(committed) => committed.generation.value(),
        };
        let sum = core.chain.sum();
        if after == sum {
            self.clear_pending(core)?;
            Ok(PendingArm::Clear)
        } else if after == sum + 1 {
            Ok(PendingArm::Backlog(pending.braid))
        } else {
            Ok(PendingArm::Discard)
        }
    }
}
