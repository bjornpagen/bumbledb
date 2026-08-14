//! Commit application (`docs/architecture/50-storage.md` § Write path):
//! the commit's bookkeeping is computed first as a value — the
//! [`plan::CommitPlan`], a pure function of (delta, schema) — then phases
//! 1-2 execute it in canonical order (all deletes, then all inserts),
//! maintaining `F`/`M`/`U`/`R` and enforcing every `Functionality`
//! statement: scalar keys by `U` put-conflict, pointwise keys by the
//! ordered-neighbor probe (`docs/architecture/30-dependencies.md`
//! § pointwise lifting).
//!
//! Because every delete lands before any insert and the insert set is
//! deduplicated by construction, a `U` conflict during inserts is a genuine
//! functionality violation; user operation order inside the transaction is
//! semantically irrelevant. A conflict is *recorded*, not thrown: phase 2
//! completes scan-complete and the commit rejects with the COMPLETE set of
//! violated key statements ([`crate::error::Violations`]) — and key
//! violations preempt phase 3, because the containment probes are defined
//! over the keyed final state. Phase 3 — the judgment phase (`judgment`) —
//! proves every containment against the final state, consuming the plan's
//! source-probe list and disestablished-determinant check sets, equally
//! scan-complete.

use std::collections::BTreeMap;

use heed::types::Bytes;
use heed::{AnyTls, Database, RoTxn};

use crate::error::{CorruptionError, Error, Result};
use crate::schema::Schema;
use crate::storage::env::{GenerationId, WriteTxn};
use crate::storage::keys::{self, KeyBuf, MAX_KEY};
use crate::storage::read::check_width;
use bumbledb_theory::schema::RelationId;

mod applier;
mod apply;
// The selection machinery (`judgment::Selections`, `judgment::satisfies`)
// is shared with `Db::verify_store` — the sweeper re-checks φ with the
// commit path's own helper, never a second implementation.
pub(crate) mod judgment;
mod plan;
mod write;

#[cfg(test)]
mod tests;

pub use apply::apply;
pub use write::commit;
pub(crate) use write::{flush_escaped_fresh_ids, flush_pending_escaped_fresh_ids};

/// Named commit-pipeline markers. The former crashpoint harness died with
/// the fuzzer; the sites remain as documentation of the atomicity
/// structure and expand to nothing.
macro_rules! crashpoint {
    ($name:literal) => {};
}
pub(crate) use crashpoint;

/// The applied-but-uncommitted state after phases 1-2: the open LMDB
/// write transaction plus the one thing the executor alone can know —
/// the row ids it minted. Everything else the later phases consume lives
/// in the [`plan::CommitPlan`].
pub struct Applied<'env> {
    /// The open, uncommitted LMDB write transaction.
    pub txn: WriteTxn<'env>,
    /// Per-relation next row id after this apply (flushed to `S` by the
    /// 50-storage doc's phase 4).
    pub row_id_next: BTreeMap<RelationId, u64>,
}

/// The commit outcome: whether logical state changed, and the resulting
/// storage generation (the 50-storage doc's cache-advance subscriber; the
/// 70-api doc wires it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitReport {
    /// Counters-only / empty delta: the generation clock did not move.
    Noop { generation: GenerationId },
    /// Logical state changed: images keyed on this generation.
    Changed { new_generation: GenerationId },
}

impl CommitReport {
    /// Whether this commit advanced the generation clock.
    #[must_use]
    pub const fn changed(self) -> bool {
        matches!(self, Self::Changed { .. })
    }

    /// Generation after this commit — unchanged on [`Self::Noop`].
    #[must_use]
    pub const fn generation(self) -> GenerationId {
        match self {
            Self::Noop { generation } => generation,
            Self::Changed { new_generation } => new_generation,
        }
    }
}

/// Working state threaded through phases 1-2: the transaction, the row-id
/// plumbing, one key scratch — no derivation state; the plan owns it —
/// and the key-violation collector (recorded conflicts, sealed into the
/// complete rejection set after phase 2).
struct Applier<'env, 's> {
    txn: WriteTxn<'env>,
    data: heed::Database<heed::types::Bytes, heed::types::Bytes>,
    schema: &'s Schema,
    row_id_next: BTreeMap<RelationId, u64>,
    key: KeyBuf,
    violations: Vec<crate::error::Violation>,
}

/// Decodes one stored `M`/`U` row-id value (applier and judgment share
/// the one decoder).
fn decode_row_id(bytes: &[u8]) -> Result<u64> {
    crate::storage::stored_u64(bytes, "M row id")
}

/// Fetches a fact's canonical bytes by row id, borrowed from the
/// transaction — the one `F` get behind the applier's violation payloads
/// and the judgment's probe subjects. Every caller resolved the row id
/// inside this same transaction's view, so a miss is corruption, never a
/// race. Own scratch: callers' key buffers stay untouched.
fn fact_by_row<'t>(
    data: Database<Bytes, Bytes>,
    txn: &'t RoTxn<'_, AnyTls>,
    schema: &Schema,
    relation: RelationId,
    row_id: u64,
) -> Result<&'t [u8]> {
    let mut key: KeyBuf = [0; MAX_KEY];
    let f_len = keys::fact_key(&mut key, relation, row_id);
    let bytes =
        data.get(txn, &key[..f_len])?
            .ok_or(Error::Corruption(CorruptionError::MissingFact {
                relation,
                row_id,
            }))?;
    check_width(schema, relation, row_id, bytes)?;
    Ok(bytes)
}
