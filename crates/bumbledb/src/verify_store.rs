//! [`Db::verify_store`] — the offline sweeper the write path defers to
//! : one read
//! snapshot, one pass per namespace, O(store). Every key derivation is
//! imported from [`crate::storage::keys`] and φ is re-checked with the
//! commit path's own selection helpers ([`judgment`]) — the sweeper's
//! knowledge is the engine's knowledge, never a second implementation.
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
//! Beyond namespace coherence, every judgment form
//! is re-verified **globally**
//! see: the incremental form was wrong once, long ago, and every commit
//! since preserved the corruption (the delta-restriction theorems'
//! missing-premise half,
//! own: duplicate scalar determinants are impossible by LMDB key uniqueness, so
//! `lean/Bumbledb/Countermodels.lean: incremental_verdict_needs_holds`).
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
    Judgment(Violation),

    Corruption(CorruptionError),
}

impl<S> Db<S> {
    /// Read-only, one LMDB snapshot, O(store) — seconds at the
    /// # Errors
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

#[derive(Default)]
struct Tally {
    rows: u64,
    max_row_id: u64,
}

struct Sweep<'a, C> {
    catalog: C,
    schema: &'a Schema,

    selections: Selections<'a>,

    dict_next_id: InternId,
    findings: Vec<StoreFinding>,

    tallies: BTreeMap<RelationId, Tally>,

    max_fresh: BTreeMap<(RelationId, FieldId), u64>,

    referenced_interns: BTreeSet<InternId>,
}

pub(super) fn namespace_bounds(tag: keys::Namespace) -> ([u8; 1], [u8; 1]) {
    let t = tag.tag();
    ([t], [t + 1])
}

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

    fn fact(&self, rel: RelationId, row_id: u64) -> Result<Option<C::Value<'_>>> {
        self.catalog.fetch_fact(rel, row_id)
    }
}
