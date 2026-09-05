use super::*;
use crate::image::{ColumnSpan, ColumnWidth};
use crate::ir::normalize::normalize_rules;
use crate::ir::validate::validate as validate_ir;
use crate::ir::{Atom, CmpOp, Comparison, ConditionTree, FindTerm, Query, Rule, Term};
use crate::plan::planner::{OccStats, plan};
use bumbledb_theory::schema::IntervalElement;
use std::collections::BTreeSet;

fn idiom_schema() -> Schema {
    let field = |name: &str, ty: ValueType| FieldDescriptor {
        name: name.into(),
        value_type: ty,
    };
    let fresh = |name: &str| FieldDescriptor {
        name: name.into(),
        value_type: ValueType::U64,
    };
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "A".into(),
                fields: vec![fresh("id"), field("v", ValueType::I64)],
            },
            RelationDescriptor {
                extension: None,
                name: "B".into(),
                fields: vec![
                    fresh("id"),
                    field("a", ValueType::U64),
                    field("at", ValueType::I64),
                ],
            },
        ],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

fn interval_schema() -> Schema {
    let interval = ValueType::Interval {
        element: IntervalElement::I64,
    };
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "P".into(),
            fields: vec![
                FieldDescriptor {
                    name: "emp".into(),
                    value_type: ValueType::U64,
                },
                FieldDescriptor {
                    name: "during".into(),
                    value_type: interval,
                },
                FieldDescriptor {
                    name: "review".into(),
                    value_type: interval,
                },
            ],
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

fn stats(rows_and_distincts: &[(u64, &[(u16, u64)])]) -> Vec<OccStats> {
    rows_and_distincts
        .iter()
        .enumerate()
        .map(|(i, (rows, distincts))| OccStats {
            occ_id: OccId(u16::try_from(i).expect("small")),
            rows: *rows,
            var_distincts: distincts.iter().map(|(v, d)| (VarId(*v), *d)).collect(),
        })
        .collect()
}

fn witness(schema: &Schema, query: &Query, occ_stats: &[OccStats]) -> ValidatedPlan {
    let validated = validate_ir(schema, query).expect("valid query");
    let normalized = normalize_rules(schema, &[], validated.rules()).remove(0);
    let join_order = plan(&normalized, schema, occ_stats);
    let mut fj_plan = binary2fj(&normalized, &join_order);
    factor(&mut fj_plan);
    let mut sink_vars: BTreeSet<VarId> = BTreeSet::new();
    for f in &query.rules()[0].finds {
        match f {
            FindTerm::Var(v) => {
                sink_vars.insert(*v);
            }
            // The sink reads a computed find through its input variables
            // (C05: the adapter evaluates per surviving binding).
            FindTerm::Compute(expr) => sink_vars.extend(expr.variables()),
            FindTerm::Count | FindTerm::Pack { .. } | FindTerm::Aggregate { .. } => {}
        }
    }
    validate(&fj_plan, &normalized, schema, &sink_vars).expect("valid plan")
}

#[test]
fn outer_join_idiom_join_half_validates_into_the_witness() {
    let schema = idiom_schema();
    let x = VarId(0);
    let y = VarId(1);
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(x), FindTerm::Var(y)],
        atoms: vec![
            Atom {
                source: crate::ir::AtomSource::Edb(RelationId(0)),
                bindings: vec![(FieldId(0), Term::Var(x))],
            },
            Atom {
                source: crate::ir::AtomSource::Edb(RelationId(1)),
                bindings: vec![(FieldId(1), Term::Var(x)), (FieldId(2), Term::Var(y))],
            },
        ],
        negated: vec![],
        conditions: vec![],
    });

    let witness = witness(
        &schema,
        &query,
        &stats(&[(100, &[(0, 100)]), (1000, &[(0, 100), (1, 800)])]),
    );

    assert_eq!(
        witness.nodes()[0].subatoms,
        vec![subatom(0, &[x]), subatom(1, &[x])]
    );
    assert_eq!(witness.nodes()[1].subatoms, vec![subatom(1, &[y])]);
    assert!(witness.nodes().iter().all(|n| n.anti_probes.is_empty()
        && n.residuals.is_empty()
        && n.word_residuals.is_empty()));
    assert_eq!(witness.occurrence(OccId(0)).trie_schema, vec![vec![x]]);
    assert_eq!(
        witness.occurrence(OccId(1)).trie_schema,
        vec![vec![x], vec![y]]
    );
    assert_eq!(witness.slots(), &[(x, SlotWidth::ONE), (y, SlotWidth::ONE)]);

    assert!(witness.distinct_witness().is_none());
}

#[test]
fn outer_join_idiom_absence_half_validates_into_the_witness() {
    let schema = idiom_schema();
    let x = VarId(0);
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(x)],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(RelationId(0)),
            bindings: vec![(FieldId(0), Term::Var(x))],
        }],
        negated: vec![Atom {
            source: crate::ir::AtomSource::Edb(RelationId(1)),
            bindings: vec![(FieldId(1), Term::Var(x))],
        }],
        conditions: vec![],
    });
    let witness = witness(&schema, &query, &stats(&[(100, &[(0, 100)])]));

    assert_eq!(witness.nodes().len(), 1);
    assert_eq!(witness.nodes()[0].subatoms, vec![subatom(0, &[x])]);
    assert_eq!(witness.nodes()[0].anti_probes.len(), 1);
    let probe = &witness.nodes()[0].anti_probes[0];
    assert_eq!(probe.occurrence, OccId(1));
    assert_eq!(probe.probe_bindings, vec![(FieldId(1), x)]);

    assert_eq!(witness.occurrence(OccId(1)).trie_schema, vec![vec![x]]);
    assert_eq!(witness.occurrence(OccId(1)).key_widths, vec![1]);

    assert!(witness.distinct_witness().is_some());
}

#[test]
fn allen_residual_query_validates_into_the_witness() {
    let schema = interval_schema();
    let e1 = VarId(0);
    let d1 = VarId(1);
    let e2 = VarId(2);
    let d2 = VarId(3);
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(e1), FindTerm::Var(e2)],
        atoms: vec![
            Atom {
                source: crate::ir::AtomSource::Edb(RelationId(0)),
                bindings: vec![(FieldId(0), Term::Var(e1)), (FieldId(1), Term::Var(d1))],
            },
            Atom {
                source: crate::ir::AtomSource::Edb(RelationId(0)),
                bindings: vec![(FieldId(0), Term::Var(e2)), (FieldId(1), Term::Var(d2))],
            },
        ],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Allen {
                mask: bumbledb_theory::allen::AllenMask::INTERSECTS,
            },
            lhs: Term::Var(d1),
            rhs: Term::Var(d2),
        })],
    });

    let witness = witness(
        &schema,
        &query,
        &stats(&[(5, &[(0, 5), (1, 5)]), (10, &[(2, 10), (3, 10)])]),
    );

    assert_eq!(
        witness.nodes()[0].subatoms,
        vec![subatom(0, &[e1, d1]), subatom(1, &[])]
    );
    assert_eq!(witness.nodes()[1].subatoms, vec![subatom(1, &[e2, d2])]);

    assert!(witness.nodes().iter().all(|n| n.residuals.is_empty()));
    assert!(witness.nodes().iter().all(|n| n.word_residuals.is_empty()));
    assert!(witness.nodes()[0].allen_residuals.is_empty());
    assert_eq!(
        witness.nodes()[1].allen_residuals,
        vec![FilterPredicate::FieldsAllen {
            left: OperandAddr::from(d1),
            right: OperandAddr::from(d2),
            mask: bumbledb_theory::allen::AllenMask::INTERSECTS,
        }]
    );

    assert_eq!(
        witness.slots(),
        &[
            (e1, SlotWidth::ONE),
            (d1, SlotWidth::TWO),
            (e2, SlotWidth::ONE),
            (d2, SlotWidth::TWO),
        ]
    );
    assert_eq!(witness.slot_of(d1), 1);
    assert_eq!(witness.slot_of(e2), 3);
    assert_eq!(witness.slot_of(d2), 4);
    assert_eq!(witness.slot_count(), 6);

    assert_eq!(witness.occurrence(OccId(0)).trie_schema, vec![vec![e1, d1]]);
    assert_eq!(witness.occurrence(OccId(0)).key_widths, vec![3]);
    assert_eq!(witness.occurrence(OccId(1)).key_widths, vec![0, 3]);

    assert_eq!(
        witness.occurrence(OccId(0)).spans.as_ref(),
        &[
            ColumnSpan {
                first_column: 0,
                width: ColumnWidth::Word
            },
            ColumnSpan {
                first_column: 1,
                width: ColumnWidth::WordPair
            },
            ColumnSpan {
                first_column: 3,
                width: ColumnWidth::WordPair
            },
        ]
    );
}

#[test]
fn interval_value_equality_joins_with_a_two_word_key() {
    let schema = interval_schema();
    let e1 = VarId(0);
    let d = VarId(1);
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(e1)],
        atoms: vec![
            Atom {
                source: crate::ir::AtomSource::Edb(RelationId(0)),
                bindings: vec![(FieldId(0), Term::Var(e1)), (FieldId(1), Term::Var(d))],
            },
            Atom {
                source: crate::ir::AtomSource::Edb(RelationId(0)),
                bindings: vec![(FieldId(2), Term::Var(d))],
            },
        ],
        negated: vec![],
        conditions: vec![],
    });
    let witness = witness(
        &schema,
        &query,
        &stats(&[(10, &[(0, 10), (1, 10)]), (1000, &[(1, 1000)])]),
    );

    assert_eq!(
        witness.occurrence(OccId(1)).trie_schema,
        vec![vec![d], vec![]]
    );
    assert_eq!(witness.occurrence(OccId(1)).key_widths, vec![2, 0]);
    assert_eq!(
        witness.slots(),
        &[(e1, SlotWidth::ONE), (d, SlotWidth::TWO)]
    );
}
