use super::lower_literal::lower_literal;
use super::*;
use crate::encoding::encode_i64;
use crate::image::view::{
    Const, FilterPredicate, IntervalConst, OperandAddr, SetConst, ViewWordSource,
};
use crate::ir::validate::validate;
use crate::ir::{
    Atom, CmpOp, Comparison, ConditionTree, FindTerm, ParamId, Query, Rule, Term, Value, WordCmp,
};
use crate::schema::Schema;
use crate::schema::ValidateDescriptor as _;
use bumbledb_theory::allen::AllenMask;
use bumbledb_theory::schema::{
    FieldDescriptor, IntervalElement, RelationDescriptor, SchemaDescriptor, ValueType,
};

fn schema() -> Schema {
    let field = |name: &str, ty: ValueType| FieldDescriptor {
        name: name.into(),
        value_type: ty,
    };
    let interval_i64 = ValueType::Interval {
        element: IntervalElement::I64,
    };
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "R".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                    },
                    field("a", ValueType::I64),
                    field("b", ValueType::I64),
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "S".into(),
                fields: vec![field("x", ValueType::U64), field("y", ValueType::I64)],
            },
            RelationDescriptor {
                extension: None,
                name: "P".into(),
                fields: vec![
                    field("emp", ValueType::U64),
                    field("during", interval_i64),
                    field("review", interval_i64),
                    field("at", ValueType::I64),
                ],
            },
            RelationDescriptor {
                extension: None,
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

const P_EMP: FieldId = FieldId(0);
const P_DURING: FieldId = FieldId(1);
const P_REVIEW: FieldId = FieldId(2);
const P_AT: FieldId = FieldId(3);

const E_AT: FieldId = FieldId(1);

fn var(id: u16) -> Term {
    Term::Var(VarId(id))
}

fn w(value: i64) -> u64 {
    u64::from_be_bytes(encode_i64(value))
}

fn normalized(query: &Query) -> NormalizedQuery {
    let schema = schema();
    let witness = validate(&schema, query).expect("valid");
    let mut rules = normalize_rules(&schema, &[], witness.rules());
    assert_eq!(rules.len(), 1, "these fixtures are one-rule queries");
    rules.remove(0)
}

fn query(atoms: Vec<Atom>, negated: Vec<Atom>, conditions: Vec<Comparison>) -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms,
        negated,
        conditions: conditions.into_iter().map(ConditionTree::Leaf).collect(),
    })
}

#[test]
fn repeated_variable_lowers_and_executes_through_the_evaluator() {
    let query = query(
        vec![Atom {
            source: crate::ir::AtomSource::Edb(R),
            bindings: vec![(FieldId(1), var(0)), (FieldId(2), var(0))],
        }],
        vec![],
        vec![],
    );
    let norm = normalized(&query);
    assert_eq!(norm.occurrences[0].role, Role::Positive);
    assert_eq!(norm.occurrences[0].vars, vec![(FieldId(1), VarId(0))]);
    assert_eq!(
        norm.occurrences[0].filters,
        vec![FilterPredicate::FieldsCompare {
            left: FieldId(1).into(),
            right: FieldId(2).into(),
            op: WordCmp::Eq,
        }]
    );
    assert!(norm.anti_probes.is_empty());
    assert_eq!(norm.slot_widths[&VarId(0)], SlotWidth::ONE);

    let schema = schema();
    let facts: Vec<Vec<crate::ir::Value>> = [(1u64, 5i64, 5i64), (2, 5, 6), (3, -1, -1)]
        .into_iter()
        .map(|(id, a, b)| {
            vec![
                crate::ir::Value::U64(id),
                crate::ir::Value::I64(a),
                crate::ir::Value::I64(b),
            ]
        })
        .collect();
    let source = crate::image::testsupport::TestSource::new(&schema, &[(R, facts)]);
    let (_cache, image) = source.image_with_cache(R);
    let filtered = crate::image::view::apply(&image, &norm.occurrences[0].filters, &[], Vec::new());

    let ids: Vec<u64> = filtered
        .positions()
        .map(|p| {
            filtered
                .bound()
                .expect("apply binds")
                .image()
                .column_words(0)[p as usize]
        })
        .collect();
    assert_eq!(ids.len(), 2);
    assert!(!ids.contains(&2));
}

#[test]
fn literal_and_param_bindings_lower_to_eq_filters() {
    let query = query(
        vec![Atom {
            source: crate::ir::AtomSource::Edb(R),
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
                field: FieldId(1).into(),
                op: WordCmp::Eq,
                value: Const::Word(w(-7)),
            },
            FilterPredicate::Compare {
                field: FieldId(2).into(),
                op: WordCmp::Eq,
                value: Const::Param(ParamId(0)),
            },
        ]
    );
}

#[test]
fn string_literals_stay_raw_as_pending_interns() {
    assert_eq!(
        lower_literal(&Value::String(Box::from("acme"))),
        Const::PendingIntern {
            bytes: Box::from(&b"acme"[..]),
        }
    );
}

#[test]
fn fixed_bytes_literals_lower_to_padded_words_with_no_dict_traffic() {
    assert_eq!(
        lower_literal(&Value::FixedBytes(Box::from(&[7u8][..]))),
        Const::Word(0x0700_0000_0000_0000)
    );
    let digest: Vec<u8> = (0u8..32).collect();
    let words = match lower_literal(&Value::FixedBytes(digest.clone().into())) {
        Const::Words(words) => words,
        other => panic!("expected a word block, got {other:?}"),
    };
    assert_eq!(words.len(), 4);
    let (digest_words, _) = digest.as_chunks::<8>();
    assert_eq!(words[0], u64::from_be_bytes(digest_words[0]));
    assert_eq!(words[3], u64::from_be_bytes(digest_words[3]));

    let nine: Vec<u8> = (1u8..=9).collect();
    assert_eq!(
        lower_literal(&Value::FixedBytes(nine.into())),
        Const::Words(Box::from(
            [0x0102_0304_0506_0708u64, 0x0900_0000_0000_0000].as_slice()
        ))
    );
}

#[test]
fn interval_literals_lower_to_encoded_word_pairs() {
    assert_eq!(
        lower_literal(&Value::IntervalU64(
            bumbledb_theory::Interval::<u64>::new(3, 9).expect("nonempty interval")
        )),
        Const::Interval { start: 3, end: 9 }
    );
    assert_eq!(
        lower_literal(&Value::IntervalI64(
            bumbledb_theory::Interval::<i64>::new(-5, 9).expect("nonempty interval")
        )),
        Const::Interval {
            start: w(-5),
            end: w(9),
        }
    );

    assert!(w(-5) < w(9));
}

#[test]
fn same_relation_atoms_get_distinct_occurrences_with_independent_filters() {
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                source: crate::ir::AtomSource::Edb(R),
                bindings: vec![
                    (FieldId(0), var(0)),
                    (FieldId(1), Term::Literal(Value::I64(1))),
                ],
            },
            Atom {
                source: crate::ir::AtomSource::Edb(R),
                bindings: vec![
                    (FieldId(0), var(1)),
                    (FieldId(1), Term::Literal(Value::I64(2))),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![],
    });
    let norm = normalized(&query);
    assert_eq!(norm.occurrences.len(), 2);
    assert_eq!(norm.occurrences[0].occ_id, OccId(0));
    assert_eq!(norm.occurrences[1].occ_id, OccId(1));
    assert_eq!(norm.occurrences[0].source().edb(), Some(R));
    assert_eq!(norm.occurrences[1].source().edb(), Some(R));
    assert_ne!(norm.occurrences[0].filters, norm.occurrences[1].filters);
}

#[test]
fn range_comparison_pushes_down_and_cross_atom_comparison_is_residual() {
    let query = query(
        vec![
            Atom {
                source: crate::ir::AtomSource::Edb(R),
                bindings: vec![(FieldId(0), var(2)), (FieldId(1), var(0))],
            },
            Atom {
                source: crate::ir::AtomSource::Edb(S),
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
            field: FieldId(1).into(),
            op: WordCmp::Ge,
            value: Const::Word(w(100)),
        }]
    );
    assert!(norm.occurrences[1].filters.is_empty());
    assert_eq!(
        norm.residuals,
        vec![FilterPredicate::FieldsCompare {
            left: OperandAddr::from(VarId(0)),
            right: OperandAddr::from(VarId(1)),
            op: WordCmp::Lt,
        }]
    );
    assert!(norm.word_residuals.is_empty());
}

#[test]
fn occurrence_vars_are_duplicate_free_over_generated_inputs() {
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

        if !bindings.iter().any(|(_, t)| *t == var(0)) {
            continue;
        }
        let query = query(
            vec![Atom {
                source: crate::ir::AtomSource::Edb(R),
                bindings,
            }],
            vec![],
            vec![],
        );

        let Ok(witness) = validate(&schema, &query) else {
            continue;
        };
        let norm = &normalize_rules(&schema, &[], witness.rules())[0];
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
                source: crate::ir::AtomSource::Edb(R),
                bindings: vec![(FieldId(0), var(0))],
            },
            Atom {
                source: crate::ir::AtomSource::Edb(S),
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
    let query = query(
        vec![Atom {
            source: crate::ir::AtomSource::Edb(R),
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
            left: FieldId(1).into(),
            right: FieldId(2).into(),
            op: WordCmp::Lt,
        }]
    );
}

#[test]
fn constant_point_membership_lowers_to_point_in() {
    let literal = query(
        vec![Atom {
            source: crate::ir::AtomSource::Edb(P),
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
            field: P_DURING.into(),
            point: ViewWordSource::Word(w(5)),
            dense: false,
        }]
    );

    let param = query(
        vec![
            Atom {
                source: crate::ir::AtomSource::Edb(P),
                bindings: vec![(P_EMP, var(0)), (P_DURING, Term::Param(ParamId(0)))],
            },
            Atom {
                source: crate::ir::AtomSource::Edb(E),
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
            field: P_DURING.into(),
            point: ViewWordSource::Param(ParamId(0)),
            dense: false,
        }]
    );
}

#[test]
fn same_atom_allen_lowers_to_the_mask_carrying_shape() {
    let allen = query(
        vec![Atom {
            source: crate::ir::AtomSource::Edb(P),
            bindings: vec![(P_DURING, var(0)), (P_REVIEW, var(1))],
        }],
        vec![],
        vec![Comparison {
            op: CmpOp::Allen {
                mask: AllenMask::INTERSECTS,
            },
            lhs: var(0),
            rhs: var(1),
        }],
    );
    let norm = normalized(&allen);
    assert!(
        norm.residuals.is_empty()
            && norm.word_residuals.is_empty()
            && norm.allen_residuals.is_empty()
    );
    assert_eq!(
        norm.occurrences[0].filters,
        vec![FilterPredicate::FieldsAllen {
            left: P_DURING.into(),
            right: P_REVIEW.into(),
            mask: AllenMask::INTERSECTS,
        }]
    );

    assert_eq!(norm.slot_widths[&VarId(0)], SlotWidth::TWO);
    assert_eq!(norm.slot_widths[&VarId(1)], SlotWidth::TWO);

    let eq = query(
        vec![Atom {
            source: crate::ir::AtomSource::Edb(P),
            bindings: vec![(P_DURING, var(0)), (P_REVIEW, var(1))],
        }],
        vec![],
        vec![Comparison {
            op: CmpOp::Eq,
            lhs: var(0),
            rhs: var(1),
        }],
    );
    assert_eq!(
        normalized(&eq).occurrences[0].filters,
        vec![FilterPredicate::FieldsAllen {
            left: P_DURING.into(),
            right: P_REVIEW.into(),
            mask: AllenMask::EQUALS,
        }]
    );
    let ne = query(
        vec![Atom {
            source: crate::ir::AtomSource::Edb(P),
            bindings: vec![(P_DURING, var(0)), (P_REVIEW, var(1))],
        }],
        vec![],
        vec![Comparison {
            op: CmpOp::Ne,
            lhs: var(0),
            rhs: var(1),
        }],
    );
    assert_eq!(
        normalized(&ne).occurrences[0].filters,
        vec![FilterPredicate::FieldsAllen {
            left: P_DURING.into(),
            right: P_REVIEW.into(),
            mask: AllenMask::EQUALS.complement(),
        }]
    );

    let point_in = query(
        vec![Atom {
            source: crate::ir::AtomSource::Edb(P),
            bindings: vec![(P_DURING, var(1)), (P_AT, var(0))],
        }],
        vec![],
        vec![Comparison {
            op: CmpOp::PointIn,
            lhs: var(1),
            rhs: var(0),
        }],
    );
    let norm = normalized(&point_in);
    assert_eq!(
        norm.occurrences[0].filters,
        vec![FilterPredicate::FieldsPointIn {
            interval: P_DURING.into(),
            point: P_AT.into(),
            dense: false,
        }]
    );
    assert_eq!(norm.slot_widths[&VarId(0)], SlotWidth::ONE);
}

#[test]
fn negated_atom_with_literal_binding_lowers_to_anti_probe() {
    let query = query(
        vec![Atom {
            source: crate::ir::AtomSource::Edb(R),
            bindings: vec![(FieldId(0), var(0))],
        }],
        vec![Atom {
            source: crate::ir::AtomSource::Edb(S),
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
    assert_eq!(negated.role, Role::Negated);
    assert_eq!(negated.source().edb(), Some(S));
    assert_eq!(negated.vars, vec![(FieldId(0), VarId(0))]);
    assert_eq!(
        negated.filters,
        vec![FilterPredicate::Compare {
            field: FieldId(1).into(),
            op: WordCmp::Eq,
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
fn cross_atom_allen_becomes_the_mask_residual() {
    let allen = query(
        vec![
            Atom {
                source: crate::ir::AtomSource::Edb(P),
                bindings: vec![(P_DURING, var(0))],
            },
            Atom {
                source: crate::ir::AtomSource::Edb(P),
                bindings: vec![(P_DURING, var(1))],
            },
        ],
        vec![],
        vec![Comparison {
            op: CmpOp::Allen {
                mask: AllenMask::INTERSECTS,
            },
            lhs: var(0),
            rhs: var(1),
        }],
    );
    let norm = normalized(&allen);
    assert!(norm.residuals.is_empty() && norm.word_residuals.is_empty());
    assert_eq!(
        norm.allen_residuals,
        vec![FilterPredicate::FieldsAllen {
            left: OperandAddr::from(VarId(0)),
            right: OperandAddr::from(VarId(1)),
            mask: AllenMask::INTERSECTS,
        }]
    );

    let eq = query(
        vec![
            Atom {
                source: crate::ir::AtomSource::Edb(P),
                bindings: vec![(P_DURING, var(0))],
            },
            Atom {
                source: crate::ir::AtomSource::Edb(P),
                bindings: vec![(P_DURING, var(1))],
            },
        ],
        vec![],
        vec![Comparison {
            op: CmpOp::Eq,
            lhs: var(0),
            rhs: var(1),
        }],
    );
    let eq_norm = normalized(&eq);
    assert!(eq_norm.residuals.is_empty());
    assert_eq!(
        eq_norm.allen_residuals,
        vec![FilterPredicate::FieldsAllen {
            left: OperandAddr::from(VarId(0)),
            right: OperandAddr::from(VarId(1)),
            mask: AllenMask::EQUALS,
        }]
    );
    assert_eq!(norm.slot_widths[&VarId(0)], SlotWidth::TWO);
    assert_eq!(norm.slot_widths[&VarId(1)], SlotWidth::TWO);

    let point_in = query(
        vec![
            Atom {
                source: crate::ir::AtomSource::Edb(P),
                bindings: vec![(P_DURING, var(0))],
            },
            Atom {
                source: crate::ir::AtomSource::Edb(E),
                bindings: vec![(E_AT, var(1))],
            },
        ],
        vec![],
        vec![Comparison {
            op: CmpOp::PointIn,
            lhs: var(0),
            rhs: var(1),
        }],
    );
    let norm = normalized(&point_in);
    assert_eq!(
        norm.word_residuals,
        vec![
            FilterPredicate::FieldsCompare {
                op: WordCmp::Le,
                left: OperandAddr::var_word(VarId(0), IntervalWord::Start.offset()),
                right: OperandAddr::var_word(VarId(1), IntervalWord::Start.offset()),
            },
            FilterPredicate::FieldsCompare {
                op: WordCmp::Lt,
                left: OperandAddr::var_word(VarId(1), IntervalWord::Start.offset()),
                right: OperandAddr::var_word(VarId(0), IntervalWord::End.offset()),
            },
        ]
    );
    assert_eq!(norm.slot_widths[&VarId(1)], SlotWidth::ONE);
}

#[test]
fn scalar_param_set_binding_is_the_selection_set_marker() {
    let scalar = query(
        vec![Atom {
            source: crate::ir::AtomSource::Edb(S),
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
            field: FieldId(1).into(),
            op: WordCmp::Eq,
            value: Const::ParamSet(ParamId(0)),
        }]
    );

    let point_set = query(
        vec![Atom {
            source: crate::ir::AtomSource::Edb(P),
            bindings: vec![(P_EMP, var(0)), (P_DURING, Term::ParamSet(ParamId(0)))],
        }],
        vec![],
        vec![],
    );
    assert_eq!(
        normalized(&point_set).occurrences[0].filters,
        vec![FilterPredicate::AnyPointIn {
            field: P_DURING.into(),
            set: SetConst::ParamSet(ParamId(0)),
            dense: false,
        }]
    );
}

#[test]
fn same_atom_membership_variable_lowers_to_the_field_composition() {
    // binding order must not matter (the membership binding comes first).
    let query = query(
        vec![Atom {
            source: crate::ir::AtomSource::Edb(P),
            bindings: vec![(P_DURING, var(0)), (P_AT, var(0))],
        }],
        vec![],
        vec![],
    );
    let norm = normalized(&query);
    assert_eq!(norm.occurrences[0].vars, vec![(P_AT, VarId(0))]);
    assert_eq!(
        norm.occurrences[0].filters,
        vec![FilterPredicate::FieldsPointIn {
            interval: P_DURING.into(),
            point: P_AT.into(),
            dense: false,
        }]
    );
}

#[test]
fn cross_atom_membership_variable_lowers_to_point_in_over_the_binding() {
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                source: crate::ir::AtomSource::Edb(P),
                bindings: vec![(P_EMP, var(1)), (P_DURING, var(0))],
            },
            Atom {
                source: crate::ir::AtomSource::Edb(E),
                bindings: vec![(E_AT, var(0))],
            },
        ],
        negated: vec![],
        conditions: vec![],
    });
    let norm = normalized(&query);
    assert_eq!(norm.occurrences[0].vars, vec![(P_EMP, VarId(1))]);
    assert_eq!(
        norm.occurrences[0].point_vars,
        vec![(P_DURING, VarId(0), false)]
    );
    assert!(norm.occurrences[0].filters.is_empty());
    assert_eq!(norm.occurrences[1].vars, vec![(E_AT, VarId(0))]);
}

#[test]
fn interval_param_equality_binding_stays_an_eq_compare() {
    let query = query(
        vec![Atom {
            source: crate::ir::AtomSource::Edb(P),
            bindings: vec![(P_EMP, var(0)), (P_DURING, Term::Param(ParamId(0)))],
        }],
        vec![],
        vec![],
    );
    assert_eq!(
        normalized(&query).occurrences[0].filters,
        vec![FilterPredicate::Compare {
            field: P_DURING.into(),
            op: WordCmp::Eq,
            value: Const::Param(ParamId(0)),
        }]
    );
}

fn assert_residuals_cross_atom(norm: &NormalizedQuery) {
    let pairs = norm
        .residuals
        .iter()
        .map(|r| {
            let (left, right, _) = r.compare_sides();
            (left.var(), right.var())
        })
        .chain(norm.word_residuals.iter().map(|r| {
            let (left, right, _) = r.compare_sides();
            (left.var(), right.var())
        }))
        .chain(norm.allen_residuals.iter().map(|r| {
            let (left, right, _) = r.allen_sides();
            (left.var(), right.var())
        }));
    for (lhs, rhs) in pairs {
        assert!(
            !norm
                .occurrences
                .iter()
                .filter(|occ| occ.role == Role::Positive)
                .any(|occ| {
                    occ.vars.iter().any(|(_, v)| *v == lhs)
                        && occ.vars.iter().any(|(_, v)| *v == rhs)
                }),
            "residual ({lhs:?}, {rhs:?}) is single-occurrence"
        );
    }
}

#[test]
fn sweep_scalar_var_var_placements() {
    for op in [CmpOp::Lt, CmpOp::Ge, CmpOp::Eq, CmpOp::Ne] {
        let same = query(
            vec![Atom {
                source: crate::ir::AtomSource::Edb(R),
                bindings: vec![
                    (FieldId(0), var(0)),
                    (FieldId(1), var(1)),
                    (FieldId(2), var(2)),
                ],
            }],
            vec![],
            vec![Comparison {
                op,
                lhs: var(1),
                rhs: var(2),
            }],
        );
        let norm = normalized(&same);
        assert_eq!(
            norm.occurrences[0].filters,
            vec![FilterPredicate::FieldsCompare {
                left: FieldId(1).into(),
                right: FieldId(2).into(),
                op: WordCmp::from_cmp(op).expect("scalar"),
            }],
            "{op:?}"
        );
        assert!(norm.residuals.is_empty(), "{op:?}");

        let cross = query(
            vec![
                Atom {
                    source: crate::ir::AtomSource::Edb(R),
                    bindings: vec![(FieldId(0), var(0)), (FieldId(1), var(1))],
                },
                Atom {
                    source: crate::ir::AtomSource::Edb(S),
                    bindings: vec![(FieldId(1), var(2))],
                },
            ],
            vec![],
            vec![Comparison {
                op,
                lhs: var(1),
                rhs: var(2),
            }],
        );
        let norm = normalized(&cross);
        assert!(norm.occurrences.iter().all(|occ| occ.filters.is_empty()));
        assert_eq!(
            norm.residuals,
            vec![FilterPredicate::FieldsCompare {
                left: OperandAddr::from(VarId(1)),
                right: OperandAddr::from(VarId(2)),
                op: WordCmp::from_cmp(op).expect("scalar"),
            }],
            "{op:?}"
        );
    }
}

#[test]
fn sweep_scalar_var_const_placements() {
    let r_atom = || Atom {
        source: crate::ir::AtomSource::Edb(R),
        bindings: vec![(FieldId(0), var(0)), (FieldId(1), var(1))],
    };
    let cases = [
        (CmpOp::Lt, false, CmpOp::Lt),
        (CmpOp::Le, true, CmpOp::Ge),
        (CmpOp::Gt, true, CmpOp::Lt),
        (CmpOp::Ge, false, CmpOp::Ge),
        (CmpOp::Eq, true, CmpOp::Eq),
        (CmpOp::Ne, false, CmpOp::Ne),
    ];
    for (op, const_first, placed_op) in cases {
        for (constant, value) in [
            (Term::Literal(Value::I64(-3)), Const::Word(w(-3))),
            (Term::Param(ParamId(0)), Const::Param(ParamId(0))),
        ] {
            let (lhs, rhs) = if const_first {
                (constant, var(1))
            } else {
                (var(1), constant)
            };
            let q = query(vec![r_atom()], vec![], vec![Comparison { op, lhs, rhs }]);
            let norm = normalized(&q);
            assert_eq!(
                norm.occurrences[0].filters,
                vec![FilterPredicate::Compare {
                    field: FieldId(1).into(),
                    op: WordCmp::from_cmp(placed_op).expect("scalar"),
                    value,
                }],
                "{op:?} const_first={const_first}"
            );
            assert!(norm.residuals.is_empty());
        }
    }
}

#[test]
fn sweep_param_set_comparison_placements() {
    for const_first in [false, true] {
        let set = Term::ParamSet(ParamId(0));
        let (lhs, rhs) = if const_first {
            (set, var(1))
        } else {
            (var(1), set)
        };
        let q = query(
            vec![Atom {
                source: crate::ir::AtomSource::Edb(R),
                bindings: vec![(FieldId(0), var(0)), (FieldId(1), var(1))],
            }],
            vec![],
            vec![Comparison {
                op: CmpOp::Eq,
                lhs,
                rhs,
            }],
        );
        assert_eq!(
            normalized(&q).occurrences[0].filters,
            vec![FilterPredicate::Compare {
                field: FieldId(1).into(),
                op: WordCmp::Eq,
                value: Const::ParamSet(ParamId(0)),
            }],
            "const_first={const_first}"
        );
    }
}

#[test]
fn sweep_contains_param_placements() {
    let point_param = query(
        vec![Atom {
            source: crate::ir::AtomSource::Edb(P),
            bindings: vec![(P_EMP, var(0)), (P_DURING, var(1))],
        }],
        vec![],
        vec![Comparison {
            op: CmpOp::PointIn,
            lhs: var(1),
            rhs: Term::Param(ParamId(0)),
        }],
    );
    assert_eq!(
        normalized(&point_param).occurrences[0].filters,
        vec![FilterPredicate::PointIn {
            field: P_DURING.into(),
            point: ViewWordSource::Param(ParamId(0)),
            dense: false,
        }]
    );

    let within_param = query(
        vec![Atom {
            source: crate::ir::AtomSource::Edb(E),
            bindings: vec![(FieldId(0), var(0)), (E_AT, var(1))],
        }],
        vec![],
        vec![Comparison {
            op: CmpOp::PointIn,
            lhs: Term::Param(ParamId(0)),
            rhs: var(1),
        }],
    );
    assert_eq!(
        normalized(&within_param).occurrences[0].filters,
        vec![FilterPredicate::FieldWithin {
            field: E_AT.into(),
            outer: IntervalConst::Param(ParamId(0)),
            dense: false,
        }]
    );
}

#[test]
fn residuals_are_never_single_occurrence_across_the_new_kinds() {
    let two_intervals_one_atom = |op| {
        query(
            vec![
                Atom {
                    source: crate::ir::AtomSource::Edb(P),
                    bindings: vec![(P_DURING, var(0)), (P_REVIEW, var(1))],
                },
                Atom {
                    source: crate::ir::AtomSource::Edb(P),
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
    for op in [
        CmpOp::Allen {
            mask: AllenMask::INTERSECTS,
        },
        CmpOp::Allen {
            mask: AllenMask::COVERS,
        },
        CmpOp::Eq,
        CmpOp::Ne,
    ] {
        let norm = normalized(&two_intervals_one_atom(op));
        assert_residuals_cross_atom(&norm);

        assert_eq!(norm.occurrences[0].filters.len(), 1, "{op:?}");
        assert_eq!(
            norm.allen_residuals.len(),
            1,
            "{op:?} cross-atom pair must residualize as a mask"
        );
    }

    let mixed = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                source: crate::ir::AtomSource::Edb(R),
                bindings: vec![(FieldId(0), var(2)), (FieldId(1), var(0))],
            },
            Atom {
                source: crate::ir::AtomSource::Edb(S),
                bindings: vec![(FieldId(1), var(1))],
            },
        ],
        negated: vec![Atom {
            source: crate::ir::AtomSource::Edb(R),
            bindings: vec![(FieldId(1), var(0)), (FieldId(2), var(1))],
        }],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Lt,
            lhs: var(0),
            rhs: var(1),
        })],
    });
    let norm = normalized(&mixed);
    assert_residuals_cross_atom(&norm);

    assert_eq!(norm.residuals.len(), 1);
    assert!(
        norm.occurrences[2].filters.is_empty(),
        "comparisons never lower into negated occurrences"
    );
}
