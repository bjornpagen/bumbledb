//! Apply: the one place a fetched log object becomes engine state. The
//! home of the refusal battery — decode (full parse before any apply),
//! the chain discipline with its three proved causes, the footprint
//! recompute — and of the first-applied-slot-must-change-state
//! instrument. Apply is idempotent by set semantics (L10): re-applying a
//! batch whose effects are present net-disposes every op, the engine
//! takes its no-op arm, and the generation does not advance, which is
//! why the crash window between an engine commit and its sidecar bump
//! needs no detection state at all.

use bumbledb::{Admission, Db, Violations};

use crate::braids::BraidId;
use crate::codec::{Codec, DecodeError, OpKind};
use crate::footprint::{FootprintError, footprint};
use crate::sidecar::{Chain, ChainEntry};

/// The three proved causes of `ChainMismatch` — one identity, each arm
/// carrying the two values whose disagreement convicts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainCause {
    /// The header's slot identity disagrees with the key the object was
    /// fetched from (braid or generation).
    Slot {
        header_braid: BraidId,
        header_gen: u64,
    },
    /// The header's backlink disagrees with the chain's head hash.
    Prev {
        header_prev: [u8; 32],
        chain_prev: [u8; 32],
    },
    /// The header's timestamp undercuts the chain's head timestamp.
    Timestamp { header_ts: u64, chain_ts: u64 },
}

/// Why the recomputed footprint could not agree with the carried one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FootprintCause {
    /// Recomputation succeeded and produced a different section.
    Diverged,
    /// Recomputation itself refused — our encoder could never have
    /// produced this batch.
    Unrecomputable(FootprintError),
}

/// The apply-time refusal battery. Every arm is corruption-class: the
/// object itself, or the chain it claims, is wrong, and no retry mends
/// bytes. Arms carrying `writer` name the misbehaving publisher from the
/// header it signed into the batch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyRefusal {
    /// The full parse refused: version, flags, fingerprint, layout,
    /// footprint order and dedup, op relation outside the braid.
    Decode(DecodeError),
    ChainMismatch {
        cause: ChainCause,
        braid: BraidId,
        slot: u64,
        writer: u64,
    },
    FootprintMismatch {
        cause: FootprintCause,
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
/// chain discipline, footprint recompute, one `db.write` with ops in
/// listed order, then the state-change instrument. `applied_pending` is
/// the wholeness identity's last term (1 exactly when a pending batch is
/// applied but unpublished, 0 otherwise). On `Advanced`/`Absorbed` the
/// in-memory chain has advanced; persisting it is the caller's step two.
pub fn apply<T>(
    db: &Db<T>,
    chain: &mut Chain,
    codec: &Codec,
    braid: BraidId,
    slot: u64,
    bytes: &[u8],
    applied_pending: u64,
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
            header_gen: header.braid_gen,
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

    let footprint_refusal = |cause: FootprintCause| {
        Applied::Refused(ApplyRefusal::FootprintMismatch {
            cause,
            braid,
            slot,
            writer: header.writer,
        })
    };
    match footprint(codec.vocabulary(), &batch.ops) {
        Ok(recomputed) if recomputed == batch.footprint => {}
        Ok(_) => return Ok(footprint_refusal(FootprintCause::Diverged)),
        Err(error) => {
            return Ok(footprint_refusal(FootprintCause::Unrecomputable(error)));
        }
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

    let identity = chain.sum() - position.g + slot + applied_pending;
    if generation < identity {
        return Ok(Applied::Refused(ApplyRefusal::PublishLawViolation {
            braid,
            slot,
            writer: header.writer,
            generation,
            identity,
        }));
    }

    chain.entries.insert(
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
