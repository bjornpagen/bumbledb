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
//! F  facts          key/schema/width/canonical-field decode, forward checks
//!                   into M/U/R, tallies, intern references, the global
//!                   containment judgment per outgoing statement, and the
//!                   global capacity judgment per ψ-selected parent
//! M  membership     resolves back to its fact, hash-verified
//! U  FD determinants      resolves back + per-group pointwise disjointness
//! R  reverse edges  resolves back to a live source inside φ (the heart:
//!                   the one namespace with no online verification) —
//!                   containment and capacity edges alike, weight slots
//!                   backed to the live fact
//! marks             the closed-parent capacity roster
//! S  counters       row count and high-water against the F tallies
//! Q  fresh sequences the never-reissue ratchet against the F fresh
//!                   tallies (finding 033)
//! _meta descriptor  blake3 of the persisted schema descriptor against the
//!                   stored fingerprint (the self-description bond; format
//!                   8 open already required the key)
//! _dict             forward/reverse coherence, referenced-id liveness,
//!                   the next-id bound (findings 004/078) — plus the
//!                   dangling-id statistic (the accepted leak)
//! ```
//!
//! Beyond namespace coherence, every judgment form
//! (`docs/architecture/30-dependencies.md`) is re-verified **globally**
//! over the full committed state — the class no incremental check can
//! see: the incremental form was wrong once, long ago, and every commit
//! since preserved the corruption (the delta-restriction theorems'
//! missing-premise half,
//! `lean/Bumbledb/Countermodels.lean: incremental_verdict_needs_holds`).
//! **Functionality** needs no pass of its
//! own: duplicate scalar determinants are impossible by LMDB key uniqueness, so
//! the global judgment *is* the F pass's every-fact-holds-its-determinant check
//! plus the U pass's per-group disjointness walk — functionality findings
//! are namespace findings. **Containment** rides the F scan (one scan,
//! shared across every statement): each fact inside a source selection φ
//! probes the target through the commit path's own scalar probe and
//! coverage walk ([`judgment`]'s `Checker` — one definition, never a
//! sweeper copy). The U pass independently re-derives pointwise
//! disjointness from stored bytes, while the shared coverage call still
//! consumes the schema's validator-minted `DisjointDeterminantProof`; a   miss is
//! [`StoreFinding::Judgment`]. **Capacity statements** ride the
//! F scan on their parent side (every ψ-selected parent measures its
//! child group through the commit path's own walk —
//! [`StoreFinding::Judgment`]); closed parents re-check in the
//! marks pass. The weighted value slot adds the weight-desync sweep,
//! both directions (C17, measured: the slot law makes the `R` slot
//! a maintained copy of one row-local field, and this sweeper is the
//! offline authority that convicts a diverged copy): F→R, the existence
//! get's value must equal the fact's weight-field encoding (unit:
//! empty); R→F, the entry's value must back to the live fact —
//! [`CorruptionError::ReverseEdgeWeightDesync`], convict-only, never
//! repaired silently. Findings are appended in pass order, and within a
//! pass in the order the cursor raised each fact — that collection
//! discipline is the sweep; only the element shape embeds.
//!
//! Findings are data, not errors: a desynced store returns `Ok` with a
//! populated report and the *caller* decides fatality. `Err` is
//! environmental — a failed LMDB operation or an unreadable `_meta`
//! counter — never a judgment about namespace coherence.

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Bound;

use crate::Db;
use crate::encoding::InternId;
use crate::error::{CorruptionError, Result, Violation};
use crate::schema::Schema;
use crate::storage::catalog::{Bounds, CatalogMap, CatalogRead, ReadCursor};
use crate::storage::commit::judgment::{self, Selections};
use crate::storage::keys;
use bumbledb_theory::schema::{FieldId, RelationId};

mod counters;
mod determinants;
mod dict_stat;
mod facts;
mod fresh;
mod marks;
mod membership;
mod reverse;

#[cfg(test)]
mod tests;

/// The sweep's verdict: coherence, or every observed desync as a typed
/// finding, plus the informational dictionary statistic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreReport {
    pub verdict: StoreVerdict,
}

/// Empty findings are unrepresentable on the desynced arm; a coherent
/// store never carries a findings list whose emptiness is the verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreVerdict {
    Coherent {
        dangling_intern_ids: u64,
    },
    Desynced {
        findings: Box<[StoreFinding]>,
        dangling_intern_ids: u64,
    },
}

impl StoreReport {
    fn from_sweep(findings: Vec<StoreFinding>, dangling_intern_ids: u64) -> Self {
        let verdict = if findings.is_empty() {
            StoreVerdict::Coherent {
                dangling_intern_ids,
            }
        } else {
            StoreVerdict::Desynced {
                findings: findings.into_boxed_slice(),
                dangling_intern_ids,
            }
        };
        Self { verdict }
    }

    /// Every desync observed, in pass order. Empty on a coherent store.
    #[must_use]
    pub fn findings(&self) -> &[StoreFinding] {
        match &self.verdict {
            StoreVerdict::Coherent { .. } => &[],
            StoreVerdict::Desynced { findings, .. } => findings,
        }
    }

    #[must_use]
    pub fn dangling_intern_ids(&self) -> u64 {
        match self.verdict {
            StoreVerdict::Coherent {
                dangling_intern_ids,
            }
            | StoreVerdict::Desynced {
                dangling_intern_ids,
                ..
            } => dangling_intern_ids,
        }
    }
}

/// One observed desync. Structural facts are [`CorruptionError`] — the
/// sweeper found them offline; judgment facts are [`Violation`]. The
/// report preserves per-fact insertion order (pass order, then the order
/// the cursor raised each fact). Payload shapes follow the
/// [`CorruptionError`] discipline: namespace ids, [`InternId`], and
/// offending key bytes — never formatted strings, never a raw `u64`
/// intern field, never the miss sentinel as a stored id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreFinding {
    /// A containment or capacity statement globally violated by the
    /// committed state — the same [`Violation`] the commit path cites.
    /// Complete admission and the sweep cite containments
    /// source-to-target ([`crate::error::Direction::TargetRequired`]): a
    /// committed store has no just-inserted side.
    Judgment(Violation),
    /// A structural desync — the same [`CorruptionError`] the runtime
    /// raises, or a twinless corruption fact the sweeper found offline.
    Corruption(CorruptionError),
}

impl<S> Db<S> {
    /// Sweeps the store for cross-namespace desyncs — F↔M, F↔U (plus
    /// per-group pointwise disjointness), F↔R (φ re-checked with the
    /// commit path's satisfaction helper), and the `S` counters against
    /// the `F` scan — and re-verifies both judgment forms globally: the
    /// containment judgment runs per source fact inside φ through the
    /// commit path's own probe and coverage walk, and the functionality
    /// judgment is the F/U namespace checks themselves (module doc).
    /// Read-only, one LMDB snapshot, O(store) — seconds at the
    /// ≤10⁷-fact axiom; no incremental mode, no parallelism.
    ///
    /// # Errors
    ///
    /// `Lmdb` on snapshot or cursor failure and `Corruption` on an
    /// unreadable `_meta` counter — environmental failure only. Store
    /// content never errors: every observation is a finding, and a
    /// desynced store returns `Ok` with a populated report.
    #[doc(hidden)]
    pub fn verify_store(&self) -> Result<StoreReport> {
        let txn = self.env().read_txn()?;
        let catalog = txn.catalog();
        let mut checker = judgment::Checker::new(&catalog, self.schema());
        let mut sweep = Sweep {
            catalog,
            schema: self.schema(),
            selections: Selections::encode_committed(self.schema(), &txn)?,
            dict_next_id: catalog.dict_next_id()?,
            findings: Vec::new(),
            tallies: BTreeMap::new(),
            max_fresh: BTreeMap::new(),
            referenced_interns: BTreeSet::new(),
        };
        let mut store_span = crate::obs::span(crate::obs::names::VERIFY_STORE);
        // One span per namespace pass, each timed and charged the findings
        // it raised — pass granularity, the per-entry cursor stays unspanned.
        sweep.pass(crate::obs::names::VERIFY_FACTS, |s| {
            facts::sweep(s, &mut checker)
        })?;
        sweep.pass(crate::obs::names::VERIFY_MEMBERSHIP, membership::sweep)?;
        sweep.pass(crate::obs::names::VERIFY_DETERMINANTS, determinants::sweep)?;
        sweep.pass(crate::obs::names::VERIFY_REVERSE, reverse::sweep)?;
        sweep.pass(crate::obs::names::VERIFY_MARKS, |s| {
            marks::sweep(s, &mut checker)
        })?;
        sweep.pass(crate::obs::names::VERIFY_COUNTERS, counters::sweep)?;
        sweep.pass(crate::obs::names::VERIFY_FRESH, fresh::sweep)?;
        let dangling_intern_ids = {
            let mut span = crate::obs::span(crate::obs::names::VERIFY_DICT);
            let before = sweep.findings.len();
            let dangling = dict_stat::dangling(&mut sweep)?;
            span.set_count((sweep.findings.len() - before) as u64);
            dangling
        };
        store_span.set_count(sweep.findings.len() as u64);
        store_span.end();
        Ok(StoreReport::from_sweep(sweep.findings, dangling_intern_ids))
    }
}

/// One relation's `F`-scan tally (`max_row_id` is meaningful only with
/// `rows > 0`; a relation with no facts never enters the map).
#[derive(Default)]
struct Tally {
    rows: u64,
    max_row_id: u64,
}

/// Working state threaded through the passes: the catalog, the schema,
/// the committed-encoded selections, and the `F`-scan tallies the counter
/// and dictionary passes reconcile.
///
/// `C: Copy` so a namespace cursor can live on one handle while point-gets
/// use another — both views of the same snapshot.
struct Sweep<'a, C> {
    catalog: C,
    schema: &'a Schema,
    /// Every containment statement's φ/ψ literals, encoded once against
    /// the committed dictionary ([`Selections::encode_committed`]).
    selections: Selections<'a>,
    /// The `_meta` dictionary next-id: every referenced intern id must
    /// sit below it. Never the miss sentinel — a stored next-id of
    /// [`InternId::SENTINEL`] would be a `_meta` decode fact, not this field.
    dict_next_id: InternId,
    findings: Vec<StoreFinding>,
    /// Per-relation `F`-scan tallies, filled by the `F` pass.
    tallies: BTreeMap<RelationId, Tally>,
    /// Per fresh field, the largest committed value the `F` scan saw —
    /// the `Q` pass's ratchet-law input (finding 033).
    max_fresh: BTreeMap<(RelationId, FieldId), u64>,
    /// Every intern id referenced by a live fact's String fields —
    /// the dictionary pass's liveness set. The miss sentinel is not a
    /// stored id and never enters this set.
    referenced_interns: BTreeSet<InternId>,
}

/// `[tag, tag + 1)` bounds for one `_data` namespace.
pub(super) fn namespace_bounds(tag: keys::Namespace) -> ([u8; 1], [u8; 1]) {
    let t = tag.tag();
    ([t], [t + 1])
}

/// Walks one `_data` namespace. `catalog` is a Copy handle so `visit` may
/// issue point-gets on a different copy of the same snapshot.
fn for_namespace<C: CatalogRead + Copy>(
    catalog: C,
    tag: keys::Namespace,
    mut visit: impl FnMut(&[u8], &[u8]) -> Result<()>,
) -> Result<()> {
    let (lo, hi) = namespace_bounds(tag);
    let mut range = catalog.range(
        CatalogMap::Data,
        Bounds {
            start: Bound::Included(&lo),
            end: Bound::Excluded(&hi),
        },
    )?;
    while let Some(entry) = ReadCursor::next(&mut range)? {
        visit(entry.key, entry.value)?;
    }
    Ok(())
}

impl<C: CatalogRead + Copy> Sweep<'_, C> {
    fn push(&mut self, finding: StoreFinding) {
        self.findings.push(finding);
    }

    fn corrupt(&mut self, err: CorruptionError) {
        self.push(StoreFinding::Corruption(err));
    }

    /// Runs one namespace sweep inside its own `obs` span, charging the
    /// span `a0` the findings the pass raised (pass granularity — the
    /// per-entry cursor inside `f` is never spanned). Inert when the
    /// `trace` feature is off.
    fn pass(
        &mut self,
        point: crate::obs::TracePoint,
        f: impl FnOnce(&mut Self) -> Result<()>,
    ) -> Result<()> {
        let before = self.findings.len();
        let mut span = crate::obs::span(point);
        f(self)?;
        span.set_count((self.findings.len() - before) as u64);
        Ok(())
    }

    fn malformed(&mut self, key: &[u8], what: &'static str) {
        self.corrupt(CorruptionError::Malformed {
            key: key.into(),
            what,
        });
    }

    /// `F` point-get by (relation, row id). `None` is the caller's finding
    /// to make — the sweeper reports, never errors on content.
    fn fact(&self, rel: RelationId, row_id: u64) -> Result<Option<C::Value<'_>>> {
        self.catalog.fetch_fact(rel, row_id)
    }
}
