use super::lower_literal::lower_literal;
use super::*;
use crate::allen::AllenMask;
use crate::encoding::{ValueRef, encode_fact, encode_i64};
use crate::image::view::{Const, MaskConst, ResolvedWordSource};
use crate::ir::validate::validate;
use crate::ir::{
    Atom, Comparison, ConditionTree, FindTerm, MaskTerm, ParamId, Query, Rule, Term, Value,
};
use crate::schema::{
    FieldDescriptor, Generation, IntervalElement, RelationDescriptor, Schema, SchemaDescriptor,
    ValueType,
};
use crate::storage::commit::commit;
use crate::storage::delta::WriteDelta;
use crate::storage::env::Environment;
use crate::testutil::TempDir;

/// R(id u64 fresh, a i64, b i64) + S(x u64, y i64)
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
                extension: None,
                name: "R".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                        generation: Generation::Fresh,
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
                    field("during", interval_i64.clone()),
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
    let mut rules = normalize(&schema, &validate(&schema, query).expect("valid"));
    assert_eq!(rules.len(), 1, "these fixtures are one-rule programs");
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
    assert_eq!(norm.occurrences[0].role, Role::Positive);
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
    assert_eq!(norm.slot_widths[&VarId(0)], SlotWidth::ONE);

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
    let filtered = crate::image::view::apply(&image, &norm.occurrences[0].filters, &[], Vec::new())
        .expect("no measure filters");
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
            bytes: Box::from(&b"acme"[..]),
        }
    );
}

#[test]
fn fixed_bytes_literals_lower_to_padded_words_with_no_dict_traffic() {
    // bytes<N> is self-encoding: N ≤ 8 is one padded BE word, N > 8 its
    // ⌈N/8⌉ words — never a PendingIntern (zero dictionary traffic).
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
    // A pad-boundary width: 9 bytes = 2 words, tail zero-padded.
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
    // Each half is encoded exactly like the scalar of its element type.
    assert_eq!(
        lower_literal(&Value::IntervalU64(
            crate::Interval::<u64>::new(3, 9).expect("nonempty interval")
        )),
        Const::Interval { start: 3, end: 9 }
    );
    assert_eq!(
        lower_literal(&Value::IntervalI64(
            crate::Interval::<i64>::new(-5, 9).expect("nonempty interval")
        )),
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
    let query = Query::single(Rule {
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
        conditions: vec![],
    });
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
        let norm = &normalize(&schema, &witness)[0];
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
fn same_atom_allen_lowers_to_the_mask_carrying_shape() {
    // Golden (b): P(during = x, review = y), Allen(x, y, INTERSECTS) —
    // the mask rides the same-atom shape as one filter kind, never a
    // residual.
    let allen = query(
        vec![Atom {
            relation: P,
            bindings: vec![(P_DURING, var(0)), (P_REVIEW, var(1))],
        }],
        vec![],
        vec![Comparison {
            op: CmpOp::Allen {
                mask: MaskTerm::Literal(AllenMask::INTERSECTS),
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
            left: P_DURING,
            right: P_REVIEW,
            mask: MaskConst::Mask(AllenMask::INTERSECTS),
        }]
    );
    // Interval variables occupy two slots each.
    assert_eq!(norm.slot_widths[&VarId(0)], SlotWidth::TWO);
    assert_eq!(norm.slot_widths[&VarId(1)], SlotWidth::TWO);

    // Interval Eq canonicalizes to the EQUALS mask (Ne to its
    // complement): exactly one interval-pair form leaves normalization.
    let eq = query(
        vec![Atom {
            relation: P,
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
            left: P_DURING,
            right: P_REVIEW,
            mask: MaskConst::Mask(AllenMask::EQUALS),
        }]
    );
    let ne = query(
        vec![Atom {
            relation: P,
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
            left: P_DURING,
            right: P_REVIEW,
            mask: MaskConst::Mask(AllenMask::EQUALS.complement()),
        }]
    );

    // PointIn's surviving point form: same-atom membership predicate.
    let point_in = query(
        vec![Atom {
            relation: P,
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
            interval: P_DURING,
            point: P_AT,
        }]
    );
    assert_eq!(norm.slot_widths[&VarId(0)], SlotWidth::ONE);
}

#[test]
fn negated_atom_with_literal_binding_lowers_to_anti_probe() {
    // Golden (c): R(id = v0), ¬S(x = v0, y = -7) — the negated atom is an
    // occurrence with the Negated role in the one table; its literal
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
    assert_eq!(negated.role, Role::Negated);
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
#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)] // one lowering case per residual form
fn cross_atom_allen_becomes_the_mask_residual() {
    // Golden (d): P(during = x), P(during = y), Allen(x, y, m) — the
    // residual carries the mask whole (four endpoint slots + mask);
    // nothing decomposes.
    let allen = query(
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
            op: CmpOp::Allen {
                mask: MaskTerm::Literal(AllenMask::INTERSECTS),
            },
            lhs: var(0),
            rhs: var(1),
        }],
    );
    let norm = normalized(&allen);
    assert!(norm.residuals.is_empty() && norm.word_residuals.is_empty());
    assert_eq!(
        norm.allen_residuals,
        vec![PlacedAllen {
            lhs: VarId(0),
            rhs: VarId(1),
            mask: MaskTerm::Literal(AllenMask::INTERSECTS),
        }]
    );
    // Cross-atom interval Eq canonicalizes into the same residual kind.
    let eq = query(
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
            op: CmpOp::Eq,
            lhs: var(0),
            rhs: var(1),
        }],
    );
    let eq_norm = normalized(&eq);
    assert!(eq_norm.residuals.is_empty());
    assert_eq!(
        eq_norm.allen_residuals,
        vec![PlacedAllen {
            lhs: VarId(0),
            rhs: VarId(1),
            mask: MaskTerm::Literal(AllenMask::EQUALS),
        }]
    );
    assert_eq!(norm.slot_widths[&VarId(0)], SlotWidth::TWO);
    assert_eq!(norm.slot_widths[&VarId(1)], SlotWidth::TWO);

    // Cross-atom PointIn over a point variable: x.start ≤ t AND t < x.end
    // — the point variable's single word is its Start word.
    let start = |id: u16| VarWord {
        var: VarId(id),
        word: IntervalWord::Start,
    };
    let end = |id: u16| VarWord {
        var: VarId(id),
        word: IntervalWord::End,
    };
    let point_in = query(
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
            op: CmpOp::PointIn,
            lhs: var(0),
            rhs: var(1),
        }],
    );
    let norm = normalized(&point_in);
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
    assert_eq!(norm.slot_widths[&VarId(1)], SlotWidth::ONE);
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
            set: Const::ParamSet(ParamId(0)),
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
        vec![FilterPredicate::FieldsPointIn {
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
    let query = Query::single(Rule {
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
        conditions: vec![],
    });
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
#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)] // a fixed list, one entry per const shape
fn constant_interval_comparisons_lower_to_fixed_const_shapes() {
    let iv = || {
        Term::Literal(Value::IntervalI64(
            crate::Interval::<i64>::new(2, 9).expect("nonempty interval"),
        ))
    };
    let iv_const = Const::Interval {
        start: w(2),
        end: w(9),
    };
    let p_atom = || Atom {
        relation: P,
        bindings: vec![(P_DURING, var(0))],
    };

    // Allen(x, [2,9), INTERSECTS) — the field stays on the left.
    let intersects = query(
        vec![p_atom()],
        vec![],
        vec![Comparison {
            op: CmpOp::Allen {
                mask: MaskTerm::Literal(AllenMask::INTERSECTS),
            },
            lhs: var(0),
            rhs: iv(),
        }],
    );
    assert_eq!(
        normalized(&intersects).occurrences[0].filters,
        vec![FilterPredicate::FieldAllen {
            field: P_DURING,
            other: iv_const.clone(),
            mask: MaskConst::Mask(AllenMask::INTERSECTS),
        }]
    );

    // Allen([2,9), x, COVERS) — constant-first mirrors as the converse
    // mask; the field stays the left operand.
    let mirrored = query(
        vec![p_atom()],
        vec![],
        vec![Comparison {
            op: CmpOp::Allen {
                mask: MaskTerm::Literal(AllenMask::COVERS),
            },
            lhs: iv(),
            rhs: var(0),
        }],
    );
    assert_eq!(
        normalized(&mirrored).occurrences[0].filters,
        vec![FilterPredicate::FieldAllen {
            field: P_DURING,
            other: iv_const.clone(),
            mask: MaskConst::Mask(AllenMask::COVERED_BY),
        }]
    );

    // ...and a mirrored param mask defers the converse to bind.
    let mirrored_param = query(
        vec![p_atom()],
        vec![],
        vec![Comparison {
            op: CmpOp::Allen {
                mask: MaskTerm::Param(ParamId(0)),
            },
            lhs: iv(),
            rhs: var(0),
        }],
    );
    assert_eq!(
        normalized(&mirrored_param).occurrences[0].filters,
        vec![FilterPredicate::FieldAllen {
            field: P_DURING,
            other: iv_const.clone(),
            mask: MaskConst::ConversedParam(ParamId(0)),
        }]
    );

    // Ne(x, [2,9)) canonicalizes: Allen(¬EQUALS) against the constant.
    let ne = query(
        vec![p_atom()],
        vec![],
        vec![Comparison {
            op: CmpOp::Ne,
            lhs: var(0),
            rhs: iv(),
        }],
    );
    assert_eq!(
        normalized(&ne).occurrences[0].filters,
        vec![FilterPredicate::FieldAllen {
            field: P_DURING,
            other: iv_const.clone(),
            mask: MaskConst::Mask(AllenMask::EQUALS.complement()),
        }]
    );

    // PointIn([2,9), t) over a scalar variable — the reversed point
    // containment on the point's field.
    let point_within = query(
        vec![Atom {
            relation: E,
            bindings: vec![(FieldId(0), var(0)), (E_AT, var(1))],
        }],
        vec![],
        vec![Comparison {
            op: CmpOp::PointIn,
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

    // PointIn(x, 5) — a constant point is membership.
    let point_in = query(
        vec![p_atom()],
        vec![],
        vec![Comparison {
            op: CmpOp::PointIn,
            lhs: var(0),
            rhs: Term::Literal(Value::I64(5)),
        }],
    );
    assert_eq!(
        normalized(&point_in).occurrences[0].filters,
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
        .chain(norm.word_residuals.iter().map(|r| (r.lhs.var, r.rhs.var)))
        .chain(norm.allen_residuals.iter().map(|r| (r.lhs, r.rhs)));
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

// --- the classified-comparison placement sweep (PRD 08 pin) -----------------
//
// One case per legal comparison shape validation accepts × a representative
// rule shape (same-atom and cross-atom where the placement differs, both
// written operand orders where mirroring applies). Pinned GREEN against the
// re-deriving placer before the classification seal landed; the assertion
// values are behavior, not implementation.

#[test]
fn sweep_scalar_var_var_placements() {
    // Same-atom: a field composition; cross-atom: a whole-value residual —
    // for the order and (scalar) equality forms alike.
    for op in [CmpOp::Lt, CmpOp::Ge, CmpOp::Eq, CmpOp::Ne] {
        let same = query(
            vec![Atom {
                relation: R,
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
                left: FieldId(1),
                right: FieldId(2),
                op,
            }],
            "{op:?}"
        );
        assert!(norm.residuals.is_empty(), "{op:?}");

        let cross = query(
            vec![
                Atom {
                    relation: R,
                    bindings: vec![(FieldId(0), var(0)), (FieldId(1), var(1))],
                },
                Atom {
                    relation: S,
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
            vec![PlacedComparison {
                op,
                lhs: VarId(1),
                rhs: VarId(2),
            }],
            "{op:?}"
        );
    }
}

#[test]
fn sweep_scalar_var_const_placements() {
    // Literal and param constants, written variable-first and
    // constant-first: the filter's operator is sealed variable-on-left
    // (a constant-first order comparison mirrors; Eq/Ne are symmetric).
    let r_atom = || Atom {
        relation: R,
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
                    field: FieldId(1),
                    op: placed_op,
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
    // The set marker under Eq (its one legal operator), both written
    // orders: the selection-level Eq compare against the set.
    for const_first in [false, true] {
        let set = Term::ParamSet(ParamId(0));
        let (lhs, rhs) = if const_first {
            (set, var(1))
        } else {
            (var(1), set)
        };
        let q = query(
            vec![Atom {
                relation: R,
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
                field: FieldId(1),
                op: CmpOp::Eq,
                value: Const::ParamSet(ParamId(0)),
            }],
            "const_first={const_first}"
        );
    }
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)] // one pinned placement per param-side Allen shape
fn sweep_allen_param_placements() {
    // The param-side Allen shapes the goldens don't already pin: a param
    // mask on the same-atom and cross-atom variable pairs, a param
    // constant side, and interval equality against a param (both written
    // orders — the EQUALS mask is its own converse).
    let mask_param = CmpOp::Allen {
        mask: MaskTerm::Param(ParamId(0)),
    };
    let same = query(
        vec![Atom {
            relation: P,
            bindings: vec![(P_DURING, var(0)), (P_REVIEW, var(1))],
        }],
        vec![],
        vec![Comparison {
            op: mask_param,
            lhs: var(0),
            rhs: var(1),
        }],
    );
    assert_eq!(
        normalized(&same).occurrences[0].filters,
        vec![FilterPredicate::FieldsAllen {
            left: P_DURING,
            right: P_REVIEW,
            mask: MaskConst::Param(ParamId(0)),
        }]
    );
    let cross = query(
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
            op: mask_param,
            lhs: var(0),
            rhs: var(1),
        }],
    );
    assert_eq!(
        normalized(&cross).allen_residuals,
        vec![PlacedAllen {
            lhs: VarId(0),
            rhs: VarId(1),
            mask: MaskTerm::Param(ParamId(0)),
        }]
    );

    // A param constant side under a literal mask.
    let p_atom = || Atom {
        relation: P,
        bindings: vec![(P_DURING, var(0))],
    };
    let against_param = query(
        vec![p_atom()],
        vec![],
        vec![Comparison {
            op: CmpOp::Allen {
                mask: MaskTerm::Literal(AllenMask::INTERSECTS),
            },
            lhs: var(0),
            rhs: Term::Param(ParamId(0)),
        }],
    );
    assert_eq!(
        normalized(&against_param).occurrences[0].filters,
        vec![FilterPredicate::FieldAllen {
            field: P_DURING,
            other: Const::Param(ParamId(0)),
            mask: MaskConst::Mask(AllenMask::INTERSECTS),
        }]
    );

    // Interval equality against a param canonicalizes to the EQUALS mask;
    // written constant-first it seals the converse (EQUALS is symmetric).
    for const_first in [false, true] {
        let param = Term::Param(ParamId(0));
        let (lhs, rhs) = if const_first {
            (param, var(0))
        } else {
            (var(0), param)
        };
        let eq = query(
            vec![p_atom()],
            vec![],
            vec![Comparison {
                op: CmpOp::Eq,
                lhs,
                rhs,
            }],
        );
        let expected_mask = if const_first {
            AllenMask::EQUALS.converse()
        } else {
            AllenMask::EQUALS
        };
        assert_eq!(
            normalized(&eq).occurrences[0].filters,
            vec![FilterPredicate::FieldAllen {
                field: P_DURING,
                other: Const::Param(ParamId(0)),
                mask: MaskConst::Mask(expected_mask),
            }],
            "const_first={const_first}"
        );
    }
    // ...and interval Ne against a param seals the complement.
    let ne = query(
        vec![p_atom()],
        vec![],
        vec![Comparison {
            op: CmpOp::Ne,
            lhs: var(0),
            rhs: Term::Param(ParamId(0)),
        }],
    );
    assert_eq!(
        normalized(&ne).occurrences[0].filters,
        vec![FilterPredicate::FieldAllen {
            field: P_DURING,
            other: Const::Param(ParamId(0)),
            mask: MaskConst::Mask(AllenMask::EQUALS.complement()),
        }]
    );
}

#[test]
fn sweep_contains_param_placements() {
    // The param sides of both containment directions (literal sides are
    // pinned by the const-shape golden): a param point in an interval
    // variable, and a scalar variable within a param interval.
    let point_param = query(
        vec![Atom {
            relation: P,
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
            field: P_DURING,
            point: ResolvedWordSource::Param(ParamId(0)),
        }]
    );

    let within_param = query(
        vec![Atom {
            relation: E,
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
            field: E_AT,
            outer: Const::Param(ParamId(0)),
        }]
    );
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)] // one pinned placement per measure shape
fn sweep_duration_placements() {
    // The measure's placements: constant sides (literal and param, both
    // written orders — measure-second mirrors the operator), the
    // same-atom variable side, and the cross-atom residual.
    let p_atom = || Atom {
        relation: P,
        bindings: vec![(P_EMP, var(0)), (P_DURING, var(1))],
    };
    let duration = || Term::Measure(VarId(1));

    let literal = query(
        vec![p_atom()],
        vec![],
        vec![Comparison {
            op: CmpOp::Lt,
            lhs: duration(),
            rhs: Term::Literal(Value::U64(5)),
        }],
    );
    assert_eq!(
        normalized(&literal).occurrences[0].filters,
        vec![FilterPredicate::DurationCompare {
            field: P_DURING,
            op: CmpOp::Lt,
            value: Const::Word(5),
        }]
    );

    // Written measure-second: `5 ≥ Duration(x)` places as `≤`.
    let mirrored = query(
        vec![p_atom()],
        vec![],
        vec![Comparison {
            op: CmpOp::Ge,
            lhs: Term::Literal(Value::U64(5)),
            rhs: duration(),
        }],
    );
    assert_eq!(
        normalized(&mirrored).occurrences[0].filters,
        vec![FilterPredicate::DurationCompare {
            field: P_DURING,
            op: CmpOp::Le,
            value: Const::Word(5),
        }]
    );

    let param = query(
        vec![p_atom()],
        vec![],
        vec![Comparison {
            op: CmpOp::Le,
            lhs: duration(),
            rhs: Term::Param(ParamId(0)),
        }],
    );
    assert_eq!(
        normalized(&param).occurrences[0].filters,
        vec![FilterPredicate::DurationCompare {
            field: P_DURING,
            op: CmpOp::Le,
            value: Const::Param(ParamId(0)),
        }]
    );

    // Same-atom u64 variable side: the field composition.
    let same_atom = query(
        vec![p_atom()],
        vec![],
        vec![Comparison {
            op: CmpOp::Gt,
            lhs: duration(),
            rhs: var(0),
        }],
    );
    assert_eq!(
        normalized(&same_atom).occurrences[0].filters,
        vec![FilterPredicate::DurationFieldsCompare {
            interval: P_DURING,
            op: CmpOp::Gt,
            scalar: P_EMP,
        }]
    );

    // Cross-atom u64 variable side: the measure residual — and written
    // measure-second it mirrors, exactly like the constant form.
    for (lhs, rhs, placed_op) in [
        (duration(), var(2), CmpOp::Lt),
        (var(2), duration(), CmpOp::Gt),
    ] {
        let cross = query(
            vec![
                p_atom(),
                Atom {
                    relation: S,
                    bindings: vec![(FieldId(0), var(2))],
                },
            ],
            vec![],
            vec![Comparison {
                op: CmpOp::Lt,
                lhs,
                rhs,
            }],
        );
        let norm = normalized(&cross);
        assert!(norm.occurrences.iter().all(|occ| occ.filters.is_empty()));
        assert_eq!(
            norm.duration_residuals,
            vec![PlacedDuration {
                interval: VarId(1),
                op: placed_op,
                scalar: VarId(2),
            }]
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
    for op in [
        CmpOp::Allen {
            mask: MaskTerm::Literal(AllenMask::INTERSECTS),
        },
        CmpOp::Allen {
            mask: MaskTerm::Literal(AllenMask::COVERS),
        },
        CmpOp::Eq,
        CmpOp::Ne,
    ] {
        let norm = normalized(&two_intervals_one_atom(op));
        assert_residuals_cross_atom(&norm);
        // The same-atom pair became a filter; the cross-atom pair a
        // residual (Allen masks for every interval-pair form — Eq/Ne
        // canonicalize into them).
        assert_eq!(norm.occurrences[0].filters.len(), 1, "{op:?}");
        assert_eq!(
            norm.allen_residuals.len(),
            1,
            "{op:?} cross-atom pair must residualize as a mask"
        );
    }

    // Scalar comparisons and membership across atoms, with a negated atom
    // in the mix: negated occurrences never absorb comparisons and never
    // host residual variables.
    let mixed = Query::single(Rule {
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
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Lt,
            lhs: var(0),
            rhs: var(1),
        })],
    });
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
