//! Grouped-judgment correctness under ordinary allocation and cancellation.
//! Error-channel choice changes error conversion, never allocation policy.

use super::{
    CandidateFacts, JudgeBudget, JudgeError, JudgeScratch, Judgment, MapState, judge_final_state,
    judge_final_state_with_scratch, store_fault,
};
use crate::schema::tests::{capacity_weighted, containment, fd, field, id_field, side};
use crate::schema::{
    Bound, FieldId, IntervalElement, RelationDescriptor, RelationId, Schema, SchemaDescriptor,
    StatementId, ValidateDescriptor as _, ValueType, Weight,
};
use crate::storage::store::StoreError;
use crate::work::{WorkContext, WorkError};
use crate::{Interval, Value};

/// A candidate state with the production error channel — spill is granted
/// only through an explicit [`JudgeScratch::channel`], not error reflection.
struct StoreErrorState(MapState);

impl CandidateFacts for StoreErrorState {
    type Error = StoreError;

    fn visit_rows(
        &self,
        relation: RelationId,
        visit: &mut dyn FnMut(&[crate::Value]) -> Result<bool, Self::Error>,
    ) -> Result<(), Self::Error> {
        let mut error = None;
        self.0
            .visit_rows(relation, &mut |row| match visit(row) {
                Ok(keep) => Ok(keep),
                Err(failure) => {
                    error = Some(failure);
                    Ok(false)
                }
            })
            .unwrap_or_else(|impossible| match impossible {});
        match error {
            Some(failure) => Err(failure),
            None => Ok(()),
        }
    }
}

/// `Note { id: u64, text: str }`, keyed on the text.
fn text_keyed_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Note".into(),
            fields: vec![
                field("id", ValueType::U64),
                field("text", ValueType::String),
            ],
        }],
        statements: vec![fd(RelationId(0), &[FieldId(1)])],
    }
    .validate()
    .expect("valid")
}

fn wide_state(rows: u64, conflict: bool) -> MapState {
    let mut state = MapState::new();
    for id in 0..rows {
        // ~200 bytes of distinct text per row: the grouped determinant
        // membership dwarfs the small working budgets below.
        let text = format!("{id:0190}-note");
        state.insert(
            RelationId(0),
            vec![Value::U64(id), Value::String(text.into())],
        );
    }
    if conflict {
        // Two extra rows fighting over one text value.
        state.insert(
            RelationId(0),
            vec![Value::U64(rows + 1), Value::String("duplicate".into())],
        );
        state.insert(
            RelationId(0),
            vec![Value::U64(rows + 2), Value::String("duplicate".into())],
        );
    }
    state
}

fn judge_production(
    schema: &Schema,
    state: &StoreErrorState,
    work: &WorkContext,
    budget: JudgeBudget,
) -> Result<Judgment, JudgeError<StoreError>> {
    judge_final_state_with_scratch(
        schema,
        state,
        work,
        budget,
        JudgeScratch::channel(store_fault),
    )
}

#[test]
fn wide_lawful_judgment_releases_its_temporary_state() {
    let schema = text_keyed_schema();
    let state = StoreErrorState(wide_state(4000, false));
    let work = WorkContext::new();
    let before = crate::alloc_counter::snapshot().absolute.live_bytes;
    assert_eq!(
        judge_production(&schema, &state, &work, JudgeBudget::default()).unwrap(),
        Judgment::Admitted
    );
    #[cfg(feature = "alloc-counter")]
    assert_eq!(crate::alloc_counter::snapshot().absolute.live_bytes, before);
    let _ = before;
}

#[test]
fn cancelled_grouped_judgment_does_not_publish_a_verdict() {
    let schema = text_keyed_schema();
    let state = StoreErrorState(wide_state(4000, false));
    let work = WorkContext::new();
    work.cancel();
    assert!(matches!(
        judge_production(&schema, &state, &work, JudgeBudget::default()),
        Err(JudgeError::Work(WorkError::Cancelled))
    ));
}

#[test]
fn wide_grouped_judgment_needs_no_storage_error_channel() {
    let schema = text_keyed_schema();
    let state = wide_state(4000, false);
    let work = WorkContext::new();
    assert_eq!(
        judge_final_state(&schema, &state, &work, JudgeBudget::default()).unwrap(),
        Judgment::Admitted
    );
}

/// Wide grouped state reports both competitors and exact truncation labels.
#[test]
fn wide_rejection_diagnostics_are_complete() {
    let schema = text_keyed_schema();
    let state = StoreErrorState(wide_state(4000, true));
    let work = WorkContext::new();
    let verdict = judge_production(&schema, &state, &work, JudgeBudget::default()).expect("judged");
    let Judgment::Rejected(violations) = verdict else {
        panic!("the duplicate text must reject");
    };
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].statement, StatementId(0));
    assert_eq!(violations[0].examples.len(), 2, "both competing rows");
    assert!(!violations[0].examples_truncated);
    for example in &violations[0].examples {
        assert_eq!(example.values[1], Value::String("duplicate".into()));
    }
}

/// A storage error adapter cannot alter any statement family's verdict.
#[test]
fn storage_error_channel_and_plain_judgments_are_identical() {
    let schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Parent".into(),
                fields: vec![id_field("id"), field("budget", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Child".into(),
                fields: vec![
                    id_field("id"),
                    field("parent", ValueType::U64),
                    field("units", ValueType::U64),
                    field(
                        "span",
                        ValueType::Interval {
                            element: IntervalElement::U64,
                        },
                    ),
                ],
            },
        ],
        statements: vec![
            fd(RelationId(0), &[FieldId(0)]),
            fd(RelationId(1), &[FieldId(0)]),
            // Pointwise key: one child interval per (parent) at a time.
            fd(RelationId(1), &[FieldId(1), FieldId(3)]),
            containment(
                side(RelationId(1), &[FieldId(1)]),
                side(RelationId(0), &[FieldId(0)]),
            ),
            capacity_weighted(
                side(RelationId(0), &[FieldId(0)]),
                Weight::Field(FieldId(2)),
                0,
                Some(Bound::TargetField(FieldId(1))),
                side(RelationId(1), &[FieldId(1)]),
            ),
        ],
    }
    .validate()
    .expect("valid");

    let mut map = MapState::new();
    map.insert(RelationId(0), vec![Value::U64(7), Value::U64(5)]);
    map.insert(RelationId(0), vec![Value::U64(8), Value::U64(100)]);
    for id in 0..200u64 {
        let parent = 7 + (id % 2);
        map.insert(
            RelationId(1),
            vec![
                Value::U64(id),
                Value::U64(parent),
                Value::U64(1),
                Value::IntervalU64(Interval::new(id * 10, id * 10 + 5).expect("span")),
            ],
        );
    }
    // One overlapping pointwise pair, one dangling child, capacity 7 blown.
    map.insert(
        RelationId(1),
        vec![
            Value::U64(900),
            Value::U64(7),
            Value::U64(1),
            Value::IntervalU64(Interval::new(3, 12).expect("span")),
        ],
    );
    map.insert(
        RelationId(1),
        vec![
            Value::U64(901),
            Value::U64(99),
            Value::U64(1),
            Value::IntervalU64(Interval::new(9000, 9010).expect("span")),
        ],
    );

    let resident = {
        let work = WorkContext::new();
        let state = StoreErrorState(clone_map(&map));
        judge_final_state(&schema, &state, &work, JudgeBudget::default()).expect("resident")
    };
    let spilled = {
        let work = WorkContext::new();
        let state = StoreErrorState(clone_map(&map));
        judge_production(&schema, &state, &work, JudgeBudget::default()).expect("spilled")
    };
    assert_eq!(resident, spilled, "one judgment, two error channels");
    let Judgment::Rejected(violations) = resident else {
        panic!("fixture violates by construction");
    };
    let ids: Vec<_> = violations.iter().map(|v| v.statement).collect();
    assert_eq!(
        ids,
        vec![StatementId(2), StatementId(3), StatementId(4)],
        "pointwise key, containment, capacity — complete and ordered"
    );
}

/// Old-judge parity: a ray-duration source row whose group NO target row
/// selects is never measured (latent), while a referencing target row
/// surfaces the explicit refusal — the sticky per-group flag reproduces
/// the reference's lazily-measured semantics exactly.
#[test]
fn unreferenced_group_failures_stay_latent_referenced_ones_refuse() {
    let schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Parent".into(),
                fields: vec![id_field("id")],
            },
            RelationDescriptor {
                extension: None,
                name: "Booking".into(),
                fields: vec![
                    id_field("id"),
                    field("parent", ValueType::U64),
                    field(
                        "span",
                        ValueType::Interval {
                            element: IntervalElement::U64,
                        },
                    ),
                ],
            },
        ],
        statements: vec![
            fd(RelationId(0), &[FieldId(0)]),
            capacity_weighted(
                side(RelationId(0), &[FieldId(0)]),
                Weight::DurationOf(FieldId(2)),
                0,
                Some(Bound::Lit(1000)),
                side(RelationId(1), &[FieldId(1)]),
            ),
        ],
    }
    .validate()
    .expect("valid");

    let ray_booking = |id: u64, parent: u64| {
        vec![
            Value::U64(id),
            Value::U64(parent),
            Value::IntervalU64(Interval::ray(5).expect("ray")),
        ]
    };

    // The ray's group (parent 99) has no parent row: latent, admitted.
    let mut latent = MapState::new();
    latent.insert(RelationId(0), vec![Value::U64(7)]);
    latent.insert(RelationId(1), ray_booking(1, 99));
    let work = WorkContext::new();
    assert_eq!(
        judge_final_state(
            &schema,
            &StoreErrorState(latent),
            &work,
            JudgeBudget::default()
        )
        .expect("judged"),
        Judgment::Admitted
    );

    // The same ray under a selected parent refuses explicitly.
    let mut referenced = MapState::new();
    referenced.insert(RelationId(0), vec![Value::U64(7)]);
    referenced.insert(RelationId(1), ray_booking(1, 7));
    let work = WorkContext::new();
    assert!(matches!(
        judge_final_state(
            &schema,
            &StoreErrorState(referenced),
            &work,
            JudgeBudget::default()
        ),
        Err(JudgeError::UndefinedDuration {
            statement: StatementId(1)
        })
    ));
}

/// Pointwise containment coverage merges ADJACENT target spans — [1,2) and
/// [2,3) together cover [1,3) — exactly the reference frontier walk,
/// through the exact run-table probe.
#[test]
fn adjacent_target_spans_cover_through_the_run_table() {
    let schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Shift".into(),
                fields: vec![
                    field("room", ValueType::U64),
                    field(
                        "span",
                        ValueType::Interval {
                            element: IntervalElement::U64,
                        },
                    ),
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Open".into(),
                fields: vec![
                    field("room", ValueType::U64),
                    field(
                        "span",
                        ValueType::Interval {
                            element: IntervalElement::U64,
                        },
                    ),
                ],
            },
        ],
        statements: vec![
            fd(RelationId(1), &[FieldId(0), FieldId(1)]),
            containment(
                side(RelationId(0), &[FieldId(0), FieldId(1)]),
                side(RelationId(1), &[FieldId(0), FieldId(1)]),
            ),
        ],
    }
    .validate()
    .expect("valid");

    let span = |a: u64, b: u64| Value::IntervalU64(Interval::new(a, b).expect("span"));
    for _ in 0..2 {
        let mut covered = MapState::new();
        covered.insert(RelationId(1), vec![Value::U64(1), span(1, 2)]);
        covered.insert(RelationId(1), vec![Value::U64(1), span(2, 3)]);
        covered.insert(RelationId(0), vec![Value::U64(1), span(1, 3)]);
        let work = WorkContext::new();
        assert_eq!(
            judge_final_state(
                &schema,
                &StoreErrorState(covered),
                &work,
                JudgeBudget::default()
            )
            .expect("judged"),
            Judgment::Admitted,
            "adjacent spans connect"
        );

        let mut gapped = MapState::new();
        gapped.insert(RelationId(1), vec![Value::U64(1), span(1, 2)]);
        gapped.insert(RelationId(1), vec![Value::U64(1), span(3, 4)]);
        gapped.insert(RelationId(0), vec![Value::U64(1), span(1, 4)]);
        let work = WorkContext::new();
        let Judgment::Rejected(violations) = judge_final_state(
            &schema,
            &StoreErrorState(gapped),
            &work,
            JudgeBudget::default(),
        )
        .expect("judged") else {
            panic!("a real gap refuses coverage");
        };
        assert_eq!(violations[0].examples.len(), 1, "the uncovered source row");
    }
}

/// Determinism: one fixture judged twice yields byte-identical verdicts —
/// citation order is the state's own deterministic iteration order.
#[test]
fn judgments_are_deterministic() {
    let schema = text_keyed_schema();
    let judge_once = || {
        let state = StoreErrorState(wide_state(600, true));
        let work = WorkContext::new();
        judge_production(&schema, &state, &work, JudgeBudget::default()).expect("judged")
    };
    assert_eq!(judge_once(), judge_once());
}

fn clone_map(map: &MapState) -> MapState {
    use super::CandidateFacts;
    let mut cloned = MapState::new();
    for relation in 0..8u32 {
        map.visit_rows(RelationId(relation), &mut |values| {
            cloned.insert(RelationId(relation), values.to_vec());
            Ok(true)
        })
        .unwrap();
    }
    cloned
}
