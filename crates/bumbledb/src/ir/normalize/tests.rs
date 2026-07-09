use super::lower_literal::lower_literal;
use super::*;
use crate::encoding::{encode_fact, encode_i64, ValueRef};
use crate::image::view::Const;
use crate::ir::validate::validate;
use crate::ir::{Atom, Comparison, FindTerm, ParamId, Query, Term, Value};
use crate::schema::{
    FieldDescriptor, Generation, RelationDescriptor, Schema, SchemaDescriptor, ValueType,
};
use crate::storage::commit::commit;
use crate::storage::delta::WriteDelta;
use crate::storage::dict::{TAG_BYTES, TAG_STRING};
use crate::storage::env::Environment;
use crate::testutil::TempDir;

/// R(id u64 serial, a i64, b i64) + S(x u64, y i64).
fn schema() -> Schema {
    let field = |name: &str, ty: ValueType| FieldDescriptor {
        name: name.into(),
        value_type: ty,
        generation: Generation::None,
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
                constraints: vec![],
            },
            RelationDescriptor {
                name: "S".into(),
                fields: vec![field("x", ValueType::U64), field("y", ValueType::I64)],
                constraints: vec![],
            },
        ],
    }
    .validate()
    .expect("valid fixture")
}

const R: RelationId = RelationId(0);
const S: RelationId = RelationId(1);

fn var(id: u16) -> Term {
    Term::Var(VarId(id))
}

fn normalized(query: &Query) -> NormalizedQuery {
    normalize(&validate(&schema(), query).expect("valid"))
}

#[test]
fn repeated_variable_lowers_and_executes_through_the_evaluator() {
    // R(a = v, b = v): one var position, one same-fact equality filter.
    let query = Query {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: R,
            bindings: vec![(FieldId(1), var(0)), (FieldId(2), var(0))],
        }],
        negated: vec![],
        predicates: vec![],
    };
    let norm = normalized(&query);
    assert_eq!(norm.occurrences[0].vars, vec![(FieldId(1), VarId(0))]);
    assert_eq!(
        norm.occurrences[0].filters,
        vec![FilterPredicate::FieldsCompare {
            left: FieldId(1),
            right: FieldId(2),
            op: CmpOp::Eq,
        }]
    );

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
    let query = Query {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: R,
            bindings: vec![
                (FieldId(0), var(0)),
                (FieldId(1), Term::Literal(Value::I64(-7))),
                (FieldId(2), Term::Param(ParamId(0))),
            ],
        }],
        negated: vec![],
        predicates: vec![],
    };
    let norm = normalized(&query);
    assert_eq!(
        norm.occurrences[0].filters,
        vec![
            FilterPredicate::Compare {
                field: FieldId(1),
                op: CmpOp::Eq,
                value: Const::Word(u64::from_be_bytes(encode_i64(-7))),
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
    // Add a string field via S? S has none — reuse R.id as U64 and use
    // a Bytes literal on a bytes field... the fixture lacks one, so
    // check lower_literal directly (the unit under test).
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
    let query = Query {
        finds: vec![FindTerm::Var(VarId(0))],
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
        negated: vec![],
        predicates: vec![
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
    };
    let norm = normalized(&query);
    assert_eq!(
        norm.occurrences[0].filters,
        vec![FilterPredicate::Compare {
            field: FieldId(1),
            op: CmpOp::Ge, // flipped
            value: Const::Word(u64::from_be_bytes(encode_i64(100))),
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
}

#[test]
fn occurrence_vars_are_duplicate_free_over_generated_inputs() {
    // A tiny deterministic generator: every subset/multiset of var
    // bindings over R's three fields, with var ids drawn from {0,1}.
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
        let query = Query {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                relation: R,
                bindings,
            }],
            negated: vec![],
            predicates: vec![],
        };
        // Field types differ (U64 vs I64): only same-typed repeats
        // validate; skip type-conflicting combinations.
        let Ok(witness) = validate(&schema(), &query) else {
            continue;
        };
        let norm = normalize(&witness);
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
    let query = Query {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            Atom {
                relation: R,
                bindings: vec![(FieldId(0), var(0))],
            },
            Atom {
                relation: S,
                bindings: vec![],
            },
        ],
        negated: vec![],
        predicates: vec![],
    };
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
    let query = Query {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: R,
            bindings: vec![(FieldId(1), var(0)), (FieldId(2), var(1))],
        }],
        negated: vec![],
        predicates: vec![Comparison {
            op: CmpOp::Lt,
            lhs: var(0),
            rhs: var(1),
        }],
    };
    let norm = normalized(&query);
    assert!(
        norm.residuals.is_empty(),
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
