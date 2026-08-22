use bumbledb::{
    Atom, CmpOp, Comparison, ConditionTree, FindTerm, FoldOp, Query, Rule, Term, Value, VarId,
};

use crate::corpus_gen::Rng;
use crate::querygen::RulesVariant;
use crate::querygen::target::{self, Domains, ids};

pub(super) fn rules(rng: &mut Rng, domains: &Domains) -> (Query, RulesVariant) {
    let variant = match rng.range(3) {
        0 => RulesVariant::Disjoint,
        1 => RulesVariant::Overlap,
        _ => RulesVariant::Aggregate,
    };
    let query = match variant {
        RulesVariant::Disjoint => disjoint_arms(rng),
        RulesVariant::Overlap => {
            if rng.chance(1, 3) {
                du_twin()
            } else {
                overlapping_arms(rng, domains)
            }
        }
        RulesVariant::Aggregate => union_fold(rng, domains),
    };
    (query, variant)
}

fn assemble(rules: Vec<Rule>) -> Query {
    Query {
        interiors: vec![],
        head: rules[0].head(),
        rules,
        rec: None,
    }
}

fn disjoint_arms(rng: &mut Rng) -> Query {
    let arms = 2 + rng.range(2);
    let mut ordinals = [0u64, 1, 2];

    ordinals.swap(0, usize::try_from(rng.range(3)).expect("small"));
    ordinals.swap(1, 1 + usize::try_from(rng.range(2)).expect("small"));
    let rules = ordinals[..usize::try_from(arms).expect("small")]
        .iter()
        .map(|ordinal| Rule {
            finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
            atoms: vec![Atom {
                source: bumbledb::AtomSource::Edb(ids::JOURNAL_ENTRY),
                bindings: vec![
                    (ids::journal_entry::ID, Term::Var(VarId(0))),
                    (
                        ids::journal_entry::SOURCE,
                        Term::Literal(Value::U64(*ordinal)),
                    ),
                    (ids::journal_entry::CREATED_AT, Term::Var(VarId(1))),
                ],
            }],
            negated: vec![],
            conditions: vec![],
        })
        .collect();
    assemble(rules)
}

fn du_twin() -> Query {
    assemble(vec![
        Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                source: bumbledb::AtomSource::Edb(ids::JOURNAL_ENTRY),
                bindings: vec![
                    (ids::journal_entry::ID, Term::Var(VarId(0))),
                    (
                        ids::journal_entry::SOURCE,
                        Term::Literal(Value::U64(target::SOURCE_IMPORT)),
                    ),
                ],
            }],
            negated: vec![],
            conditions: vec![],
        },
        Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                source: bumbledb::AtomSource::Edb(ids::IMPORT_BATCH),
                bindings: vec![(ids::import_batch::ENTRY, Term::Var(VarId(0)))],
            }],
            negated: vec![],
            conditions: vec![],
        },
    ])
}

fn posting_arm(finds: Vec<FindTerm>, floor: i64) -> Rule {
    Rule {
        finds,
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::POSTING),
            bindings: vec![
                (ids::posting::ACCOUNT, Term::Var(VarId(0))),
                (ids::posting::AT, Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Ge,
            lhs: Term::Var(VarId(1)),
            rhs: Term::Literal(Value::I64(floor)),
        })],
    }
}

fn overlapping_arms(rng: &mut Rng, domains: &Domains) -> Query {
    let arms = 2 + rng.range(3);
    let span = i64::try_from(domains.postings).expect("fits") * target::AT_STEP;
    let wide_head = rng.chance(1, 2);
    let rules = (0..arms)
        .map(|arm| {
            let floor = target::AT_BASE + i64::try_from(arm).expect("small") * (span / 8);
            let finds = if wide_head {
                vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))]
            } else {
                vec![FindTerm::Var(VarId(0))]
            };
            posting_arm(finds, floor)
        })
        .collect();
    assemble(rules)
}

fn union_fold(rng: &mut Rng, domains: &Domains) -> Query {
    let arms = 2 + rng.range(2);
    let span = i64::try_from(domains.postings).expect("fits") * target::AT_STEP;
    let aggregate = match rng.range(3) {
        0 => FindTerm::Aggregate {
            op: FoldOp::Sum,
            over: VarId(1),
        },

        // the typed `CountAcrossRules` refusal now (ruled 2026-07-23,
        1 => FindTerm::Aggregate {
            op: FoldOp::Min,
            over: VarId(1),
        },
        _ => FindTerm::Aggregate {
            op: FoldOp::Max,
            over: VarId(1),
        },
    };
    let over_amount = true;
    let rules = (0..arms)
        .map(|arm| {
            let floor = target::AT_BASE + i64::try_from(arm).expect("small") * (span / 6);
            let mut rule = posting_arm(vec![FindTerm::Var(VarId(0)), aggregate.clone()], floor);
            if over_amount {
                rule.atoms[0]
                    .bindings
                    .push((ids::posting::AMOUNT, Term::Var(VarId(1))));
                rule.conditions.clear();
                rule.conditions.push(ConditionTree::Leaf(Comparison {
                    op: CmpOp::Ge,
                    lhs: Term::Var(VarId(2)),
                    rhs: Term::Literal(Value::I64(floor)),
                }));

                rule.atoms[0].bindings[1] = (ids::posting::AT, Term::Var(VarId(2)));
            }
            rule
        })
        .collect();
    assemble(rules)
}
