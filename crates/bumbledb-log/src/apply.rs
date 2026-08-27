//! Apply: the one place a fetched log object becomes engine state. The
//! home of the refusal battery — decode (full parse before any apply)
//! and the chain discipline with its three proved causes — and of the
//! first-applied-slot-must-change-state instrument. Apply is idempotent
//! by set semantics (L10): re-applying a batch whose effects are present
//! net-disposes every op, the engine takes its no-op arm, and the
//! generation does not advance, which is why the crash window between an
//! engine commit and its sidecar bump needs no detection state at all.

use bumbledb::{Admission, Db, Violations};

use crate::braids::BraidId;
use crate::codec::{Codec, DecodeError, OpKind};
use crate::sidecar::{Chain, ChainEntry};

/// Classification of a pending batch against the occupant and the
/// generation the store shows. One fold: publisher, fallback, and
/// open-recovery match the same arms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingFold {
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

/// Re-judges a pending batch against the winner-current occupant and
/// the generation the store shows. Remaining work is data: the arm
/// names what is left to publish, clear, or discard.
#[must_use]
pub fn fold_pending(
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

/// The three proved causes of `ChainMismatch` — one identity, each arm
/// carrying the two values whose disagreement convicts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainCause {
    /// The header's slot identity disagrees with the key the object was
    /// fetched from (braid or slot).
    Slot {
        header_braid: BraidId,
        header_slot: u64,
    },
    /// The header's backlink disagrees with the chain's head hash.
    Prev {
        header_prev: [u8; 32],
        chain_prev: [u8; 32],
    },
    /// The header's timestamp undercuts the chain's head timestamp.
    Timestamp { header_ts: u64, chain_ts: u64 },
}

/// The apply-time refusal battery. Every arm is corruption-class: the
/// object itself, or the chain it claims, is wrong, and no retry mends
/// bytes. Arms carrying `writer` name the misbehaving publisher from the
/// header it signed into the batch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyRefusal {
    /// The full parse refused: version, flags, fingerprint, layout, op
    /// relation outside the braid, trailing bytes.
    Decode(DecodeError),
    ChainMismatch {
        cause: ChainCause,
        braid: BraidId,
        slot: u64,
        writer: u64,
    },
    /// A first-applied slot that changed nothing while the store sat
    /// below the identity — a publish-law violation in the log.
    PublishLawViolation {
        braid: BraidId,
        slot: u64,
        writer: u64,
        generation: u64,
        identity: u64,
    },
}

/// What one apply did. `Advanced` and `Absorbed` both advance the chain
/// to `(slot, blake3(bytes), header.ts)`; the caller persists the
/// sidecar. `Rejected` is the engine's verdict as data — the replica
/// maps it to a discard (open phase) or `ReplayDiverged` (a store that
/// has proven itself whole).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Applied {
    /// The engine commit advanced the generation — the ordinary arm.
    Advanced {
        generation: u64,
    },
    /// The engine took its no-op arm and the identity landed exact: the
    /// legitimate crash-window re-absorption.
    Absorbed {
        generation: u64,
    },
    Rejected(Violations),
    Refused(ApplyRefusal),
}

/// Applies the log object at `(braid, slot)` to the store: full decode,
/// chain discipline, one `db.write` with ops in listed order, then the
/// state-change instrument. The identity is the vector sum with this
/// braid's count replaced by `slot`. `Pending` already counts one in
/// `generation(chain)` — that extra is the store the engine shows, not
/// a second addend here. On `Advanced`/`Absorbed` the in-memory chain
/// has advanced;
/// persisting it is the caller's step two.
///
/// # Errors
pub fn apply<T>(
    db: &Db<T>,
    chain: &mut Chain,
    codec: &Codec,
    braid: BraidId,
    slot: u64,
    bytes: &[u8],
) -> bumbledb::Result<Applied> {
    let batch = match codec.decode(bytes) {
        Ok(batch) => batch,
        Err(error) => return Ok(Applied::Refused(ApplyRefusal::Decode(error))),
    };
    let header = batch.header;
    let refuse = |cause: ChainCause| {
        Applied::Refused(ApplyRefusal::ChainMismatch {
            cause,
            braid,
            slot,
            writer: header.writer,
        })
    };
    if header.braid != braid || header.braid_gen != slot {
        return Ok(refuse(ChainCause::Slot {
            header_braid: header.braid,
            header_slot: header.braid_gen,
        }));
    }
    let position = chain.position(braid);
    if header.prev != position.prev {
        return Ok(refuse(ChainCause::Prev {
            header_prev: header.prev,
            chain_prev: position.prev,
        }));
    }
    if header.timestamp < position.ts {
        return Ok(refuse(ChainCause::Timestamp {
            header_ts: header.timestamp,
            chain_ts: position.ts,
        }));
    }

    let before = db.generation()?.value();
    let committed = match db.write(|tx| {
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
    })? {
        Admission::Accepted(committed) => committed,
        Admission::Rejected(violations) => return Ok(Applied::Rejected(violations)),
    };
    let generation = committed.generation.value();

    let identity = chain.sum() - position.g + slot;
    if generation < identity {
        return Ok(Applied::Refused(ApplyRefusal::PublishLawViolation {
            braid,
            slot,
            writer: header.writer,
            generation,
            identity,
        }));
    }

    chain.entries_mut().insert(
        braid,
        ChainEntry {
            g: slot,
            prev: *blake3::hash(bytes).as_bytes(),
            ts: header.timestamp,
        },
    );
    if generation == before {
        Ok(Applied::Absorbed { generation })
    } else {
        Ok(Applied::Advanced { generation })
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
