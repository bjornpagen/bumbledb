//! The dangling relation/field ids, head/rule misalignment, unsafe negation,

use bumbledb::{
    AllenMask, Atom, AtomSource, CmpOp, Comparison, ConditionTree, FieldId, FindTerm, FoldOp,
    Interior, InteriorId, NonEmpty, ParamId, ProjectionRule, Query, Rec, RecRule, RecStep,
    RelationId, Rule, Term, Value, VarId,
};

use super::Rng;
use crate::querygen::target;

const RELATION_SPAN: u64 = 13;

const FIELD_SPAN: u64 = 8;

const VAR_SPAN: u64 = 6;
const PARAM_SPAN: u64 = 4;

pub fn random_query(rng: &mut Rng) -> Query {

    if rng.chance(1, 2) {
        return plausible(rng);
    }

    let rule_count = if rng.chance(1, 32) {
        17 + rng.range(4)
    } else {
        rng.range(4) 
    };
    let rules: Vec<Rule> = (0..rule_count).map(|_| random_rule(rng)).collect();

    let head = match rules.first() {
        Some(rule) if rng.chance(7, 8) => rule.head(),
        _ => (0..rng.range(4))
            .map(|_| random_find(rng).head_term())
            .collect(),
    };
    let interiors = if rng.chance(1, 4) {
        (0..rng.range(4)).map(|_| random_interior(rng)).collect()
    } else {
        vec![]
    };

    if rng.chance(1, 4) {
        Query {
            interiors,
            rec: Some(random_rec(rng)),
            head,
            rules,
        }
    } else {
        Query {
            interiors,
            head,
            rules,
            rec: None,
        }
    }
}

fn plausible(rng: &mut Rng) -> Query {
    let schema = target::schema();
    let rel = RelationId(
        u32::try_from(rng.range(u64::from(target::TARGET_RELATIONS))).expect("id fits u32"),
    );
    let field_count = u64::try_from(schema.relation(rel).fields().len()).expect("fits u64");
    let vars = 1 + rng.range(field_count.min(3));
    let start = rng.range(field_count);
    let bindings: Vec<(FieldId, Term)> = (0..vars)
        .map(|i| {
            (
                FieldId(u16::try_from((start + i) % field_count).expect("field id fits u16")),
                Term::Var(VarId(u16::try_from(i).expect("var id fits u16"))),
            )
        })
        .collect();
    let mut rule = Rule {
        finds: (0..vars)
            .map(|i| FindTerm::Var(VarId(u16::try_from(i).expect("var id fits u16"))))
            .collect(),
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(rel),
            bindings,
        }],
        negated: vec![],
        conditions: vec![],
    };

    if rng.chance(1, 4) {
        rule.conditions.push(random_tree(rng, 2));
    }
    if rng.chance(1, 8) {
        rule.negated.push(random_atom(rng));
    }
    if rng.chance(1, 8) {
        rule.finds.push(random_find(rng));
    }
    if rng.chance(1, 8) {
        let atom = &mut rule.atoms[0];
        let slot = usize::try_from(
            rng.range(u64::try_from(atom.bindings.len()).expect("binding count fits u64")),
        )
        .expect("slot fits usize");
        atom.bindings[slot].1 = random_term(rng);
    }
    Query::single(rule)
}

fn random_rule(rng: &mut Rng) -> Rule {
    Rule {
        finds: (0..rng.range(4)).map(|_| random_find(rng)).collect(),
        atoms: (0..rng.range(3)).map(|_| random_atom(rng)).collect(),
        negated: (0..rng.range(2)).map(|_| random_atom(rng)).collect(),
        conditions: (0..rng.range(3)).map(|_| random_tree(rng, 3)).collect(),
    }
}

fn random_find(rng: &mut Rng) -> FindTerm {
    match rng.range(5) {
        0 | 1 => FindTerm::Var(var(rng)),
        2 => FindTerm::Aggregate {
            op: random_fold(rng),
            over: var(rng),
        },
        3 => FindTerm::Count,
        _ => FindTerm::Pack { over: var(rng) },
    }
}

fn random_fold(rng: &mut Rng) -> FoldOp {
    match rng.range(3) {
        0 => FoldOp::Sum,
        1 => FoldOp::Min,
        _ => FoldOp::Max,
    }
}

fn random_atom(rng: &mut Rng) -> Atom {
    let source = if rng.chance(3, 4) {
        AtomSource::Edb(relation(rng))
    } else {
        AtomSource::Interior(InteriorId(
            u32::try_from(rng.range(5)).expect("interior id fits u32"),
        ))
    };
    Atom {
        source,
        bindings: (0..rng.range(4))
            .map(|_| (field(rng), random_term(rng)))
            .collect(),
    }
}

fn random_projection(rng: &mut Rng) -> ProjectionRule {
    ProjectionRule {
        finds: (0..rng.range(4)).map(|_| var(rng)).collect(),
        atoms: (0..rng.range(3)).map(|_| random_atom(rng)).collect(),
        negated: (0..rng.range(2)).map(|_| random_atom(rng)).collect(),
        conditions: (0..rng.range(3)).map(|_| random_tree(rng, 3)).collect(),
    }
}

fn random_interior(rng: &mut Rng) -> Interior {
    Interior {
        rules: (0..rng.range(3)).map(|_| random_projection(rng)).collect(),
    }
}

fn random_rec_rule(rng: &mut Rng) -> RecRule {
    RecRule {
        finds: (0..rng.range(4)).map(|_| var(rng)).collect(),
        atoms: (0..rng.range(3)).map(|_| random_atom(rng)).collect(),
        conditions: (0..rng.range(3)).map(|_| random_tree(rng, 3)).collect(),
    }
}

fn random_rec_step(rng: &mut Rng) -> RecStep {
    RecStep {
        finds: (0..rng.range(4)).map(|_| var(rng)).collect(),
        self_bindings: (0..rng.range(3))
            .map(|_| (field(rng), random_term(rng)))
            .collect(),
        atoms: (0..rng.range(3)).map(|_| random_atom(rng)).collect(),
        conditions: (0..rng.range(3)).map(|_| random_tree(rng, 3)).collect(),
    }
}

fn nonempty<T>(rng: &mut Rng, mut make: impl FnMut(&mut Rng) -> T) -> NonEmpty<T> {
    let extra = rng.range(2);
    NonEmpty {
        first: make(rng),
        rest: (0..extra).map(|_| make(rng)).collect(),
    }
}

fn random_rec(rng: &mut Rng) -> Rec {
    Rec {
        base: nonempty(rng, random_rec_rule),
        rec: nonempty(rng, random_rec_step),
    }
}

fn random_term(rng: &mut Rng) -> Term {
    match rng.range(7) {
        0..=2 => Term::Var(var(rng)),
        3 => Term::Param(param(rng)),
        4 => Term::ParamSet(param(rng)),
        _ => Term::Literal(random_value(rng)),
    }
}

fn random_value(rng: &mut Rng) -> Value {
    match rng.range(8) {
        0 => Value::Bool(rng.chance(1, 2)),
        1 | 2 => Value::U64(rng.range(16)),
        3 => Value::I64(signed(rng)),
        4 => Value::String(Box::from("Fee")),
        5 => {
            Value::FixedBytes(vec![0xA5; usize::try_from(rng.range(4) * 16).expect("small")].into())
        }
        6 => {
            let start = rng.range(8);
            let end = match rng.range(4) {
                0 => start + 1,
                1 => start + 1 + rng.range(6),
                2 => u64::MAX, 
                _ => start + 2,
            };
            Value::IntervalU64(
                bumbledb::Interval::<u64>::new(start, end).expect("nonempty interval"),
            )
        }
        _ => {
            let start = signed(rng);
            let end = match rng.range(3) {
                0 => start.saturating_add(1),
                1 => start.saturating_add(1 + i64::try_from(rng.range(6)).expect("small")),
                _ => i64::MAX,
            };
            Value::IntervalI64(
                bumbledb::Interval::<i64>::new(start, end).expect("nonempty interval"),
            )
        }
    }
}

fn random_tree(rng: &mut Rng, depth: u64) -> ConditionTree {
    if rng.chance(1, 32) {
        let mut spine = ConditionTree::Leaf(random_comparison(rng));
        for _ in 0..60 + rng.range(16) {
            spine = ConditionTree::And(vec![spine]);
        }
        return spine;
    }
    if depth == 0 || rng.chance(2, 5) {
        return ConditionTree::Leaf(random_comparison(rng));
    }
    let children = (0..rng.range(3))
        .map(|_| random_tree(rng, depth - 1))
        .collect();
    if rng.chance(1, 2) {
        ConditionTree::And(children) 
    } else {
        ConditionTree::Or(children) 
    }
}

fn random_comparison(rng: &mut Rng) -> Comparison {
    let op = match rng.range(8) {
        0 => CmpOp::Eq,
        1 => CmpOp::Ne,
        2 => CmpOp::Lt,
        3 => CmpOp::Le,
        4 => CmpOp::Gt,
        5 => CmpOp::Ge,
        6 => CmpOp::Allen {
            mask: random_mask(rng),
        },
        _ => CmpOp::PointIn,
    };
    Comparison {
        op,
        lhs: random_term(rng),
        rhs: random_term(rng),
    }
}

fn random_mask(rng: &mut Rng) -> AllenMask {
    let bits = u16::try_from(rng.range(1 << 13)).expect("13 bits fit u16");
    AllenMask::new(bits).expect("13-bit draw is a mask")
}

fn relation(rng: &mut Rng) -> RelationId {
    let id = if rng.chance(7, 8) {
        rng.range(RELATION_SPAN)
    } else {
        rng.range(RELATION_SPAN + 3)
    };
    RelationId(u32::try_from(id).expect("relation id fits u32"))
}

fn field(rng: &mut Rng) -> FieldId {
    FieldId(u16::try_from(rng.range(FIELD_SPAN + 2)).expect("field id fits u16"))
}

fn var(rng: &mut Rng) -> VarId {
    VarId(u16::try_from(rng.range(VAR_SPAN)).expect("var id fits u16"))
}

fn param(rng: &mut Rng) -> ParamId {
    ParamId(u16::try_from(rng.range(PARAM_SPAN)).expect("param id fits u16"))
}

fn signed(rng: &mut Rng) -> i64 {
    i64::try_from(rng.range(16)).expect("small draw fits i64") - 8
}

#[cfg(test)]
mod tests {
    use super::random_query;
    use crate::corpus_gen::Rng;
    use crate::querygen::target;
    use bumbledb::Query;

    #[test]
    fn the_same_bytes_yield_the_same_query() {
        let bytes: Vec<u8> = (1..=96u64)
            .flat_map(|i| i.wrapping_mul(0x9E37_79B9_7F4A_7C15).to_le_bytes())
            .collect();
        let first = random_query(&mut Rng::from_bytes(&bytes));
        assert_eq!(
            first,
            random_query(&mut Rng::from_bytes(&bytes)),
            "same bytes, same query"
        );
        let other: Vec<u8> = (1..=96u64)
            .flat_map(|i| i.wrapping_mul(0xC2B2_AE3D_27D4_EB4F).to_le_bytes())
            .collect();
        assert_ne!(
            first,
            random_query(&mut Rng::from_bytes(&other)),
            "bytes steer the query"
        );
    }

    #[test]
    fn the_arm_reaches_both_verdict_classes() {
        let dir = std::env::temp_dir().join("bumbledb-bench-irgen");
        let _ = std::fs::remove_dir_all(&dir);
        let db = target::publish_admitted(&dir);
        let mut accepted = 0u32;
        let mut rejected = 0u32;
        let mut saw_interiors = false;
        let mut saw_rec = false;
        for seed in 0..512 {
            let query = random_query(&mut Rng::new(seed));
            saw_interiors |= !query.interiors().is_empty();
            saw_rec |= matches!(query, Query { .. });
            match db.prepare(&query) {
                Ok(_) => accepted += 1,
                Err(_) => rejected += 1,
            }
        }
        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
        assert!(accepted > 0, "no accepted query in 512 seeds");
        assert!(rejected > 0, "no rejected query in 512 seeds");
        assert!(saw_interiors, "no interiors-bearing query in 512 seeds");
        assert!(saw_rec, "no rec-bearing query in 512 seeds");
        eprintln!("mix: {accepted} accepted / {rejected} rejected");
    }
}
