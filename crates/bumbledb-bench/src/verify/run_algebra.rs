//! - **DNF**: seeded random predicate trees to depth 3 — the naive
//! representation, judged before anything is timed):

use bumbledb::{
    AllenMask, Atom, CmpOp, Comparison, ConditionTree, Db, Error, FindTerm, FoldOp, Query, Rule,
    Term, Value, VarId,
};

use crate::corpus_gen::{AT_BASE, AT_STEP, Rng, Sizes};
use crate::differential::Op;
use crate::fixture::var;
use crate::naive::query::dnf_width;
use crate::schema::ids;
use crate::translate::{Inexpressible, LaneCase, sqlite_expressible};
use crate::verify::Run;

fn leaf(op: CmpOp, lhs: Term, rhs: Term) -> ConditionTree {
    ConditionTree::Leaf(Comparison { op, lhs, rhs })
}

fn mandate_atom() -> Atom {
    Atom {
        source: bumbledb::AtomSource::Edb(ids::MANDATE),
        bindings: vec![
            (ids::mandate::ACCOUNT, var(0)),
            (ids::mandate::ACTIVE, var(1)),
        ],
    }
}

fn posting_atom() -> Atom {
    Atom {
        source: bumbledb::AtomSource::Edb(ids::POSTING),
        bindings: vec![(ids::posting::ACCOUNT, var(0)), (ids::posting::AT, var(1))],
    }
}

fn query(query: Query) -> Op {
    Op::Query {
        query,
        params: vec![],
    }
}

fn rules_ops(sizes: &Sizes) -> Vec<Op> {
    let entry_arm = |ordinal: u64| Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::JOURNAL_ENTRY),
            bindings: vec![
                (ids::journal_entry::ID, var(0)),
                (
                    ids::journal_entry::SOURCE,
                    Term::Literal(Value::U64(ordinal)),
                ),
                (ids::journal_entry::CREATED_AT, var(1)),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    };
    let span = i64::try_from(sizes.postings).expect("fits") * AT_STEP;
    let posting_arm = |floor: i64, finds: Vec<FindTerm>| Rule {
        finds,
        atoms: vec![posting_atom()],
        negated: vec![],
        conditions: vec![leaf(CmpOp::Ge, var(1), Term::Literal(Value::I64(floor)))],
    };
    let assemble = |rules: Vec<Rule>| Query {
        interiors: vec![],
        head: rules[0].head(),
        rules,
        rec: None,
    };
    vec![

        query(assemble(vec![entry_arm(0), entry_arm(2)])),
        query(assemble(vec![entry_arm(0), entry_arm(1), entry_arm(2)])),

        query(assemble(vec![
            posting_arm(AT_BASE, vec![FindTerm::Var(VarId(0))]),
            posting_arm(AT_BASE + span / 4, vec![FindTerm::Var(VarId(0))]),
            posting_arm(AT_BASE + span / 2, vec![FindTerm::Var(VarId(0))]),
        ])),

        // (ruled 2026-07-23, R1 — `CountAcrossRules`); the flipped
        // refusal row lives in [`error_parity`].
        query(assemble(vec![
            posting_arm(
                AT_BASE,
                vec![
                    FindTerm::Var(VarId(0)),
                    FindTerm::Aggregate {
                        op: FoldOp::Max,
                        over: VarId(1),
                    },
                ],
            ),
            posting_arm(
                AT_BASE + span / 3,
                vec![
                    FindTerm::Var(VarId(0)),
                    FindTerm::Aggregate {
                        op: FoldOp::Max,
                        over: VarId(1),
                    },
                ],
            ),
        ])),
    ]
}

fn tree(
    rng: &mut Rng,
    depth: u64,
    leaf: &mut impl FnMut(&mut Rng) -> ConditionTree,
) -> ConditionTree {
    if depth == 0 || rng.chance(2, 5) {
        return leaf(rng);
    }
    let arity = 1 + rng.range(3);
    let children = (0..arity).map(|_| tree(rng, depth - 1, leaf)).collect();
    if rng.chance(1, 2) {
        ConditionTree::And(children)
    } else {
        ConditionTree::Or(children)
    }
}

fn dnf_ops(seed: u64, sizes: &Sizes) -> Vec<Op> {
    let mut rng = Rng::new(seed ^ 0x0115_D2F0);
    let span = i64::try_from(sizes.postings).expect("fits") * AT_STEP;
    let mut tree_leaf = |rng: &mut Rng| {
        let op = match rng.range(6) {
            0 => CmpOp::Eq,
            1 => CmpOp::Ne,
            2 => CmpOp::Lt,
            3 => CmpOp::Le,
            4 => CmpOp::Gt,
            _ => CmpOp::Ge,
        };
        if rng.chance(1, 2) {
            leaf(
                op,
                var(0),
                Term::Literal(Value::U64(rng.range(sizes.accounts + 2))),
            )
        } else {
            let at = AT_BASE
                + i64::try_from(rng.range(u64::try_from(span).expect("positive"))).expect("fits");
            leaf(op, var(1), Term::Literal(Value::I64(at)))
        }
    };
    let mut ops: Vec<Op> = (0..12)
        .map(|_| {
            let conditions: Vec<ConditionTree> = (0..=rng.range(2))
                .map(|_| tree(&mut rng, 3, &mut tree_leaf))
                .collect();
            let rule = Rule {
                finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
                atoms: vec![posting_atom()],
                negated: vec![],
                conditions,
            };

            if dnf_width(&rule) > bumbledb::MAX_RULES || dnf_width(&rule) == 0 {
                let mut trimmed = rule;
                trimmed.conditions = vec![tree_leaf(&mut rng)];
                query(Query::single(trimmed))
            } else {
                query(Query::single(rule))
            }
        })
        .collect();
    ops.extend(rich_dnf_ops(seed, sizes));
    ops
}

#[expect(
    clippy::too_many_lines,
    reason = "one OR-tree grammar, every leaf shape in one place — clearer kept together"
)]
fn rich_dnf_ops(seed: u64, sizes: &Sizes) -> Vec<Op> {
    let mut rng = Rng::new(seed ^ 0x0085_D2F1);
    let at_span = i64::try_from(sizes.postings).expect("fits") * AT_STEP;
    let at_literal = |rng: &mut Rng| {
        AT_BASE + i64::try_from(rng.range(u64::try_from(at_span).expect("positive"))).expect("fits")
    };
    let interval_literal = |rng: &mut Rng| {
        let start = at_literal(rng);
        let width = 1 + i64::try_from(rng.range(u64::try_from(4 * AT_STEP).expect("positive")))
            .expect("fits");
        Term::Literal(Value::IntervalI64(
            bumbledb::Interval::<i64>::new(start, start + width).expect("nonempty by construction"),
        ))
    };
    (0..12)
        .map(|round| {

            let joined = rng.chance(1, 2);
            let mut atoms = vec![mandate_atom()];
            if joined {
                atoms.push(Atom {
                    source: bumbledb::AtomSource::Edb(ids::POSTING),
                    bindings: vec![(ids::posting::ACCOUNT, var(0)), (ids::posting::AT, var(2))],
                });
            }
            let negated = if rng.chance(1, 4) {
                vec![Atom {
                    source: bumbledb::AtomSource::Edb(ids::POSTING),
                    bindings: vec![(ids::posting::ACCOUNT, var(0))],
                }]
            } else {
                vec![]
            };

            let (param_leaf, params) = match rng.range(3) {
                0 => (None, vec![]),
                1 => (
                    Some(leaf(CmpOp::Eq, var(0), Term::Param(bumbledb::ParamId(0)))),
                    vec![crate::naive::ParamValue::Scalar(Value::U64(
                        rng.range(sizes.accounts + 2),
                    ))],
                ),
                _ => (
                    Some(leaf(
                        CmpOp::Eq,
                        var(0),
                        Term::ParamSet(bumbledb::ParamId(0)),
                    )),
                    vec![crate::naive::ParamValue::Set(
                        (0..rng.range(4))
                            .map(|_| Value::U64(rng.range(sizes.accounts + 2)))
                            .collect(),
                    )],
                ),
            };
            let mut rich_leaf = |rng: &mut Rng| match rng.range(3) {
                0 => leaf(
                    CmpOp::Allen {
                        mask: match rng.range(4) {
                            0 => AllenMask::INTERSECTS,
                            1 => AllenMask::DISJOINT,
                            2 => AllenMask::COVERS,
                            _ => AllenMask::BEFORE,
                        },
                    },
                    var(1),
                    interval_literal(rng),
                ),
                1 => leaf(
                    CmpOp::PointIn,
                    var(1),
                    Term::Literal(Value::I64(at_literal(rng))),
                ),
                _ => leaf(
                    op_of(rng),
                    var(0),
                    Term::Literal(Value::U64(rng.range(sizes.accounts + 2))),
                ),
            };
            let mut conditions: Vec<ConditionTree> = (0..=rng.range(2))
                .map(|_| tree(&mut rng, 3, &mut rich_leaf))
                .collect();
            conditions.extend(param_leaf);

            let finds = match round % 3 {
                0 if joined => vec![
                    FindTerm::Var(VarId(0)),
                    FindTerm::Aggregate {
                        op: FoldOp::Max,
                        over: VarId(2),
                    },
                ],
                0 | 1 => vec![FindTerm::Var(VarId(0)), FindTerm::Count],
                _ => vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
            };
            let rule = Rule {
                finds,
                atoms,
                negated,
                conditions,
            };
            let rule = if dnf_width(&rule) > bumbledb::MAX_RULES || dnf_width(&rule) == 0 {
                let mut trimmed = rule;
                trimmed.conditions = vec![rich_leaf(&mut rng)];
                trimmed.conditions.extend(param_leaf_of(&params));
                trimmed
            } else {
                rule
            };
            Op::Query {
                query: Query::single(rule),
                params,
            }
        })
        .collect()
}

fn param_leaf_of(params: &[crate::naive::ParamValue]) -> Option<ConditionTree> {
    match params.first() {
        None => None,
        Some(crate::naive::ParamValue::Scalar(_)) => {
            Some(leaf(CmpOp::Eq, var(0), Term::Param(bumbledb::ParamId(0))))
        }
        Some(crate::naive::ParamValue::Set(_)) => Some(leaf(
            CmpOp::Eq,
            var(0),
            Term::ParamSet(bumbledb::ParamId(0)),
        )),
    }
}

fn op_of(rng: &mut Rng) -> CmpOp {
    match rng.range(6) {
        0 => CmpOp::Eq,
        1 => CmpOp::Ne,
        2 => CmpOp::Lt,
        3 => CmpOp::Le,
        4 => CmpOp::Gt,
        _ => CmpOp::Ge,
    }
}

fn pack_and_measure_ops() -> (Vec<Op>, u64) {
    let pack = |rules: Vec<Rule>| Query {
        interiors: vec![],
        head: rules[0].head(),
        rules,
        rec: None,
    };
    let grouped = pack(vec![Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Pack { over: VarId(1) }],
        atoms: vec![mandate_atom()],
        negated: vec![],
        conditions: vec![],
    }]);
    let global = pack(vec![Rule {
        finds: vec![FindTerm::Pack { over: VarId(1) }],
        atoms: vec![mandate_atom()],
        negated: vec![],
        conditions: vec![],
    }]);
    // The multi-rule Pack: per-org arms whose claims union before the

    let org_arm = |org: u64| Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Pack { over: VarId(1) }],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::MANDATE),
            bindings: vec![
                (ids::mandate::ACCOUNT, var(0)),
                (ids::mandate::ORG, Term::Literal(Value::U64(org))),
                (ids::mandate::ACTIVE, var(1)),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    };
    let multi = pack(vec![org_arm(0), org_arm(1)]);
    let pack_queries = [grouped, global, multi];
    for q in &pack_queries {
        assert_eq!(
            sqlite_expressible(&LaneCase::Query(q)),
            Err(Inexpressible::PackAggregate),
            "Pack heads are the enumerated SQLite-inexpressible query set"
        );
    }
    let naive_only = pack_queries.len() as u64;
    let ops: Vec<Op> = pack_queries.into_iter().map(query).collect();
    (ops, naive_only)
}

pub(super) fn algebra_ops(seed: u64, sizes: &Sizes) -> (Vec<Op>, u64) {
    let mut ops = rules_ops(sizes);
    ops.extend(dnf_ops(seed, sizes));
    let (rest, naive_only) = pack_and_measure_ops();
    ops.extend(rest);
    (ops, naive_only)
}

enum Expected {

    DnfCap { naive_width: usize },

    Vanished,

    EmptyMask,

    FullMask,

    /// typed refusal since R1 (the flipped acceptance row).
    CountAcrossRules { rules: usize },
}

fn parity_cases() -> Vec<(&'static str, Query, Expected)> {
    let posting_rule = |conditions: Vec<ConditionTree>| Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![posting_atom()],
        negated: vec![],
        conditions,
    };
    let account_leaf = |k: u64| leaf(CmpOp::Eq, var(0), Term::Literal(Value::U64(k)));
    let wide_or = |n: u64| ConditionTree::Or((0..n).map(account_leaf).collect());
    let mask_query = |mask: AllenMask| {
        Query::single(Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![
                mandate_atom(),
                Atom {
                    source: bumbledb::AtomSource::Edb(ids::MANDATE),
                    bindings: vec![
                        (ids::mandate::ACCOUNT, var(0)),
                        (ids::mandate::ACTIVE, var(2)),
                    ],
                },
            ],
            negated: vec![],
            conditions: vec![leaf(CmpOp::Allen { mask }, var(1), var(2))],
        })
    };
    vec![
        {

            let q = Query::single(posting_rule(vec![wide_or(17)]));
            let naive_width = dnf_width(&q.rules()[0]);
            ("dnf cap (wide Or)", q, Expected::DnfCap { naive_width })
        },
        {

            let q = Query::single(posting_rule(vec![wide_or(5), wide_or(4)]));
            let naive_width = dnf_width(&q.rules()[0]);
            ("dnf cap (product)", q, Expected::DnfCap { naive_width })
        },
        (

            "vanished query (empty Or)",
            Query::single(posting_rule(vec![
                ConditionTree::Or(vec![]),
                account_leaf(0),
            ])),
            Expected::Vanished,
        ),
        (
            "vacuous mask (EMPTY)",
            mask_query(AllenMask::EMPTY),
            Expected::EmptyMask,
        ),
        (
            "vacuous mask (FULL)",
            mask_query(AllenMask::FULL),
            Expected::FullMask,
        ),
        {

            let count_head = || vec![FindTerm::Var(VarId(0)), FindTerm::Count];
            let arm = |floor: i64| Rule {
                finds: count_head(),
                atoms: vec![posting_atom()],
                negated: vec![],
                conditions: vec![leaf(CmpOp::Ge, var(1), Term::Literal(Value::I64(floor)))],
            };
            let q = Query {
                interiors: vec![],
                head: arm(0).head(),
                rules: vec![arm(0), arm(1)],
                rec: None,
            };
            let rules = q.rules().len();
            (
                "count across rules (R1)",
                q,
                Expected::CountAcrossRules { rules },
            )
        },
    ]
}

pub(super) fn error_parity<S, T>(db: &Db<S>, run: &mut Run<'_, T>) {
    for (label, q, expected) in parity_cases() {
        run.cases += 1;
        let verdict = match db.prepare(&q) {
            Err(Error::Validation(error)) => error,
            Ok(_) => {
                parity_bundle(run, label, &q, "engine ACCEPTED a roster rejection");
                continue;
            }
            Err(other) => {
                parity_bundle(run, label, &q, &format!("non-validation error: {other:?}"));
                continue;
            }
        };
        let agree = match expected {
            Expected::DnfCap { naive_width } => matches!(
                verdict,
                bumbledb::error::ValidationError::DnfExceedsRules { exceeded }
                    if exceeded.observed == naive_width && naive_width > exceeded.ceiling
            ),

            Expected::Vanished => {
                dnf_width(&q.rules()[0]) == 0
                    && matches!(verdict, bumbledb::error::ValidationError::EmptyRuleSet)
            }
            Expected::EmptyMask => {
                matches!(
                    verdict,
                    bumbledb::error::ValidationError::EmptyAllenMask { .. }
                )
            }
            Expected::FullMask => {
                matches!(
                    verdict,
                    bumbledb::error::ValidationError::FullAllenMask { .. }
                )
            }
            Expected::CountAcrossRules { rules } => matches!(
                verdict,
                bumbledb::error::ValidationError::CountAcrossRules { rules: found }
                    if found == rules
            ),
        };
        if !agree {
            parity_bundle(
                run,
                label,
                &q,
                &format!("engine verdict {verdict:?} disagrees with the naive computation"),
            );
        }
        if run.bundles.len() >= super::MAX_BUNDLES {
            return;
        }
    }
}

fn parity_bundle<S>(run: &mut Run<'_, S>, label: &str, q: &Query, mismatch: &str) {
    let bundle = run.out_dir.join(format!("mismatch-{}", run.bundles.len()));
    std::fs::create_dir_all(&bundle).expect("bundle dir");
    std::fs::write(
        bundle.join("mismatch.txt"),
        format!("error parity: {label}\n{mismatch}\n{q:#?}\n"),
    )
    .expect("bundle");
    eprintln!(
        "verify: ERROR-PARITY MISMATCH {label} -> {}",
        bundle.display()
    );
    run.bundles.push(bundle);
}
