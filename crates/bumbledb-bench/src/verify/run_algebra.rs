//! The algebra oracle rows of the naive lane (one obligation per landed
//! representation, judged before anything is timed):
//!
//! - **Rules**: multi-rule programs replayed engine-vs-naive — the
//!   naive model evaluates the rules *directly* (union of per-rule
//!   binding sets, no engine sink mechanics) — disjoint vocabulary-selected
//!   arms, overlapping arms with duplicate head answers,
//!   and the multi-rule aggregate union fold.
//! - **DNF**: seeded random predicate trees to depth 3 — the naive
//!   model evaluates the *input tree*; the engine evaluates the lowered
//!   rules. The differential is the lowering proof, now inside every
//!   verify run — and the tree grammar meets the engine OUTSIDE the
//!   degenerate corner (finding 085): Allen, `PointIn`, params, and
//!   ray-bearing measure leaves under 1–2 atoms, optional negation,
//!   and aggregate heads over overlapping disjuncts (R2's re-keyed
//!   union fold; R6's Kleene raise, tree-quantified).
//! - **`Pack`**: `SQLite` cannot express the coalescing fold
//!   ([`crate::translate::Inexpressible::PackAggregate`]) — these rows
//!   run **naive-only by decision**, counted and reported, never
//!   silently dropped.
//! - **The measure's rays**: `Duration` over the ray-bearing mandate
//!   corpus — `MeasureOfRay` on both sides (typed identity through the
//!   differential runner's `Answers` verdict), and the `Allen(DISJOINT)`
//!   ray filter keeping the same query answers.
//! - **Error parity** ([`error_parity`]): cap-exceeding DNF, the
//!   vanished program (every disjunct empty), and the vacuous masks
//!   (EMPTY and FULL) — the engine's typed validation verdict compared
//!   against the naive model's own from-the-definition computation.

use bumbledb::{
    AggOp, AllenMask, Atom, CmpOp, Comparison, ConditionTree, Db, Error, FindTerm, MaskTerm, Query,
    Rule, Term, Value, VarId,
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

/// `Mandate(account = v0, active = v1)`.
fn mandate_atom() -> Atom {
    Atom {
        source: bumbledb::AtomSource::Edb(ids::MANDATE),
        bindings: vec![
            (ids::mandate::ACCOUNT, var(0)),
            (ids::mandate::ACTIVE, var(1)),
        ],
    }
}

/// `Posting(account = v0, at = v1)`.
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

/// The multi-rule rows: the naive model's rule-union evaluation against
/// the engine's one-sink union.
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
        head: rules[0].head(),
        rules,
    };
    vec![
        // Disjoint arms (distinct vocabulary selections),
        // two and three wide.
        query(assemble(vec![entry_arm(0), entry_arm(2)])),
        query(assemble(vec![entry_arm(0), entry_arm(1), entry_arm(2)])),
        // Overlapping arms: nested `at` floors — every later arm's head
        // rows duplicate earlier ones (the union's teeth).
        query(assemble(vec![
            posting_arm(AT_BASE, vec![FindTerm::Var(VarId(0))]),
            posting_arm(AT_BASE + span / 4, vec![FindTerm::Var(VarId(0))]),
            posting_arm(AT_BASE + span / 2, vec![FindTerm::Var(VarId(0))]),
        ])),
        // The union fold: a valued fold over the head projection. The
        // nullary-Count twin this row once carried is definitionally
        // constant 1 under the head-projection law and REFUSES now
        // (ruled 2026-07-23, R1 — `CountAcrossRules`); the flipped
        // refusal row lives in [`error_parity`].
        query(assemble(vec![
            posting_arm(
                AT_BASE,
                vec![
                    FindTerm::Var(VarId(0)),
                    FindTerm::Aggregate {
                        op: AggOp::Max,
                        over: Some(VarId(1)),
                    },
                ],
            ),
            posting_arm(
                AT_BASE + span / 3,
                vec![
                    FindTerm::Var(VarId(0)),
                    FindTerm::Aggregate {
                        op: AggOp::Max,
                        over: Some(VarId(1)),
                    },
                ],
            ),
        ])),
    ]
}

/// The DNF rows: seeded random trees to depth 3 over one `Posting`
/// scope — the naive model evaluates the tree, the engine the lowered
/// rules. Child counts stay ≥ 1: the vanished-program shapes (empty
/// disjunctions) are [`error_parity`]'s, where both sides *reject*.
/// A random tree to `depth`, every node with ≥ 1 child (the vanished
/// shapes are constructed deliberately, never drawn).
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
            // The generated tree must stay under the cap — width control
            // is the generator's duty; the exceeders are constructed
            // deliberately in `error_parity`.
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

/// The OR-tree grammar OUT of its degenerate corner (finding 085): the
/// leaf pool widens to the full vocabulary the spec admits under a tree
/// — Allen with a literal interval, `PointIn`, params and param sets,
/// and the MEASURE over the ray-bearing mandate lane (the Kleene
/// verdict engine-vs-naive, ruled R6, quantified over trees for the
/// first time) — while the rule template varies over 1–2 atoms, an
/// optional negated atom, and aggregate heads over deliberately
/// overlapping disjuncts (Count/Max under Or — R2's re-keyed union
/// fold, the class the known divergence lived in).
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
            // The scope: Mandate(account = v0, active = v1), half the
            // rounds joined through Posting(account = v0, at = v2), a
            // quarter carrying a negated per-account Posting probe.
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
            // One optional param, referenced by a dedicated conjunct so
            // a supplied binding is never dangling.
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
            let mut rich_leaf = |rng: &mut Rng| match rng.range(6) {
                0 => leaf(
                    op_of(rng),
                    var(0),
                    Term::Literal(Value::U64(rng.range(sizes.accounts + 2))),
                ),
                1 => leaf(
                    CmpOp::Allen {
                        mask: MaskTerm::Literal(match rng.range(4) {
                            0 => AllenMask::INTERSECTS,
                            1 => AllenMask::DISJOINT,
                            2 => AllenMask::COVERS,
                            _ => AllenMask::BEFORE,
                        }),
                    },
                    var(1),
                    interval_literal(rng),
                ),
                2 => leaf(
                    CmpOp::PointIn,
                    var(1),
                    Term::Literal(Value::I64(at_literal(rng))),
                ),
                // The measure leaf — the tree grammar's one partial
                // predicate, over rays (even accounts carry `[s, ∞)`).
                3 => leaf(
                    order_op(rng),
                    Term::Measure(VarId(1)),
                    Term::Literal(Value::U64(
                        rng.range(u64::try_from(4 * AT_STEP).expect("positive")),
                    )),
                ),
                _ if joined => leaf(
                    op_of(rng),
                    var(2),
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
            // The head: projection, or an aggregate over the Or tree —
            // the fold-transparency class (R2): the disjuncts overlap
            // by construction (random bands over eight accounts).
            let finds = match round % 3 {
                0 if joined => vec![
                    FindTerm::Var(VarId(0)),
                    FindTerm::Aggregate {
                        op: AggOp::Max,
                        over: Some(VarId(2)),
                    },
                ],
                0 | 1 => vec![
                    FindTerm::Var(VarId(0)),
                    FindTerm::Aggregate {
                        op: AggOp::Count,
                        over: None,
                    },
                ],
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

/// The trim fallback's param conjunct: re-derived from the params the
/// op carries, so a trimmed rule never dangles its binding.
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

/// A uniformly drawn scalar comparison operator.
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

/// A uniformly drawn order operator (the measure's roster).
fn order_op(rng: &mut Rng) -> CmpOp {
    match rng.range(4) {
        0 => CmpOp::Lt,
        1 => CmpOp::Le,
        2 => CmpOp::Gt,
        _ => CmpOp::Ge,
    }
}

/// The `Pack` rows (naive-only by decision) and the measure's ray rows.
/// Returns the ops and the count of `SQLite`-inexpressible cases, each
/// one asserted to be exactly the enumerated `PackAggregate` routing.
fn pack_and_measure_ops() -> (Vec<Op>, u64) {
    let pack = |rules: Vec<Rule>| Query {
        head: rules[0].head(),
        rules,
    };
    let grouped = pack(vec![Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Pack,
                over: Some(VarId(1)),
            },
        ],
        atoms: vec![mandate_atom()],
        negated: vec![],
        conditions: vec![],
    }]);
    let global = pack(vec![Rule {
        finds: vec![FindTerm::Aggregate {
            op: AggOp::Pack,
            over: Some(VarId(1)),
        }],
        atoms: vec![mandate_atom()],
        negated: vec![],
        conditions: vec![],
    }]);
    // The multi-rule Pack: per-org arms whose claims union before the
    // coalesce — the union fold's relation-shaped form.
    let org_arm = |org: u64| Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Pack,
                over: Some(VarId(1)),
            },
        ],
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
    let mut ops: Vec<Op> = pack_queries.into_iter().map(query).collect();

    // The measure over the ray-bearing corpus (even accounts carry a
    // `[s, ∞)` segment by construction): unfiltered, both sides raise
    // `MeasureOfRay` — the typed verdict compared whole by the
    // differential runner; filtered by the ray probe, both answers.
    let ray_filter = leaf(
        CmpOp::Allen {
            mask: MaskTerm::Literal(AllenMask::DISJOINT),
        },
        var(1),
        Term::Literal(Value::IntervalI64(
            bumbledb::Interval::<i64>::new(i64::MAX - 1, i64::MAX).expect("nonempty interval"),
        )),
    );
    let measure = |finds: Vec<FindTerm>, conditions: Vec<ConditionTree>| {
        query(Query::single(Rule {
            finds,
            atoms: vec![mandate_atom()],
            negated: vec![],
            conditions,
        }))
    };
    ops.push(measure(
        vec![FindTerm::Var(VarId(0)), FindTerm::Measure(VarId(1))],
        vec![],
    ));
    ops.push(measure(
        vec![FindTerm::Var(VarId(0)), FindTerm::Measure(VarId(1))],
        vec![ray_filter.clone()],
    ));
    ops.push(measure(
        vec![FindTerm::AggregateMeasure {
            op: AggOp::Sum,
            over: VarId(1),
        }],
        vec![],
    ));
    ops.push(measure(
        vec![FindTerm::AggregateMeasure {
            op: AggOp::Sum,
            over: VarId(1),
        }],
        vec![ray_filter],
    ));
    (ops, naive_only)
}

/// Every algebra op for the naive differential slice, plus the count of
/// naive-only (`SQLite`-inexpressible) cases the caller reports —
/// enumerated, never silently skipped.
pub(super) fn algebra_ops(seed: u64, sizes: &Sizes) -> (Vec<Op>, u64) {
    let mut ops = rules_ops(sizes);
    ops.extend(dnf_ops(seed, sizes));
    let (rest, naive_only) = pack_and_measure_ops();
    ops.extend(rest);
    (ops, naive_only)
}

/// One error-parity expectation: the engine's typed validation verdict
/// against the naive model's own from-the-definition computation.
enum Expected {
    /// `DnfExceedsRules { produced, cap }` where `produced` equals the
    /// naive width and exceeds the cap.
    DnfCap { naive_width: usize },
    /// Every disjunct vanished (the naive width is zero): the empty
    /// union is not a query.
    Vanished,
    /// The vacuous "never" (the naive twin: `mask.is_empty()`).
    EmptyMask,
    /// The vacuous "always" (the naive twin: `mask.is_full()`).
    FullMask,
    /// The bind-time "never": prepare accepts the param mask, execution
    /// with an EMPTY binding raises the typed sibling (finding 086 —
    /// the bind-time rejection roster, previously unpinned by any
    /// parity row).
    EmptyMaskParam,
    /// The bind-time "always", `FULL` at execute.
    FullMaskParam,
    /// The fold-free nullary Count across 2+ written rules —
    /// definitionally constant 1 under the head-projection law, a
    /// typed refusal since R1 (the flipped acceptance row).
    CountAcrossRules { rules: usize },
}

/// The parity cases: the invalid shapes the roster owns, each paired
/// with the naive side's expectation.
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
            conditions: vec![leaf(
                CmpOp::Allen {
                    mask: MaskTerm::Literal(mask),
                },
                var(1),
                var(2),
            )],
        })
    };
    vec![
        {
            // One Or of 17 arms: width 17 > 16.
            let q = Query::single(posting_rule(vec![wide_or(17)]));
            let naive_width = dnf_width(&q.rules[0]);
            ("dnf cap (wide Or)", q, Expected::DnfCap { naive_width })
        },
        {
            // Conjoined Ors multiply: 5 × 4 = 20 > 16.
            let q = Query::single(posting_rule(vec![wide_or(5), wide_or(4)]));
            let naive_width = dnf_width(&q.rules[0]);
            ("dnf cap (product)", q, Expected::DnfCap { naive_width })
        },
        (
            // The empty disjunction is false; conjoined with anything the
            // rule vanishes, and a one-rule program vanishes whole.
            "vanished program (empty Or)",
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
            // The flipped R1 row: the once-accepted multi-rule nullary
            // Count (one Count per disjunct is the modeling answer).
            let count_head = || {
                vec![
                    FindTerm::Var(VarId(0)),
                    FindTerm::Aggregate {
                        op: AggOp::Count,
                        over: None,
                    },
                ]
            };
            let arm = |floor: i64| Rule {
                finds: count_head(),
                atoms: vec![posting_atom()],
                negated: vec![],
                conditions: vec![leaf(CmpOp::Ge, var(1), Term::Literal(Value::I64(floor)))],
            };
            let q = Query {
                head: arm(0).head(),
                rules: vec![arm(0), arm(1)],
            };
            let rules = q.rules.len();
            (
                "count across rules (R1)",
                q,
                Expected::CountAcrossRules { rules },
            )
        },
        (
            "vacuous mask param (EMPTY)",
            mask_param_query(),
            Expected::EmptyMaskParam,
        ),
        (
            "vacuous mask param (FULL)",
            mask_param_query(),
            Expected::FullMaskParam,
        ),
    ]
}

/// The [`parity_cases`] mask query with the mask as a bind-time param
/// (`MaskTerm::Param(0)`): validation accepts — vacuity is decided per
/// execution.
fn mask_param_query() -> Query {
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
        conditions: vec![leaf(
            CmpOp::Allen {
                mask: MaskTerm::Param(bumbledb::ParamId(0)),
            },
            var(1),
            var(2),
        )],
    })
}

/// Cap-exceeding DNF, the vanished program, and the vacuous masks:
/// both sides must reject, verdict identity included
/// (`docs/architecture/60-validation.md` § error parity). The engine's
/// verdict is its typed `ValidationError`; the naive side computes the
/// width / the mask cardinality from the definition and must agree on
/// the payload, not just the kind.
pub(super) fn error_parity<S, T>(db: &Db<S>, run: &mut Run<'_, T>) {
    for (label, q, expected) in parity_cases() {
        run.cases += 1;
        // The bind-time rows: prepare ACCEPTS the param mask; the
        // vacuous binding is a typed execution refusal carrying the
        // param id.
        if matches!(expected, Expected::EmptyMaskParam | Expected::FullMaskParam) {
            let (mask, empty) = match expected {
                Expected::EmptyMaskParam => (bumbledb::AllenMask::EMPTY, true),
                _ => (bumbledb::AllenMask::FULL, false),
            };
            match db.prepare(&q) {
                Ok(mut prepared) => {
                    let params = [crate::naive::ParamValue::Scalar(Value::AllenMask(mask))];
                    let args = crate::families::param_args(&params);
                    let outcome = db.read(|snap| snap.execute_collect_args(&mut prepared, &args));
                    let agree = match outcome {
                        Err(Error::EmptyAllenMaskParam { param }) => {
                            empty && param == bumbledb::ParamId(0)
                        }
                        Err(Error::FullAllenMaskParam { param }) => {
                            !empty && param == bumbledb::ParamId(0)
                        }
                        _ => false,
                    };
                    if !agree {
                        parity_bundle(
                            run,
                            label,
                            &q,
                            "the vacuous mask binding must raise its typed bind-time error",
                        );
                    }
                }
                Err(e) => {
                    parity_bundle(
                        run,
                        label,
                        &q,
                        &format!("prepare refused the bind-time shape: {e:?}"),
                    );
                }
            }
            if run.bundles.len() >= super::MAX_BUNDLES {
                return;
            }
            continue;
        }
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
                bumbledb::error::ValidationError::DnfExceedsRules { produced, cap }
                    if produced == naive_width && naive_width > cap
            ),
            // The vanished program surfaces as the empty union.
            Expected::Vanished => {
                dnf_width(&q.rules[0]) == 0
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
            Expected::EmptyMaskParam | Expected::FullMaskParam => {
                unreachable!("the bind-time rows are handled above")
            }
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
