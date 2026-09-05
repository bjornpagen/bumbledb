//! Delta-local judgment behavioral tests (chapter 10 §4 equivalence
//! evidence, judge half): [`judge_final_state_delta_local`] against the
//! complete reference [`judge_final_state`] over one shared state — verdicts
//! and violation sets must be EQUAL on every lawful parent, across all
//! statement families, citation orders, and truncation labels. The
//! lawful-parent premise is pinned honestly: an unlawful parent seeded
//! directly into the state CAN hide from the delta-local judge (asserted,
//! not hidden); the complete judge — the sweeper's path — still convicts.
//!
//! Structural no-scan evidence: the state double refuses to stream ANY
//! relation, so a completing delta-local judgment proves keys and
//! affected containment/capacity groups were judged from compiled indexes
//! and untouched statements were skipped.

use super::{
    CandidateFacts, DeltaFacts, DeltaShape, JudgeBudget, JudgeScratch, Judgment, LawfulParent,
    judge_final_state, judge_incremental,
};
use crate::schema::tests::{capacity, containment, fd, field, side};
use crate::schema::{
    FieldId, IntervalElement, RelationDescriptor, RelationId, Schema, SchemaDescriptor,
    StatementId, ValidateDescriptor as _, ValueType,
};
use crate::work::ExecutionPolicy;
use crate::{Interval, Value, WorkContext};
use std::time::Duration;

const USER: RelationId = RelationId(0);
const BOOKING: RelationId = RelationId(1);
const ROOM: RelationId = RelationId(2);
const USER_ID_KEY: StatementId = StatementId(0);
const USER_EMAIL_KEY: StatementId = StatementId(1);
const BOOKING_KEY: StatementId = StatementId(2);
const ROOM_KEY: StatementId = StatementId(3);
const BOOKING_ROOM_EXISTS: StatementId = StatementId(4);
const ROOM_CAPACITY: StatementId = StatementId(5);

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

/// `User(id)`, `User(email)`, pointwise `Booking(room, span)`, `Room(id)`,
/// `Booking(room) ⊆ Room(id)`, and `Booking(room) <= {0..2} Room(id)` —
/// every judged statement family over three relations.
fn theory() -> Schema {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "User".into(),
                fields: vec![field("id", ValueType::U64), field("email", ValueType::String)],
            },
            RelationDescriptor {
                extension: None,
                name: "Booking".into(),
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
                name: "Room".into(),
                fields: vec![field("id", ValueType::U64)],
            },
        ],
        statements: vec![
            fd(USER, &[FieldId(0)]),
            fd(USER, &[FieldId(1)]),
            fd(BOOKING, &[FieldId(0), FieldId(1)]),
            fd(ROOM, &[FieldId(0)]),
            containment(side(BOOKING, &[FieldId(0)]), side(ROOM, &[FieldId(0)])),
            capacity(
                side(BOOKING, &[FieldId(0)]),
                0,
                Some(2),
                side(ROOM, &[FieldId(0)]),
            ),
        ],
    }
    .validate()
    .expect("delta theory validates")
}

fn user(id: u64, email: &str) -> Vec<Value> {
    vec![Value::U64(id), Value::String(email.into())]
}

fn booking(room: u64, start: u64, end: u64) -> Vec<Value> {
    vec![
        Value::U64(room),
        Value::IntervalU64(Interval::new(start, end).expect("nonempty span")),
    ]
}

fn room(id: u64) -> Vec<Value> {
    vec![Value::U64(id)]
}

/// One statement's group index in the double: `(statement, relation, scalar
/// determinant field indices)`.
type KeyIndex = (StatementId, RelationId, &'static [usize]);

const ALL_KEYS: &[KeyIndex] = &[
    (USER_ID_KEY, USER, &[0]),
    (USER_EMAIL_KEY, USER, &[1]),
    (BOOKING_KEY, BOOKING, &[0]),
    (ROOM_KEY, ROOM, &[0]),
];

/// The [`DeltaFacts`] test double: an in-memory final state in a fixed
/// deterministic order, the delta's shape and added rows, and an exact
/// group index (a filter over the final rows — trivially the store index's
/// denotation). `refuse_stream` turns any relation stream into a panic, so
/// tests can PROVE the delta-local judge never scanned.
pub(super) struct DeltaState {
    rows: Vec<(RelationId, Vec<Value>)>,
    shapes: Vec<(RelationId, DeltaShape)>,
    added: Vec<(RelationId, Vec<Value>)>,
    removed: Vec<(RelationId, Vec<Value>)>,
    keys: &'static [KeyIndex],
    refuse_stream: bool,
    row_visits: std::cell::Cell<u64>,
    group_visits: std::cell::Cell<u64>,
}

impl DeltaState {
    /// Build the final state from a lawful-or-not parent plus one net delta.
    /// Final order is parent order (minus removed) then adds — the store's
    /// row-id order shape.
    pub(super) fn new(
        parent: &[(RelationId, Vec<Value>)],
        adds: &[(RelationId, Vec<Value>)],
        removes: &[(RelationId, Vec<Value>)],
    ) -> Self {
        let mut rows: Vec<(RelationId, Vec<Value>)> = parent
            .iter()
            .filter(|entry| !removes.contains(entry))
            .cloned()
            .collect();
        for add in adds {
            if !rows.contains(add) {
                rows.push(add.clone());
            }
        }
        let mut shapes: Vec<(RelationId, DeltaShape)> = Vec::new();
        let mut touch = |relation: RelationId, add: bool| {
            let shape = match shapes.iter_mut().find(|(id, _)| *id == relation) {
                Some((_, shape)) => shape,
                None => {
                    shapes.push((relation, DeltaShape::default()));
                    &mut shapes.last_mut().unwrap().1
                }
            };
            if add {
                shape.adds = true;
            } else {
                shape.removes = true;
            }
        };
        for (relation, _) in adds {
            touch(*relation, true);
        }
        for (relation, _) in removes {
            touch(*relation, false);
        }
        Self {
            rows,
            shapes,
            added: adds.to_vec(),
            removed: removes.to_vec(),
            keys: ALL_KEYS,
            refuse_stream: false,
            row_visits: std::cell::Cell::new(0),
            group_visits: std::cell::Cell::new(0),
        }
    }

    fn without_index(mut self) -> Self {
        self.keys = &[];
        self
    }

    pub(super) fn refusing_streams(mut self) -> Self {
        self.refuse_stream = true;
        self
    }

    pub(super) fn row_visits(&self) -> u64 {
        self.row_visits.get()
    }

    pub(super) fn group_visits(&self) -> u64 {
        self.group_visits.get()
    }
}

impl CandidateFacts for DeltaState {
    type Error = std::convert::Infallible;

    fn visit_rows(
        &self,
        relation: RelationId,
        visit: &mut dyn FnMut(&[Value]) -> Result<bool, Self::Error>,
    ) -> Result<(), Self::Error> {
        assert!(
            !self.refuse_stream,
            "the delta-local judge streamed relation {relation:?} — a skip \
             or index path failed structurally"
        );
        for (id, values) in &self.rows {
            if *id != relation {
                continue;
            }
            self.row_visits.set(self.row_visits.get() + 1);
            if !visit(values)? {
                break;
            }
        }
        Ok(())
    }
}

impl DeltaFacts for DeltaState {
    fn delta_shape(&self, relation: RelationId) -> DeltaShape {
        self.shapes
            .iter()
            .find(|(id, _)| *id == relation)
            .map_or_else(DeltaShape::default, |(_, shape)| *shape)
    }

    fn visit_added_rows(
        &self,
        relation: RelationId,
        visit: &mut dyn FnMut(&[Value]) -> Result<bool, Self::Error>,
    ) -> Result<(), Self::Error> {
        for (id, values) in &self.added {
            if *id == relation && !visit(values)? {
                break;
            }
        }
        Ok(())
    }

    fn visit_removed_rows(
        &self,
        relation: RelationId,
        visit: &mut dyn FnMut(&[Value]) -> Result<bool, Self::Error>,
    ) -> Result<(), Self::Error> {
        for (id, values) in &self.removed {
            if *id == relation && !visit(values)? {
                break;
            }
        }
        Ok(())
    }

    fn visit_key_competitors(
        &self,
        statement: StatementId,
        determinant: &[Value],
        visit: &mut dyn FnMut(&[Value]) -> Result<bool, Self::Error>,
    ) -> Result<Option<()>, Self::Error> {
        let Some((_, relation, fields)) = self.keys.iter().find(|(id, _, _)| *id == statement)
        else {
            return Ok(None);
        };
        self.group_visits.set(self.group_visits.get().saturating_add(1));
        for (id, values) in &self.rows {
            if id != relation {
                continue;
            }
            let projected: Vec<Value> = fields.iter().map(|&at| values[at].clone()).collect();
            if projected.as_slice() == determinant && !visit(values)? {
                break;
            }
        }
        Ok(Some(()))
    }

    fn visit_compiled_group(
        &self,
        projection: &crate::schema::compiled::CompiledProjection,
        determinant: &[Value],
        visit: &mut dyn FnMut(&[Value]) -> Result<bool, Self::Error>,
    ) -> Result<Option<()>, Self::Error> {
        if self.keys.is_empty() {
            return Ok(None);
        }
        self.group_visits
            .set(self.group_visits.get().saturating_add(1));
        for (id, values) in &self.rows {
            if *id != projection.relation {
                continue;
            }
            if projection.scalar_values(values).as_slice() == determinant && !visit(values)? {
                break;
            }
        }
        Ok(Some(()))
    }
}

/// Judge BOTH ways with one budget and require exact equality — verdicts,
/// statement sets, citation content and order, truncation labels.
fn assert_equivalent(schema: &Schema, state: &DeltaState, budget: JudgeBudget) -> Judgment {
    let complete =
        judge_final_state(schema, state, &work(), budget).expect("complete judgment completes");
    let delta = judge_incremental(
        LawfulParent::established(),
        schema,
        state,
        &work(),
        budget,
        JudgeScratch::disabled(),
    )
    .expect("delta judgment");
    assert_eq!(
        delta, complete,
        "delta-local judgment must equal the complete reference"
    );
    delta
}

fn lawful_parent() -> Vec<(RelationId, Vec<Value>)> {
    vec![
        (USER, user(1, "a@example")),
        (USER, user(2, "b@example")),
        (ROOM, room(10)),
        (ROOM, room(11)),
        (BOOKING, booking(10, 0, 5)),
        (BOOKING, booking(10, 5, 9)),
        (BOOKING, booking(11, 3, 7)),
    ]
}

#[test]
fn delta_local_judgment_equals_the_complete_judge_on_lawful_parents() {
    let schema = theory();
    let parent = lawful_parent();
    let fixtures: Vec<(
        &str,
        Vec<(RelationId, Vec<Value>)>,
        Vec<(RelationId, Vec<Value>)>,
        bool,
    )> = vec![
        ("benign insert", vec![(USER, user(3, "c@example"))], vec![], true),
        (
            "duplicate email",
            vec![(USER, user(3, "a@example"))],
            vec![],
            false,
        ),
        (
            "email replace",
            vec![(USER, user(1, "a2@example"))],
            vec![(USER, user(1, "a@example"))],
            true,
        ),
        (
            "overlapping booking",
            vec![(BOOKING, booking(10, 4, 6))],
            vec![],
            false,
        ),
        (
            "adjacent booking",
            vec![(BOOKING, booking(11, 7, 9))],
            vec![],
            true,
        ),
        (
            "orphan booking",
            vec![(BOOKING, booking(99, 0, 1))],
            vec![],
            false,
        ),
        (
            "room removal strands bookings",
            vec![],
            vec![(ROOM, room(10))],
            false,
        ),
        (
            "third booking exceeds room capacity",
            vec![(BOOKING, booking(10, 20, 21))],
            vec![],
            false,
        ),
        ("booking delete", vec![], vec![(BOOKING, booking(11, 3, 7))], true),
        (
            "room delete without bookings after booking delete",
            vec![],
            vec![(BOOKING, booking(11, 3, 7)), (ROOM, room(11))],
            true,
        ),
    ];
    for (label, adds, removes, admitted) in fixtures {
        let state = DeltaState::new(&parent, &adds, &removes);
        let verdict = assert_equivalent(&schema, &state, JudgeBudget::default());
        assert_eq!(
            matches!(verdict, Judgment::Admitted),
            admitted,
            "{label}: unexpected verdict {verdict:?}"
        );
    }
}

#[test]
fn a_multi_statement_rejection_reports_every_family_both_ways() {
    let schema = theory();
    let parent = lawful_parent();
    // One delta: duplicate email, overlapping booking, orphan booking, and
    // a third booking blowing room 10's capacity — every judged family.
    let adds = vec![
        (USER, user(4, "b@example")),
        (BOOKING, booking(10, 4, 6)),
        (BOOKING, booking(10, 20, 22)),
        (BOOKING, booking(99, 0, 1)),
    ];
    let state = DeltaState::new(&parent, &adds, &[]);
    let verdict = assert_equivalent(&schema, &state, JudgeBudget::default());
    let Judgment::Rejected(violations) = verdict else {
        panic!("expected a rejection");
    };
    let cited: Vec<StatementId> = violations.iter().map(|v| v.statement).collect();
    assert_eq!(
        cited,
        vec![USER_EMAIL_KEY, BOOKING_KEY, BOOKING_ROOM_EXISTS, ROOM_CAPACITY],
        "every violated statement is named, in canonical order"
    );
}

#[test]
fn citation_order_and_truncation_labels_match_under_every_budget() {
    let schema = theory();
    let parent = vec![(USER, user(1, "x@example")), (ROOM, room(1))];
    // Two more rows land in the same email group: three competitors total.
    let adds = vec![(USER, user(2, "x@example")), (USER, user(3, "x@example"))];
    for examples_per_statement in [0usize, 1, 2, 3, 4] {
        let state = DeltaState::new(&parent, &adds, &[]);
        let budget = JudgeBudget {
            examples_per_statement,
        };
        let verdict = assert_equivalent(&schema, &state, budget);
        let Judgment::Rejected(violations) = verdict else {
            panic!("expected a rejection at budget {examples_per_statement}");
        };
        assert_eq!(violations.len(), 1);
        let violation = &violations[0];
        assert_eq!(violation.examples.len(), examples_per_statement.min(3));
        assert_eq!(
            violation.examples_truncated,
            examples_per_statement < 3,
            "truncation labels exactly the offenders beyond the budget"
        );
    }
}

#[test]
fn pointwise_offender_selection_is_the_reference_adjacent_pair_sweep() {
    let schema = theory();
    // Empty parent (trivially lawful); one delta proposes A[0,10), B[2,3),
    // C[5,6) in one room group. Sorted by start the reference flags the
    // adjacent pair (A, B) and NOT C (whose predecessor B ends before it),
    // even though A covers C — the delta-local sweep must reproduce that
    // exact offender selection, not merely the verdict.
    let parent = vec![(ROOM, room(7))];
    let adds = vec![
        (BOOKING, booking(7, 0, 10)),
        (BOOKING, booking(7, 2, 3)),
        (BOOKING, booking(7, 5, 6)),
    ];
    let state = DeltaState::new(&parent, &adds, &[]);
    let verdict = assert_equivalent(&schema, &state, JudgeBudget::default());
    let Judgment::Rejected(violations) = verdict else {
        panic!("expected the pointwise rejection");
    };
    let key = violations
        .iter()
        .find(|v| v.statement == BOOKING_KEY)
        .expect("the pointwise key is violated");
    let spans: Vec<&Value> = key.examples.iter().map(|fact| &fact.values[1]).collect();
    assert_eq!(
        spans,
        vec![
            &Value::IntervalU64(Interval::new(0, 10).unwrap()),
            &Value::IntervalU64(Interval::new(2, 3).unwrap()),
        ],
        "exactly the adjacent overlapping pair is cited, in state order"
    );
    assert!(!key.examples_truncated);
}

#[test]
fn an_unlawful_parent_can_hide_from_the_delta_local_judge_by_design() {
    let schema = theory();
    // The parent ALREADY violates the email key and strands a booking —
    // states unreachable through admission, seeded directly (the test-hook
    // shape). The delta touches only untouched groups/relations.
    let unlawful_parent = vec![
        (USER, user(1, "dup@example")),
        (USER, user(2, "dup@example")),
        (BOOKING, booking(99, 0, 1)),
        (ROOM, room(1)),
    ];
    let adds = vec![(USER, user(3, "fresh@example"))];
    let state = DeltaState::new(&unlawful_parent, &adds, &[]);
    let complete = judge_final_state(&schema, &state, &work(), JudgeBudget::default())
        .expect("complete judgment");
    let delta = judge_incremental(
        LawfulParent::established(),
        &schema,
        &state,
        &work(),
        JudgeBudget::default(),
        JudgeScratch::disabled(),
    )
    .expect("delta judgment");
    // The complete judge — the sweeper's judgment — convicts the standing
    // violations; the delta-local judge, whose soundness ASSUMES a lawful
    // parent, admits. This divergence is the documented premise, pinned so
    // it can never be mistaken for equivalence without it.
    let Judgment::Rejected(violations) = complete else {
        panic!("the complete judge must convict the unlawful parent");
    };
    let convicted: Vec<StatementId> = violations.iter().map(|v| v.statement).collect();
    assert_eq!(convicted, vec![USER_EMAIL_KEY, BOOKING_ROOM_EXISTS]);
    assert_eq!(
        delta,
        Judgment::Admitted,
        "the delta-local judge misses violations the delta does not touch — \
         the lawful-parent premise, stated honestly"
    );
}

#[test]
fn a_delta_touching_an_unlawful_group_still_convicts_delta_locally() {
    let schema = theory();
    // Same unlawful parent, but the delta ADDS a row into the standing
    // duplicate-email group: the group is now delta-touched, so its full
    // final membership is judged and all three competitors are cited.
    let unlawful_parent = vec![
        (USER, user(1, "dup@example")),
        (USER, user(2, "dup@example")),
        (ROOM, room(1)),
    ];
    let adds = vec![(USER, user(3, "dup@example"))];
    let state = DeltaState::new(&unlawful_parent, &adds, &[]);
    let delta = judge_incremental(
        LawfulParent::established(),
        &schema,
        &state,
        &work(),
        JudgeBudget::default(),
        JudgeScratch::disabled(),
    )
    .expect("delta judgment");
    let Judgment::Rejected(violations) = delta else {
        panic!("a delta-touched unlawful group must convict");
    };
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].statement, USER_EMAIL_KEY);
    assert_eq!(violations[0].examples.len(), 3, "all competitors cited");
}

#[test]
fn a_state_without_a_group_index_falls_back_to_the_streaming_judge() {
    let schema = theory();
    let parent = lawful_parent();
    let adds = vec![(USER, user(3, "a@example"))];
    let state = DeltaState::new(&parent, &adds, &[]).without_index();
    let verdict = assert_equivalent(&schema, &state, JudgeBudget::default());
    let Judgment::Rejected(violations) = verdict else {
        panic!("the fallback must still convict the duplicate email");
    };
    assert_eq!(violations[0].statement, USER_EMAIL_KEY);
}

#[test]
fn delta_local_key_judgment_never_streams_any_relation() {
    let schema = theory();
    let parent = lawful_parent();
    // Admission: benign insert. Rejection: duplicate email. Both must
    // complete with EVERY relation stream refusing — keys are judged from
    // the group index alone and untouched containment/capacity statements
    // are skipped, structurally.
    let benign = DeltaState::new(&parent, &[(USER, user(3, "c@example"))], &[]).refusing_streams();
    let verdict = judge_incremental(
        LawfulParent::established(),
        &schema,
        &benign,
        &work(),
        JudgeBudget::default(),
        JudgeScratch::disabled(),
    )
    .expect("delta judgment");
    assert_eq!(verdict, Judgment::Admitted);

    let dup = DeltaState::new(&parent, &[(USER, user(3, "b@example"))], &[]).refusing_streams();
    let verdict = judge_incremental(
        LawfulParent::established(),
        &schema,
        &dup,
        &work(),
        JudgeBudget::default(),
        JudgeScratch::disabled(),
    )
    .expect("delta judgment");
    let Judgment::Rejected(violations) = verdict else {
        panic!("expected the duplicate-email rejection");
    };
    assert_eq!(violations[0].statement, USER_EMAIL_KEY);
    assert_eq!(violations[0].examples.len(), 2, "both competitors cited");
}
