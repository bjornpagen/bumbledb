//! The algebra oracle rows of the naive lane (one obligation per landed
//! representation, judged before anything is timed):
//!
//! - **Rules**: multi-rule programs replayed engine-vs-naive — the
//!   naive model evaluates the rules *directly* (union of per-rule
//!   binding sets, no engine sink mechanics) — disjoint vocabulary-selected
//!   arms, overlapping arms with duplicate head rows,
//!   and the multi-rule aggregate union fold.
//! - **DNF**: seeded random predicate trees to depth 3 — the naive
//!   model evaluates the *input tree*; the engine evaluates the lowered
//!   rules. The differential is the lowering proof, now inside every
//!   verify run.
//! - **`Pack`**: `SQLite` cannot express the coalescing fold
//!   ([`crate::translate::Inexpressible::PackAggregate`]) — these rows
//!   run **naive-only by decision**, counted and reported, never
//!   silently dropped.
//! - **The measure's rays**: `Duration` over the ray-bearing mandate
//!   corpus — `MeasureOfRay` on both sides (typed identity through the
//!   differential runner's `Rows` verdict), and the `Allen(DISJOINT)`
//!   ray filter keeping the same query answering rows.
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
        relation: ids::MANDATE,
        bindings: vec![
            (ids::mandate::ACCOUNT, var(0)),
            (ids::mandate::ACTIVE, var(1)),
        ],
    }
}

/// `Posting(account = v0, at = v1)`.
fn posting_atom() -> Atom {
    Atom {
        relation: ids::POSTING,
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
            relation: ids::JOURNAL_ENTRY,
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
        // The union folds: Count's constant-filler head position, and a
        // valued fold over the head projection.
        query(assemble(vec![
            posting_arm(
                AT_BASE,
                vec![
                    FindTerm::Var(VarId(0)),
                    FindTerm::Aggregate {
                        op: AggOp::Count,
                        over: None,
                    },
                ],
            ),
            posting_arm(
                AT_BASE + span / 3,
                vec![
                    FindTerm::Var(VarId(0)),
                    FindTerm::Aggregate {
                        op: AggOp::Count,
                        over: None,
                    },
                ],
            ),
        ])),
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
    (0..12)
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
        .collect()
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
            relation: ids::MANDATE,
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
    // differential runner; filtered by the ray probe, both answer rows.
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
        vec![FindTerm::Var(VarId(0)), FindTerm::Duration(VarId(1))],
        vec![],
    ));
    ops.push(measure(
        vec![FindTerm::Var(VarId(0)), FindTerm::Duration(VarId(1))],
        vec![ray_filter.clone()],
    ));
    ops.push(measure(
        vec![FindTerm::AggregateDuration {
            op: AggOp::Sum,
            over: VarId(1),
        }],
        vec![],
    ));
    ops.push(measure(
        vec![FindTerm::AggregateDuration {
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
                    relation: ids::MANDATE,
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
    ]
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
