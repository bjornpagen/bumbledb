use crate::ir::normalize::NormalizedQuery;
use crate::plan::pinned_fields;
use crate::schema::Schema;
use std::collections::BTreeSet;

/// Proof that distinct facts imply distinct bindings for this rule:
/// every participating occurrence's bound fields cover a key of its
/// relation. Carrying this witness is the license to construct an
/// aggregate sink without a binding seen-set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DistinctWitness(());

/// The distinct-bindings elision check (40-execution): every participating
/// occurrence's bound fields — variable-bound or equality-pinned to one
/// constant — cover the projection of one of its keys (`Functionality`
/// statements), so distinct facts imply distinct bindings and the
/// aggregate sink may skip its seen-set. Only participating occurrences
/// are quantified: negated occurrences bind nothing (they only reject)
/// and grounding-eliminated occurrences contribute no facts at all
/// (`plan/ground.rs`), so neither can break the proof.
///
/// Two checks keep the proof honest:
/// - **Pointwise keys**: coverage requires the interval field bound **by
///   value** — `vars` holds value bindings only (membership positions
///   lowered to filters and never enter it), and membership filter kinds
///   are not counted below, so a scalar-prefix-only binding fails
///   coverage: two facts may share the prefix with disjoint intervals.
/// - **Set-bound fields pin nothing**: an Eq against a `ParamSet`/
///   `WordSet` matches any element, so two distinct facts can differ on
///   that field while producing one binding — sets are excluded from the
///   pinned-constant field set (the shared vocabulary,
///   [`pinned_fields`]).
pub(crate) fn provably_distinct(
    normalized: &NormalizedQuery,
    schema: &Schema,
) -> Option<DistinctWitness> {
    normalized
        .occurrences
        .iter()
        .filter(|occurrence| occurrence.role.participates())
        .all(|occurrence| {
            // An `Idb` occurrence carries no keys — a predicate is a
            // transient answer set, not a keyed store — so no rule
            // reading one can prove distinct bindings through key
            // coverage (40-execution.md § the fixpoint driver: cross-round re-derivation
            // is the seen-set's job regardless).
            let Some(stored) = occurrence.source.edb() else {
                return false;
            };
            let relation = schema.relation(stored);
            let bound_fields: BTreeSet<bumbledb_theory::schema::FieldId> = occurrence
                .vars
                .iter()
                .map(|(f, _)| *f)
                .chain(pinned_fields(occurrence))
                .collect();
            relation.keys().iter().any(|id| {
                schema
                    .key(*id)
                    .projection
                    .iter()
                    .all(|f| bound_fields.contains(f))
            })
        })
        .then_some(DistinctWitness(()))
}
