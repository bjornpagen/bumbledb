//! Pending recovery: one fold over occupant, generation, and floor.
//! Remaining one-by-one work is data. A lost slot takes the one path —
//! re-open to tip and race once.

use std::sync::Arc;

use bumbledb::Admission;

use crate::apply::{Applied, apply};
use crate::braids::BraidId;
use crate::codec::{Op, OpKind};
use crate::manifest::log_key;
use crate::replica::{Fault, OpenRefusal};
use crate::sidecar::{Chain, Pending};

use super::{
    Core, Error, Inner, Live, ObjectStore, PublishEnd, Result, Settled, StepHook, Theory,
    WriterState, WriterStep,
};

pub(crate) enum PendingArm {
    Clear,
    Backlog(BraidId),
    Discard,
}

/// Classification of a pending batch against occupant, store
/// generation, and the floor when one is present.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PendingFold {
    /// Occupant bytes are the pending bytes — the slot is ours.
    Ours,
    /// Occupant is someone else; the store sits at the vector sum.
    TheirsUnapplied,
    /// Occupant is someone else; the store already counts the pending.
    TheirsApplied,
    /// No occupant; the store sits at the vector sum.
    AbsentUnapplied,
    /// No occupant; the store already counts the pending.
    AbsentApplied,
    /// The slot sits at or below the published floor.
    BelowFloor,
    /// The store generation is neither the vector sum nor sum+1.
    Phantom,
}

/// Leftover one-by-one work. A mid-fold refusal yields this value; the
/// fold does not abort.
struct Remaining<'a>(&'a [Vec<Op>]);

/// Re-judges the pending bytes against the winner-current occupant and
/// the generation the store shows. Remaining work is the arm.
fn fold_pending(
    sum: u64,
    generation: u64,
    occupant: Option<&[u8]>,
    pending_bytes: &[u8],
    below_floor: bool,
) -> PendingFold {
    if below_floor {
        return PendingFold::BelowFloor;
    }
    match occupant {
        Some(bytes) if bytes == pending_bytes => PendingFold::Ours,
        Some(_) if generation == sum => PendingFold::TheirsUnapplied,
        Some(_) => PendingFold::TheirsApplied,
        None if generation == sum => PendingFold::AbsentUnapplied,
        None if generation == sum.saturating_add(1) => PendingFold::AbsentApplied,
        None => PendingFold::Phantom,
    }
}

impl<T, S, H> Inner<T, S, H>
where
    T: Theory + Clone + Send + Sync + 'static,
    S: ObjectStore + 'static,
    H: StepHook + 'static,
{
    /// The published floor covers `slot` when a floor is present and
    /// the braid's head is at or past it.
    fn floor_covers(core: &Core<T>, braid: BraidId, slot: u64) -> bool {
        core.floor
            .as_ref()
            .and_then(|(_, doc)| doc.braids.get(&braid))
            .is_some_and(|head| slot <= head.g)
    }

    /// Pending resolution — one fold: occupant, the generation the
    /// store shows, and the floor when it is present. `Rejected`
    /// clears and publishes nothing (a resurrected never-judged
    /// batch); an accepted no-op at the exact vector sum was born a
    /// no-op and clears; a slot at or below the floor is already
    /// published and clears; otherwise the commit is real and
    /// unpublished. A never-judged absent pending applies then
    /// publishes; a pending that already moved generation re-applies
    /// in the engine no-op arm, then publishes. A lost slot takes the
    /// one path: re-open to tip and race once. One-by-one leftover
    /// work is `Remaining`, never an abort.
    pub(crate) fn resolve_backlog(
        self: &Arc<Self>,
        core: &mut Core<T>,
        segments: Option<&[Vec<Op>]>,
        live: &mut Live,
    ) -> Result<()> {
        let pending = match &core.chain {
            Chain::Pending { batch, .. } => batch.clone(),
            Chain::Settled { .. } => return Ok(()),
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
        let key = log_key(&self.prefix, braid, pending.slot);
        let occupant = self
            .store
            .get(&key)
            .map_err(|err| Error::Fault(Fault::Store(err)))?;
        let occupant_bytes = occupant.as_ref().map(|fetched| fetched.bytes.as_slice());
        let below_floor = Self::floor_covers(core, braid, pending.slot);
        match fold_pending(
            core.chain.sum(),
            core.generation()?,
            occupant_bytes,
            &pending.bytes,
            below_floor,
        ) {
            PendingFold::BelowFloor => self.clear_pending(core),
            PendingFold::Ours => self.absorb_ours(
                core,
                braid,
                pending.slot,
                &occupant.expect("ours carries the matching occupant").bytes,
            ),
            PendingFold::TheirsUnapplied | PendingFold::TheirsApplied => {
                self.lose(
                    core,
                    braid,
                    pending.slot,
                    &occupant.expect("theirs arms carry an occupant").bytes,
                    live,
                )?;
                self.rejudge(core, braid, &ops, live, segments)
            }
            PendingFold::AbsentUnapplied => self.apply_then_publish(
                core,
                &pending,
                batch.header.timestamp,
                &ops,
                live,
                segments,
            ),
            PendingFold::AbsentApplied => {
                // Generation already moved: re-apply is the engine
                // no-op arm, then publish the surviving pending.
                let _ = self.apply_local(core, &ops)?;
                match self.publish(
                    core,
                    braid,
                    pending.slot,
                    batch.header.timestamp,
                    &pending.bytes,
                )? {
                    PublishEnd::Done(_) => Ok(()),
                    PublishEnd::Lost { winner } => {
                        self.lose(core, braid, pending.slot, &winner, live)?;
                        self.rejudge(core, braid, &ops, live, segments)
                    }
                }
            }
            PendingFold::Phantom => self.re_establish(core, None),
        }
    }

    fn absorb_ours(
        &self,
        core: &mut Core<T>,
        braid: BraidId,
        slot: u64,
        bytes: &[u8],
    ) -> Result<()> {
        let applied = apply(
            match &core.db {
                WriterState::Mounted { db } => db.as_ref(),
                WriterState::Unmounted => {
                    return Err(Error::Refused(OpenRefusal::Unmounted));
                }
            },
            &mut core.chain,
            &self.codec,
            braid,
            slot,
            bytes,
        )
        .map_err(|err| Error::Fault(Fault::Engine(err)))?;
        match applied {
            Applied::Absorbed { .. } | Applied::Advanced { .. } => self.clear_pending(core),
            Applied::Rejected(_) | Applied::Refused(_) => self.re_establish(core, None),
        }
    }

    fn apply_then_publish(
        self: &Arc<Self>,
        core: &mut Core<T>,
        pending: &Pending,
        timestamp: u64,
        ops: &[Op],
        live: &mut Live,
        segments: Option<&[Vec<Op>]>,
    ) -> Result<()> {
        let before = core.generation()?;
        let Some(after) = self.apply_local(core, ops)? else {
            return Ok(());
        };
        let sum = core.chain.sum();
        if after == sum {
            // Born a net no-op: the crash landed between the no-op
            // verdict and its pending clear.
            return self.clear_pending(core);
        }
        if after != sum + 1 || before > after {
            // A generation no pending term accounts for: phantom or
            // torn store.
            return self.re_establish(core, None);
        }
        match self.publish(core, pending.braid, pending.slot, timestamp, &pending.bytes)? {
            PublishEnd::Done(_) => Ok(()),
            PublishEnd::Lost { winner } => {
                self.lose(core, pending.braid, pending.slot, &winner, live)?;
                self.rejudge(core, pending.braid, ops, live, segments)
            }
        }
    }

    fn apply_local(&self, core: &mut Core<T>, ops: &[Op]) -> Result<Option<u64>> {
        let admission = core
            .db()?
            .write(|tx| {
                for op in ops {
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
        match admission {
            Admission::Rejected(_) => {
                // Nothing was acked at `Published`, and a `Local` ack
                // spent its recorded loss window; a born-rejected batch
                // reaching the log is the publish law's cardinal sin,
                // structurally impossible here.
                self.clear_pending(core)?;
                Ok(None)
            }
            Admission::Accepted(committed) => Ok(Some(committed.generation.value())),
        }
    }

    fn rejudge(
        self: &Arc<Self>,
        core: &mut Core<T>,
        braid: BraidId,
        ops: &[Op],
        live: &mut Live,
        segments: Option<&[Vec<Op>]>,
    ) -> Result<()> {
        let settled = self.discipline(core, braid, ops, live, None)?;
        if matches!(settled, Settled::Rejected(_))
            && let Some(segments) = segments
            && segments.len() > 1
        {
            let mut rest = segments;
            while !rest.is_empty() {
                let Remaining(next) = self.fold_remaining(core, braid, rest);
                rest = next;
            }
        }
        Ok(())
    }

    /// One-by-one leftover work. A mid-fold `Err` yields the tail as
    /// `Remaining`; the fold's type is not `Result`.
    fn fold_remaining<'a>(
        self: &Arc<Self>,
        core: &mut Core<T>,
        braid: BraidId,
        segments: &'a [Vec<Op>],
    ) -> Remaining<'a> {
        let mut rest = segments;
        while let Some((head, tail)) = rest.split_first() {
            match self.discipline(core, braid, head, &mut Live::default(), None) {
                Ok(_) => rest = tail,
                Err(_) => return Remaining(tail),
            }
        }
        Remaining(rest)
    }

    /// Applies the pending batch and reads the arm: the fold plus the
    /// wholeness instrument decide everything. A floor, when present,
    /// that already covers the slot is published — Clear, no re-judge.
    pub(crate) fn pending_arm(&self, core: &mut Core<T>) -> Result<PendingArm> {
        let pending = match &core.chain {
            Chain::Pending { batch, .. } => batch.clone(),
            Chain::Settled { .. } => panic!("caller checked pending presence"),
        };
        let Ok(batch) = self.codec.decode(&pending.bytes) else {
            return Ok(PendingArm::Discard);
        };
        if Self::floor_covers(core, pending.braid, pending.slot) {
            self.clear_pending(core)?;
            return Ok(PendingArm::Clear);
        }
        let Some(after) = self.apply_local(core, &batch.ops)? else {
            return Ok(PendingArm::Clear);
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

#[cfg(test)]
mod tests {
    use super::{PendingFold, fold_pending};

    #[test]
    fn fold_consults_floor_first() {
        assert_eq!(
            fold_pending(0, 0, None, b"ours", true),
            PendingFold::BelowFloor
        );
        assert_eq!(
            fold_pending(0, 0, Some(b"theirs"), b"ours", true),
            PendingFold::BelowFloor
        );
    }

    #[test]
    fn fold_names_occupant_and_generation() {
        assert_eq!(
            fold_pending(3, 3, Some(b"ours"), b"ours", false),
            PendingFold::Ours
        );
        assert_eq!(
            fold_pending(3, 3, Some(b"theirs"), b"ours", false),
            PendingFold::TheirsUnapplied
        );
        assert_eq!(
            fold_pending(3, 4, Some(b"theirs"), b"ours", false),
            PendingFold::TheirsApplied
        );
        assert_eq!(
            fold_pending(3, 3, None, b"ours", false),
            PendingFold::AbsentUnapplied
        );
        assert_eq!(
            fold_pending(3, 4, None, b"ours", false),
            PendingFold::AbsentApplied
        );
        assert_eq!(
            fold_pending(3, 6, None, b"ours", false),
            PendingFold::Phantom
        );
    }
}
