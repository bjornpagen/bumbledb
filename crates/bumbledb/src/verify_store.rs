//! [`Db::verify_store`] — the offline sweeper the write path defers to
//! (`docs/architecture/50-storage.md` § R-delete verification): one read
//! snapshot, one pass per namespace, O(store). Every key derivation is
//! imported from [`crate::storage::keys`] and φ is re-checked with the
//! commit path's own selection helpers ([`judgment`]) — the sweeper's
//! knowledge is the engine's knowledge, never a second implementation.
//!
//! The passes mirror the key-layout table of `50-storage.md`:
//!
//! ```text
//! F  facts          forward checks into M/U/R, tallies, intern references
//! M  membership     resolves back to its fact, hash-verified
//! U  FD guards      resolves back + per-group pointwise disjointness
//! R  reverse edges  resolves back to a live source inside φ (the heart:
//!                   the one namespace with no online verification)
//! S  counters       row count and high-water against the F tallies
//! _dict             dangling-id statistic (the accepted leak)
//! ```
//!
//! Findings are data, not errors: a desynced store returns `Ok` with a
//! populated report and the *caller* decides fatality. `Err` is
//! environmental — a failed LMDB operation or an unreadable `_meta`
//! counter — never a judgment about namespace coherence.

use std::collections::{BTreeMap, BTreeSet};

use crate::error::Result;
use crate::schema::{RelationId, Schema, StatementId};
use crate::storage::commit::judgment::Selections;
use crate::storage::env::ReadTxn;
use crate::storage::keys;
use crate::Db;

mod counters;
mod dict_stat;
mod facts;
mod guards;
mod membership;
mod reverse;

#[cfg(test)]
mod tests;

/// The sweep's verdict: every observed desync as a typed finding, plus the
/// informational dictionary statistic. Empty `findings` = coherent store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreReport {
    /// Every desync observed, in pass order (F, M, U, R, S).
    pub findings: Vec<StoreFinding>,
    /// `_dict` reverse entries referenced by no live fact — the accepted
    /// leak (`docs/architecture/50-storage.md`): an informational
    /// statistic, never a finding.
    pub dangling_intern_ids: u64,
}

/// One observed desync. Payloads follow the [`CorruptionError`] discipline:
/// namespace ids and offending key bytes, never formatted strings.
///
/// [`CorruptionError`]: crate::error::CorruptionError
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreFinding {
    /// A live `F` fact whose `M` entry is absent or names another row.
    FactWithoutMembership {
        relation: RelationId,
        row_id: u64,
        membership_key: Box<[u8]>,
    },
    /// An `M` entry whose row id resolves to no `F` fact hashing back to
    /// its key.
    MembershipWithoutFact {
        relation: RelationId,
        row_id: u64,
        membership_key: Box<[u8]>,
    },
    /// A live `F` fact whose guard tuple is absent from `U` under a key
    /// statement (or held there by another row).
    FactWithoutGuard {
        relation: RelationId,
        statement: StatementId,
        row_id: u64,
        guard_key: Box<[u8]>,
    },
    /// A `U` entry whose row id resolves to no live fact re-deriving the
    /// same guard bytes.
    GuardWithoutFact {
        relation: RelationId,
        statement: StatementId,
        guard_key: Box<[u8]>,
    },
    /// Two successive guard entries of one scalar-prefix group with
    /// overlapping intervals — the pointwise-key invariant the neighbor
    /// probe assumes but never re-checks globally.
    PointwiseOverlap {
        relation: RelationId,
        statement: StatementId,
        first: Box<[u8]>,
        second: Box<[u8]>,
    },
    /// A live source fact inside φ whose `R` edge is absent — the class
    /// the commit path deletes blind (`docs/architecture/50-storage.md`
    /// § R-delete verification).
    FactWithoutReverseEdge {
        statement: StatementId,
        relation: RelationId,
        row_id: u64,
        reverse_key: Box<[u8]>,
    },
    /// An `R` edge that resolves to no live source fact still inside φ
    /// re-deriving the same key bytes.
    ReverseEdgeWithoutFact {
        statement: StatementId,
        reverse_key: Box<[u8]>,
    },
    /// The stored `S` row count disagrees with the `F`-scan cardinality.
    RowCountDesync {
        relation: RelationId,
        stored: u64,
        counted: u64,
    },
    /// The stored `S` row-id high-water (the next id to assign) does not
    /// exceed an observed row id.
    RowIdHighWaterLow {
        relation: RelationId,
        stored: u64,
        max_row_id: u64,
    },
    /// A fact references an intern id at or beyond the `_meta` dictionary
    /// next-id counter.
    InternBeyondNextId {
        relation: RelationId,
        row_id: u64,
        intern_id: u64,
        next_id: u64,
    },
    /// An entry that does not parse under the schema; the static string
    /// names the failing shape, [`CorruptionError::MalformedValue`]-style.
    ///
    /// [`CorruptionError::MalformedValue`]: crate::error::CorruptionError::MalformedValue
    Malformed { key: Box<[u8]>, what: &'static str },
}

impl Db<'_> {
    /// Sweeps the store for cross-namespace desyncs: F↔M, F↔U (plus
    /// per-group pointwise disjointness), F↔R (φ re-checked with the
    /// commit path's satisfaction helper), and the `S` counters against
    /// the `F` scan. Read-only, one LMDB snapshot, O(store) — seconds at
    /// the ≤10⁷-fact axiom; no incremental mode, no parallelism.
    ///
    /// # Errors
    ///
    /// `Lmdb` on snapshot or cursor failure and `Corruption` on an
    /// unreadable `_meta` counter — environmental failure only. Store
    /// content never errors: every observation is a finding, and a
    /// desynced store returns `Ok` with a populated report.
    pub fn verify_store(&self) -> Result<StoreReport> {
        let txn = self.env().read_txn()?;
        let mut sweep = Sweep {
            data: self.env().data(),
            txn: &txn,
            schema: self.schema(),
            selections: Selections::encode_committed(self.schema(), &txn)?,
            dict_next_id: txn.dict_next_id()?,
            findings: Vec::new(),
            tallies: BTreeMap::new(),
            referenced_interns: BTreeSet::new(),
        };
        facts::sweep(&mut sweep)?;
        membership::sweep(&mut sweep)?;
        guards::sweep(&mut sweep)?;
        reverse::sweep(&mut sweep)?;
        counters::sweep(&mut sweep)?;
        let dangling_intern_ids = dict_stat::dangling(&mut sweep)?;
        Ok(StoreReport {
            findings: sweep.findings,
            dangling_intern_ids,
        })
    }
}

/// One relation's `F`-scan tally (`max_row_id` is meaningful only with
/// `rows > 0`; a relation with no facts never enters the map).
#[derive(Default)]
struct Tally {
    rows: u64,
    max_row_id: u64,
}

/// Working state threaded through the passes: the snapshot, the schema,
/// the committed-encoded selections, and the `F`-scan tallies the counter
/// and dictionary passes reconcile.
struct Sweep<'a, 'env> {
    data: heed::Database<heed::types::Bytes, heed::types::Bytes>,
    txn: &'a ReadTxn<'env>,
    schema: &'a Schema,
    /// Every containment statement's φ/ψ literals, encoded once against
    /// the committed dictionary ([`Selections::encode_committed`]).
    selections: Selections,
    /// The `_meta` dictionary next-id: every referenced intern id must
    /// sit below it.
    dict_next_id: u64,
    findings: Vec<StoreFinding>,
    /// Per-relation `F`-scan tallies, filled by the `F` pass.
    tallies: BTreeMap<RelationId, Tally>,
    /// Every intern id referenced by a live fact's String/Bytes fields —
    /// the dictionary pass's liveness set.
    referenced_interns: BTreeSet<u64>,
}

impl<'a> Sweep<'a, '_> {
    fn push(&mut self, finding: StoreFinding) {
        self.findings.push(finding);
    }

    fn malformed(&mut self, key: &[u8], what: &'static str) {
        self.push(StoreFinding::Malformed {
            key: key.into(),
            what,
        });
    }

    /// `F` point-get by (relation, row id), borrowed for the snapshot's
    /// lifetime. `None` is the caller's finding to make — the sweeper
    /// reports, never errors on content.
    fn fact(&self, rel: RelationId, row_id: u64) -> Result<Option<&'a [u8]>> {
        let txn = self.txn;
        let mut key = [0u8; keys::FACT_KEY_LEN];
        let len = keys::fact_key(&mut key, rel, row_id);
        Ok(self.data.get(txn.raw(), &key[..len])?)
    }
}
