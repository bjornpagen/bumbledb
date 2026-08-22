use bumbledb::{
    AtomSource, CmpOp, Comparison, ConditionTree, FieldId, Query, RelationId, Rule, Term, Value,
};

use crate::corpus_gen::{GenConfig, Rng};
use crate::querygen::target::ids;
use crate::walk;

pub fn contradiction_query(rng: &mut Rng, cfg: GenConfig) -> Query {
    loop {
        let mut query = crate::querygen::random_query(rng, cfg);
        if walk::every_rule_mut(&mut query, |rule| plant(rule, rng)) {
            return query;
        }
    }
}

pub(super) fn plant(rule: &mut Rule, rng: &mut Rng) -> bool {
    let Some((var, signed)) = rule.atoms.iter().find_map(|atom| {
        let AtomSource::Edb(relation) = atom.source else {
            return None;
        };
        atom.bindings.iter().find_map(|(field, term)| match term {
            Term::Var(var) => int_field(relation, *field).map(|signed| (*var, signed)),
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
        rule.conditions.push(ConditionTree::Leaf(Comparison {
            op,
            lhs: Term::Var(var),
            rhs,
        }));
    };
    match rng.range(3) {
        0 => {
            leaf(CmpOp::Eq, literal(1));
            leaf(CmpOp::Eq, literal(2));
        }

        1 => {
            leaf(CmpOp::Gt, literal(6));
            leaf(CmpOp::Lt, literal(3));
        }

        _ => {
            leaf(CmpOp::Ge, literal(2));
            leaf(CmpOp::Le, literal(5));
            leaf(CmpOp::Eq, literal(7));
        }
    }
    true
}

fn int_field(relation: RelationId, field: FieldId) -> Option<bool> {
    let unsigned = matches!(
        (relation, field),
        (ids::HOLDER, ids::holder::ID)
            | (ids::ACCOUNT, ids::account::ID | ids::account::HOLDER)
            | (ids::INSTRUMENT, ids::instrument::ID)
            | (ids::JOURNAL_ENTRY, ids::journal_entry::ID)
            | (
                ids::POSTING,
                ids::posting::ID
                    | ids::posting::ENTRY
                    | ids::posting::ACCOUNT
                    | ids::posting::INSTRUMENT,
            )
            | (ids::POSTING_TAG, ids::posting_tag::POSTING)
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
