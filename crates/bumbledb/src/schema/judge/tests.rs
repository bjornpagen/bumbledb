//! E-ADMIT: the reference judge sees the whole proposed multimap before
//! any unique index installs, reports the COMPLETE violated-statement set
//! with bounded labeled examples, and returns a resource error rather than
//! a falsely complete rejection. These are authored acceptance tests
//! (executed in F3), mapped to ENG-005/ENG-007 and the E-* / F-* gates
//! named per test.
use super::{JudgeBudget, JudgeError, JudgedViolation, Judgment, MapState, judge_final_state};
use crate::schema::tests::{capacity_weighted, containment, fd, field, id_field, side};
use crate::schema::{
    Bound, FieldId, IntervalElement, RelationDescriptor, RelationId, Schema, SchemaDescriptor,
    StatementId, StatementKind, ValidateDescriptor as _, ValueType, Weight,
};
use crate::work::ExecutionPolicy;
use crate::{F64, Id128, Interval, Value, WorkContext};
use std::time::Duration;

fn work() -> WorkContext {
    ExecutionPolicy {
        input_bytes: 1_000_000,
        working_bytes: 1_000_000,
        scratch_bytes: 0,
        result_bytes: 0,
        rows: 100_000,
        work_units: 1_000_000,
        timeout: Duration::from_secs(60),
    }
    .start()
    .unwrap()
}

fn judge(schema: &Schema, state: &MapState) -> Judgment {
    judge_final_state(schema, state, &work(), JudgeBudget::default()).expect("judgment completes")
}

fn rejected(schema: &Schema, state: &MapState) -> Vec<JudgedViolation> {
    match judge(schema, state) {
        Judgment::Rejected(violations) => violations.into_vec(),
        Judgment::Admitted => panic!("expected a rejection"),
    }
}

fn dense(start: f64, end: f64) -> Interval<F64> {
    Interval::<F64>::new(F64::from(start), F64::from(end)).expect("checked fixture")
}

/// `User { id: id128, email: str }` with two keys: id and email.
fn user_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "User".into(),
            fields: vec![
                field("id", ValueType::Id128),
                field("email", ValueType::String),
            ],
        }],
        statements: vec![
            fd(RelationId(0), &[FieldId(0)]),
            fd(RelationId(0), &[FieldId(1)]),
        ],
    }
    .validate()
    .expect("valid")
}

fn user(id: u8, email: &str) -> Vec<Value> {
    vec![
        Value::Id128(Id128::from_bytes([id; 16])),
        Value::String(email.into()),
    ]
}

/// The original E-ADMIT counterexample, without the old fresh mechanism:
/// two NEW rows with distinct application ids share one email. A physical
/// unique index would install one and blame the other; the judged
/// multimap reports the email-key statement with BOTH competing rows,
/// under every insertion order.
#[test]
fn two_fresh_rows_sharing_an_email_report_the_key_with_both_rows() {
    let schema = user_schema();
    let permutations: [[Vec<Value>; 2]; 2] = [
        [user(1, "a@example"), user(2, "a@example")],
        [user(2, "a@example"), user(1, "a@example")],
    ];
    for rows in permutations {
        let mut state = MapState::new();
        for row in rows {
            state.insert(RelationId(0), row);
        }
        let violations = rejected(&schema, &state);
        assert_eq!(violations.len(), 1, "exactly the email key is violated");
        let violation = &violations[0];
        assert_eq!(violation.statement, StatementId(1));
        assert_eq!(violation.kind, StatementKind::Functionality);
        assert!(!violation.examples_truncated);
        let mut cited: Vec<_> = violation
            .examples
            .iter()
            .map(|fact| fact.values.clone())
            .collect();
        cited.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
        assert_eq!(cited.len(), 2, "both competing rows are evidence");
    }
}

/// Complete statement diagnostics: when one candidate violates several
/// statements, ALL of their ids come back in canonical order — never the
/// first landing failure.
#[test]
fn every_violated_statement_is_reported_in_canonical_order() {
    let schema = user_schema();
    let mut state = MapState::new();
    // Same id AND same email across two distinct rows: both keys violated.
    state.insert(RelationId(0), user(1, "a@example"));
    state.insert(
        RelationId(0),
        vec![
            Value::Id128(Id128::from_bytes([1; 16])),
            Value::String("a@example".into()),
        ],
    );
    // The two rows above are identical, so they collapse to one: no
    // violation at all — repeated facts are a no-op (set semantics).
    assert_eq!(judge(&schema, &state), Judgment::Admitted);

    let mut state = MapState::new();
    state.insert(RelationId(0), user(1, "a@example"));
    // Distinct full row, same id, same email: both key statements fire.
    state.insert(
        RelationId(0),
        vec![
            Value::Id128(Id128::from_bytes([1; 16])),
            Value::String("a@example ".into()),
        ],
    );
    state.insert(RelationId(0), user(2, "a@example"));
    let violations = rejected(&schema, &state);
    let ids: Vec<_> = violations
        .iter()
        .map(|violation| violation.statement)
        .collect();
    assert_eq!(
        ids,
        vec![StatementId(0), StatementId(1)],
        "complete and ordered"
    );
}

/// The example budget is explicit: a wide conflict truncates its citations
/// and says so, rather than promising every conflicting pair.
#[test]
fn example_truncation_is_labeled_never_silent() {
    let schema = user_schema();
    let mut state = MapState::new();
    for id in 0..10u8 {
        state.insert(RelationId(0), user(id, "shared@example"));
    }
    let violations = rejected(&schema, &state);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].examples.len(), 4, "the default budget");
    assert!(violations[0].examples_truncated, "truncation is labeled");

    // A zero budget keeps the verdict complete with no cited facts.
    let judgment = judge_final_state(
        &schema,
        &state,
        &work(),
        JudgeBudget {
            examples_per_statement: 0,
        },
    )
    .expect("completes");
    let Judgment::Rejected(violations) = judgment else {
        panic!("still rejected");
    };
    assert!(violations[0].examples.is_empty());
    assert!(violations[0].examples_truncated);
}

/// An expired allowance is a resource error, never a shorter rejection.
#[test]
fn exhausted_work_refuses_instead_of_returning_a_partial_verdict() {
    let schema = user_schema();
    let mut state = MapState::new();
    for id in 0..50u8 {
        state.insert(RelationId(0), user(id, "shared@example"));
    }
    let tiny = ExecutionPolicy {
        input_bytes: 0,
        working_bytes: 0,
        scratch_bytes: 0,
        result_bytes: 0,
        rows: 0,
        work_units: 10,
        timeout: Duration::from_secs(60),
    }
    .start()
    .unwrap();
    assert!(matches!(
        judge_final_state(&schema, &state, &tiny, JudgeBudget::default()),
        Err(JudgeError::Work(_))
    ));
}

/// Containment: a child whose parent is absent from the proposed final
/// state is cited; adding the parent in the same candidate admits — the
/// final state is judged, not the statement order.
#[test]
fn containment_judges_the_final_state_not_the_landing_order() {
    let schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Parent".into(),
                fields: vec![id_field("id")],
            },
            RelationDescriptor {
                extension: None,
                name: "Child".into(),
                fields: vec![id_field("id"), field("parent", ValueType::U64)],
            },
        ],
        statements: vec![
            fd(RelationId(0), &[FieldId(0)]),
            fd(RelationId(1), &[FieldId(0)]),
            containment(
                side(RelationId(1), &[FieldId(1)]),
                side(RelationId(0), &[FieldId(0)]),
            ),
        ],
    }
    .validate()
    .expect("valid");

    let mut dangling = MapState::new();
    dangling.insert(RelationId(1), vec![Value::U64(1), Value::U64(7)]);
    let violations = rejected(&schema, &dangling);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].statement, StatementId(2));
    assert_eq!(violations[0].kind, StatementKind::Containment);
    assert_eq!(violations[0].examples[0].relation, RelationId(1));

    let mut repaired = MapState::new();
    repaired.insert(RelationId(1), vec![Value::U64(1), Value::U64(7)]);
    repaired.insert(RelationId(0), vec![Value::U64(7)]);
    assert_eq!(judge(&schema, &repaired), Judgment::Admitted);
}

/// `Student(id) <=[units]{0..budget} Attempt(student)`: exact grouped
/// measures over whole scalar-key groups (E-ADMIT capacity children).
fn capacity_schema(weight: Weight, lo: u64, hi: Option<Bound>) -> Schema {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Student".into(),
                fields: vec![id_field("id"), field("budget", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Attempt".into(),
                fields: vec![
                    id_field("id"),
                    field("student", ValueType::U64),
                    field("units", ValueType::U64),
                    field(
                        "active",
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
            capacity_weighted(
                side(RelationId(0), &[FieldId(0)]),
                weight,
                lo,
                hi,
                side(RelationId(1), &[FieldId(1)]),
            ),
        ],
    }
    .validate()
    .expect("valid")
}

fn attempt(id: u64, student: u64, units: u64, active: (u64, u64)) -> Vec<Value> {
    vec![
        Value::U64(id),
        Value::U64(student),
        Value::U64(units),
        Value::IntervalU64(Interval::new(active.0, active.1).expect("fixture")),
    ]
}

/// An existing selected parent with NO children has total zero: a floor
/// violates it, a pure ceiling admits it. A missing parent imposes no
/// window at all.
#[test]
fn empty_parent_zero_totals_and_missing_parent_vacuity_are_distinct() {
    // Ceiling only: empty group total is zero, admitted.
    let ceiling = capacity_schema(Weight::Unit, 0, Some(Bound::Lit(2)));
    let mut state = MapState::new();
    state.insert(RelationId(0), vec![Value::U64(7), Value::U64(10)]);
    assert_eq!(judge(&ceiling, &state), Judgment::Admitted);

    // Floor 1 (the existence window): the same empty group violates, with
    // the parent cited and the exact zero measure witnessed.
    let floor = capacity_schema(Weight::Unit, 1, None);
    let violations = rejected(&floor, &state);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].kind, StatementKind::Capacity);
    assert_eq!(violations[0].measure, Some(0));
    assert_eq!(violations[0].examples[0].relation, RelationId(0));

    // No parent at all: no group, no window, admitted even under the floor.
    let mut orphans = MapState::new();
    orphans.insert(RelationId(1), attempt(1, 7, 1, (0, 10)));
    // (The attempt's student has no containment law in this fixture.)
    assert_eq!(judge(&floor, &orphans), Judgment::Admitted);
}

/// Count is unit weight over DISTINCT facts: the two-enrollment
/// counterexample from chapter 02 — each state fits capacity one, their
/// union measures two and refuses.
#[test]
fn unit_count_over_distinct_children_exceeds_the_ceiling() {
    let schema = capacity_schema(Weight::Unit, 0, Some(Bound::Lit(1)));
    let mut state = MapState::new();
    state.insert(RelationId(0), vec![Value::U64(7), Value::U64(10)]);
    state.insert(RelationId(1), attempt(1, 7, 1, (0, 10)));
    state.insert(RelationId(1), attempt(2, 7, 1, (0, 10)));
    let violations = rejected(&schema, &state);
    assert_eq!(violations[0].measure, Some(2));
    // The parent and both children are cited within the budget.
    assert_eq!(violations[0].examples.len(), 3);
    assert!(!violations[0].examples_truncated);
}

/// Zero-weight children still exist: weighted total zero does not prove
/// the group empty, and the same rows fail a unit-count ceiling of one.
#[test]
fn zero_weight_children_are_membership_not_absence() {
    let weighted = capacity_schema(Weight::Field(FieldId(2)), 0, Some(Bound::Lit(10)));
    let mut state = MapState::new();
    state.insert(RelationId(0), vec![Value::U64(7), Value::U64(10)]);
    state.insert(RelationId(1), attempt(1, 7, 0, (0, 10)));
    state.insert(RelationId(1), attempt(2, 7, 0, (10, 20)));
    // Weighted total is zero: the weighted ceiling admits…
    assert_eq!(judge(&weighted, &state), Judgment::Admitted);
    // …while the unit count of the same group is two.
    let counted = capacity_schema(Weight::Unit, 0, Some(Bound::Lit(1)));
    let violations = rejected(&counted, &state);
    assert_eq!(violations[0].measure, Some(2));
}

/// Duration weights sum each distinct fact's complete exact length; two
/// overlapping source intervals contribute BOTH durations (whole-group
/// measure, not pointwise occupancy).
#[test]
fn overlapping_durations_sum_completely_not_pointwise() {
    let schema = capacity_schema(Weight::DurationOf(FieldId(3)), 0, Some(Bound::Lit(15)));
    let mut state = MapState::new();
    state.insert(RelationId(0), vec![Value::U64(7), Value::U64(10)]);
    // [0,10) and [5,15): pointwise union is 15, whole-group total is 20.
    state.insert(RelationId(1), attempt(1, 7, 1, (0, 10)));
    state.insert(RelationId(1), attempt(2, 7, 1, (5, 15)));
    let violations = rejected(&schema, &state);
    assert_eq!(violations[0].measure, Some(20));
}

/// A dependent bound reads the TARGET row; a ray in a duration-measured
/// position refuses explicitly even where an absent parent would have made
/// the group law vacuous.
#[test]
fn dependent_bounds_and_ray_duration_refusal() {
    let schema = capacity_schema(
        Weight::Field(FieldId(2)),
        0,
        Some(Bound::TargetField(FieldId(1))),
    );
    let mut state = MapState::new();
    state.insert(RelationId(0), vec![Value::U64(7), Value::U64(5)]);
    state.insert(RelationId(1), attempt(1, 7, 3, (0, 10)));
    state.insert(RelationId(1), attempt(2, 7, 3, (10, 20)));
    // 3 + 3 > budget 5.
    let violations = rejected(&schema, &state);
    assert_eq!(violations[0].measure, Some(6));

    // A ray in the duration weight is an explicit refusal, not a verdict.
    let duration = capacity_schema(Weight::DurationOf(FieldId(3)), 0, Some(Bound::Lit(100)));
    let mut rayed = MapState::new();
    rayed.insert(RelationId(0), vec![Value::U64(7), Value::U64(10)]);
    rayed.insert(
        RelationId(1),
        vec![
            Value::U64(1),
            Value::U64(7),
            Value::U64(1),
            Value::IntervalU64(Interval::ray(5).expect("ray")),
        ],
    );
    assert!(matches!(
        judge_final_state(&duration, &rayed, &work(), JudgeBudget::default()),
        Err(JudgeError::UndefinedDuration {
            statement: StatementId(2)
        })
    ));
}

/// Pointwise keys over the DENSE float line: same scalar prefix with
/// overlapping float spans is a key conflict; a gap of one representable
/// float is a real gap (F-INTERVAL through admission).
#[test]
fn pointwise_float_interval_keys_use_exact_dense_endpoint_order() {
    let schema = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Reading".into(),
            fields: vec![
                field("sensor", ValueType::U64),
                field(
                    "span",
                    ValueType::Interval {
                        element: IntervalElement::F64,
                    },
                ),
            ],
        }],
        statements: vec![fd(RelationId(0), &[FieldId(0), FieldId(1)])],
    }
    .validate()
    .expect("valid");

    // Adjacent representable endpoints: [1.0, nextUp(1.0)) meets
    // [nextUp(1.0), 2.0) — disjoint, admitted.
    let one = F64::from(1.0);
    let next_up = F64::from_bits(one.to_bits() + 1);
    let mut adjacent = MapState::new();
    adjacent.insert(
        RelationId(0),
        vec![
            Value::U64(9),
            Value::IntervalF64(Interval::<F64>::new(one, next_up).expect("gap")),
        ],
    );
    adjacent.insert(
        RelationId(0),
        vec![
            Value::U64(9),
            Value::IntervalF64(Interval::<F64>::new(next_up, F64::from(2.0)).expect("rest")),
        ],
    );
    assert_eq!(judge(&schema, &adjacent), Judgment::Admitted);

    // Overlap on the dense line, including through an unbounded endpoint.
    let mut overlapping = MapState::new();
    overlapping.insert(
        RelationId(0),
        vec![Value::U64(9), Value::IntervalF64(dense(0.0, 1.5))],
    );
    overlapping.insert(
        RelationId(0),
        vec![
            Value::U64(9),
            Value::IntervalF64(
                Interval::<F64>::new(F64::from(1.0), F64::INFINITY).expect("right ray"),
            ),
        ],
    );
    let violations = rejected(&schema, &overlapping);
    assert_eq!(violations[0].statement, StatementId(0));
    assert_eq!(violations[0].examples.len(), 2);

    // Different sensors never conflict: the group key is the scalar prefix.
    let mut split = MapState::new();
    split.insert(
        RelationId(0),
        vec![Value::U64(1), Value::IntervalF64(dense(0.0, 10.0))],
    );
    split.insert(
        RelationId(0),
        vec![Value::U64(2), Value::IntervalF64(dense(0.0, 10.0))],
    );
    assert_eq!(judge(&schema, &split), Judgment::Admitted);
}

/// Forced fingerprint collisions cannot reach this judgment at all: the
/// reference path compares decoded canonical values only. Two long rows
/// that share a forced-constant fingerprint stay distinct facts and are
/// judged as such (HASH-02's semantic half).
#[test]
fn forced_fingerprint_collisions_cannot_merge_judged_facts() {
    use crate::storage::store::Fingerprinter;
    let forced = Fingerprinter::Constant([7; 16]);
    let schema = user_schema();
    let long = "x".repeat(4096);
    let mut state = MapState::new();
    state.insert(RelationId(0), user(1, &long));
    state.insert(RelationId(0), user(2, &long));
    // The two rows collide under the forced fingerprinter…
    assert_eq!(
        forced.row(RelationId(0), format!("{:?}", user(1, &long)).as_bytes()),
        forced.row(RelationId(0), format!("{:?}", user(2, &long)).as_bytes()),
    );
    // …and the judgment still sees two distinct facts fighting one email.
    let violations = rejected(&schema, &state);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].statement, StatementId(1));
    assert_eq!(violations[0].examples.len(), 2);
}

/// Metadata-only / no-op candidates: an empty state and a state equal to
/// itself both admit — the judgment is total over empty relations.
#[test]
fn the_empty_candidate_admits() {
    let schema = user_schema();
    assert_eq!(judge(&schema, &MapState::new()), Judgment::Admitted);
}
