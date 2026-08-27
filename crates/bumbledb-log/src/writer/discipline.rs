//! The commit discipline: one shape for every pass. Durability is
//! `Pending → durable → Settled`, in that order: the batch is fsynced
//! as `Pending` before any apply, and the transition to `Settled` is
//! written only after the verdict. A below-floor `put_create` is
//! refused as retired before it touches the store. A loss loops back
//! through the same `db.write` of the same recorded ops.

use std::io;
use std::sync::Arc;
use std::time::SystemTime;

use bumbledb::Admission;

use crate::braids::BraidId;
use crate::codec::{BatchHeader, Op, OpKind};
use crate::manifest::log_key;
use crate::replica::Fault;
use crate::sidecar::{Chain, ChainEntry, Pending};
use crate::store::{Create, CreateProbe, resolve_ambiguous_create, retry_read};

use super::{
    AckMode, ContentionCause, Core, Durability, Error, Inner, LOSS_BOUND, Live, ObjectStore,
    Resolved, Result, Slotted, StepHook, Theory, Waiter, WriterStep,
};

pub(crate) enum Settled {
    /// The engine's verdict; the accepted payload is the braid slot.
    Judged(Admission<u64>),
    /// Waiters were acked `LocalPending`; publication continues on the
    /// detached publisher, keyed by the pending bytes.
    Detached {
        bytes: Vec<u8>,
    },
}

pub(crate) enum PublishEnd {
    Done(Settled),
    /// The slot was lost to these winner bytes; the one path answers.
    Lost {
        winner: Vec<u8>,
    },
}

pub(crate) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |elapsed| {
            u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
        })
}

impl<T, S, H> Inner<T, S, H>
where
    T: Theory + Clone + Send + Sync + 'static,
    S: ObjectStore + 'static,
    H: StepHook + 'static,
{
    /// The commit discipline, one shape for every pass: encode at the
    /// current head with the monotone ts clamp, fsync as `Pending`
    /// BEFORE first judgment, apply in one `db.write`, then publish.
    /// `Settled` is written only after that verdict — never a
    /// `pending: null` write ahead of the resolution, and a refusal
    /// never advances the vector. A loss loops back here through the
    /// disposable law — the re-judgment is the same `db.write` of the
    /// same recorded ops at the re-opened tip, never a body re-run.
    /// At the loss bound the final re-judgment sources the contention
    /// cause: its own rejection is `HotKey` (statement and offending
    /// values from the engine's violation), an accepted-but-unpublished
    /// apply is `SlotRace` with the batch retained in `Pending`.
    pub(crate) fn discipline(
        self: &Arc<Self>,
        core: &mut Core<T>,
        braid: BraidId,
        ops: &[Op],
        live: &mut Live,
        mut waiters: Option<&[Arc<Waiter>]>,
    ) -> Result<Settled> {
        loop {
            if core.wedged.contains_key(&braid) {
                return Err(Error::Wedged { braid });
            }
            let head = core.chain.position(braid);
            let slot = head.g + 1;
            let header = BatchHeader {
                fingerprint: self.fingerprint,
                braid,
                braid_gen: slot,
                prev: head.prev,
                writer: self.writer_id,
                timestamp: now_ms().max(head.ts),
            };
            let bytes = self.codec.encode(&header, ops).map_err(Error::Encode)?;
            self.step(WriterStep::Encode)?;

            // Pending → durable, before any apply.
            self.persist_pending(
                core,
                Pending {
                    braid,
                    slot,
                    bytes: bytes.clone(),
                },
            )?;

            let before = core.generation()?;
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
                Admission::Rejected(violations) => {
                    self.clear_pending(core)?;
                    if live.losses >= LOSS_BOUND {
                        self.scream("the terminal re-judgment rejected");
                        return Err(Error::Contention {
                            braid,
                            cause: self.hot_key(&violations),
                        });
                    }
                    return Ok(Settled::Judged(Admission::Rejected(violations)));
                }
                Admission::Accepted(committed) => {
                    if committed.generation.value() == before {
                        // The publish law: the empty commit is not a
                        // commit, and the log never gains a no-op slot.
                        self.clear_pending(core)?;
                        return Ok(Settled::Judged(Admission::Accepted(head.g)));
                    }
                }
            }

            if live.losses >= LOSS_BOUND {
                // The bound is spent and the final re-judgment
                // accepted: the applied batch stays in Pending, and
                // publication retries on the next commit.
                self.scream("the terminal re-judgment was outraced");
                return Err(Error::Contention {
                    braid,
                    cause: ContentionCause::SlotRace { tip: live.tip },
                });
            }

            if let Some(acked) = waiters.take()
                && core.ack == AckMode::Local
            {
                for waiter in acked {
                    waiter.resolve(Resolved::Judged(Admission::Accepted(Slotted {
                        value: (),
                        braid,
                        slot,
                        durability: Durability::LocalPending,
                    })));
                }
                self.step(WriterStep::AckLocal)?;
                return Ok(Settled::Detached { bytes });
            }

            match self.publish(core, braid, slot, header.timestamp, &bytes)? {
                PublishEnd::Done(settled) => return Ok(settled),
                PublishEnd::Lost { winner } => {
                    self.lose(core, braid, slot, &winner, live)?;
                }
            }
        }
    }

    /// One publication attempt: refuse a below-floor create, then
    /// `put_create`, then `Created`, the byte-equal absorption of our
    /// own ambiguous PUT, or the loss. A `put_create` failure is an
    /// ambiguous outcome — the request may have landed — so it is
    /// never retried blindly: the follow-up is a GET of the target key
    /// comparing content, and only a proven-absent create *above* the
    /// floor is reissued. An occupant that then vanishes is retired,
    /// not a loop that forges the swept slot.
    pub(crate) fn publish(
        self: &Arc<Self>,
        core: &mut Core<T>,
        braid: BraidId,
        slot: u64,
        ts: u64,
        bytes: &[u8],
    ) -> Result<PublishEnd> {
        const CREATE_ATTEMPTS: u32 = 6;
        if below_floor(core, braid, slot) {
            return Err(slot_retired());
        }
        let key = log_key(&self.prefix, braid, slot);
        let mut attempt: u32 = 0;
        let created = loop {
            match self.store.put_create(&key, bytes) {
                Ok(Create::Ambiguous) => {
                    match resolve_ambiguous_create(self.store.as_ref(), &key, bytes)
                        .map_err(|probe_err| Error::Fault(Fault::Store(probe_err)))?
                    {
                        CreateProbe::Landed(etag) => break Create::Created(etag),
                        CreateProbe::Lost(_) => break Create::Exists,
                        CreateProbe::Absent => {
                            if below_floor(core, braid, slot) {
                                return Err(slot_retired());
                            }
                            attempt += 1;
                            if attempt == CREATE_ATTEMPTS {
                                return Err(Error::Fault(Fault::Io(io::Error::other(
                                    "ambiguous create stayed absent",
                                ))));
                            }
                        }
                    }
                }
                Ok(created) => break created,
                Err(err) => {
                    match resolve_ambiguous_create(self.store.as_ref(), &key, bytes)
                        .map_err(|probe_err| Error::Fault(Fault::Store(probe_err)))?
                    {
                        CreateProbe::Landed(etag) => break Create::Created(etag),
                        CreateProbe::Lost(_) => break Create::Exists,
                        CreateProbe::Absent => {
                            if below_floor(core, braid, slot) {
                                return Err(slot_retired());
                            }
                            attempt += 1;
                            if attempt == CREATE_ATTEMPTS {
                                return Err(Error::Fault(Fault::Store(err)));
                            }
                        }
                    }
                }
            }
        };
        self.step(WriterStep::PutLog)?;
        match created {
            Create::Created(_) => {
                self.advance_and_clear(core, braid, slot, ts, bytes)?;
                Ok(PublishEnd::Done(Settled::Judged(Admission::Accepted(slot))))
            }
            Create::Ambiguous => {
                let Some(winner) = retry_read(|| self.store.get(&key))
                    .map_err(|err| Error::Fault(Fault::Store(err)))?
                else {
                    self.scream("slot vanished after create");
                    return Err(if below_floor(core, braid, slot) {
                        slot_retired()
                    } else {
                        Error::Fault(Fault::Io(io::Error::other(
                            "ambiguous create stayed absent",
                        )))
                    });
                };
                if winner.bytes == bytes {
                    self.advance_and_clear(core, braid, slot, ts, bytes)?;
                    return Ok(PublishEnd::Done(Settled::Judged(Admission::Accepted(slot))));
                }
                Ok(PublishEnd::Lost {
                    winner: winner.bytes,
                })
            }
            Create::Exists => {
                let Some(winner) = retry_read(|| self.store.get(&key))
                    .map_err(|err| Error::Fault(Fault::Store(err)))?
                else {
                    // Exists then null: the occupant was swept. Refuse
                    // rather than loop back into put_create.
                    self.scream("slot vanished after create");
                    return Err(slot_retired());
                };
                if winner.bytes == bytes {
                    self.advance_and_clear(core, braid, slot, ts, bytes)?;
                    return Ok(PublishEnd::Done(Settled::Judged(Admission::Accepted(slot))));
                }
                Ok(PublishEnd::Lost {
                    winner: winner.bytes,
                })
            }
        }
    }

    pub(crate) fn advance_and_clear(
        self: &Arc<Self>,
        core: &mut Core<T>,
        braid: BraidId,
        slot: u64,
        ts: u64,
        bytes: &[u8],
    ) -> Result<()> {
        core.chain.entries_mut().insert(
            braid,
            ChainEntry {
                g: slot,
                prev: *blake3::hash(bytes).as_bytes(),
                ts,
            },
        );
        self.step(WriterStep::ChainAdvance)?;
        core.log_bytes += bytes.len() as u64;
        // Durable Settled at the new vector — advancing *is* this write.
        self.clear_pending(core)?;
        self.maybe_duty(core);
        Ok(())
    }

    /// Fsync the batch as `Pending` before first judgment.
    fn persist_pending(&self, core: &mut Core<T>, batch: Pending) -> Result<()> {
        let entries = std::mem::take(core.chain.entries_mut());
        core.chain = Chain::Pending { entries, batch };
        core.chain
            .write_atomic(&self.dir)
            .map_err(|err| Error::Fault(Fault::Io(err)))?;
        self.step(WriterStep::PendingWrite)?;
        Ok(())
    }

    /// Write `Settled` after the verdict. Never called ahead of apply.
    pub(crate) fn clear_pending(&self, core: &mut Core<T>) -> Result<()> {
        let entries = std::mem::take(core.chain.entries_mut());
        core.chain = Chain::Settled { entries };
        core.chain
            .write_atomic(&self.dir)
            .map_err(|err| Error::Fault(Fault::Io(err)))?;
        self.step(WriterStep::PendingClear)?;
        Ok(())
    }
}

/// The published checkpoint vector is the one floor: a slot at or
/// below it is retired, and a create must not touch the store.
fn below_floor<T: Theory + Clone>(core: &Core<T>, braid: BraidId, slot: u64) -> bool {
    core.floor
        .as_ref()
        .and_then(|(_, doc)| doc.braids.get(&braid))
        .is_some_and(|head| slot <= head.g)
}

fn slot_retired() -> Error {
    Error::Fault(Fault::Io(io::Error::other("the slot is retired")))
}
