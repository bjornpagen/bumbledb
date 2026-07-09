use super::lower_literal::lower_literal;
use super::*;
use crate::encoding::{encode_fact, encode_i64, ValueRef};
use crate::image::view::{Const, ResolvedWordSource};
use crate::ir::validate::validate;
use crate::ir::{Atom, Comparison, FindTerm, ParamId, Query, Term, Value};
use crate::schema::{
    FieldDescriptor, Generation, IntervalElement, RelationDescriptor, Schema, SchemaDescriptor,
    ValueType,
};
use crate::storage::commit::commit;
use crate::storage::delta::WriteDelta;
use crate::storage::dict::{TAG_BYTES, TAG_STRING};
use crate::storage::env::Environment;
use crate::testutil::TempDir;

/// R(id u64 serial, a i64, b i64) + S(x u64, y i64)
/// + P(emp u64, during interval<i64>, review interval<i64>, at i64)
/// + E(emp u64, at i64).
fn schema() -> Schema {
    let field = |name: &str, ty: ValueType| FieldDescriptor {
        name: name.into(),
        value_type: ty,
        generation: Generation::None,
    };
    let interval_i64 = ValueType::Interval {
        element: IntervalElement::I64,
    };
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "R".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                        generation: Generation::Serial,
                    },
                    field("a", ValueType::I64),
                    field("b", ValueType::I64),
                ],
            },
            RelationDescriptor {
                name: "S".into(),
                fields: vec![field("x", ValueType::U64), field("y", ValueType::I64)],
            },
            RelationDescriptor {
                name: "P".into(),
                fields: vec![
                    field("emp", ValueType::U64),
                    field("during", interval_i64.clone()),
                    field("review", interval_i64),
                    field("at", ValueType::I64),
                ],
            },
            RelationDescriptor {
                name: "E".into(),
                fields: vec![field("emp", ValueType::U64), field("at", ValueType::I64)],
            },
        ],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

const R: RelationId = RelationId(0);
const S: RelationId = RelationId(1);
const P: RelationId = RelationId(2);
const E: RelationId = RelationId(3);

/// P's fields by position.
const P_EMP: FieldId = FieldId(0);
const P_DURING: FieldId = FieldId(1);
const P_REVIEW: FieldId = FieldId(2);
const P_AT: FieldId = FieldId(3);
/// E's fields by position.
const E_AT: FieldId = FieldId(1);

fn var(id: u16) -> Term {
    Term::Var(VarId(id))
}

/// The biased I64 column word.
fn w(value: i64) -> u64 {
    u64::from_be_bytes(encode_i64(value))
}

fn normalized(query: &Query) -> NormalizedQuery {
    let schema = schema();
    normalize(&schema, &validate(&schema, query).expect("valid"))
}

fn query(atoms: Vec<Atom>, negated: Vec<Atom>, predicates: Vec<Comparison>) -> Query {
    Query {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms,
        negated,
        predicates,
    }
}

#[test]
fn repeated_variable_lowers_and_executes_through_the_evaluator() {
    // R(a = v, b = v): one var position, one same-fact equality filter.
    let query = query(
        vec![Atom {
            relation: R,
            bindings: vec![(FieldId(1), var(0)), (FieldId(2), var(0))],
        }],
        vec![],
        vec![],
    );
    let norm = normalized(&query);
    assert_eq!(norm.occurrences[0].polarity, Polarity::Positive);
    assert_eq!(norm.occurrences[0].vars, vec![(FieldId(1), VarId(0))]);
    assert_eq!(
        norm.occurrences[0].filters,
        vec![FilterPredicate::FieldsCompare {
            left: FieldId(1),
            right: FieldId(2),
            op: CmpOp::Eq,
        }]
    );
    assert!(norm.anti_probes.is_empty());
    assert_eq!(norm.slot_widths[&VarId(0)], SlotWidth::One);

    // ...and the lowered filter executes on a real image.
    let dir = TempDir::new("normalize-execute");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    for (id, a, b) in [(1u64, 5i64, 5i64), (2, 5, 6), (3, -1, -1)] {
        let mut bytes = Vec::new();
        encode_fact(
            &[ValueRef::U64(id), ValueRef::I64(a), ValueRef::I64(b)],
            schema.relation(R).layout(),
            &mut bytes,
        );
        delta.insert(&view, R, &bytes).expect("insert");
    }
    drop(view);
    commit(delta, &env).expect("commit");
    let txn = env.read_txn().expect("txn");
    let image = crate::image::build(&txn, &schema, R).expect("build");
    let filtered = crate::image::view::apply(&image, &norm.occurrences[0].filters, &[], Vec::new());
    // Exactly the a == b rows survive.
    let ids: Vec<u64> = filtered
        .positions()
        .map(|p| filtered.image().column_words(0)[p as usize])
        .collect();
    assert_eq!(ids.len(), 2);
    assert!(!ids.contains(&2));
}

#[test]
fn literal_and_param_bindings_lower_to_eq_filters() {
    let query = query(
        vec![Atom {
            relation: R,
            bindings: vec![
                (FieldId(0), var(0)),
                (FieldId(1), Term::Literal(Value::I64(-7))),
                (FieldId(2), Term::Param(ParamId(0))),
            ],
        }],
        vec![],
        vec![],
    );
    let norm = normalized(&query);
    assert_eq!(
        norm.occurrences[0].filters,
        vec![
            FilterPredicate::Compare {
                field: FieldId(1),
                op: CmpOp::Eq,
                value: Const::Word(w(-7)),
            },
            FilterPredicate::Compare {
                field: FieldId(2),
                op: CmpOp::Eq,
                value: Const::Param(ParamId(0)),
            },
        ]
    );
}

#[test]
fn string_literals_stay_raw_as_pending_interns() {
    // The fixture lacks a string field, so check lower_literal directly
    // (the unit under test).
    assert_eq!(
        lower_literal(&Value::String(Box::from(&b"acme"[..]))),
        Const::PendingIntern {
            tag: TAG_STRING,
            bytes: Box::from(&b"acme"[..]),
        }
    );
    assert_eq!(
        lower_literal(&Value::Bytes(Box::from(&[7u8][..]))),
        Const::PendingIntern {
            tag: TAG_BYTES,
            bytes: Box::from(&[7u8][..]),
        }
    );
}

#[test]
fn interval_literals_lower_to_encoded_word_pairs() {
    // Each half is encoded exactly like the scalar of its element type.
    assert_eq!(
        lower_literal(&Value::IntervalU64(3, 9)),
        Const::Interval { start: 3, end: 9 }
    );
    assert_eq!(
        lower_literal(&Value::IntervalI64(-5, 9)),
        Const::Interval {
            start: w(-5),
            end: w(9),
        }
    );
    // The bias preserves order across the sign boundary in word space.
    assert!(w(-5) < w(9));
}

#[test]
fn same_relation_atoms_get_distinct_occurrences_with_independent_filters() {
    // A self-join: R(id=v0, a=1) x R(id=v1, a=2).
    let query = Query {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                relation: R,
                bindings: vec![
                    (FieldId(0), var(0)),
                    (FieldId(1), Term::Literal(Value::I64(1))),
                ],
            },
            Atom {
                relation: R,
                bindings: vec![
                    (FieldId(0), var(1)),
                    (FieldId(1), Term::Literal(Value::I64(2))),
                ],
            },
        ],
        negated: vec![],
        predicates: vec![],
    };
    let norm = normalized(&query);
    assert_eq!(norm.occurrences.len(), 2);
    assert_eq!(norm.occurrences[0].occ_id, OccId(0));
    assert_eq!(norm.occurrences[1].occ_id, OccId(1));
    assert_eq!(norm.occurrences[0].relation, R);
    assert_eq!(norm.occurrences[1].relation, R);
    assert_ne!(norm.occurrences[0].filters, norm.occurrences[1].filters);
}

#[test]
fn range_comparison_pushes_down_and_cross_atom_comparison_is_residual() {
    // 100 <= R.a (constant on the left: flips to a >= 100); R.a < S.y
    // stays a residual.
    let query = query(
        vec![
            Atom {
                relation: R,
                bindings: vec![(FieldId(0), var(2)), (FieldId(1), var(0))],
            },
            Atom {
                relation: S,
                bindings: vec![(FieldId(1), var(1))],
            },
        ],
        vec![],
        vec![
            Comparison {
                op: CmpOp::Le,
                lhs: Term::Literal(Value::I64(100)),
                rhs: var(0),
            },
            Comparison {
                op: CmpOp::Lt,
                lhs: var(0),
                rhs: var(1),
            },
        ],
    );
    let norm = normalized(&query);
    assert_eq!(
        norm.occurrences[0].filters,
        vec![FilterPredicate::Compare {
            field: FieldId(1),
            op: CmpOp::Ge, // flipped
            value: Const::Word(w(100)),
        }]
    );
    assert!(norm.occurrences[1].filters.is_empty());
    assert_eq!(
        norm.residuals,
        vec![PlacedComparison {
            op: CmpOp::Lt,
            lhs: VarId(0),
            rhs: VarId(1),
        }]
    );
    assert!(norm.word_residuals.is_empty());
}

#[test]
fn occurrence_vars_are_duplicate_free_over_generated_inputs() {
    // A tiny deterministic generator: every subset/multiset of var
    // bindings over R's three fields, with var ids drawn from {0,1}.
    let schema = schema();
    let mut checked = 0;
    for mask in 0..3u16.pow(3) {
        let mut bindings = Vec::new();
        let mut m = mask;
        for field in 0..3u16 {
            let choice = m % 3;
            m /= 3;
            match choice {
                0 => {}
                1 => bindings.push((FieldId(field), var(0))),
                _ => bindings.push((FieldId(field), var(1))),
            }
        }
        if bindings.is_empty() {
            continue;
        }
        // Var 0 must be findable; ensure it is bound.
        if !bindings.iter().any(|(_, t)| *t == var(0)) {
            continue;
        }
        let query = query(
            vec![Atom {
                relation: R,
                bindings,
            }],
            vec![],
            vec![],
        );
        // Field types differ (U64 vs I64): only same-typed repeats
        // validate; skip type-conflicting combinations.
        let Ok(witness) = validate(&schema, &query) else {
            continue;
        };
        let norm = normalize(&schema, &witness);
        for occurrence in &norm.occurrences {
            let mut seen = std::collections::BTreeSet::new();
            for (_, v) in &occurrence.vars {
                assert!(seen.insert(*v), "occurrence vars must be distinct");
            }
        }
        checked += 1;
    }
    assert!(checked > 3, "the sweep exercised real shapes: {checked}");
}

#[test]
fn zero_binding_atom_becomes_an_empty_occurrence() {
    let query = query(
        vec![
            Atom {
                relation: R,
                bindings: vec![(FieldId(0), var(0))],
            },
            Atom {
                relation: S,
                bindings: vec![],
            },
        ],
        vec![],
        vec![],
    );
    let norm = normalized(&query);
    assert_eq!(norm.occurrences[1].occ_id, OccId(1));
    assert!(norm.occurrences[1].vars.is_empty());
    assert!(norm.occurrences[1].filters.is_empty());
}

#[test]
fn same_atom_var_var_comparison_lowers_to_a_filter() {
    // R(a = x, b = y), x < y — one atom, both sides: a per-atom
    // FieldsCompare filter, never a residual (residuals are cross-atom
    // only, docs/architecture/20-query-ir.md).
    let query = query(
        vec![Atom {
            relation: R,
            bindings: vec![(FieldId(1), var(0)), (FieldId(2), var(1))],
        }],
        vec![],
        vec![Comparison {
            op: CmpOp::Lt,
            lhs: var(0),
            rhs: var(1),
        }],
    );
    let norm = normalized(&query);
    assert!(
        norm.residuals.is_empty() && norm.word_residuals.is_empty(),
        "same-atom pairs never residualize"
    );
    assert_eq!(
        norm.occurrences[0].filters,
        vec![FilterPredicate::FieldsCompare {
            left: FieldId(1),
            right: FieldId(2),
            op: CmpOp::Lt,
        }]
    );
}

// --- lowering goldens (PRD 13) --------------------------------------------

#[test]
fn constant_point_membership_lowers_to_point_in() {
    // Golden (a): P(emp = v0, during ∋ 5) — a literal point in the
    // interval field lowers to a per-atom PointIn range filter.
    let literal = query(
        vec![Atom {
            relation: P,
            bindings: vec![(P_EMP, var(0)), (P_DURING, Term::Literal(Value::I64(5)))],
        }],
        vec![],
        vec![],
    );
    let norm = normalized(&literal);
    assert_eq!(norm.occurrences[0].vars, vec![(P_EMP, VarId(0))]);
    assert_eq!(
        norm.occurrences[0].filters,
        vec![FilterPredicate::PointIn {
            field: P_DURING,
            point: ResolvedWordSource::Word(w(5)),
        }]
    );

    // A scalar param point (anchored I64 by E.at) lowers the same way,
    // resolved at bind.
    let param = query(
        vec![
            Atom {
                relation: P,
                bindings: vec![(P_EMP, var(0)), (P_DURING, Term::Param(ParamId(0)))],
            },
            Atom {
                relation: E,
                bindings: vec![(E_AT, Term::Param(ParamId(0)))],
            },
        ],
        vec![],
        vec![],
    );
    let norm = normalized(&param);
    assert_eq!(
        norm.occurrences[0].filters,
        vec![FilterPredicate::PointIn {
            field: P_DURING,
            point: ResolvedWordSource::Param(ParamId(0)),
        }]
    );
}

#[test]
fn same_atom_overlaps_lowers_to_the_fixed_word_composition() {
    // Golden (b): P(during = x, review = y), Overlaps(x, y) — the fixed
    // three-word-comparison shape as one filter kind, never a residual.
    let overlaps = query(
        vec![Atom {
            relation: P,
            bindings: vec![(P_DURING, var(0)), (P_REVIEW, var(1))],
        }],
        vec![],
        vec![Comparison {
            op: CmpOp::Overlaps,
            lhs: var(0),
            rhs: var(1),
        }],
    );
    let norm = normalized(&overlaps);
    assert!(norm.residuals.is_empty() && norm.word_residuals.is_empty());
    assert_eq!(
        norm.occurrences[0].filters,
        vec![FilterPredicate::FieldsOverlap {
            left: P_DURING,
            right: P_REVIEW,
        }]
    );
    // Interval variables occupy two slots each.
    assert_eq!(norm.slot_widths[&VarId(0)], SlotWidth::Two);
    assert_eq!(norm.slot_widths[&VarId(1)], SlotWidth::Two);

    // The two Contains shapes: interval ⊇ interval and point membership.
    let contains = query(
        vec![Atom {
            relation: P,
            bindings: vec![(P_DURING, var(0)), (P_REVIEW, var(1))],
        }],
        vec![],
        vec![Comparison {
            op: CmpOp::Contains,
            lhs: var(0),
            rhs: var(1),
        }],
    );
    assert_eq!(
        normalized(&contains).occurrences[0].filters,
        vec![FilterPredicate::FieldsContain {
            outer: P_DURING,
            inner: P_REVIEW,
        }]
    );
    let contains_point = query(
        vec![Atom {
            relation: P,
            bindings: vec![(P_DURING, var(1)), (P_AT, var(0))],
        }],
        vec![],
        vec![Comparison {
            op: CmpOp::Contains,
            lhs: var(1),
            rhs: var(0),
        }],
    );
    let norm = normalized(&contains_point);
    assert_eq!(
        norm.occurrences[0].filters,
        vec![FilterPredicate::FieldsContainPoint {
            interval: P_DURING,
            point: P_AT,
        }]
    );
    assert_eq!(norm.slot_widths[&VarId(0)], SlotWidth::One);
}

#[test]
fn negated_atom_with_literal_binding_lowers_to_anti_probe() {
    // Golden (c): R(id = v0), ¬S(x = v0, y = -7) — the negated atom is an
    // occurrence with Negated polarity in the one table; its literal
    // binding is its own filter list (evaluated inside the probe); the
    // descriptor carries the occurrence and its variable set.
    let query = query(
        vec![Atom {
            relation: R,
            bindings: vec![(FieldId(0), var(0))],
        }],
        vec![Atom {
            relation: S,
            bindings: vec![
                (FieldId(0), var(0)),
                (FieldId(1), Term::Literal(Value::I64(-7))),
            ],
        }],
        vec![],
    );
    let norm = normalized(&query);
    assert_eq!(norm.occurrences.len(), 2);
    let negated = &norm.occurrences[1];
    assert_eq!(negated.occ_id, OccId(1));
    assert_eq!(negated.polarity, Polarity::Negated);
    assert_eq!(negated.relation, S);
    assert_eq!(negated.vars, vec![(FieldId(0), VarId(0))]);
    assert_eq!(
        negated.filters,
        vec![FilterPredicate::Compare {
            field: FieldId(1),
            op: CmpOp::Eq,
            value: Const::Word(w(-7)),
        }]
    );
    assert_eq!(
        norm.anti_probes,
        vec![AntiProbe {
            occurrence: OccId(1),
            probe_bindings: vec![(FieldId(0), VarId(0))],
        }]
    );
    assert!(norm.residuals.is_empty() && norm.word_residuals.is_empty());
}

#[test]
fn cross_atom_overlaps_decomposes_into_slot_pair_word_comparisons() {
    // Golden (d): P(during = x), P(during = y), Overlaps(x, y) — the
    // residual references two slot pairs as word comparisons:
    // x.start < y.end AND y.start < x.end.
    let overlaps = query(
        vec![
            Atom {
                relation: P,
                bindings: vec![(P_DURING, var(0))],
            },
            Atom {
                relation: P,
                bindings: vec![(P_DURING, var(1))],
            },
        ],
        vec![],
        vec![Comparison {
            op: CmpOp::Overlaps,
            lhs: var(0),
            rhs: var(1),
        }],
    );
    let norm = normalized(&overlaps);
    assert!(norm.residuals.is_empty());
    let start = |id: u16| VarWord {
        var: VarId(id),
        word: IntervalWord::Start,
    };
    let end = |id: u16| VarWord {
        var: VarId(id),
        word: IntervalWord::End,
    };
    assert_eq!(
        norm.word_residuals,
        vec![
            PlacedWordComparison {
                op: CmpOp::Lt,
                lhs: start(0),
                rhs: end(1),
            },
            PlacedWordComparison {
                op: CmpOp::Lt,
                lhs: start(1),
                rhs: end(0),
            },
        ]
    );
    assert_eq!(norm.slot_widths[&VarId(0)], SlotWidth::Two);
    assert_eq!(norm.slot_widths[&VarId(1)], SlotWidth::Two);

    // Cross-atom Contains over a point variable: x.start ≤ t AND t < x.end
    // — the point variable's single word is its Start word.
    let contains_point = query(
        vec![
            Atom {
                relation: P,
                bindings: vec![(P_DURING, var(0))],
            },
            Atom {
                relation: E,
                bindings: vec![(E_AT, var(1))],
            },
        ],
        vec![],
        vec![Comparison {
            op: CmpOp::Contains,
            lhs: var(0),
            rhs: var(1),
        }],
    );
    let norm = normalized(&contains_point);
    assert_eq!(
        norm.word_residuals,
        vec![
            PlacedWordComparison {
                op: CmpOp::Le,
                lhs: start(0),
                rhs: start(1),
            },
            PlacedWordComparison {
                op: CmpOp::Lt,
                lhs: start(1),
                rhs: end(0),
            },
        ]
    );
    assert_eq!(norm.slot_widths[&VarId(1)], SlotWidth::One);
}

#[test]
fn scalar_param_set_binding_is_the_selection_set_marker() {
    // Golden (e): S(x = v0, y ∈ ?set0) — an Eq compare against the set
    // marker, which the plan's selection split routes into
    // `PlanOccurrence::selections` (the word-set resolution is bind-time;
    // executor side is PRD 17).
    let scalar = query(
        vec![Atom {
            relation: S,
            bindings: vec![
                (FieldId(0), var(0)),
                (FieldId(1), Term::ParamSet(ParamId(0))),
            ],
        }],
        vec![],
        vec![],
    );
    assert_eq!(
        normalized(&scalar).occurrences[0].filters,
        vec![FilterPredicate::Compare {
            field: FieldId(1),
            op: CmpOp::Eq,
            value: Const::ParamSet(ParamId(0)),
        }]
    );

    // On an interval field the set holds points: AnyPointIn.
    let point_set = query(
        vec![Atom {
            relation: P,
            bindings: vec![(P_EMP, var(0)), (P_DURING, Term::ParamSet(ParamId(0)))],
        }],
        vec![],
        vec![],
    );
    assert_eq!(
        normalized(&point_set).occurrences[0].filters,
        vec![FilterPredicate::AnyPointIn {
            field: P_DURING,
            set: ParamId(0),
        }]
    );
}

// --- membership-variable bindings ------------------------------------------

#[test]
fn same_atom_membership_variable_lowers_to_the_field_composition() {
    // P(during ∋ t, at = t): the point variable is scalar-bound in the
    // same atom, so the membership is a same-fact field composition —
    // binding order must not matter (the membership binding comes first).
    let query = query(
        vec![Atom {
            relation: P,
            bindings: vec![(P_DURING, var(0)), (P_AT, var(0))],
        }],
        vec![],
        vec![],
    );
    let norm = normalized(&query);
    assert_eq!(norm.occurrences[0].vars, vec![(P_AT, VarId(0))]);
    assert_eq!(
        norm.occurrences[0].filters,
        vec![FilterPredicate::FieldsContainPoint {
            interval: P_DURING,
            point: P_AT,
        }]
    );
}

#[test]
fn cross_atom_membership_variable_lowers_to_point_in_over_the_binding() {
    // P(emp = e, during ∋ t), E(at = t): the point variable is bound by
    // the other occurrence — the membership stays a per-atom filter whose
    // point resolves from the variable's binding once bound (the
    // point-membership scan, docs/architecture/40-execution.md); the
    // membership position binds no variable of P.
    let query = Query {
        finds: vec![FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                relation: P,
                bindings: vec![(P_EMP, var(1)), (P_DURING, var(0))],
            },
            Atom {
                relation: E,
                bindings: vec![(E_AT, var(0))],
            },
        ],
        negated: vec![],
        predicates: vec![],
    };
    let norm = normalized(&query);
    assert_eq!(norm.occurrences[0].vars, vec![(P_EMP, VarId(1))]);
    assert_eq!(
        norm.occurrences[0].filters,
        vec![FilterPredicate::PointIn {
            field: P_DURING,
            point: ResolvedWordSource::Var(VarId(0)),
        }]
    );
    assert_eq!(norm.occurrences[1].vars, vec![(E_AT, VarId(0))]);
}

// --- constant-side interval comparisons -------------------------------------

#[test]
fn constant_interval_comparisons_lower_to_fixed_const_shapes() {
    let iv = || Term::Literal(Value::IntervalI64(2, 9));
    let iv_const = Const::Interval {
        start: w(2),
        end: w(9),
    };
    let p_atom = || Atom {
        relation: P,
        bindings: vec![(P_DURING, var(0))],
    };

    // Overlaps(x, [2,9)) — symmetric, field on the left.
    let overlaps = query(
        vec![p_atom()],
        vec![],
        vec![Comparison {
            op: CmpOp::Overlaps,
            lhs: var(0),
            rhs: iv(),
        }],
    );
    assert_eq!(
        normalized(&overlaps).occurrences[0].filters,
        vec![FilterPredicate::Compare {
            field: P_DURING,
            op: CmpOp::Overlaps,
            value: iv_const.clone(),
        }]
    );

    // Contains(x, [2,9)) — the field's interval covers the constant.
    let covers = query(
        vec![p_atom()],
        vec![],
        vec![Comparison {
            op: CmpOp::Contains,
            lhs: var(0),
            rhs: iv(),
        }],
    );
    assert_eq!(
        normalized(&covers).occurrences[0].filters,
        vec![FilterPredicate::Compare {
            field: P_DURING,
            op: CmpOp::Contains,
            value: iv_const.clone(),
        }]
    );

    // Contains([2,9), x) — reversed: the field lies within the constant.
    let within = query(
        vec![p_atom()],
        vec![],
        vec![Comparison {
            op: CmpOp::Contains,
            lhs: iv(),
            rhs: var(0),
        }],
    );
    assert_eq!(
        normalized(&within).occurrences[0].filters,
        vec![FilterPredicate::FieldWithin {
            field: P_DURING,
            outer: iv_const.clone(),
        }]
    );

    // Contains([2,9), t) over a scalar variable — the same reversed shape
    // on the point's field.
    let point_within = query(
        vec![Atom {
            relation: E,
            bindings: vec![(FieldId(0), var(0)), (E_AT, var(1))],
        }],
        vec![],
        vec![Comparison {
            op: CmpOp::Contains,
            lhs: iv(),
            rhs: var(1),
        }],
    );
    assert_eq!(
        normalized(&point_within).occurrences[0].filters,
        vec![FilterPredicate::FieldWithin {
            field: E_AT,
            outer: iv_const.clone(),
        }]
    );

    // Contains(x, 5) — a constant point is membership: PointIn.
    let contains_point = query(
        vec![p_atom()],
        vec![],
        vec![Comparison {
            op: CmpOp::Contains,
            lhs: var(0),
            rhs: Term::Literal(Value::I64(5)),
        }],
    );
    assert_eq!(
        normalized(&contains_point).occurrences[0].filters,
        vec![FilterPredicate::PointIn {
            field: P_DURING,
            point: ResolvedWordSource::Word(w(5)),
        }]
    );

    // during = [2,9) — interval value equality is an ordinary Eq compare
    // (a probeable selection over the two-word pair).
    let equality = query(
        vec![Atom {
            relation: P,
            bindings: vec![(P_EMP, var(0)), (P_DURING, iv())],
        }],
        vec![],
        vec![],
    );
    assert_eq!(
        normalized(&equality).occurrences[0].filters,
        vec![FilterPredicate::Compare {
            field: P_DURING,
            op: CmpOp::Eq,
            value: iv_const,
        }]
    );
}

#[test]
fn interval_param_equality_binding_stays_an_eq_compare() {
    // P(during = ?0) with no element anchor: the bivalent param resolves
    // to the interval reading — value equality, a bind-resolved selection.
    let query = query(
        vec![Atom {
            relation: P,
            bindings: vec![(P_EMP, var(0)), (P_DURING, Term::Param(ParamId(0)))],
        }],
        vec![],
        vec![],
    );
    assert_eq!(
        normalized(&query).occurrences[0].filters,
        vec![FilterPredicate::Compare {
            field: P_DURING,
            op: CmpOp::Eq,
            value: Const::Param(ParamId(0)),
        }]
    );
}

// --- the single-occurrence-residual assertion --------------------------------

/// Nothing single-occurrence survives to the residual list — across every
/// residual kind (docs/architecture/20-query-ir.md, § normalization
/// step 5).
fn assert_residuals_cross_atom(norm: &NormalizedQuery) {
    let pairs = norm
        .residuals
        .iter()
        .map(|r| (r.lhs, r.rhs))
        .chain(norm.word_residuals.iter().map(|r| (r.lhs.var, r.rhs.var)));
    for (lhs, rhs) in pairs {
        assert!(
            !norm
                .occurrences
                .iter()
                .filter(|occ| occ.polarity == Polarity::Positive)
                .any(|occ| {
                    occ.vars.iter().any(|(_, v)| *v == lhs)
                        && occ.vars.iter().any(|(_, v)| *v == rhs)
                }),
            "residual ({lhs:?}, {rhs:?}) is single-occurrence"
        );
    }
}

#[test]
fn residuals_are_never_single_occurrence_across_the_new_kinds() {
    // Targeted cases over the full comparison vocabulary: same-atom pairs
    // must lower to filters, cross-atom pairs to residuals — and every
    // residual must span occurrences.
    let two_intervals_one_atom = |op| {
        query(
            vec![
                Atom {
                    relation: P,
                    bindings: vec![(P_DURING, var(0)), (P_REVIEW, var(1))],
                },
                Atom {
                    relation: P,
                    bindings: vec![(P_DURING, var(2))],
                },
            ],
            vec![],
            vec![
                Comparison {
                    op,
                    lhs: var(0),
                    rhs: var(1),
                },
                Comparison {
                    op,
                    lhs: var(0),
                    rhs: var(2),
                },
            ],
        )
    };
    for op in [CmpOp::Overlaps, CmpOp::Contains, CmpOp::Eq, CmpOp::Ne] {
        let norm = normalized(&two_intervals_one_atom(op));
        assert_residuals_cross_atom(&norm);
        // The same-atom pair became a filter; the cross-atom pair a
        // residual (word comparisons for the interval predicates,
        // whole-value for Eq/Ne).
        assert_eq!(norm.occurrences[0].filters.len(), 1, "{op:?}");
        let residual_count = norm.residuals.len() + norm.word_residuals.len();
        assert!(
            residual_count > 0,
            "{op:?} cross-atom pair must residualize"
        );
    }

    // Scalar comparisons and membership across atoms, with a negated atom
    // in the mix: negated occurrences never absorb comparisons and never
    // host residual variables.
    let mixed = Query {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                relation: R,
                bindings: vec![(FieldId(0), var(2)), (FieldId(1), var(0))],
            },
            Atom {
                relation: S,
                bindings: vec![(FieldId(1), var(1))],
            },
        ],
        negated: vec![Atom {
            relation: R,
            bindings: vec![(FieldId(1), var(0)), (FieldId(2), var(1))],
        }],
        predicates: vec![Comparison {
            op: CmpOp::Lt,
            lhs: var(0),
            rhs: var(1),
        }],
    };
    let norm = normalized(&mixed);
    assert_residuals_cross_atom(&norm);
    // The pair co-occurs only in the *negated* atom — it must residualize
    // anyway (plan validity quantifies over positive occurrences).
    assert_eq!(norm.residuals.len(), 1);
    assert!(
        norm.occurrences[2].filters.is_empty(),
        "comparisons never lower into negated occurrences"
    );
}
