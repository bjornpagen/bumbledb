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
//! source-probe list and disestablished-guard check sets, equally
//! scan-complete.

use std::collections::BTreeMap;

use heed::types::Bytes;
use heed::{AnyTls, Database, RoTxn};

use crate::error::{CorruptionError, Error, Result};
use crate::schema::RelationId;
use crate::storage::env::WriteTxn;
use crate::storage::keys::{self, KeyBuf, MAX_KEY};

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

/// A crashpoint's expected recovery side (the crucible packet (git ecec1dc3)):
/// which committed state a store killed at that point must reopen to.
/// The boundary is `mdb_txn_commit` — LMDB's single durability point.
#[cfg(feature = "crashpoint")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrashpointSide {
    /// Death here loses the victim commit whole: recovery equals the
    /// state before it (the transaction never reached the durability
    /// boundary; nothing persists).
    Prefix,
    /// Death here keeps the victim commit whole: `mdb_txn_commit`
    /// already returned, so recovery equals the post-commit state
    /// (only in-memory bookkeeping died).
    Post,
}

/// The crashpoint table: the commit pipeline's phase boundaries, NAMED,
/// in pipeline order, each with its expected recovery side. The set of
/// crashpoints IS the claimed atomicity structure — reviewable in one
/// grep (the hook macro's call sites), and this table matches that grep
/// exactly:
///
/// | point | site | recovery |
/// |---|---|---|
/// | `after-staging` | `write.rs` `commit`: past the empty-delta gate, before plan derivation | prefix |
/// | `mid-write-m` | `applier.rs` `insert_fact`: after a fact's `M` put | prefix |
/// | `mid-write-f` | `applier.rs` `insert_fact`: after a fact's `F` put | prefix |
/// | `mid-write-u` | `applier.rs` `insert_fact`: after a `U` guard put | prefix |
/// | `mid-write-r` | `applier.rs` `insert_fact`: after an `R` edge put | prefix |
/// | `before-judgment` | `write.rs` `commit`: phases 1–2 applied, before phase 3 | prefix |
/// | `mid-write-s` | `write.rs` `flush_counters`: after an `S` row-count put (phase 4) | prefix |
/// | `after-judgment` | `write.rs` `commit`: phases 3–4 done, before `mdb_txn_commit` | prefix |
/// | `after-commit` | `write.rs` `commit`: `mdb_txn_commit` returned, before the memo update | post |
/// | `after-memo-update` | `api/db/write.rs` `write_witnessed`: after the image-cache eviction and commit-seq bump | post |
///
/// The counters-only no-op commit (`flush_escaped_fresh_ids`) is
/// deliberately outside the table: it never changes query-visible state,
/// and its crash story is the existing kill test
/// (`tests/crash.rs::kill_during_counters_only_commit_leaves_q_consistent`).
#[cfg(feature = "crashpoint")]
pub const CRASHPOINTS: &[(&str, CrashpointSide)] = &[
    ("after-staging", CrashpointSide::Prefix),
    ("mid-write-m", CrashpointSide::Prefix),
    ("mid-write-f", CrashpointSide::Prefix),
    ("mid-write-u", CrashpointSide::Prefix),
    ("mid-write-r", CrashpointSide::Prefix),
    ("before-judgment", CrashpointSide::Prefix),
    ("mid-write-s", CrashpointSide::Prefix),
    ("after-judgment", CrashpointSide::Prefix),
    ("after-commit", CrashpointSide::Post),
    ("after-memo-update", CrashpointSide::Post),
];

/// One armed-point check: aborts the process — a real unclean death, no
/// unwinding cleanup — when `BUMBLEDB_CRASHPOINT` names this point. The
/// marker line printed first is the harness's classifier (a crashpoint
/// death versus any other abort). The environment is consulted per hit,
/// never cached: the crash harness arms the variable mid-process,
/// between its ops prefix and its victim commit.
#[cfg(feature = "crashpoint")]
pub fn crashpoint_hit(name: &str) {
    if std::env::var_os("BUMBLEDB_CRASHPOINT").is_some_and(|armed| armed == *name) {
        eprintln!("crashpoint {name}: aborting");
        std::process::abort();
    }
}

// The hook macro. On (the `crashpoint` feature): consult the
// environment and abort on match. Off (the default): expands to
// NOTHING — no code, no branch, no string in the default build.
#[cfg(feature = "crashpoint")]
macro_rules! crashpoint {
    ($name:literal) => {
        $crate::storage::commit::crashpoint_hit($name)
    };
}
#[cfg(not(feature = "crashpoint"))]
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
/// storage generation (the 50-storage doc's cache-eviction subscriber; the
/// 70-api doc wires it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommitReport {
    pub changed: bool,
    pub new_generation: u64,
}

/// Working state threaded through phases 1-2: the transaction, the row-id
/// plumbing, one key scratch — no derivation state; the plan owns it —
/// and the key-violation collector (recorded conflicts, sealed into the
/// complete rejection set after phase 2).
struct Applier<'env> {
    txn: WriteTxn<'env>,
    data: heed::Database<heed::types::Bytes, heed::types::Bytes>,
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
    relation: RelationId,
    row_id: u64,
) -> Result<&'t [u8]> {
    let mut key: KeyBuf = [0; MAX_KEY];
    let f_len = keys::fact_key(&mut key, relation, row_id);
    data.get(txn, &key[..f_len])?
        .ok_or(Error::Corruption(CorruptionError::MissingFact {
            relation,
            row_id,
        }))
}
