//! measure abuse, param-id gaps) through validate → normalize → prepare
use std::panic::{AssertUnwindSafe, catch_unwind};

use bumbledb::{
    AllenMask, Atom, AtomSource, CmpOp, Comparison, ConditionTree, Db, FieldId, FindTerm, FoldOp,
    HeadTerm, Interior, InteriorId, MAX_CONDITION_DEPTH, MAX_RULES, NonEmpty, ParamId,
    ProjectionRule, Query, Rec, RecRule, RecStep, RelationId, Rule, Term, Value, VarId,
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

const SWEEP: u64 = 12_000;

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

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    fn chance(&mut self, one_in: u64) -> bool {
        self.below(one_in) == 0
    }
}

fn relation_id(rng: &mut Rng) -> RelationId {
    match rng.below(8) {
        0 => RelationId(3),
        1 => RelationId(u32::MAX),
        n => RelationId(u32::from(n % 2 == 0)),
    }
}

fn field_id(rng: &mut Rng) -> FieldId {
    match rng.below(12) {
        0 => FieldId(u16::MAX),
        1 => FieldId(9),
        n => FieldId(u16::try_from(n % 9).expect("small")),
    }
}

fn value(rng: &mut Rng) -> Value {
    match rng.below(13) {
        0 => Value::Bool(rng.chance(2)),
        1 => Value::U64(rng.below(100)),
        2 => Value::U64(u64::MAX),
        3 => Value::U64(u64::MAX - 1),
        4 => Value::I64(i64::MAX),
        5 => Value::I64(-1),
        6 => Value::U64(rng.below(6)),
        7 => Value::String(Box::from("note")),
        8 => Value::String(Box::from("ghost")),
        9 => {
            let len = usize::try_from(rng.below(4) * 8 + rng.below(2)).expect("small");
            Value::FixedBytes(vec![0xAB; len].into_boxed_slice())
        }
        10 => {
            let start = rng.below(50);
            let end = start + rng.below(10) + 1;
            Value::IntervalU64(
                bumbledb::Interval::<u64>::new(start, end).expect("nonempty interval"),
            )
        }
        11 => Value::IntervalU64(
            bumbledb::Interval::<u64>::ray(rng.below(10)).expect("ray start is below MAX"),
        ),
        12 => Value::IntervalI64(
            bumbledb::Interval::<i64>::new(-5, i64::MAX).expect("nonempty interval"),
        ),
        _ => unreachable!("below(13)"),
    }
}

fn term(rng: &mut Rng) -> Term {
    match rng.below(9) {
        0..=2 => Term::Var(VarId(u16::try_from(rng.below(5)).expect("small"))),
        3 => Term::Var(VarId(999)),
        4 => Term::Param(ParamId(u16::try_from(rng.below(3)).expect("small"))),
        5 => Term::Param(ParamId(40)),
        6 => Term::ParamSet(ParamId(u16::try_from(rng.below(3)).expect("small"))),
        _ => Term::Literal(value(rng)),
    }
}

fn atom(rng: &mut Rng) -> Atom {
    let bindings = (0..rng.below(4))
        .map(|_| (field_id(rng), term(rng)))
        .collect();
    Atom {
        source: atom_source(rng),
        bindings,
    }
}

fn atom_source(rng: &mut Rng) -> AtomSource {
    if rng.chance(4) {
        let id = match rng.below(4) {
            0 => 0,
            1 => 1,
            2 => 4,
            _ => u32::MAX,
        };
        AtomSource::Interior(InteriorId(id))
    } else {
        AtomSource::Edb(relation_id(rng))
    }
}

fn cmp_op(rng: &mut Rng) -> CmpOp {
    match rng.below(8) {
        0 => CmpOp::Eq,
        1 => CmpOp::Ne,
        2 => CmpOp::Lt,
        3 => CmpOp::Le,
        4 => CmpOp::Gt,
        5 => CmpOp::Ge,
        6 => CmpOp::PointIn,
        7 => CmpOp::Allen {
            mask: match rng.below(4) {
                0 => AllenMask::EMPTY,
                1 => AllenMask::FULL,
                2 => AllenMask::INTERSECTS,
                _ => AllenMask::new(u16::try_from(rng.below(1 << 13)).expect("13 bits"))
                    .expect("13-bit mask"),
            },
        },
        _ => unreachable!("below(8)"),
    }
}

fn comparison(rng: &mut Rng) -> Comparison {
    Comparison {
        op: cmp_op(rng),
        lhs: term(rng),
        rhs: term(rng),
    }
}

fn tree(rng: &mut Rng, depth: u64) -> ConditionTree {
    if depth == 0 || rng.chance(2) {
        return ConditionTree::Leaf(comparison(rng));
    }
    let children = (0..rng.below(4)).map(|_| tree(rng, depth - 1)).collect();
    if rng.chance(2) {
        ConditionTree::And(children)
    } else {
        ConditionTree::Or(children)
    }
}

fn find_term(rng: &mut Rng) -> FindTerm {
    let var = |rng: &mut Rng| VarId(u16::try_from(rng.below(5)).expect("small"));
    let fold_op = |rng: &mut Rng| match rng.below(3) {
        0 => FoldOp::Sum,
        1 => FoldOp::Min,
        _ => FoldOp::Max,
    };
    match rng.below(5) {
        0..=1 => FindTerm::Var(var(rng)),
        2 => FindTerm::Count,
        3 => FindTerm::Aggregate {
            op: fold_op(rng),
            over: var(rng),
        },
        _ => FindTerm::Pack { over: var(rng) },
    }
}

fn random_rule(rng: &mut Rng) -> Rule {
    Rule {
        finds: (0..rng.below(4)).map(|_| find_term(rng)).collect(),
        atoms: (0..rng.below(4)).map(|_| atom(rng)).collect(),
        negated: (0..rng.below(3)).map(|_| atom(rng)).collect(),
        conditions: (0..rng.below(3)).map(|_| tree(rng, 4)).collect(),
    }
}

fn random_query(rng: &mut Rng) -> Query {
    let rules: Vec<Rule> = (0..rng.below(4)).map(|_| random_rule(rng)).collect();

    let head = match rules.first() {
        Some(rule) if rng.chance(2) => rule.head(),
        _ => (0..rng.below(4))
            .map(|_| find_term(rng).head_term())
            .collect(),
    };
    let interiors = if rng.chance(4) {
        (0..rng.below(4)).map(|_| random_interior(rng)).collect()
    } else {
        vec![]
    };
    if rng.chance(5) {
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

fn random_interior(rng: &mut Rng) -> Interior {
    Interior {
        rules: (0..rng.below(3))
            .map(|_| ProjectionRule {
                finds: (0..rng.below(3))
                    .map(|_| VarId(u16::try_from(rng.below(5)).expect("small")))
                    .collect(),
                atoms: (0..rng.below(4)).map(|_| atom(rng)).collect(),
                negated: (0..rng.below(3)).map(|_| atom(rng)).collect(),
                conditions: (0..rng.below(3)).map(|_| tree(rng, 4)).collect(),
            })
            .collect(),
    }
}

fn random_rec(rng: &mut Rng) -> Rec {
    let n = |rng: &mut Rng| rng.below(3).max(1);
    Rec {
        base: NonEmpty::from_vec(
            (0..n(rng))
                .map(|_| RecRule {
                    finds: (0..rng.below(3))
                        .map(|_| VarId(u16::try_from(rng.below(5)).expect("small")))
                        .collect(),
                    atoms: (0..rng.below(4)).map(|_| atom(rng)).collect(),
                    conditions: (0..rng.below(3)).map(|_| tree(rng, 4)).collect(),
                })
                .collect(),
        )
        .expect("nonempty base"),
        rec: NonEmpty::from_vec(
            (0..n(rng))
                .map(|_| RecStep {
                    finds: (0..rng.below(3))
                        .map(|_| VarId(u16::try_from(rng.below(5)).expect("small")))
                        .collect(),
                    self_bindings: (0..rng.below(3))
                        .map(|_| {
                            (
                                FieldId(u16::try_from(rng.below(4)).expect("small")),
                                Term::Var(VarId(u16::try_from(rng.below(5)).expect("small"))),
                            )
                        })
                        .collect(),
                    atoms: (0..rng.below(4)).map(|_| atom(rng)).collect(),
                    conditions: (0..rng.below(3)).map(|_| tree(rng, 4)).collect(),
                })
                .collect(),
        )
        .expect("nonempty rec"),
    }
}

const BUSY: RelationId = Gauntlet::BUSY;
const OOO: RelationId = Gauntlet::OOO;

fn plausible_query(rng: &mut Rng) -> Query {
    let busy_atom = |bindings: Vec<(FieldId, Term)>| Atom {
        source: bumbledb::AtomSource::Edb(BUSY),
        bindings,
    };
    let projection = |relation: RelationId, person: FieldId, during: FieldId| Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(relation),
            bindings: vec![(person, Term::Var(VarId(0))), (during, Term::Var(VarId(1)))],
        }],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Allen {
                mask: AllenMask::INTERSECTS,
            },
            lhs: Term::Var(VarId(1)),
            rhs: Term::Param(ParamId(0)),
        })],
    };
    match rng.below(6) {
        0 => Query::single(projection(
            BUSY,
            Gauntlet::BUSY_PERSON,
            Gauntlet::BUSY_DURING,
        )),

        1 => {
            let busy = projection(BUSY, Gauntlet::BUSY_PERSON, Gauntlet::BUSY_DURING);
            let ooo = projection(OOO, Gauntlet::OOO_PERSON, Gauntlet::OOO_DURING);
            Query {
                interiors: vec![],
                head: busy.head(),
                rules: vec![busy, ooo],
                rec: None,
            }
        }

        2 => Query::single(Rule {
            finds: vec![
                FindTerm::Var(VarId(0)),
                FindTerm::Aggregate {
                    op: FoldOp::Sum,
                    over: VarId(1),
                },
            ],
            atoms: vec![busy_atom(vec![
                (Gauntlet::BUSY_PERSON, Term::Var(VarId(0))),
                (Gauntlet::BUSY_OFFSET, Term::Var(VarId(1))),
            ])],
            negated: vec![],
            conditions: vec![],
        }),

        3 => Query::single(Rule {
            finds: vec![FindTerm::Var(VarId(0)), FindTerm::Pack { over: VarId(1) }],
            atoms: vec![busy_atom(vec![
                (Gauntlet::BUSY_PERSON, Term::Var(VarId(0))),
                (Gauntlet::BUSY_DURING, Term::Var(VarId(1))),
            ])],
            negated: vec![],
            conditions: vec![],
        }),

        _ => Query::single(Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![busy_atom(vec![
                (Gauntlet::BUSY_PERSON, Term::Var(VarId(0))),
                (Gauntlet::BUSY_DURING, Term::Var(VarId(1))),
                (Gauntlet::BUSY_KIND, Term::Literal(Value::U64(rng.below(3)))),
            ])],
            negated: vec![Atom {
                source: bumbledb::AtomSource::Edb(OOO),
                bindings: vec![(Gauntlet::OOO_PERSON, Term::Var(VarId(0)))],
            }],
            conditions: vec![ConditionTree::Leaf(Comparison {
                op: CmpOp::PointIn,
                lhs: Term::Var(VarId(1)),
                rhs: Term::Literal(Value::U64(rng.below(100))),
            })],
        }),
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)]
fn mutate(rng: &mut Rng, query: &mut Query) {
    match rng.below(16) {
        0 => {
            if let Some(atom) = query
                .rules_mut()
                .first_mut()
                .and_then(|r| r.atoms.first_mut())
            {
                atom.source =
                    bumbledb::AtomSource::Edb(RelationId(if rng.chance(2) { 3 } else { u32::MAX }));
            }
        }

        1 => {
            if let Some((field, _)) = query
                .rules_mut()
                .first_mut()
                .and_then(|r| r.atoms.first_mut())
                .and_then(|a| a.bindings.first_mut())
            {
                *field = FieldId(if rng.chance(2) { 9 } else { u16::MAX });
            }
        }

        2 => {
            if let Some(rule) = query.rules_mut().first().cloned() {
                query.rules_mut().push(rule);
            }
        }

        3 => {
            if let Some(rule) = query.rules_mut().first_mut() {
                rule.finds.push(FindTerm::Var(VarId(0)));
            }
        }

        4 => {
            if let Some(rule) = query.rules_mut().first().cloned() {
                while query.rules_mut().len() <= MAX_RULES {
                    query.rules_mut().push(rule.clone());
                }
            }
        }

        5 => {
            if let Some(rule) = query.rules_mut().first_mut() {
                rule.conditions.push(ConditionTree::Leaf(Comparison {
                    op: CmpOp::Allen {
                        mask: if rng.chance(2) {
                            AllenMask::EMPTY
                        } else {
                            AllenMask::FULL
                        },
                    },
                    lhs: Term::Var(VarId(1)),
                    rhs: Term::Var(VarId(1)),
                }));
            }
        }

        6 => {
            if let Some(atom) = query
                .rules_mut()
                .first_mut()
                .and_then(|r| r.atoms.first_mut())
            {
                atom.bindings
                    .push((Gauntlet::BUSY_DURING, Term::Literal(Value::U64(u64::MAX))));
            }
        }

        7 => {
            assert!(bumbledb::Interval::<u64>::new(7, 7).is_none());
        }

        8 => {
            if let Some(rule) = query.rules_mut().first_mut() {
                let leaf = || {
                    ConditionTree::Leaf(Comparison {
                        op: CmpOp::Ge,
                        lhs: Term::Var(VarId(0)),
                        rhs: Term::Literal(Value::U64(1)),
                    })
                };
                let or = ConditionTree::Or((0..5).map(|_| leaf()).collect());
                rule.conditions = vec![or.clone(), or];
            }
        }

        9 => {
            if let Some(rule) = query.rules_mut().first_mut() {
                let mut chain = ConditionTree::Leaf(Comparison {
                    op: CmpOp::Ge,
                    lhs: Term::Var(VarId(0)),
                    rhs: Term::Literal(Value::U64(1)),
                });
                for level in 0..200 {
                    chain = if level % 2 == 0 {
                        ConditionTree::And(vec![chain])
                    } else {
                        ConditionTree::Or(vec![chain])
                    };
                }
                rule.conditions.push(chain);
            }
        }

        10 => {
            if let Some(atom) = query
                .rules_mut()
                .first_mut()
                .and_then(|r| r.atoms.first_mut())
            {
                atom.bindings
                    .push((Gauntlet::BUSY_NOTE, Term::Param(ParamId(7))));
            }
        }

        12 => {
            if rng.chance(2) {
                query.rules_mut().clear();
            } else {
                query.head_mut().clear();
                for rule in query.rules_mut() {
                    rule.finds.clear();
                }
            }
        }

        13 => {
            if let Some(rule) = query.rules_mut().first_mut() {
                let gate = Atom {
                    source: bumbledb::AtomSource::Edb(OOO),
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

        14 => {
            if let Some(rule) = query.rules_mut().first_mut() {
                for atom_idx in 0..15u16 {
                    rule.atoms.push(Atom {
                        source: bumbledb::AtomSource::Edb(BUSY),
                        bindings: (0..9u16)
                            .map(|field| {
                                (FieldId(field), Term::Var(VarId(100 + atom_idx * 9 + field)))
                            })
                            .collect(),
                    });
                }
            }
        }

        _ => {
            if let Some(atom) = query
                .rules_mut()
                .first_mut()
                .and_then(|r| r.atoms.first_mut())
                && let Some((_, term_slot)) = atom.bindings.first_mut()
            {
                *term_slot = term(rng);
            }
        }
    }
}

#[test]
fn adversarial_ir_never_panics() {
    let dir = common::TempDir::new("adversarial-ir");
    let db = Db::create(dir.path(), Gauntlet)
        .expect("create")
        .expect("accepted");

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

        let outcome = catch_unwind(AssertUnwindSafe(|| db.prepare(&query).map(|_| ())));

        #[expect(
            clippy::match_wild_err_arm,
            reason = "the test intentionally rejects every non-target error uniformly"
        )]
        match outcome {
            Ok(Ok(())) => ok += 1,
            Ok(Err(_)) => rejected += 1,
            Err(_) => panic!(
                "prepare panicked on IR data (seed {seed}) — the trust-boundary law is \
                 violated by:\n{query:#?}"
            ),
        }
    }

    assert!(ok > 0, "no generated query validated — vacuous sweep");
    assert!(
        rejected > 0,
        "no generated query was rejected — vacuous sweep"
    );
    assert_eq!(ok + rejected, SWEEP);
}

#[test]
fn adversarial_query_with_interiors_never_panics() {
    let dir = common::TempDir::new("adversarial-interiors");
    let db = Db::create(dir.path(), Gauntlet)
        .expect("create")
        .expect("accepted");

    let mut ok = 0u64;
    let mut rejected = 0u64;
    for seed in 0..SWEEP / 2 {
        let mut rng = Rng::new(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let query = if seed % 2 == 0 {
            random_query(&mut rng)
        } else {
            let mut query = plausible_query(&mut rng);
            for _ in 0..rng.below(3) {
                mutate(&mut rng, &mut query);
            }
            if rng.chance(2) {
                let target = InteriorId(u32::try_from(rng.below(3)).expect("small"));
                let read = Atom {
                    source: AtomSource::Interior(target),
                    bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
                };
                let rules = query.rules_mut();
                let slot =
                    usize::try_from(rng.below(u64::try_from(rules.len()).expect("small").max(1)))
                        .expect("small");
                if let Some(rule) = rules.get_mut(slot) {
                    if rng.chance(2) {
                        rule.negated.push(read);
                    } else {
                        rule.atoms.push(read);
                    }
                }
            }
            if rng.chance(3) {
                query.interiors_mut().push(random_interior(&mut rng));
            }
            if rng.chance(4) {
                query.rec = Some(random_rec(&mut rng));
            }
            query
        };
        let rendered = format!("{query:#?}");
        let outcome = catch_unwind(AssertUnwindSafe(|| db.prepare(&query).map(|_| ())));
        #[expect(
            clippy::match_wild_err_arm,
            reason = "the test intentionally rejects every non-target error uniformly"
        )]
        match outcome {
            Ok(Ok(())) => ok += 1,
            Ok(Err(err)) => {
                let msg = format!("{err:?}");
                assert!(
                    !msg.contains("TooManyCtes"),
                    "TooManyCtes must not return (seed {seed}): {err}"
                );
                rejected += 1;
            }
            Err(_) => panic!(
                "prepare panicked on IR data (seed {seed}) — the trust-boundary law is \
                 violated by:\n{rendered}\n{query:#?}"
            ),
        }
    }
    assert!(ok > 0, "no generated query validated — vacuous sweep");
    assert!(
        rejected > 0,
        "no generated query was rejected — vacuous sweep"
    );
    assert_eq!(ok + rejected, SWEEP / 2);
}

/// Must not panic.
#[test]
fn a_hundred_thousand_interiors_is_not_too_many_ctes() {
    use bumbledb::Theory;
    use bumbledb::schema::ValidateDescriptor as _;
    let schema = Gauntlet
        .descriptor()
        .validate()
        .expect("the test schema is valid");
    let rule = Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: AtomSource::Edb(BUSY),
            bindings: vec![(Gauntlet::BUSY_PERSON, Term::Var(VarId(0)))],
        }],
        negated: vec![],
        conditions: vec![],
    };
    let interiors: Vec<Interior> = (0..100_000)
        .map(|_| Interior {
            rules: vec![ProjectionRule {
                finds: vec![VarId(0)],
                atoms: rule.atoms.clone(),
                negated: vec![],
                conditions: vec![],
            }],
        })
        .collect();
    let query = Query {
        interiors,
        head: vec![HeadTerm::Var],
        rules: vec![Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                source: AtomSource::Interior(InteriorId(99_999)),
                bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
            }],
            negated: vec![],
            conditions: vec![],
        }],
        rec: None,
    };
    let result = catch_unwind(AssertUnwindSafe(|| {
        bumbledb::ir::validate::validate(&schema, &query).map(|_| ())
    }))
    .unwrap_or_else(|_| panic!("validate panicked on interiors.len() == 100_000"));
    match result {
        Ok(()) => {}
        Err(err) => {
            let msg = format!("{err:?}");
            assert!(
                !msg.contains("TooManyCtes"),
                "100_000 interiors must not invent TooManyCtes: {err:?}"
            );
        }
    }
}

/// Hostile nesting alone, far past the sweep's per-query depth: a deep
/// alternating And/Or chain is the typed `ConditionNestingTooDeep` — judged
/// iteratively, so neither validation nor distribution ever recurses into it
/// (the sweep's founding find: before the boundary check existed, this input
/// exhausted the stack).
#[test]
fn deep_predicate_nesting_is_a_typed_rejection() {
    let dir = common::TempDir::new("adversarial-ir-nesting");
    let db = Db::create(dir.path(), Gauntlet)
        .expect("create")
        .expect("accepted");
    let leaf = || {
        ConditionTree::Leaf(Comparison {
            op: CmpOp::Ge,
            lhs: Term::Var(VarId(0)),
            rhs: Term::Literal(Value::U64(1)),
        })
    };
    let chain = |depth: usize| {
        let mut tree = leaf();
        for level in 1..depth {
            tree = if level % 2 == 0 {
                ConditionTree::And(vec![tree])
            } else {
                ConditionTree::Or(vec![tree])
            };
        }
        tree
    };
    let query = |tree: ConditionTree| {
        Query::single(Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                source: bumbledb::AtomSource::Edb(OOO),
                bindings: vec![(Gauntlet::OOO_PERSON, Term::Var(VarId(0)))],
            }],
            negated: vec![],
            conditions: vec![tree],
        })
    };

    let err = db
        .prepare(&query(chain(3_000)))
        .map(|_| ())
        .expect_err("hostile nesting is rejected");
    assert!(
        matches!(
            err,
            bumbledb::Error::Validation(
                bumbledb::error::ValidationError::ConditionNestingTooDeep {
                    exceeded: bumbledb::Exceeded {
                        observed: 3_000,
                        ceiling: MAX_CONDITION_DEPTH,
                    },
                    ..
                }
            )
        ),
        "{err:?}"
    );

    let _ = db
        .prepare(&query(chain(MAX_CONDITION_DEPTH)))
        .expect("cap-deep nesting is an ordinary query");
}
