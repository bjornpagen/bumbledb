//! Visit, capacity-owner and index-roster counters.
//!
//! Counts come from visitor returns and [`WorkContext::used`] snapshots —
//! not default-build atomics on every tuple (L01/L03). Timing cells must
//! not enable per-row atomics; `obs` alloc windows stay a separate pass.

use bumbledb::schema::{
    CompiledTheory, DistinctnessWitness, ProjectionId, VisitControl, VisitOutcome,
};
use bumbledb::work::{Resource, WorkContext};

/// One compiled access path as the scorecard's index roster row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RosterEntry {
    pub projection: ProjectionId,
    pub relation: u16,
    pub scalar_fields: usize,
    pub routing_bytes: usize,
    pub interval_tail_bytes: u8,
    pub complete_key_bytes: usize,
    pub encoding: &'static str,
}

/// Snapshot of charged owners after a cell. Logical native charges, not RSS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OwnerSnapshot {
    pub input_bytes: u64,
    pub working_bytes: u64,
    pub scratch_bytes: u64,
    pub result_bytes: u64,
    pub rows: u64,
    pub work_units: u64,
}

impl OwnerSnapshot {
    #[must_use]
    pub fn from_work(work: &WorkContext) -> Self {
        Self {
            input_bytes: work.used(Resource::InputBytes),
            working_bytes: work.used(Resource::WorkingBytes),
            scratch_bytes: work.used(Resource::ScratchBytes),
            result_bytes: work.used(Resource::ResultBytes),
            rows: work.used(Resource::Rows),
            work_units: work.used(Resource::WorkUnits),
        }
    }
}

/// Source/group visits recorded by the visitor, plus the roster and owners.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Instrument {
    pub source_visits: u64,
    pub group_visits: u64,
    pub owners: OwnerSnapshot,
    pub roster: Vec<RosterEntry>,
}

impl Instrument {
    #[must_use]
    pub fn roster_of(theory: &CompiledTheory, relation: bumbledb::RelationId) -> Vec<RosterEntry> {
        theory
            .projections_of_relation(relation)
            .iter()
            .filter_map(|&id| {
                let projection = theory.projection(id)?;
                Some(RosterEntry {
                    projection: id,
                    relation: relation.0,
                    scalar_fields: projection.scalar_fields.len(),
                    routing_bytes: projection.encoding.routing_width(),
                    interval_tail_bytes: projection.interval_tail_width,
                    complete_key_bytes: projection.complete_key_width(),
                    encoding: match projection.encoding {
                        bumbledb::schema::KeyEncoding::ExactBounded { .. } => "exact-bounded",
                        bumbledb::schema::KeyEncoding::FingerprintBucket => "fingerprint",
                    },
                })
            })
            .collect()
    }
}

/// Count visits through the compiled existence-only / full-walk contract.
/// The visitor is the production counter — no extra atomic on `T`.
///
/// # Errors
/// The visitor's error.
pub fn count_visits<T, E>(
    witness: DistinctnessWitness,
    candidates: impl IntoIterator<Item = T>,
    mut on_item: impl FnMut(&T) -> Result<VisitControl, E>,
) -> Result<(VisitOutcome, u64), E> {
    let mut counted = 0u64;
    let outcome = CompiledTheory::consume_visits(witness, candidates, &mut |item| {
        counted = counted.saturating_add(1);
        on_item(&item)
    })?;
    Ok((outcome, counted))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn existence_only_stops_after_the_first_sufficient_witness() {
        let items = [10u32, 20, 30, 40];
        let (outcome, counted) = count_visits(
            DistinctnessWitness::ExistenceOnly {
                projection: ProjectionId(0),
            },
            items,
            |&item| {
                Ok::<_, ()>(if item == 20 {
                    VisitControl::Sufficient
                } else {
                    VisitControl::Continue
                })
            },
        )
        .expect("visit");
        assert_eq!(counted, 2, "unrelated later items are not visited");
        assert_eq!(outcome, VisitOutcome::Sufficient { visited: 2 });
    }

    #[test]
    fn full_walk_does_not_treat_sufficient_as_stop() {
        let items = [1u32, 2, 3];
        let (outcome, counted) =
            count_visits(DistinctnessWitness::FullRowEquality, items, |_| {
                Ok::<_, ()>(VisitControl::Sufficient)
            })
            .expect("visit");
        assert_eq!(counted, 3);
        assert_eq!(outcome, VisitOutcome::Exhausted { visited: 3 });
    }

    #[test]
    fn stop_prevents_later_visits() {
        let items = [1u32, 2, 3];
        let (outcome, counted) =
            count_visits(DistinctnessWitness::FullRowEquality, items, |&item| {
                Ok::<_, ()>(if item == 1 {
                    VisitControl::Stop
                } else {
                    VisitControl::Continue
                })
            })
            .expect("visit");
        assert_eq!(counted, 1);
        assert_eq!(outcome, VisitOutcome::Stopped { visited: 1 });
    }
}
