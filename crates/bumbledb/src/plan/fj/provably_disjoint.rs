use crate::image::view::{Const, FilterPredicate};
use crate::ir::normalize::{NormalizedQuery, Occurrence};
use crate::ir::{AggOp, CmpOp, FindTerm, VarId};
use crate::schema::{FieldId, RelationId, Schema};

/// The rule-disjointness proof's witness (docs/architecture/40-execution.md
/// § set semantics): the relation and field whose differing pinned
/// literals make the rules' head rows collision-free. EXPLAIN renders it
/// as `disjoint_rules: proven (R.f)` — an elision must name its proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisjointWitness {
    pub relation: RelationId,
    pub field: FieldId,
}

/// The rule-disjointness check — the exclusivity theorem's third consumer
/// (docs/architecture/30-dependencies.md: the checker enforces it, the
/// chase spends it, and here the executor spends it again). A pair of
/// rules is **provably disjoint** when there is a relation R and a field
/// f such that both rules bind a positive occurrence of R whose filters
/// pin f to *different* concrete literals, **and** that occurrence's
/// bound key columns flow to the same head positions in both rules. Two
/// equal head rows would then force the two pinned facts to agree on a
/// key of R — one fact, whose f cannot equal two different literals. The
/// DU-arm union (one rule per `kind`) is exactly this shape via the
/// parent occurrence's discriminator selection.
///
/// Conservative and sound, the `provably_distinct` discipline:
///
/// - **Pairwise over all rules; one witness for every pair** — any
///   unprovable pair (under every candidate) returns `None` and the
///   seen-set stays. The single shared witness is the workload's own
///   shape (every DU arm selects the one discriminator) and keeps the
///   EXPLAIN line honest.
/// - **Concrete literals only**: params resolve at bind and pin nothing
///   here; a set matches any element and pins nothing; mixed constant
///   forms (a resolved word against a pending intern) stay unknown. Two
///   `PendingIntern`s with different bytes are provably different — the
///   dictionary is injective, and a miss empties its rule outright.
/// - **Positive occurrences only**: a negated occurrence certifies the
///   *absence* of a matching fact, so its pins witness nothing.
/// - **Head positions are dedup-key positions**: a projection head reads
///   every find; an aggregate head reads group variables and fold inputs
///   (the union key, docs/architecture/40-execution.md § the rule loop) —
///   the nullary `Count` reads nothing and can carry no key column, and
///   Arg-restriction never crosses rules (validation).
#[must_use]
pub fn provably_disjoint_rules(
    rules: &[(&[FindTerm], &NormalizedQuery)],
    schema: &Schema,
) -> Option<DisjointWitness> {
    debug_assert!(rules.len() > 1, "disjointness is a multi-rule property");
    // Candidates come from rule 0's pins: a witness must pin in EVERY
    // rule, so one absent from rule 0 proves nothing.
    let (_, first) = rules[0];
    let mut candidates: Vec<DisjointWitness> = first
        .occurrences
        .iter()
        .filter(|occurrence| occurrence.role.participates())
        .flat_map(|occurrence| {
            pinned_fields(occurrence).map(|(field, _)| DisjointWitness {
                relation: occurrence.relation,
                field,
            })
        })
        .collect();
    candidates.dedup();
    candidates.into_iter().find(|witness| {
        rules.iter().enumerate().all(|(i, a)| {
            rules[i + 1..]
                .iter()
                .all(|b| pair_disjoint(*a, *b, *witness, schema))
        })
    })
}

/// One rule pair under one candidate witness: some pinned occurrence in
/// each rule with provably different literals and a key flowing to
/// common head positions.
fn pair_disjoint(
    a: (&[FindTerm], &NormalizedQuery),
    b: (&[FindTerm], &NormalizedQuery),
    witness: DisjointWitness,
    schema: &Schema,
) -> bool {
    pins_of(a.1, witness).any(|(occ_a, const_a)| {
        pins_of(b.1, witness).any(|(occ_b, const_b)| {
            provably_different(const_a, const_b)
                && key_flows_to_common_head(occ_a, a.0, occ_b, b.0, witness.relation, schema)
        })
    })
}

/// A rule's pinned occurrences of the witness relation: each positive
/// occurrence with an `Eq`-to-constant filter on the witness field.
fn pins_of(
    normalized: &NormalizedQuery,
    witness: DisjointWitness,
) -> impl Iterator<Item = (&Occurrence, &Const)> {
    normalized
        .occurrences
        .iter()
        .filter(move |occurrence| {
            occurrence.role.participates() && occurrence.relation == witness.relation
        })
        .filter_map(move |occurrence| {
            pinned_fields(occurrence)
                .find(|(field, _)| *field == witness.field)
                .map(|(_, value)| (occurrence, value))
        })
}

/// The `Eq`-pinned (field, constant) pairs of one occurrence's filters.
fn pinned_fields(occurrence: &Occurrence) -> impl Iterator<Item = (FieldId, &Const)> {
    occurrence.filters.iter().filter_map(|filter| match filter {
        FilterPredicate::Compare {
            field,
            op: CmpOp::Eq,
            value,
        } => Some((*field, value)),
        _ => None,
    })
}

/// Whether two plan-time constants can never resolve to one column
/// value. Same concrete forms compare by payload; everything symbolic
/// (params, sets) or mixed is unknown — conservative `false`.
fn provably_different(a: &Const, b: &Const) -> bool {
    match (a, b) {
        (Const::Word(a), Const::Word(b)) => a != b,
        (Const::Byte(a), Const::Byte(b)) => a != b,
        (
            Const::Interval { start, end },
            Const::Interval {
                start: other_start,
                end: other_end,
            },
        ) => (start, end) != (other_start, other_end),
        (Const::Words(a), Const::Words(b)) => a != b,
        // Distinct raw literals resolve injectively (or miss, emptying
        // their rule) — the str-only dictionary is one-to-one.
        (Const::PendingIntern { bytes }, Const::PendingIntern { bytes: other_bytes }) => {
            bytes != other_bytes
        }
        _ => false,
    }
}

/// Whether some key of R is value-bound in both occurrences with every
/// key column flowing to a common head position — the step that turns
/// "equal head rows" into "one fact of R pinned by both rules".
/// Value-bound means present in `vars` (membership bindings lowered to
/// filters and never enter it), so a pointwise key's interval column is
/// carried by both words and equal spans mean one fact, exactly the
/// `provably_distinct` guard.
fn key_flows_to_common_head(
    a: &Occurrence,
    head_a: &[FindTerm],
    b: &Occurrence,
    head_b: &[FindTerm],
    relation: RelationId,
    schema: &Schema,
) -> bool {
    schema.relation(relation).keys().iter().any(|key| {
        schema.key_projection(*key).iter().all(|field| {
            let (Some(var_a), Some(var_b)) = (var_at(a, *field), var_at(b, *field)) else {
                return false;
            };
            head_a.iter().zip(head_b).any(|(term_a, term_b)| {
                head_reads(term_a) == Some(var_a) && head_reads(term_b) == Some(var_b)
            })
        })
    })
}

/// The variable an occurrence binds at a field, if any.
fn var_at(occurrence: &Occurrence, field: FieldId) -> Option<VarId> {
    occurrence
        .vars
        .iter()
        .find(|(f, _)| *f == field)
        .map(|(_, var)| *var)
}

/// The variable one head position contributes to the sink's dedup key: a
/// projected variable or a fold input (both enter the union key and the
/// projected tuple); the nullary `Count` contributes nothing, and Arg
/// terms are conservatively nothing (validation refuses them across
/// rules anyway).
fn head_reads(term: &FindTerm) -> Option<VarId> {
    match term {
        FindTerm::Var(var) => Some(*var),
        FindTerm::Aggregate {
            op: AggOp::Sum | AggOp::Min | AggOp::Max | AggOp::CountDistinct,
            over,
        } => *over,
        // The remaining positions witness nothing: the nullary Count and
        // Arg terms as before, and the measure positions — `end − start`
        // is a NON-injective map of its variable, so equal head rows do
        // not force equal interval values.
        FindTerm::Aggregate { .. } | FindTerm::Duration(_) | FindTerm::AggregateDuration { .. } => {
            None
        }
    }
}
