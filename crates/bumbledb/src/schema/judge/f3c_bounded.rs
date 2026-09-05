//! F3 finding C regressions: the judge streams relations and keeps grouped
//! state in the charged RAM→scratch tiers — beyond-budget judgment is real
//! (F-RESOURCE / Q-BUDGET / Q-DISK unit half; the public-to-native call
//! path is exercised in `crates/bumbledb/tests/gate-bounded-admission.rs`).
//!
//! The `StoreErrorState` here is a state whose error channel is the
//! store's own — exactly what the production candidate view presents — so
//! these tests prove the automatic spill channel without any bridge code.

use std::time::Duration;

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
use crate::work::{ExecutionPolicy, Resource, WorkError};
use crate::{Interval, Value, WorkContext};

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

fn policy(working: u64, scratch: u64) -> WorkContext {
    ExecutionPolicy {
        input_bytes: 1 << 30,
        working_bytes: working,
        scratch_bytes: scratch,
        result_bytes: 0,
        rows: 1 << 30,
        work_units: 1 << 40,
        timeout: Duration::from_secs(120),
    }
    .start()
    .expect("policy starts")
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

/// The production-channel state admits a lawful wide state under a working
/// budget FAR smaller than the grouped determinant set — the disk tier
/// carries it (Q-DISK), and the working allowance is never exceeded (the
/// charge would have refused).
#[test]
fn beyond_working_budget_admission_spills_and_admits() {
    let schema = text_keyed_schema();
    let state = StoreErrorState(wide_state(4000, false));
    let work = policy(256 << 10, 64 << 20);
    let verdict =
        judge_production(&schema, &state, &work, JudgeBudget::default()).expect("judged");
    assert_eq!(verdict, Judgment::Admitted);
    assert!(
        work.used(Resource::WorkingBytes) <= 256 << 10,
        "working stayed within the small budget"
    );
}

/// The SAME judgment with a zero scratch allowance refuses with the typed
/// scratch exhaustion: the success above went through the charged disk
/// tier, not through unaccounted memory.
#[test]
fn the_spill_is_charged_scratch_a_zero_allowance_refuses() {
    let schema = text_keyed_schema();
    let state = StoreErrorState(wide_state(4000, false));
    let work = policy(256 << 10, 0);
    let error = judge_production(&schema, &state, &work, JudgeBudget::default())
        .expect_err("no scratch allowance");
    assert!(
        matches!(
            error,
            JudgeError::Work(WorkError::Exhausted {
                resource: Resource::ScratchBytes,
                ..
            })
        ),
        "typed scratch refusal, got {error:?}"
    );
}

/// A state WITHOUT a spill channel keeps grouped state in charged RAM: the
/// same wide judgment refuses with the typed WORKING exhaustion instead of
/// growing unaccounted (finding C's accounting rule) — and never touches
/// disk the caller did not permit.
#[test]
fn without_a_channel_grouped_state_stays_charged_ram_and_refuses() {
    let schema = text_keyed_schema();
    let state = wide_state(4000, false);
    let work = policy(256 << 10, 64 << 20);
    let error = judge_final_state(&schema, &state, &work, JudgeBudget::default())
        .expect_err("RAM-only grouped state exceeds the working budget");
    assert!(
        matches!(
            error,
            JudgeError::Work(WorkError::Exhausted {
                resource: Resource::WorkingBytes,
                ..
            })
        ),
        "typed working refusal, got {error:?}"
    );
    assert_eq!(
        work.used(Resource::ScratchBytes),
        0,
        "no disk tier without a channel"
    );
}

/// Complete rejection diagnostics under pressure: the conflict inside a
/// beyond-budget relation is judged on disk, and the verdict still names
/// the statement with BOTH competing rows cited and truncation labeled
/// exactly.
#[test]
fn rejection_diagnostics_are_complete_through_the_disk_tier() {
    let schema = text_keyed_schema();
    let state = StoreErrorState(wide_state(4000, true));
    let work = policy(256 << 10, 64 << 20);
    let verdict =
        judge_production(&schema, &state, &work, JudgeBudget::default()).expect("judged");
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

/// The disk tier and the charged RAM tier are the SAME judgment: a fixture
/// exercising every statement family judges byte-identically under an
/// ample budget (RAM) and under a tiny working budget (spilled).
#[test]
fn spilled_and_resident_judgments_are_identical() {
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
        let work = policy(64 << 20, 0);
        let state = StoreErrorState(clone_map(&map));
        judge_final_state(&schema, &state, &work, JudgeBudget::default()).expect("resident")
    };
    let spilled = {
        let work = policy(48 << 10, 64 << 20);
        let state = StoreErrorState(clone_map(&map));
        judge_production(&schema, &state, &work, JudgeBudget::default()).expect("spilled")
    };
    assert_eq!(resident, spilled, "one judgment, two tiers");
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
    let work = policy(64 << 20, 0);
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
    let work = policy(64 << 20, 0);
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
/// through the run-table probe in both tiers.
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
    for (working, scratch) in [(64 << 20, 0u64), (16 << 10, 64 << 20)] {
        let mut covered = MapState::new();
        covered.insert(RelationId(1), vec![Value::U64(1), span(1, 2)]);
        covered.insert(RelationId(1), vec![Value::U64(1), span(2, 3)]);
        covered.insert(RelationId(0), vec![Value::U64(1), span(1, 3)]);
        let work = policy(working, scratch);
        assert_eq!(
            judge_final_state(
                &schema,
                &StoreErrorState(covered),
                &work,
                JudgeBudget::default()
            )
            .expect("judged"),
            Judgment::Admitted,
            "adjacent spans connect (working budget {working})"
        );

        let mut gapped = MapState::new();
        gapped.insert(RelationId(1), vec![Value::U64(1), span(1, 2)]);
        gapped.insert(RelationId(1), vec![Value::U64(1), span(3, 4)]);
        gapped.insert(RelationId(0), vec![Value::U64(1), span(1, 4)]);
        let work = policy(working, scratch);
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
        let work = policy(64 << 10, 64 << 20);
        judge_production(&schema, &state, &work, JudgeBudget::default()).expect("judged")
    };
    assert_eq!(judge_once(), judge_once());
}

fn clone_map(map: &MapState) -> MapState {
    let mut cloned = MapState::new();
    for relation in 0..8u32 {
        for row in map.rows(RelationId(relation)) {
            let Ok(values) = row;
            cloned.insert(RelationId(relation), values.into_vec());
        }
    }
    cloned
}
