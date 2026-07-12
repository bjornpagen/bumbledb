//! The foldability conditions, positive and negative per condition,
//! against the honest pipeline (validate → normalize → chase) over a
//! closed-relation fixture theory — plus direct predicate tests where a
//! condition's refusal shape is easier to pin in isolation.

use super::*;
use crate::allen::AllenMask;
use crate::ir::normalize::{normalize, NormalizedQuery};
use crate::ir::validate::validate;
use crate::ir::{
    Atom, Comparison, FindTerm, HeadTerm, MaskTerm, PredicateTree, Query, Rule, Term, Value,
};
use crate::plan::chase::{chase, with_chase_disabled};
use crate::schema::{
    FieldDescriptor, Generation, RelationDescriptor, Row, Schema, SchemaDescriptor, Side,
};

fn field(name: &str, value_type: ValueType) -> FieldDescriptor {
    FieldDescriptor {
        name: name.into(),
        value_type,
        generation: Generation::None,
    }
}

fn fresh(name: &str) -> FieldDescriptor {
    FieldDescriptor {
        name: name.into(),
        value_type: ValueType::U64,
        generation: Generation::Fresh,
    }
}

const ITEM: u32 = 0;
const LOOSE: u32 = 1;
const SCHED: u32 = 2;
const KIND: u32 = 3;
const CAL: u32 = 4;

/// Item(id fresh, kind u64, score i64) — `kind` references Kind;
/// Loose(id fresh, k u64) — NO containment (the domain-guarantee
/// negative); Sched(id fresh, cal u64) — no containment either (the
/// positive fold needs none); Kind closed (rank u64; ranks 10/20/20/30);
/// Cal closed (span interval<u64>; 2..5 and 5..9). One statement:
/// Item(kind) <= Kind(id).
fn theory() -> Schema {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Item".into(),
                fields: vec![
                    fresh("id"),
                    field("kind", ValueType::U64),
                    field("score", ValueType::I64),
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Loose".into(),
                fields: vec![fresh("id"), field("k", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Sched".into(),
                fields: vec![fresh("id"), field("cal", ValueType::U64)],
            },
            RelationDescriptor {
                extension: Some(Box::new([
                    Row {
                        handle: "A".into(),
                        values: Box::new([Value::U64(10)]),
                    },
                    Row {
                        handle: "B".into(),
                        values: Box::new([Value::U64(20)]),
                    },
                    Row {
                        handle: "C".into(),
                        values: Box::new([Value::U64(20)]),
                    },
                    Row {
                        handle: "D".into(),
                        values: Box::new([Value::U64(30)]),
                    },
                ])),
                name: "Kind".into(),
                fields: vec![field("rank", ValueType::U64)],
            },
            RelationDescriptor {
                extension: Some(Box::new([
                    Row {
                        handle: "X".into(),
                        values: Box::new([Value::IntervalU64(2, 5)]),
                    },
                    Row {
                        handle: "Y".into(),
                        values: Box::new([Value::IntervalU64(5, 9)]),
                    },
                ])),
                name: "Cal".into(),
                fields: vec![field(
                    "span",
                    ValueType::Interval {
                        element: IntervalElement::U64,
                    },
                )],
            },
        ],
        statements: vec![StatementDescriptor::Containment {
            source: Side {
                relation: RelationId(ITEM),
                projection: Box::new([FieldId(1)]),
                selection: Box::new([]),
            },
            target: Side {
                relation: RelationId(KIND),
                projection: Box::new([FieldId(0)]),
                selection: Box::new([]),
            },
        }],
    }
    .validate()
    .expect("valid fixture")
}

fn atom(relation: u32, bindings: &[(u16, Term)]) -> Atom {
    Atom {
        relation: RelationId(relation),
        bindings: bindings
            .iter()
            .map(|(f, t)| (FieldId(*f), t.clone()))
            .collect(),
    }
}

fn var(id: u16) -> Term {
    Term::Var(VarId(id))
}

/// Runs the full honest pipeline over one rule: validate → normalize →
/// chase (elimination and evaluation in the one fixpoint).
fn chased(schema: &Schema, query: &Query) -> NormalizedQuery {
    let witness = validate(schema, query).expect("valid fixture query");
    let mut normalized = normalize(schema, &witness).remove(0);
    chase(&mut normalized, schema, &query.rules[0].finds);
    normalized
}

fn roles(normalized: &NormalizedQuery) -> Vec<Role> {
    normalized.occurrences.iter().map(|o| o.role).collect()
}

/// The plan-constant sets attached to one occurrence's filter list.
fn attached_sets(normalized: &NormalizedQuery, idx: usize) -> Vec<Vec<u64>> {
    normalized.occurrences[idx]
        .filters
        .iter()
        .filter_map(|filter| match filter {
            FilterPredicate::Compare {
                op: CmpOp::Eq,
                value: Const::WordSet(words),
                ..
            } => Some(words.clone()),
            _ => None,
        })
        .collect()
}

fn folded(ids: u16, negated: bool) -> Role {
    Role::Folded(FoldedMark { ids, negated })
}

/// `Q(i, v) :- Item(id = i, kind = x, score = v), Kind(id = x, rank == 20)`.
fn selected_fold_query(rank: u64) -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(2))],
        atoms: vec![
            atom(ITEM, &[(0, var(0)), (1, var(1)), (2, var(2))]),
            atom(KIND, &[(0, var(1)), (1, Term::Literal(Value::U64(rank)))]),
        ],
        negated: vec![],
        predicates: vec![],
    })
}

/// The fold: a ψ-selected closed atom whose only escaping variable is
/// the join id becomes `Role::Folded` and its surviving id-set lands on
/// the sibling as a plan-constant membership.
#[test]
fn a_filtered_closed_atom_folds_to_a_membership_set() {
    let schema = theory();
    let normalized = chased(&schema, &selected_fold_query(20));
    assert_eq!(
        roles(&normalized),
        vec![Role::Positive, folded(2, false)],
        "the Kind occurrence folded with |S| = 2"
    );
    assert_eq!(
        attached_sets(&normalized, 0),
        vec![vec![1, 2]],
        "the sibling's kind field carries exactly the σ-surviving ids"
    );
    assert!(normalized.dead.is_none());
}

/// The off switch covers the evaluator too — the dual-run differential's
/// contract (`with_chase_disabled` bypasses the whole fixpoint).
#[test]
fn the_off_switch_bypasses_the_evaluator() {
    let schema = theory();
    let query = selected_fold_query(20);
    let witness = validate(&schema, &query).expect("valid fixture query");
    let mut normalized = normalize(&schema, &witness).remove(0);
    with_chase_disabled(|| chase(&mut normalized, &schema, &query.rules[0].finds));
    assert_eq!(roles(&normalized), vec![Role::Positive, Role::Positive]);
    assert!(attached_sets(&normalized, 0).is_empty());
}

/// Condition 1 negative — a live payload variable (projected rank)
/// escapes the atom: the fold refuses and the virtual-image join stays.
#[test]
fn a_live_payload_variable_blocks_the_fold() {
    let schema = theory();
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(2))],
        atoms: vec![
            atom(ITEM, &[(0, var(0)), (1, var(1))]),
            atom(KIND, &[(0, var(1)), (1, var(2))]),
        ],
        negated: vec![],
        predicates: vec![],
    });
    let normalized = chased(&schema, &query);
    assert_eq!(roles(&normalized), vec![Role::Positive, Role::Positive]);
    assert!(attached_sets(&normalized, 0).is_empty());
}

/// Condition 1 positive control — the same payload variable bound but
/// dead folds (S = the whole extension; the containment-eliminator
/// refuses closed targets, so the mark is the evaluator's).
#[test]
fn a_dead_payload_variable_folds() {
    let schema = theory();
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            atom(ITEM, &[(0, var(0)), (1, var(1))]),
            atom(KIND, &[(0, var(1)), (1, var(2))]),
        ],
        negated: vec![],
        predicates: vec![],
    });
    let normalized = chased(&schema, &query);
    assert_eq!(roles(&normalized), vec![Role::Positive, folded(4, false)]);
    assert_eq!(attached_sets(&normalized, 0), vec![vec![0, 1, 2, 3]]);
}

/// Condition 2 negative — a param-bearing filter defers to bind time,
/// which is REFUSED v0: the fold must not judge stage-3 values.
#[test]
fn a_param_filter_blocks_the_fold() {
    let schema = theory();
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            atom(ITEM, &[(0, var(0)), (1, var(1))]),
            atom(
                KIND,
                &[(0, var(1)), (1, Term::Param(crate::ir::ParamId(0)))],
            ),
        ],
        negated: vec![],
        predicates: vec![],
    });
    let normalized = chased(&schema, &query);
    assert_eq!(roles(&normalized), vec![Role::Positive, Role::Positive]);
    assert!(attached_sets(&normalized, 0).is_empty());
}

/// Condition 2, in isolation: the resolvability predicate admits the
/// constant shapes and refuses params, param sets, param masks, and the
/// measure kinds.
#[test]
fn prepare_resolvability_admits_constants_and_refuses_stage_three() {
    let eq = |value: Const| FilterPredicate::Compare {
        field: FieldId(1),
        op: CmpOp::Eq,
        value,
    };
    assert!(filters_prepare_resolvable(&[
        eq(Const::Word(7)),
        eq(Const::WordSet(vec![1, 2])),
        FilterPredicate::Compare {
            field: FieldId(1),
            op: CmpOp::Ge,
            value: Const::Word(3),
        },
        FilterPredicate::PointIn {
            field: FieldId(1),
            point: ResolvedWordSource::Word(4),
        },
        FilterPredicate::FieldAllen {
            field: FieldId(1),
            other: Const::Interval { start: 1, end: 2 },
            mask: MaskConst::Mask(AllenMask::BEFORE),
        },
    ]));
    assert!(!filters_prepare_resolvable(&[eq(Const::Param(
        crate::ir::ParamId(0)
    ))]));
    assert!(!filters_prepare_resolvable(&[eq(Const::ParamSet(
        crate::ir::ParamId(0)
    ))]));
    assert!(!filters_prepare_resolvable(&[
        FilterPredicate::FieldAllen {
            field: FieldId(1),
            other: Const::Interval { start: 1, end: 2 },
            mask: MaskConst::Param(crate::ir::ParamId(0)),
        }
    ]));
    // The measure kinds refuse: their ray error is per-execution, and a
    // prepare-time evaluation would move it (module doc).
    assert!(!filters_prepare_resolvable(&[
        FilterPredicate::DurationCompare {
            field: FieldId(1),
            op: CmpOp::Ge,
            value: Const::Word(2),
        }
    ]));
}

/// Direction 4 — a negated closed atom with `k` bound positively folds
/// to membership in the COMPLEMENT: the anti-probe rejected `k ∈ S`, so
/// the sibling keeps `k ∈ extension ∖ S`, and the probe descriptor is
/// deleted.
#[test]
fn a_negated_closed_atom_folds_to_the_complement() {
    let schema = theory();
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(ITEM, &[(0, var(0)), (1, var(1))])],
        negated: vec![atom(
            KIND,
            &[(0, var(1)), (1, Term::Literal(Value::U64(20)))],
        )],
        predicates: vec![],
    });
    let normalized = chased(&schema, &query);
    assert_eq!(roles(&normalized), vec![Role::Positive, folded(2, true)]);
    assert_eq!(
        attached_sets(&normalized, 0),
        vec![vec![0, 3]],
        "the complement of {{1, 2}} in the 4-row extension"
    );
    assert!(
        normalized.anti_probes.is_empty(),
        "the folded probe's descriptor is deleted"
    );
}

/// Direction 4 negative — the domain guarantee: without a containment
/// carrying `k` into the closed relation's ids, `k ∉ S` and
/// `k ∈ complement` disagree on out-of-extension values, so the fold
/// refuses and the anti-probe stays. (Loose.k has no statement — the
/// same query over Item folds above.)
#[test]
fn a_negated_fold_without_the_domain_guarantee_refuses() {
    let schema = theory();
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(LOOSE, &[(0, var(0)), (1, var(1))])],
        negated: vec![atom(
            KIND,
            &[(0, var(1)), (1, Term::Literal(Value::U64(20)))],
        )],
        predicates: vec![],
    });
    let normalized = chased(&schema, &query);
    assert_eq!(roles(&normalized), vec![Role::Positive, Role::Negated]);
    assert!(attached_sets(&normalized, 0).is_empty());
    assert_eq!(normalized.anti_probes.len(), 1, "the probe stays");
}

/// Direction 4, the |S| == 0 side of the direction pin: an empty
/// surviving set means the anti-probe never rejects — the negated atom
/// deletes outright (NO membership, NO rule death), domain guarantee
/// not needed (`k ∉ ∅` holds for every `k`; Loose's guarantee-free
/// binder proves the case).
#[test]
fn a_negated_atom_over_an_empty_set_deletes_and_rejects_nothing() {
    let schema = theory();
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(LOOSE, &[(0, var(0)), (1, var(1))])],
        negated: vec![atom(
            KIND,
            &[(0, var(1)), (1, Term::Literal(Value::U64(99)))],
        )],
        predicates: vec![],
    });
    let normalized = chased(&schema, &query);
    assert_eq!(roles(&normalized), vec![Role::Positive, folded(0, true)]);
    assert!(attached_sets(&normalized, 0).is_empty());
    assert!(normalized.anti_probes.is_empty());
    assert!(normalized.dead.is_none(), "the rule is NOT empty");
}

/// Direction 4, the complement-∅ side: an unfiltered negated closed
/// atom's S is the whole extension — under the domain guarantee every
/// binding's `k` is rejected, so the rule is dead.
#[test]
fn an_empty_complement_kills_the_rule() {
    let schema = theory();
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(ITEM, &[(0, var(0)), (1, var(1))])],
        negated: vec![atom(KIND, &[(0, var(1))])],
        predicates: vec![],
    });
    let normalized = chased(&schema, &query);
    assert_eq!(
        normalized.dead.as_deref(),
        Some("folded: !Kind{} rejects every binding"),
        "S = the whole extension ∧ k ∈ ids ⇒ every binding rejected"
    );
}

/// The dead guard: a var-less closed gate with a satisfiable selection
/// deletes outright — no membership to attach, nothing multiplies any
/// fold domain.
#[test]
fn a_satisfied_var_less_guard_deletes_outright() {
    let schema = theory();
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            atom(ITEM, &[(0, var(0))]),
            atom(KIND, &[(1, Term::Literal(Value::U64(20)))]),
        ],
        negated: vec![],
        predicates: vec![],
    });
    let normalized = chased(&schema, &query);
    assert_eq!(roles(&normalized), vec![Role::Positive, folded(2, false)]);
    assert!(attached_sets(&normalized, 0).is_empty());
    assert!(normalized.dead.is_none());
}

/// The guard's negative — a guard binding a variable (dead, but bound)
/// refuses: under an aggregate sink the fold domain is over ALL query
/// variables, and deleting the binder would collapse |S| bindings into
/// one.
#[test]
fn a_var_binding_guard_refuses() {
    let schema = theory();
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            atom(ITEM, &[(0, var(0))]),
            // rank == 20 plus a bound-but-dead id variable: no live k
            // (nothing else binds Var 1), so this is guard-shaped — and
            // must refuse.
            atom(KIND, &[(0, var(1)), (1, Term::Literal(Value::U64(20)))]),
        ],
        negated: vec![],
        predicates: vec![],
    });
    let normalized = chased(&schema, &query);
    assert_eq!(roles(&normalized), vec![Role::Positive, Role::Positive]);
}

/// |S| == 0 kills the rule — the statically-empty channel with the
/// evaluator's rendered reason (the fold's `dead` picture discipline).
#[test]
fn an_empty_surviving_set_kills_the_rule() {
    let schema = theory();
    let normalized = chased(&schema, &selected_fold_query(99));
    assert_eq!(
        normalized.dead.as_deref(),
        Some("folded to ∅: Kind{rank == 99}"),
        "the rendered reason names the refuting atom"
    );
}

/// |S| == 0 on a var-less guard kills the rule too (the guard's own
/// negative-side twin of the delete above).
#[test]
fn an_unsatisfiable_guard_kills_the_rule() {
    let schema = theory();
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            atom(ITEM, &[(0, var(0))]),
            atom(KIND, &[(1, Term::Literal(Value::U64(99)))]),
        ],
        negated: vec![],
        predicates: vec![],
    });
    let normalized = chased(&schema, &query);
    assert_eq!(
        normalized.dead.as_deref(),
        Some("folded to ∅: Kind{rank == 99}")
    );
}

/// A single-atom closed scan with a live projected handle refuses: the
/// membership set has no other binder to land on, and deleting the only
/// participating occurrence would leave the rule bodyless.
#[test]
fn a_fold_with_no_membership_home_refuses() {
    let schema = theory();
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(
            KIND,
            &[(0, var(0)), (1, Term::Literal(Value::U64(20)))],
        )],
        negated: vec![],
        predicates: vec![],
    });
    let normalized = chased(&schema, &query);
    assert_eq!(roles(&normalized), vec![Role::Positive]);
    assert!(normalized.dead.is_none());
}

/// Multi-rule programs fold per rule, independently: the same closed
/// atom folds in the rule where its payload is dead and refuses in the
/// rule projecting it (no cross-rule state — the chase's per-rule law).
#[test]
fn multi_rule_programs_fold_per_rule_independently() {
    let schema = theory();
    let fold_rule = Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            atom(ITEM, &[(0, var(0)), (1, var(1))]),
            atom(KIND, &[(0, var(1)), (1, Term::Literal(Value::U64(20)))]),
        ],
        negated: vec![],
        predicates: vec![],
    };
    let refusing_rule = Rule {
        finds: vec![FindTerm::Var(VarId(2))],
        atoms: vec![
            atom(ITEM, &[(0, var(0)), (1, var(1))]),
            atom(KIND, &[(0, var(1)), (1, var(2))]),
        ],
        negated: vec![],
        predicates: vec![],
    };
    let query = Query {
        head: vec![HeadTerm::Var],
        rules: vec![fold_rule, refusing_rule],
    };
    let witness = validate(&schema, &query).expect("valid fixture query");
    let mut rules = normalize(&schema, &witness);
    for (idx, rule) in rules.iter_mut().enumerate() {
        chase(rule, &schema, &witness.rule(idx).rule().finds);
    }
    assert_eq!(roles(&rules[0]), vec![Role::Positive, folded(2, false)]);
    assert_eq!(attached_sets(&rules[0], 0), vec![vec![1, 2]]);
    assert_eq!(
        roles(&rules[1]),
        vec![Role::Positive, Role::Positive],
        "rule 1 projects the payload — its own refusal, untouched by rule 0's fold"
    );
    assert!(attached_sets(&rules[1], 0).is_empty());
}

/// The interval evaluation paths: a literal membership binding
/// (`PointIn`) and a literal `Allen` predicate (`FieldAllen`) evaluate
/// against the sealed rows through the scalar classify — n ≤ 256, never
/// the batch kernel.
#[test]
fn interval_filters_evaluate_against_the_sealed_extension() {
    let schema = theory();
    // 3 ∈ span: only X = 2..5 survives.
    let membership = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            atom(SCHED, &[(0, var(0)), (1, var(1))]),
            atom(CAL, &[(0, var(1)), (1, Term::Literal(Value::U64(3)))]),
        ],
        negated: vec![],
        predicates: vec![],
    });
    let normalized = chased(&schema, &membership);
    assert_eq!(roles(&normalized), vec![Role::Positive, folded(1, false)]);
    assert_eq!(attached_sets(&normalized, 0), vec![vec![0]]);

    // Allen(span, BEFORE, 6..8): X = 2..5 is before, Y = 5..9 covers.
    let allen = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            atom(SCHED, &[(0, var(0)), (1, var(1))]),
            atom(CAL, &[(0, var(1)), (1, var(2))]),
        ],
        negated: vec![],
        predicates: vec![PredicateTree::Leaf(Comparison {
            op: CmpOp::Allen {
                mask: MaskTerm::Literal(AllenMask::BEFORE),
            },
            lhs: var(2),
            rhs: Term::Literal(Value::IntervalU64(6, 8)),
        })],
    });
    let normalized = chased(&schema, &allen);
    assert_eq!(roles(&normalized), vec![Role::Positive, folded(1, false)]);
    assert_eq!(attached_sets(&normalized, 0), vec![vec![0]]);
}

/// The fixpoint composes folds: a second closed atom over the same join
/// variable receives the first fold's membership and evaluates it as an
/// ordinary filter — the surviving set intersects.
#[test]
fn a_second_closed_atom_folds_over_the_first_folds_set() {
    let schema = theory();
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            atom(ITEM, &[(0, var(0)), (1, var(1))]),
            // rank >= 20 survives {1, 2, 3}; rank <= 20 survives
            // {0, 1, 2}; the sibling must end with both sets attached
            // (their conjunction is {1, 2}).
            atom(KIND, &[(0, var(1)), (1, Term::Literal(Value::U64(20)))]),
            atom(KIND, &[(0, var(1)), (1, var(2))]),
        ],
        negated: vec![],
        predicates: vec![PredicateTree::Leaf(Comparison {
            op: CmpOp::Ge,
            lhs: var(2),
            rhs: Term::Literal(Value::U64(20)),
        })],
    });
    let normalized = chased(&schema, &query);
    assert_eq!(
        roles(&normalized),
        vec![Role::Positive, folded(2, false), folded(2, false)],
        "both closed occurrences fold (the second sees the first's set as a filter)"
    );
    // The Item occurrence carries both memberships; their conjunction
    // is the honest intersection.
    let sets = attached_sets(&normalized, 0);
    assert_eq!(sets.len(), 2);
    assert!(
        sets.contains(&vec![1, 2]),
        "rank == 20 → {{1, 2}}: {sets:?}"
    );
}

/// The fold's picture prints the vocabulary's names: a word at the
/// relation's own id position renders as its handle, a membership set as
/// a handle set, and an out-of-range word visibly wrong as `Kind(9?)` —
/// the `ir/render` fallback convention, byte-exact.
#[test]
fn the_folded_picture_prints_handles_at_the_id_position() {
    let schema = theory();
    let relation = RelationId(KIND);
    let eq_id = |value: Const| FilterPredicate::Compare {
        field: FieldId(0),
        op: CmpOp::Eq,
        value,
    };
    assert_eq!(
        folded_picture(&schema, relation, &[eq_id(Const::Word(1))]),
        "Kind{id == B}"
    );
    assert_eq!(
        folded_picture(&schema, relation, &[eq_id(Const::Word(9))]),
        "Kind{id == Kind(9?)}"
    );
    assert_eq!(
        folded_picture(&schema, relation, &[eq_id(Const::WordSet(vec![0, 2]))]),
        "Kind{id ∈ {A, C}}"
    );
    // A payload column stays a plain value — handles live at the id.
    assert_eq!(
        folded_picture(
            &schema,
            relation,
            &[FilterPredicate::Compare {
                field: FieldId(1),
                op: CmpOp::Eq,
                value: Const::Word(20),
            }]
        ),
        "Kind{rank == 20}"
    );
}
