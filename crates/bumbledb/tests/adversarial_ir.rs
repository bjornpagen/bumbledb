//! The adversarial-IR panic sweep (docs/architecture/20-query-ir.md § the
//! validation boundary — the trust-boundary law): queries arrive as data
//! — eventually foreign data — so **no panic is reachable from an
//! `ir::Query` value**. This property test drives structurally-random
//! MALFORMED queries (unknown ids, arity mismatches, duplicate rules,
//! cap-exceeders, vacuous masks, MAX-point literals, hostile nesting,
//! measure abuse, param-id gaps) through validate → normalize → prepare
//! and asserts every outcome is `Ok` or a typed error. Any panic is a red
//! run. `unreachable!` arms *downstream* of validation are exempt by
//! construction — the sweep's point is proving the guard total, so an
//! input that detonates one of them is a validation hole, and the sweep
//! reports the seed that found it.
//!
//! Two generator lanes, half the budget each: a fully random lane
//! (arbitrary shapes over hostile value/id distributions) and a
//! mutation lane (the querygen idea inverted — start from a plausible
//! query template and inject faults from the hostile catalog), so the
//! sweep both exercises the roster's rejections and drives *valid*
//! queries deep into the planner.

use std::panic::{AssertUnwindSafe, catch_unwind};

use bumbledb::{
    AggOp, AllenMask, Atom, CmpOp, Comparison, Db, FieldId, FindTerm, MAX_PREDICATE_DEPTH,
    MAX_RULES, MaskTerm, ParamId, PredicateTree, Query, RelationId, Rule, Term, Value, VarId,
};

mod common;

bumbledb::schema! {
    pub Gauntlet;

    closed relation Kind as KindId = { Meeting, Focus, Travel };

    relation Busy {
        id: u64 as ClaimId, fresh,
        person: u64,
        during: interval<u64>,
        kind: u64 as KindId,
        note: str,
        digest: bytes<16>,
        billable: bool,
        offset: i64,
        window: interval<i64>,
    }
    relation Ooo { person: u64, during: interval<u64> }

    Busy(kind) <= Kind(id);
}

/// The sweep budget: at least 10⁴ malformed queries (PRD 20's passing
/// criterion), split across the two lanes.
const SWEEP: u64 = 12_000;

/// xorshift64* — the hand-rolled generator (the engine crates carry no
/// randomness dependency, by the dependency law).
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(
            seed.wrapping_mul(2_654_435_761)
                .wrapping_add(0x9E37_79B9_7F4A_7C15),
        )
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform draw below `n` (n > 0; modulo bias is irrelevant here).
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    fn chance(&mut self, one_in: u64) -> bool {
        self.below(one_in) == 0
    }
}

// --- the fully random lane -------------------------------------------

/// A hostile relation id: usually a real one (the closed vocabulary
/// included), sometimes just past the roster, sometimes the far end of
/// the id space.
fn relation_id(rng: &mut Rng) -> RelationId {
    match rng.below(8) {
        0 => RelationId(3),
        1 => RelationId(u32::MAX),
        n => RelationId(u32::from(n % 2 == 0)),
    }
}

/// A hostile field id over the widest relation (Busy has 9 fields).
fn field_id(rng: &mut Rng) -> FieldId {
    match rng.below(12) {
        0 => FieldId(u16::MAX),
        1 => FieldId(9),
        n => FieldId(u16::try_from(n % 9).expect("small")),
    }
}

/// A literal over every `Value` variant, boundary shapes included:
/// domain ceilings, empty and ray intervals, non-UTF-8 strings,
/// wrong-width digests, row-id-shaped smalls straddling the closed
/// roster, and the mask value that is never a term.
fn value(rng: &mut Rng) -> Value {
    match rng.below(14) {
        0 => Value::Bool(rng.chance(2)),
        1 => Value::U64(rng.below(100)),
        2 => Value::U64(u64::MAX),
        3 => Value::U64(u64::MAX - 1),
        4 => Value::I64(i64::MAX),
        5 => Value::I64(-1),
        6 => Value::U64(rng.below(6)),
        7 => Value::String(Box::from(&b"note"[..])),
        8 => Value::String(Box::from(&[0xFF, 0xFE, 0x00][..])),
        9 => {
            let len = usize::try_from(rng.below(4) * 8 + rng.below(2)).expect("small");
            Value::FixedBytes(vec![0xAB; len].into_boxed_slice())
        }
        10 => {
            let start = rng.below(50);
            // start < end, start == end, and start > end all occur.
            let end = rng.below(60);
            Value::IntervalU64(start, end)
        }
        11 => Value::IntervalU64(rng.below(10), u64::MAX), // the ray
        12 => Value::IntervalI64(-5, i64::MAX),
        13 => Value::AllenMask(AllenMask::DISJOINT),
        _ => unreachable!("below(14)"),
    }
}

/// A term over every kind: variables and params from a small pool (so
/// joins happen), occasionally far ids (so param-id gaps and unbound
/// variables happen), measures anywhere.
fn term(rng: &mut Rng) -> Term {
    match rng.below(10) {
        0..=2 => Term::Var(VarId(u16::try_from(rng.below(5)).expect("small"))),
        3 => Term::Var(VarId(999)),
        4 => Term::Param(ParamId(u16::try_from(rng.below(3)).expect("small"))),
        5 => Term::Param(ParamId(40)), // a param-id gap
        6 => Term::ParamSet(ParamId(u16::try_from(rng.below(3)).expect("small"))),
        7 => Term::Duration(VarId(u16::try_from(rng.below(5)).expect("small"))),
        _ => Term::Literal(value(rng)),
    }
}

fn atom(rng: &mut Rng) -> Atom {
    let bindings = (0..rng.below(4))
        .map(|_| (field_id(rng), term(rng)))
        .collect();
    Atom {
        relation: relation_id(rng),
        bindings,
    }
}

fn cmp_op(rng: &mut Rng) -> CmpOp {
    match rng.below(9) {
        0 => CmpOp::Eq,
        1 => CmpOp::Ne,
        2 => CmpOp::Lt,
        3 => CmpOp::Le,
        4 => CmpOp::Gt,
        5 => CmpOp::Ge,
        6 => CmpOp::Contains,
        7 => CmpOp::Allen {
            mask: MaskTerm::Param(ParamId(u16::try_from(rng.below(3)).expect("small"))),
        },
        8 => CmpOp::Allen {
            // ∅ and full both occur (the vacuity rejections), plus
            // arbitrary 13-bit masks.
            mask: MaskTerm::Literal(match rng.below(4) {
                0 => AllenMask::EMPTY,
                1 => AllenMask::FULL,
                2 => AllenMask::INTERSECTS,
                _ => AllenMask::new(u16::try_from(rng.below(1 << 13)).expect("13 bits"))
                    .expect("13-bit mask"),
            }),
        },
        _ => unreachable!("below(9)"),
    }
}

fn comparison(rng: &mut Rng) -> Comparison {
    Comparison {
        op: cmp_op(rng),
        lhs: term(rng),
        rhs: term(rng),
    }
}

/// A predicate tree with hostile nesting: leaves mostly, `And`/`Or`
/// nodes (empty child lists included) down to a bounded depth.
fn tree(rng: &mut Rng, depth: u64) -> PredicateTree {
    if depth == 0 || rng.chance(2) {
        return PredicateTree::Leaf(comparison(rng));
    }
    let children = (0..rng.below(4)).map(|_| tree(rng, depth - 1)).collect();
    if rng.chance(2) {
        PredicateTree::And(children)
    } else {
        PredicateTree::Or(children)
    }
}

fn find_term(rng: &mut Rng) -> FindTerm {
    let var = |rng: &mut Rng| VarId(u16::try_from(rng.below(5)).expect("small"));
    let agg_op = |rng: &mut Rng| match rng.below(8) {
        0 => AggOp::Sum,
        1 => AggOp::Min,
        2 => AggOp::Max,
        3 => AggOp::Count,
        4 => AggOp::CountDistinct,
        5 => AggOp::Pack,
        6 => AggOp::ArgMax { key: VarId(1) },
        _ => AggOp::ArgMin { key: VarId(999) },
    };
    match rng.below(6) {
        0..=2 => FindTerm::Var(var(rng)),
        3 => FindTerm::Duration(var(rng)),
        4 => FindTerm::Aggregate {
            op: agg_op(rng),
            over: rng.chance(4).then(|| var(rng)),
        },
        _ => FindTerm::AggregateDuration {
            op: agg_op(rng),
            over: var(rng),
        },
    }
}

fn random_rule(rng: &mut Rng) -> Rule {
    Rule {
        finds: (0..rng.below(4)).map(|_| find_term(rng)).collect(),
        atoms: (0..rng.below(4)).map(|_| atom(rng)).collect(),
        negated: (0..rng.below(3)).map(|_| atom(rng)).collect(),
        predicates: (0..rng.below(3)).map(|_| tree(rng, 4)).collect(),
    }
}

fn random_query(rng: &mut Rng) -> Query {
    let rules: Vec<Rule> = (0..rng.below(4)).map(|_| random_rule(rng)).collect();
    // The head sometimes agrees with rule 0 (deeper reach) and sometimes
    // is independently random (arity/shape mismatches).
    let head = match rules.first() {
        Some(rule) if rng.chance(2) => rule.head(),
        _ => (0..rng.below(4))
            .map(|_| find_term(rng).head_term())
            .collect(),
    };
    Query { head, rules }
}

// --- the mutation lane -------------------------------------------------

/// Busy's declaration-order ids, through the macro's emitted constants —
/// the sweep is also a consumer of PRD 20's named data.
const BUSY: RelationId = Gauntlet::BUSY;
const OOO: RelationId = Gauntlet::OOO;

/// One plausible query: a valid template drawn from the workload shapes
/// (projection+Allen, union, aggregate, Pack, the measure, negation with
/// selection and membership).
fn plausible_query(rng: &mut Rng) -> Query {
    let busy_atom = |bindings: Vec<(FieldId, Term)>| Atom {
        relation: BUSY,
        bindings,
    };
    let projection = |relation: RelationId, person: FieldId, during: FieldId| Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            relation,
            bindings: vec![(person, Term::Var(VarId(0))), (during, Term::Var(VarId(1)))],
        }],
        negated: vec![],
        predicates: vec![PredicateTree::Leaf(Comparison {
            op: CmpOp::Allen {
                mask: MaskTerm::Literal(AllenMask::INTERSECTS),
            },
            lhs: Term::Var(VarId(1)),
            rhs: Term::Param(ParamId(0)),
        })],
    };
    match rng.below(6) {
        // Busy ⋈ window, projected.
        0 => Query::single(projection(
            BUSY,
            Gauntlet::BUSY_PERSON,
            Gauntlet::BUSY_DURING,
        )),
        // The union: unavailability is Busy ∪ Ooo against one window.
        1 => {
            let busy = projection(BUSY, Gauntlet::BUSY_PERSON, Gauntlet::BUSY_DURING);
            let ooo = projection(OOO, Gauntlet::OOO_PERSON, Gauntlet::OOO_DURING);
            Query {
                head: busy.head(),
                rules: vec![busy, ooo],
            }
        }
        // Aggregate: balance-by-person over the i64 offset.
        2 => Query::single(Rule {
            finds: vec![
                FindTerm::Var(VarId(0)),
                FindTerm::Aggregate {
                    op: AggOp::Sum,
                    over: Some(VarId(1)),
                },
            ],
            atoms: vec![busy_atom(vec![
                (Gauntlet::BUSY_PERSON, Term::Var(VarId(0))),
                (Gauntlet::BUSY_OFFSET, Term::Var(VarId(1))),
            ])],
            negated: vec![],
            predicates: vec![],
        }),
        // Pack: the coalesced calendar.
        3 => Query::single(Rule {
            finds: vec![
                FindTerm::Var(VarId(0)),
                FindTerm::Aggregate {
                    op: AggOp::Pack,
                    over: Some(VarId(1)),
                },
            ],
            atoms: vec![busy_atom(vec![
                (Gauntlet::BUSY_PERSON, Term::Var(VarId(0))),
                (Gauntlet::BUSY_DURING, Term::Var(VarId(1))),
            ])],
            negated: vec![],
            predicates: vec![],
        }),
        // The measure, projected and compared.
        4 => Query::single(Rule {
            finds: vec![FindTerm::Var(VarId(0)), FindTerm::Duration(VarId(1))],
            atoms: vec![busy_atom(vec![
                (Gauntlet::BUSY_PERSON, Term::Var(VarId(0))),
                (Gauntlet::BUSY_DURING, Term::Var(VarId(1))),
            ])],
            negated: vec![],
            predicates: vec![PredicateTree::Leaf(Comparison {
                op: CmpOp::Ge,
                lhs: Term::Duration(VarId(1)),
                rhs: Term::Literal(Value::U64(rng.below(10_000))),
            })],
        }),
        // Negation + selection + membership.
        _ => Query::single(Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![busy_atom(vec![
                (Gauntlet::BUSY_PERSON, Term::Var(VarId(0))),
                (Gauntlet::BUSY_DURING, Term::Var(VarId(1))),
                (Gauntlet::BUSY_KIND, Term::Literal(Value::U64(rng.below(3)))),
            ])],
            negated: vec![Atom {
                relation: OOO,
                bindings: vec![(Gauntlet::OOO_PERSON, Term::Var(VarId(0)))],
            }],
            predicates: vec![PredicateTree::Leaf(Comparison {
                op: CmpOp::Contains,
                lhs: Term::Var(VarId(1)),
                rhs: Term::Literal(Value::U64(rng.below(100))),
            })],
        }),
    }
}

/// The hostile catalog: one fault injected into a query in place — the
/// querygen machinery inverted (generate *invalid* shapes deliberately).
#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)] // the catalog: one arm per fault class
fn mutate(rng: &mut Rng, query: &mut Query) {
    match rng.below(16) {
        // Unknown relation id.
        0 => {
            if let Some(atom) = query.rules.first_mut().and_then(|r| r.atoms.first_mut()) {
                atom.relation = RelationId(if rng.chance(2) { 3 } else { u32::MAX });
            }
        }
        // Unknown field id.
        1 => {
            if let Some((field, _)) = query
                .rules
                .first_mut()
                .and_then(|r| r.atoms.first_mut())
                .and_then(|a| a.bindings.first_mut())
            {
                *field = FieldId(if rng.chance(2) { 9 } else { u16::MAX });
            }
        }
        // Duplicate rule.
        2 => {
            if let Some(rule) = query.rules.first().cloned() {
                query.rules.push(rule);
            }
        }
        // Head arity mismatch.
        3 => {
            if let Some(rule) = query.rules.first_mut() {
                rule.finds.push(FindTerm::Var(VarId(0)));
            }
        }
        // Rule cap + 1.
        4 => {
            if let Some(rule) = query.rules.first().cloned() {
                while query.rules.len() <= MAX_RULES {
                    query.rules.push(rule.clone());
                }
            }
        }
        // The vacuous masks.
        5 => {
            if let Some(rule) = query.rules.first_mut() {
                rule.predicates.push(PredicateTree::Leaf(Comparison {
                    op: CmpOp::Allen {
                        mask: MaskTerm::Literal(if rng.chance(2) {
                            AllenMask::EMPTY
                        } else {
                            AllenMask::FULL
                        }),
                    },
                    lhs: Term::Var(VarId(1)),
                    rhs: Term::Var(VarId(1)),
                }));
            }
        }
        // A MAX-point literal at an interval position (membership).
        6 => {
            if let Some(atom) = query.rules.first_mut().and_then(|r| r.atoms.first_mut()) {
                atom.bindings
                    .push((Gauntlet::BUSY_DURING, Term::Literal(Value::U64(u64::MAX))));
            }
        }
        // An empty interval literal.
        7 => {
            if let Some(atom) = query.rules.first_mut().and_then(|r| r.atoms.first_mut()) {
                atom.bindings.push((
                    Gauntlet::BUSY_DURING,
                    Term::Literal(Value::IntervalU64(7, 7)),
                ));
            }
        }
        // The DNF blowup: wide Or of Ands past the cap.
        8 => {
            if let Some(rule) = query.rules.first_mut() {
                let leaf = || {
                    PredicateTree::Leaf(Comparison {
                        op: CmpOp::Ge,
                        lhs: Term::Var(VarId(0)),
                        rhs: Term::Literal(Value::U64(1)),
                    })
                };
                let or = PredicateTree::Or((0..5).map(|_| leaf()).collect());
                rule.predicates = vec![or.clone(), or];
            }
        }
        // Hostile nesting: a deep And/Or chain.
        9 => {
            if let Some(rule) = query.rules.first_mut() {
                let mut chain = PredicateTree::Leaf(Comparison {
                    op: CmpOp::Ge,
                    lhs: Term::Var(VarId(0)),
                    rhs: Term::Literal(Value::U64(1)),
                });
                for level in 0..200 {
                    chain = if level % 2 == 0 {
                        PredicateTree::And(vec![chain])
                    } else {
                        PredicateTree::Or(vec![chain])
                    };
                }
                rule.predicates.push(chain);
            }
        }
        // A param-id gap.
        10 => {
            if let Some(atom) = query.rules.first_mut().and_then(|r| r.atoms.first_mut()) {
                atom.bindings
                    .push((Gauntlet::BUSY_NOTE, Term::Param(ParamId(7))));
            }
        }
        // The measure in a binding position.
        11 => {
            if let Some(atom) = query.rules.first_mut().and_then(|r| r.atoms.first_mut()) {
                atom.bindings
                    .push((Gauntlet::BUSY_PERSON, Term::Duration(VarId(1))));
            }
        }
        // The empty program / the empty head.
        12 => {
            if rng.chance(2) {
                query.rules.clear();
            } else {
                query.head.clear();
                for rule in &mut query.rules {
                    rule.finds.clear();
                }
            }
        }
        // Occurrence cap + 1 (negated occurrences count too).
        13 => {
            if let Some(rule) = query.rules.first_mut() {
                let gate = Atom {
                    relation: OOO,
                    bindings: vec![],
                };
                for _ in 0..21 {
                    if rng.chance(4) {
                        rule.negated.push(gate.clone());
                    } else {
                        rule.atoms.push(gate.clone());
                    }
                }
            }
        }
        // Distinct-variable cap + 1: 15 wide atoms bind 135 distinct
        // variables while staying under the occurrence cap, so the
        // variable roster item is the one that fires.
        14 => {
            if let Some(rule) = query.rules.first_mut() {
                for atom_idx in 0..15u16 {
                    rule.atoms.push(Atom {
                        relation: BUSY,
                        bindings: (0..9u16)
                            .map(|field| {
                                (FieldId(field), Term::Var(VarId(100 + atom_idx * 9 + field)))
                            })
                            .collect(),
                    });
                }
            }
        }
        // A random term swapped into a random binding.
        _ => {
            if let Some(atom) = query.rules.first_mut().and_then(|r| r.atoms.first_mut())
                && let Some((_, term_slot)) = atom.bindings.first_mut()
            {
                *term_slot = term(rng);
            }
        }
    }
}

// --- the sweep ----------------------------------------------------------

#[test]
fn adversarial_ir_never_panics() {
    let dir = common::TempDir::new("adversarial-ir");
    let db = Db::create(dir.path(), Gauntlet).expect("create");

    let mut ok = 0u64;
    let mut rejected = 0u64;
    for seed in 0..SWEEP {
        let mut rng = Rng::new(seed);
        let query = if seed % 2 == 0 {
            random_query(&mut rng)
        } else {
            let mut query = plausible_query(&mut rng);
            for _ in 0..rng.below(3) {
                mutate(&mut rng, &mut query);
            }
            query
        };
        // The law under test: validate → normalize → prepare returns Ok
        // or a typed error on arbitrary input — no panic is reachable
        // from IR data.
        let outcome = catch_unwind(AssertUnwindSafe(|| db.prepare(&query).map(|_| ())));
        // A caught unwind IS the red case — there is no error to match:
        // the panic payload already printed through the hook.
        #[expect(
            clippy::match_wild_err_arm,
            reason = "the test intentionally rejects every non-target error uniformly"
        )]
        match outcome {
            Ok(Ok(())) => ok += 1,
            Ok(Err(_)) => rejected += 1,
            Err(_) => panic!(
                "prepare panicked on IR data (seed {seed}) — the trust-boundary law is \
                 violated by:\n{}\n{query:#?}",
                db.render_query(&query)
            ),
        }
    }
    // The sweep must exercise both sides of the boundary: some queries
    // reach the planner whole, most are typed rejections — a lane that
    // produced neither would be a vacuous run.
    assert!(ok > 0, "no generated query validated — vacuous sweep");
    assert!(
        rejected > 0,
        "no generated query was rejected — vacuous sweep"
    );
    assert_eq!(ok + rejected, SWEEP);
}

/// Hostile nesting alone, far past the sweep's per-query depth: a deep
/// alternating And/Or chain is the typed `PredicateNestingTooDeep` —
/// judged iteratively, so neither validation nor distribution ever
/// recurses into it (the sweep's founding find: before the boundary
/// guard existed, this input exhausted the stack).
#[test]
fn deep_predicate_nesting_is_a_typed_rejection() {
    let dir = common::TempDir::new("adversarial-ir-nesting");
    let db = Db::create(dir.path(), Gauntlet).expect("create");
    let leaf = || {
        PredicateTree::Leaf(Comparison {
            op: CmpOp::Ge,
            lhs: Term::Var(VarId(0)),
            rhs: Term::Literal(Value::U64(1)),
        })
    };
    let chain = |depth: usize| {
        let mut tree = leaf();
        for level in 1..depth {
            tree = if level % 2 == 0 {
                PredicateTree::And(vec![tree])
            } else {
                PredicateTree::Or(vec![tree])
            };
        }
        tree
    };
    let query = |tree: PredicateTree| {
        Query::single(Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                relation: OOO,
                bindings: vec![(Gauntlet::OOO_PERSON, Term::Var(VarId(0)))],
            }],
            negated: vec![],
            predicates: vec![tree],
        })
    };
    // Past the cap: the typed rejection, never a stack exhaustion.
    let err = db
        .prepare(&query(chain(3_000)))
        .map(|_| ())
        .expect_err("hostile nesting is rejected");
    assert!(
        matches!(
            err,
            bumbledb::Error::Validation(
                bumbledb::error::ValidationError::PredicateNestingTooDeep {
                    depth: 3_000,
                    cap: MAX_PREDICATE_DEPTH,
                    ..
                }
            )
        ),
        "{err:?}"
    );
    // At the cap: an ordinary query (the chain is one disjunct).
    let _ = db
        .prepare(&query(chain(MAX_PREDICATE_DEPTH)))
        .expect("cap-deep nesting is an ordinary query");
}
