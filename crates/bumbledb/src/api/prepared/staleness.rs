//! The plan-staleness signal (docs/architecture/70-api.md): pull-based,
//! engine-policy-free. Plans pin the statistics read at prepare and are
//! never invalidated (docs/architecture/20-query-ir.md, § prepared
//! queries) — generational rebinding keeps stale plans *correct*; only
//! optimality drifts. This module is the compensating control: the host
//! asks how far a snapshot's live `S` counters have drifted from the
//! pinned ones and owns every decision about what to do. The engine
//! never calls it and holds no thresholds.

use super::PreparedQuery;
use crate::api::db::Snapshot;
use crate::error::Result;
use crate::ir::normalize::OccId;
use crate::schema::RelationId;
use crate::storage::read;

/// One occurrence's pinned prepare-time statistics — the pin record
/// [`PreparedQuery::staleness`] compares and the stats surface renders
/// ("estimated from (pinned rows at prepare)"). Cold data: written once
/// at build, read only by the diagnostic surfaces, never by execution.
/// Participating occurrences only — negated and chase-eliminated
/// occurrences enter no DP state and earn no statistics read at prepare
/// (`build.rs`), so they carry no pin: absence is the honest record.
#[derive(Debug, Clone, Copy)]
pub(super) struct OccurrencePin {
    pub occ_id: OccId,
    pub relation: RelationId,
    /// The `S`-counter row count the plan was costed with.
    pub rows: u64,
    /// The filtered view's survivor count as measured at prepare, where
    /// the occurrence carries filters: the selectivity ladder's
    /// post-filter estimate — exact where a resident image was measured,
    /// documented bounds and floors otherwise (`plan/selectivity.rs`).
    /// `None` = unfiltered (the view is the whole relation).
    pub survivors: Option<u64>,
}

/// One occurrence's plan drift: the row count the plan was costed with
/// against the snapshot's live `S` counter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OccurrenceDrift {
    /// The occurrence's relation.
    pub relation: RelationId,
    /// The row count pinned at prepare.
    pub pinned: u64,
    /// The snapshot's live row count.
    pub live: u64,
    /// `max(live, pinned) / max(1, min(live, pinned))` — shrink and
    /// growth both read as drift ≥ 1 (both counts zero is no drift: 1).
    pub ratio: f64,
}

/// The plan-drift report [`PreparedQuery::staleness`] returns: one
/// [`OccurrenceDrift`] per participating occurrence, plus the worst
/// ratio for hosts that want one number.
#[derive(Debug, Clone, PartialEq)]
pub struct Staleness {
    /// Per participating occurrence, in occurrence-id order.
    pub per_occurrence: Box<[OccurrenceDrift]>,
    /// The worst per-occurrence ratio — 1.0 when nothing drifted, or
    /// when nothing was pinned (a guard probe reads no statistics).
    pub max_ratio: f64,
}

impl PreparedQuery<'_> {
    /// How far the snapshot's live row counts have drifted from the
    /// statistics this plan was costed with — the pull-based staleness
    /// signal, the pin-at-prepare decision's compensating control
    /// (docs/architecture/20-query-ir.md, § prepared queries). One O(1)
    /// `S`-counter get per participating occurrence (≤ 20 by the roster
    /// cap). Each ratio is `max(live, pinned) / max(1, min(live,
    /// pinned))`, so shrink and growth both read as drift ≥ 1.
    ///
    /// The engine never calls this and no threshold exists in code —
    /// the host owns policy (docs/architecture/00-product.md). As a
    /// convention, not a contract: re-prepare at `max_ratio >= 4.0`
    /// (the worst measured est/actual on a fresh plan is 3.3×, so 4×
    /// separates plan drift from estimation noise).
    ///
    /// This call allocates (the per-occurrence report) and is a
    /// diagnostic surface, not a warm-path call — keep it outside
    /// measured windows; the zero-allocation contract is `execute`'s.
    ///
    /// # Errors
    ///
    /// [`crate::error::Error::ForeignPreparedQuery`] on a snapshot of
    /// any environment other than the preparing one — the same guard,
    /// same error as every execution entry; `Lmdb`/`Corruption` from
    /// the counter gets.
    pub fn staleness(&self, snap: &Snapshot<'_>) -> Result<Staleness> {
        self.check_snapshot(snap.txn())?;
        let per_occurrence = self
            .pinned
            .iter()
            .map(|pin| {
                let live = read::row_count(snap.txn(), pin.relation)?;
                Ok(OccurrenceDrift {
                    relation: pin.relation,
                    pinned: pin.rows,
                    live,
                    ratio: drift_ratio(pin.rows, live),
                })
            })
            .collect::<Result<Box<[OccurrenceDrift]>>>()?;
        let max_ratio = per_occurrence.iter().map(|d| d.ratio).fold(1.0, f64::max);
        Ok(Staleness {
            per_occurrence,
            max_ratio,
        })
    }

    /// The stats surface's rendering of the pin record
    /// ([`crate::api::stats::ExecutionStats::pinned`]): per
    /// participating occurrence, the statistics every node estimate
    /// derives from, with the relation name resolved.
    pub(super) fn pinned_rows(&self) -> Vec<crate::api::stats::PinnedRows> {
        self.pinned
            .iter()
            .map(|pin| crate::api::stats::PinnedRows {
                occurrence: pin.occ_id.0,
                relation: self.schema.relation(pin.relation).name().to_owned(),
                rows: pin.rows,
                survivors: pin.survivors,
            })
            .collect()
    }
}

/// The drift-ratio convention: `max(live, pinned) / max(1, min(live,
/// pinned))`, with the numerator also floored at 1 so two zero counts
/// (prepared empty, still empty) read as no drift, not zero.
fn drift_ratio(pinned: u64, live: u64) -> f64 {
    #[allow(clippy::cast_precision_loss)] // row counts sit far below 2^52
    {
        (pinned.max(live).max(1) as f64) / (pinned.min(live).max(1) as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::drift_ratio;

    #[test]
    fn the_ratio_convention_reads_shrink_and_growth_symmetrically() {
        assert!((drift_ratio(100, 400) - 4.0).abs() < f64::EPSILON, "growth");
        assert!((drift_ratio(400, 100) - 4.0).abs() < f64::EPSILON, "shrink");
        assert!((drift_ratio(7, 7) - 1.0).abs() < f64::EPSILON, "no drift");
        assert!(
            (drift_ratio(0, 5) - 5.0).abs() < f64::EPSILON,
            "prepared empty, grew"
        );
        assert!(
            (drift_ratio(5, 0) - 5.0).abs() < f64::EPSILON,
            "emptied since prepare"
        );
        assert!(
            (drift_ratio(0, 0) - 1.0).abs() < f64::EPSILON,
            "empty then, empty now: no drift"
        );
    }
}
