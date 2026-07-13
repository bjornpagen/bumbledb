//! The contradiction knob (PRD 10): one drawing rule that renders every
//! rule of a valid draw stage-2-unsatisfiable on constants. The fold
//! (`ir/normalize/fold.rs`) must judge these ∅ at prepare; the naive
//! model judges them ∅ by evaluation — the differential closes the loop,
//! because folding is set-preserving by construction and a semantic fold
//! would diverge here first.

use bumbledb::{CmpOp, Comparison, FieldId, PredicateTree, Query, RelationId, Rule, Term, Value};

use crate::corpus_gen::{GenConfig, Rng};
use crate::querygen::target::ids;

/// A generated query whose EVERY rule carries a constant contradiction
/// on one of its integer variables — the whole program denotes ∅
/// regardless of data. Redraws until every rule offers an integer
/// binding to poison (the shapes provide one in practice; the loop is
/// the honest fallback, seeded and terminating).
pub fn contradiction_query(rng: &mut Rng, cfg: GenConfig) -> Query {
    loop {
        let mut query = crate::querygen::random_query(rng, cfg);
        if query.rules.iter_mut().all(|rule| plant(rule, rng)) {
            return query;
        }
    }
}

/// Poisons one rule: the first positive-atom binding at a scalar
/// integer field gains a pair of mutually unsatisfiable constant
/// comparisons, drawn from the fold's own contradiction vocabulary
/// (twin `Eq`, empty range, `Eq` outside the folded range).
fn plant(rule: &mut Rule, rng: &mut Rng) -> bool {
    let Some((var, signed)) = rule.atoms.iter().find_map(|atom| {
        atom.bindings.iter().find_map(|(field, term)| match term {
            Term::Var(var) => int_field(atom.relation, *field).map(|signed| (*var, signed)),
            _ => None,
        })
    }) else {
        return false;
    };
    let literal = |value: i64| {
        Term::Literal(if signed {
            Value::I64(value)
        } else {
            Value::U64(value.unsigned_abs())
        })
    };
    let mut leaf = |op: CmpOp, rhs: Term| {
        rule.predicates.push(PredicateTree::Leaf(Comparison {
            op,
            lhs: Term::Var(var),
            rhs,
        }));
    };
    match rng.range(3) {
        // Twin Eq: one slot, two distinct constants.
        0 => {
            leaf(CmpOp::Eq, literal(1));
            leaf(CmpOp::Eq, literal(2));
        }
        // Empty range: lo > hi once the bounds fold.
        1 => {
            leaf(CmpOp::Gt, literal(6));
            leaf(CmpOp::Lt, literal(3));
        }
        // Eq pinned outside the folded range.
        _ => {
            leaf(CmpOp::Ge, literal(2));
            leaf(CmpOp::Le, literal(5));
            leaf(CmpOp::Eq, literal(7));
        }
    }
    true
}

/// The target theory's scalar integer bindings — `Some(signed)`.
/// Interval, `str`, `bytes<N>`, and `bool` positions never host the
/// plant (order literals would be type errors there).
fn int_field(relation: RelationId, field: FieldId) -> Option<bool> {
    let unsigned = matches!(
        (relation, field),
        (ids::HOLDER, ids::holder::ID)
            | (
                ids::ACCOUNT,
                ids::account::ID | ids::account::HOLDER | ids::account::CURRENCY,
            )
            | (ids::INSTRUMENT, ids::instrument::ID)
            | (
                ids::JOURNAL_ENTRY,
                ids::journal_entry::ID | ids::journal_entry::SOURCE,
            )
            | (
                ids::POSTING,
                ids::posting::ID
                    | ids::posting::ENTRY
                    | ids::posting::ACCOUNT
                    | ids::posting::INSTRUMENT,
            )
            | (
                ids::POSTING_TAG,
                ids::posting_tag::POSTING | ids::posting_tag::TAG,
            )
            | (ids::ORG, ids::org::ID)
            | (
                ids::ORG_PARENT,
                ids::org_parent::CHILD | ids::org_parent::PARENT,
            )
            | (ids::MANDATE, ids::mandate::ACCOUNT | ids::mandate::ORG)
            | (ids::TRANSFER, ids::transfer::ID)
            | (
                ids::IMPORT_BATCH,
                ids::import_batch::ENTRY | ids::import_batch::BATCH,
            )
            | (ids::CURRENCY | ids::SOURCE | ids::TAG, FieldId(0))
    );
    if unsigned {
        return Some(false);
    }
    let signed = matches!(
        (relation, field),
        (ids::JOURNAL_ENTRY, ids::journal_entry::CREATED_AT)
            | (ids::POSTING, ids::posting::AMOUNT | ids::posting::AT)
    );
    signed.then_some(true)
}
