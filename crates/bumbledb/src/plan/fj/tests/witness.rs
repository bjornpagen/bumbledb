//! Witness-construction goldens (PRD 15): real IR through validation,
//! normalization, the DP planner, `binary2fj` + `factor`, and plan
//! validation — asserting the exact node/residual/anti-probe shapes.

use super::*;
use crate::image::{ColumnSpan, ColumnWidth};
use crate::ir::normalize::{normalize, IntervalWord, VarWord};
use crate::ir::validate::validate as validate_ir;
use crate::ir::{Atom, CmpOp, Comparison, FindTerm, Query, Term};
use crate::plan::planner::{plan, OccStats};
use crate::schema::IntervalElement;
use std::collections::BTreeSet;

/// A(id u64 serial, v i64); B(id u64 serial, a u64, at i64) — the
/// outer-join-idiom fixture (`docs/architecture/20-query-ir.md`).
fn idiom_schema() -> Schema {
    let field = |name: &str, ty: ValueType| FieldDescriptor {
        name: name.into(),
        value_type: ty,
        generation: Generation::None,
    };
    let serial = |name: &str| FieldDescriptor {
        name: name.into(),
        value_type: ValueType::U64,
        generation: Generation::Serial,
    };
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "A".into(),
                fields: vec![serial("id"), field("v", ValueType::I64)],
            },
            RelationDescriptor {
                name: "B".into(),
                fields: vec![
                    serial("id"),
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

/// P(emp u64, during interval<i64>, review interval<i64>) — the interval
/// fixture.
fn interval_schema() -> Schema {
    let interval = ValueType::Interval {
        element: IntervalElement::I64,
    };
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "P".into(),
            fields: vec![
                FieldDescriptor {
                    name: "emp".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "during".into(),
                    value_type: interval.clone(),
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "review".into(),
                    value_type: interval,
                    generation: Generation::None,
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

/// The full pipeline: validate the IR, normalize, plan over the given
/// stats, lower, factor, validate into the witness.
fn witness(schema: &Schema, query: &Query, occ_stats: &[OccStats]) -> ValidatedPlan {
    let validated = validate_ir(schema, query).expect("valid query");
    let normalized = normalize(schema, &validated);
    let join_order = plan(&normalized, schema, occ_stats);
    let mut fj_plan = binary2fj(&normalized, &join_order);
    factor(&mut fj_plan);
    let sink_vars: BTreeSet<VarId> = query
        .finds
        .iter()
        .filter_map(|f| match f {
            FindTerm::Var(v) => Some(*v),
            FindTerm::Aggregate { .. } => None,
        })
        .collect();
    validate(
        &fj_plan,
        &normalized,
        schema,
        join_order.estimates.clone(),
        &sink_vars,
    )
    .expect("valid plan")
}

/// The outer-join idiom's join half: `A ⋈ B` — golden node shapes, no
/// anti-probes anywhere.
#[test]
fn outer_join_idiom_join_half_validates_into_the_witness() {
    let schema = idiom_schema();
    let x = VarId(0);
    let y = VarId(1);
    let query = Query {
        finds: vec![FindTerm::Var(x), FindTerm::Var(y)],
        atoms: vec![
            Atom {
                relation: RelationId(0),
                bindings: vec![(FieldId(0), Term::Var(x))],
            },
            Atom {
                relation: RelationId(1),
                bindings: vec![(FieldId(1), Term::Var(x)), (FieldId(2), Term::Var(y))],
            },
        ],
        negated: vec![],
        predicates: vec![],
    };
    // 100 A-rows into 1000 B-rows on a non-key field of B: the walk
    // iterates A (its serial key makes the reverse direction fanout 1,
    // but iterating the small side first still wins on cost).
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
    assert_eq!(witness.slots(), &[(x, SlotWidth::One), (y, SlotWidth::One)]);
    // B's bound fields (a, at) cover no key of B — no elision proof.
    assert!(!witness.distinct_bindings());
}

/// The outer-join idiom's absence half: `A` with a negated `B` atom —
/// the anti-probe attaches to the root with B's trie schema in probe
/// order.
#[test]
fn outer_join_idiom_absence_half_validates_into_the_witness() {
    let schema = idiom_schema();
    let x = VarId(0);
    let query = Query {
        finds: vec![FindTerm::Var(x)],
        atoms: vec![Atom {
            relation: RelationId(0),
            bindings: vec![(FieldId(0), Term::Var(x))],
        }],
        negated: vec![Atom {
            relation: RelationId(1),
            bindings: vec![(FieldId(1), Term::Var(x))],
        }],
        predicates: vec![],
    };
    let witness = witness(&schema, &query, &stats(&[(100, &[(0, 100)])]));

    // One node — the negated occurrence joined nothing.
    assert_eq!(witness.nodes().len(), 1);
    assert_eq!(witness.nodes()[0].subatoms, vec![subatom(0, &[x])]);
    assert_eq!(witness.nodes()[0].anti_probes.len(), 1);
    let probe = &witness.nodes()[0].anti_probes[0];
    assert_eq!(probe.occurrence, OccId(1));
    assert_eq!(probe.probe_bindings, vec![(FieldId(1), x)]);
    // The negated occurrence's probe-order trie schema and key width.
    assert_eq!(witness.occurrence(OccId(1)).trie_schema, vec![vec![x]]);
    assert_eq!(witness.occurrence(OccId(1)).key_widths, vec![1]);
    // A alone binds its serial key: the elision proof holds (the
    // negated occurrence binds nothing and cannot break it).
    assert!(witness.distinct_bindings());
}

/// An `Overlaps` residual query: two P occurrences with no shared
/// variable, `Overlaps(d1, d2)` decomposed into two word comparisons
/// attached to the node binding the second interval — plus the two-slot
/// interval layout and `ColumnSpan` field maps.
#[test]
fn overlaps_residual_query_validates_into_the_witness() {
    let schema = interval_schema();
    let e1 = VarId(0);
    let d1 = VarId(1);
    let e2 = VarId(2);
    let d2 = VarId(3);
    let query = Query {
        finds: vec![FindTerm::Var(e1), FindTerm::Var(e2)],
        atoms: vec![
            Atom {
                relation: RelationId(0),
                bindings: vec![(FieldId(0), Term::Var(e1)), (FieldId(1), Term::Var(d1))],
            },
            Atom {
                relation: RelationId(0),
                bindings: vec![(FieldId(0), Term::Var(e2)), (FieldId(1), Term::Var(d2))],
            },
        ],
        negated: vec![],
        predicates: vec![Comparison {
            op: CmpOp::Overlaps,
            lhs: Term::Var(d1),
            rhs: Term::Var(d2),
        }],
    };
    // Asymmetric rows force the order: the 5-row side iterates first
    // (a disconnected pair is a cross product either way; cost counts
    // the root iteration).
    let witness = witness(
        &schema,
        &query,
        &stats(&[(5, &[(0, 5), (1, 5)]), (10, &[(2, 10), (3, 10)])]),
    );

    // Disconnected pair: node 0 iterates occ 0 (occ 1 rides along as an
    // empty probe), node 1 opens occ 1.
    assert_eq!(
        witness.nodes()[0].subatoms,
        vec![subatom(0, &[e1, d1]), subatom(1, &[])]
    );
    assert_eq!(witness.nodes()[1].subatoms, vec![subatom(1, &[e2, d2])]);

    // The decomposed Overlaps lands on the node binding d2 — as word
    // comparisons over the interval slot pairs, never a whole-value
    // residual.
    assert!(witness.nodes().iter().all(|n| n.residuals.is_empty()));
    assert!(witness.nodes()[0].word_residuals.is_empty());
    assert_eq!(
        witness.nodes()[1].word_residuals,
        vec![
            PlacedWordComparison {
                op: CmpOp::Lt,
                lhs: VarWord {
                    var: d1,
                    word: IntervalWord::Start
                },
                rhs: VarWord {
                    var: d2,
                    word: IntervalWord::End
                },
            },
            PlacedWordComparison {
                op: CmpOp::Lt,
                lhs: VarWord {
                    var: d2,
                    word: IntervalWord::Start
                },
                rhs: VarWord {
                    var: d1,
                    word: IntervalWord::End
                },
            },
        ]
    );

    // The two-slot interval layout: d1 and d2 hold two slots each.
    assert_eq!(
        witness.slots(),
        &[
            (e1, SlotWidth::One),
            (d1, SlotWidth::Two),
            (e2, SlotWidth::One),
            (d2, SlotWidth::Two),
        ]
    );
    assert_eq!(witness.slot_of(d1), 1);
    assert_eq!(witness.slot_of(e2), 3);
    assert_eq!(witness.slot_of(d2), 4);
    assert_eq!(witness.slot_count(), 6);

    // Trie levels count interval variables at two words.
    assert_eq!(witness.occurrence(OccId(0)).trie_schema, vec![vec![e1, d1]]);
    assert_eq!(witness.occurrence(OccId(0)).key_widths, vec![3]);
    assert_eq!(witness.occurrence(OccId(1)).key_widths, vec![0, 3]);

    // The ColumnSpan field map: emp one word column, during/review two
    // word columns each.
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

/// An interval-typed variable joining two atoms by value is one plan
/// variable with a two-word probe key (docs/architecture/40-execution.md).
#[test]
fn interval_value_equality_joins_with_a_two_word_key() {
    let schema = interval_schema();
    let e1 = VarId(0);
    let d = VarId(1);
    let query = Query {
        finds: vec![FindTerm::Var(e1)],
        atoms: vec![
            Atom {
                relation: RelationId(0),
                bindings: vec![(FieldId(0), Term::Var(e1)), (FieldId(1), Term::Var(d))],
            },
            Atom {
                relation: RelationId(0),
                bindings: vec![(FieldId(2), Term::Var(d))],
            },
        ],
        negated: vec![],
        predicates: vec![],
    };
    let witness = witness(
        &schema,
        &query,
        &stats(&[(10, &[(0, 10), (1, 10)]), (1000, &[(1, 1000)])]),
    );

    // Occ 1 is probed on d alone: one level, two words.
    assert_eq!(
        witness.occurrence(OccId(1)).trie_schema,
        vec![vec![d], vec![]]
    );
    assert_eq!(witness.occurrence(OccId(1)).key_widths, vec![2, 0]);
    assert_eq!(
        witness.slots(),
        &[(e1, SlotWidth::One), (d, SlotWidth::Two)]
    );
}
