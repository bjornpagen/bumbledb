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
use crate::schema::tests::{capacity, closed, containment, fd, field, row, side, side_where};
use crate::schema::{
    FieldId, IntervalElement, RelationDescriptor, RelationId, Schema, SchemaDescriptor,
    StatementId, ValidateDescriptor as _, ValueType,
};
use crate::work::WorkContext;
use crate::{Interval, Value};

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
    WorkContext::new()
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
                fields: vec![
                    field("id", ValueType::U64),
                    field("email", ValueType::String),
                ],
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

struct RankedOrder {
    ranks: Vec<u64>,
    full: Vec<usize>,
    group: Vec<usize>,
}

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
    key_row_visits: std::cell::Cell<u64>,
    cancel_on_key_row: Option<(u64, WorkContext)>,
    compiled_row_visits: std::cell::Cell<u64>,
    unindexed_group: Option<(RelationId, Vec<Value>)>,
    unindexed_key_group: Option<(StatementId, Vec<Value>)>,
    indexed_matches_before_decline: std::cell::Cell<u64>,
    declined_groups: std::cell::Cell<u64>,
    ranked_order: Option<RankedOrder>,
    ranked_available: bool,
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
            let shape = if let Some((_, shape)) = shapes.iter_mut().find(|(id, _)| *id == relation)
            {
                shape
            } else {
                shapes.push((relation, DeltaShape::default()));
                &mut shapes.last_mut().unwrap().1
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
            key_row_visits: std::cell::Cell::new(0),
            cancel_on_key_row: None,
            compiled_row_visits: std::cell::Cell::new(0),
            unindexed_group: None,
            unindexed_key_group: None,
            indexed_matches_before_decline: std::cell::Cell::new(0),
            declined_groups: std::cell::Cell::new(0),
            ranked_order: None,
            ranked_available: true,
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

    fn ranked_rows(
        &self,
        relation: RelationId,
        group: bool,
    ) -> impl Iterator<Item = (u64, &[Value])> {
        (0..self.rows.len())
            .map(move |position| {
                self.ranked_order.as_ref().map_or(position, |order| {
                    if group {
                        order.group[position]
                    } else {
                        order.full[position]
                    }
                })
            })
            .filter(move |&index| self.rows[index].0 == relation)
            .enumerate()
            .map(move |(ordinal, index)| {
                let rank = self
                    .ranked_order
                    .as_ref()
                    .map_or(ordinal as u64, |order| order.ranks[index]);
                (rank, self.rows[index].1.as_slice())
            })
    }

    fn visit_group(
        &self,
        projection: &crate::schema::compiled::CompiledProjection,
        determinant: &[Value],
        visit: super::RankedRowVisitor<'_, std::convert::Infallible>,
    ) -> Result<Option<()>, std::convert::Infallible> {
        if self.keys.is_empty() {
            return Ok(None);
        }
        if self
            .unindexed_group
            .as_ref()
            .is_some_and(|(relation, values)| {
                *relation == projection.relation && values.as_slice() == determinant
            })
        {
            let remaining = self.indexed_matches_before_decline.get();
            if remaining == 0 {
                self.declined_groups.set(self.declined_groups.get() + 1);
                return Ok(None);
            }
            self.indexed_matches_before_decline.set(remaining - 1);
        }
        self.group_visits
            .set(self.group_visits.get().saturating_add(1));
        // Rank the entire relation before filtering the bucket. Numbering
        // only matching rows would reuse rank zero for distinct groups.
        for (rank, values) in self.ranked_rows(projection.relation, true) {
            if projection.scalar_values(values).as_slice() == determinant {
                self.compiled_row_visits
                    .set(self.compiled_row_visits.get() + 1);
                if !visit(rank, values)? {
                    break;
                }
            }
        }
        Ok(Some(()))
    }
}

impl CandidateFacts for DeltaState {
    type Error = std::convert::Infallible;

    fn visit_rows(
        &self,
        relation: RelationId,
        visit: &mut dyn FnMut(&[Value]) -> Result<bool, Self::Error>,
    ) -> Result<(), Self::Error> {
        self.visit_ranked_rows(relation, &mut |_rank, values| visit(values))
    }

    fn visit_ranked_rows(
        &self,
        relation: RelationId,
        visit: super::RankedRowVisitor<'_, Self::Error>,
    ) -> Result<(), Self::Error> {
        assert!(
            !self.refuse_stream,
            "the delta-local judge streamed relation {relation:?} — a skip \
             or index path failed structurally"
        );
        for (rank, values) in self.ranked_rows(relation, false) {
            self.row_visits.set(self.row_visits.get() + 1);
            if !visit(rank, values)? {
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
        if self
            .unindexed_key_group
            .as_ref()
            .is_some_and(|(id, values)| *id == statement && values.as_slice() == determinant)
        {
            let remaining = self.indexed_matches_before_decline.get();
            if remaining == 0 {
                self.declined_groups.set(self.declined_groups.get() + 1);
                return Ok(None);
            }
            self.indexed_matches_before_decline.set(remaining - 1);
        }
        self.group_visits
            .set(self.group_visits.get().saturating_add(1));
        for (id, values) in &self.rows {
            if id != relation {
                continue;
            }
            let projected: Vec<Value> = fields.iter().map(|&at| values[at].clone()).collect();
            if projected.as_slice() == determinant {
                self.key_row_visits.set(self.key_row_visits.get() + 1);
                if let Some((at, context)) = &self.cancel_on_key_row
                    && self.key_row_visits.get() == *at
                {
                    context.cancel();
                }
                if !visit(values)? {
                    break;
                }
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
        self.visit_group(projection, determinant, &mut |_rank, values| visit(values))
    }

    fn visit_ranked_compiled_group(
        &self,
        projection: &crate::schema::compiled::CompiledProjection,
        determinant: &[Value],
        visit: super::RankedRowVisitor<'_, Self::Error>,
    ) -> Result<Option<()>, Self::Error> {
        if !self.ranked_available {
            return Ok(None);
        }
        self.visit_group(projection, determinant, visit)
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

type Facts = Vec<(RelationId, Vec<Value>)>;

/// The open twin keeps identical field/statement ids but cannot use the
/// closed-member shortcut: its complete judge exercises grouped containment.
fn closed_edge_pair(
    members: usize,
    selection: Vec<(FieldId, Value)>,
    equality: bool,
) -> (Schema, Schema, Facts) {
    let source = RelationDescriptor {
        extension: None,
        name: "Source".into(),
        fields: vec![
            field("member", ValueType::U64),
            field("id", ValueType::U64),
            field("active", ValueType::Bool),
        ],
    };
    let target_rows: Facts = (0..members)
        .map(|index| {
            (
                RelationId(1),
                vec![
                    Value::U64(index as u64),
                    Value::Bool(index.is_multiple_of(2)),
                ],
            )
        })
        .collect();
    let source_side = side_where(
        RelationId(0),
        &[FieldId(0)],
        vec![(FieldId(2), Value::Bool(true))],
    );
    let target_side = side_where(RelationId(1), &[FieldId(0)], selection);
    let mut statements = Vec::new();
    if equality {
        statements.push(fd(RelationId(0), &[FieldId(0)]));
    }
    statements.push(containment(source_side.clone(), target_side.clone()));
    if equality {
        statements.push(containment(target_side, source_side));
    }
    let closed_schema = SchemaDescriptor {
        relations: vec![
            source.clone(),
            closed(
                "Target",
                vec![field("enabled", ValueType::Bool)],
                target_rows
                    .iter()
                    .enumerate()
                    .map(|(index, (_, values))| row(&format!("v{index}"), vec![values[1].clone()]))
                    .collect(),
            ),
        ],
        statements: statements.clone(),
    }
    .validate()
    .unwrap();
    statements.insert(0, fd(RelationId(1), &[FieldId(0)]));
    let open_schema = SchemaDescriptor {
        relations: vec![
            source,
            RelationDescriptor {
                extension: None,
                name: "Target".into(),
                fields: vec![
                    field("id", ValueType::U64),
                    field("enabled", ValueType::Bool),
                ],
            },
        ],
        statements,
    }
    .validate()
    .unwrap();
    (closed_schema, open_schema, target_rows)
}

fn closed_source(member: u64, id: u64, active: bool) -> (RelationId, Vec<Value>) {
    (
        RelationId(0),
        vec![Value::U64(member), Value::U64(id), Value::Bool(active)],
    )
}

fn assert_closed_open_twin(
    pair: &(Schema, Schema, Facts),
    parent: &[(RelationId, Vec<Value>)],
    adds: &[(RelationId, Vec<Value>)],
    removes: &[(RelationId, Vec<Value>)],
    budget: JudgeBudget,
) -> Judgment {
    let (closed_schema, open_schema, ground) = pair;
    let mut open_parent = parent.to_vec();
    open_parent.extend_from_slice(ground);
    let open = DeltaState::new(&open_parent, adds, removes);
    let closed = DeltaState::new(parent, adds, removes);
    let expected = judge_final_state(open_schema, &open, &work(), budget).unwrap();
    assert_eq!(
        judge_final_state(closed_schema, &closed, &work(), budget).unwrap(),
        expected
    );
    assert_eq!(
        judge_incremental(
            LawfulParent::established(),
            closed_schema,
            &closed,
            &work(),
            budget,
            JudgeScratch::disabled(),
        )
        .unwrap(),
        expected
    );
    expected
}

#[test]
fn closed_member_fast_path_matches_open_twin_for_selection_delta_and_citations() {
    let pair = closed_edge_pair(8, vec![(FieldId(1), Value::Bool(true))], false);
    let parent = vec![closed_source(0, 0, true), closed_source(999, 1, false)];
    let cases = [
        (vec![parent[0].clone()], vec![], true),
        (vec![parent[0].clone()], vec![parent[0].clone()], true),
        (vec![closed_source(2, 2, true)], vec![], true),
        (vec![closed_source(1, 2, false)], vec![], true),
        (vec![], vec![parent[0].clone()], true),
        (
            vec![closed_source(4, 0, true)],
            vec![parent[0].clone()],
            true,
        ),
        (
            vec![closed_source(256, 0, true)],
            vec![parent[0].clone()],
            false,
        ),
        (
            vec![closed_source(256, 4, true), closed_source(256, 4, true)],
            vec![],
            false,
        ),
        (
            vec![
                closed_source(1, 8, true),
                closed_source(256, 7, true),
                closed_source(u64::MAX, 6, true),
            ],
            vec![],
            false,
        ),
    ];
    for (adds, removes, admitted) in cases {
        for examples in [0, 1, 8] {
            let verdict = assert_closed_open_twin(
                &pair,
                &parent,
                &adds,
                &removes,
                JudgeBudget {
                    examples_per_statement: examples,
                },
            );
            assert_eq!(matches!(verdict, Judgment::Admitted), admitted);
        }
    }
    // Selecting the unprojected flag excludes this sole (enabled) axiom.
    // A selection on the projected handle itself is rejected by sealing.
    let pair = closed_edge_pair(1, vec![(FieldId(1), Value::Bool(false))], false);
    let parent = [closed_source(0, 0, false)];
    assert!(matches!(
        assert_closed_open_twin(
            &pair,
            &parent,
            &[closed_source(0, 1, true)],
            &[],
            JudgeBudget::default()
        ),
        Judgment::Rejected(_)
    ));
}

#[test]
fn closed_member_equality_keeps_the_mutable_opposite_direction() {
    let pair = closed_edge_pair(4, vec![(FieldId(1), Value::Bool(true))], true);
    let parent = [closed_source(0, 0, true), closed_source(2, 1, true)];
    assert_eq!(
        assert_closed_open_twin(&pair, &parent, &[], &[], JudgeBudget::default()),
        Judgment::Admitted
    );
    let verdict =
        assert_closed_open_twin(&pair, &parent, &[], &parent[..1], JudgeBudget::default());
    let Judgment::Rejected(violations) = verdict else {
        panic!("deleting the open-side witness must break equality");
    };
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].statement, StatementId(3));
    assert_eq!(violations[0].examples[0].relation, RelationId(1));
    assert!(matches!(
        assert_closed_open_twin(
            &pair,
            &parent,
            &[closed_source(1, 2, true)],
            &[],
            JudgeBudget::default()
        ),
        Judgment::Rejected(_)
    ));
}

#[test]
fn valid_closed_delta_uses_no_full_scans_scratch_or_parent_cardinality_work() {
    for members in [2, 256] {
        let (schema, _, _) =
            closed_edge_pair(members, vec![(FieldId(1), Value::Bool(true))], false);
        // Shared compiled schema outlives a judgment; measure only its temporary state.
        schema.compiled_theory().unwrap();
        for size in [0, 1000] {
            let parent: Facts = (0..size).map(|id| closed_source(0, id, true)).collect();
            let state =
                DeltaState::new(&parent, &[closed_source(0, size, true)], &[]).refusing_streams();
            let work = WorkContext::new();
            let before = crate::alloc_counter::snapshot().absolute.live_bytes;
            assert_eq!(
                judge_incremental(
                    LawfulParent::established(),
                    &schema,
                    &state,
                    &work,
                    JudgeBudget::default(),
                    JudgeScratch::disabled()
                )
                .unwrap(),
                Judgment::Admitted
            );
            assert_eq!(state.row_visits(), 0);
            assert_eq!(state.group_visits(), 0);
            #[cfg(feature = "alloc-counter")]
            assert_eq!(crate::alloc_counter::snapshot().absolute.live_bytes, before);
            let _ = before;
        }
    }
}

#[test]
fn closed_member_bitset_boundaries_match_the_open_reference_without_truncation() {
    let even = closed_edge_pair(256, vec![(FieldId(1), Value::Bool(true))], false);
    let valid: Facts = [0, 62, 64, 126, 128, 190, 192, 254]
        .into_iter()
        .map(|member| closed_source(member, member, true))
        .collect();
    assert_eq!(
        assert_closed_open_twin(&even, &[], &valid, &[], JudgeBudget::default()),
        Judgment::Admitted
    );
    let invalid = [
        closed_source(255, 1, true),
        closed_source(256, 2, true),
        closed_source(u64::MAX, 3, true),
    ];
    assert!(matches!(
        assert_closed_open_twin(&even, &[], &invalid, &[], JudgeBudget::default()),
        Judgment::Rejected(_)
    ));
    let odd = closed_edge_pair(256, vec![(FieldId(1), Value::Bool(false))], false);
    assert_eq!(
        assert_closed_open_twin(
            &odd,
            &[],
            &[closed_source(255, 0, true)],
            &[],
            JudgeBudget::default()
        ),
        Judgment::Admitted
    );
}

#[test]
fn invalid_closed_delta_cites_only_added_offenders_without_rescanning_parent() {
    let pair = closed_edge_pair(2, vec![(FieldId(1), Value::Bool(true))], false);
    let parent: Facts = (0..1000).map(|id| closed_source(0, id, true)).collect();
    let adds = [
        closed_source(256, 1, true),
        closed_source(1, 2, true),
        closed_source(999, 3, false),
    ];
    let budget = JudgeBudget {
        examples_per_statement: 1,
    };
    let expected = assert_closed_open_twin(&pair, &parent, &adds, &[], budget);
    let state = DeltaState::new(&parent, &adds, &[]).refusing_streams();
    assert_eq!(
        judge_incremental(
            LawfulParent::established(),
            &pair.0,
            &state,
            &work(),
            budget,
            JudgeScratch::disabled()
        )
        .unwrap(),
        expected
    );
    assert_eq!(state.row_visits(), 0);
    assert_eq!(state.group_visits(), 0);
}

/// A provider failure after one offered row must not publish provisional
/// citations or mask a work refusal, even if it fails after a stop request.
struct ClosedAddedFailure(Vec<Value>);

impl CandidateFacts for ClosedAddedFailure {
    type Error = &'static str;

    fn visit_rows(
        &self,
        _: RelationId,
        _: super::RowVisitor<'_, Self::Error>,
    ) -> Result<(), Self::Error> {
        panic!("closed containment must not ask the state for complete/closed rows")
    }
}

impl DeltaFacts for ClosedAddedFailure {
    fn delta_shape(&self, relation: RelationId) -> DeltaShape {
        DeltaShape {
            adds: relation == RelationId(0),
            removes: false,
        }
    }

    fn visit_added_rows(
        &self,
        relation: RelationId,
        visit: super::RowVisitor<'_, Self::Error>,
    ) -> Result<(), Self::Error> {
        assert_eq!(relation, RelationId(0));
        let _ = visit(&self.0)?;
        Err("injected added-row failure")
    }

    fn visit_removed_rows(
        &self,
        _: RelationId,
        _: super::RowVisitor<'_, Self::Error>,
    ) -> Result<(), Self::Error> {
        panic!("immutable target cannot have removed rows")
    }

    fn visit_key_competitors(
        &self,
        _: StatementId,
        _: &[Value],
        _: super::RowVisitor<'_, Self::Error>,
    ) -> Result<Option<()>, Self::Error> {
        panic!("closed-member check requires no key competitors")
    }
}

#[test]
fn closed_delta_propagates_cancellation_and_provider_errors_without_partial_verdict() {
    let (schema, _, _) = closed_edge_pair(2, vec![(FieldId(1), Value::Bool(true))], false);
    let state = ClosedAddedFailure(closed_source(256, 0, true).1);
    let context = work();
    assert!(matches!(
        judge_incremental(
            LawfulParent::established(),
            &schema,
            &state,
            &context,
            JudgeBudget::default(),
            JudgeScratch::disabled()
        ),
        Err(super::JudgeError::State("injected added-row failure"))
    ));
    context.cancel();
    assert!(matches!(
        judge_incremental(
            LawfulParent::established(),
            &schema,
            &state,
            &context,
            JudgeBudget::default(),
            JudgeScratch::disabled()
        ),
        Err(super::JudgeError::Work(crate::WorkError::Cancelled))
    ));
}

#[test]
fn delta_local_judgment_equals_the_complete_judge_on_lawful_parents() {
    let schema = theory();
    let parent = lawful_parent();
    let fixtures = vec![
        (
            "benign insert",
            vec![(USER, user(3, "c@example"))],
            vec![],
            true,
        ),
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
        (
            "booking delete",
            vec![],
            vec![(BOOKING, booking(11, 3, 7))],
            true,
        ),
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
        vec![
            USER_EMAIL_KEY,
            BOOKING_KEY,
            BOOKING_ROOM_EXISTS,
            ROOM_CAPACITY
        ],
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
fn sparse_ranked_pointwise_ties_keep_both_offenders_under_shuffled_traversal() {
    let relation = RelationId(0);
    let schema = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Span".into(),
            fields: vec![
                field("group", ValueType::U64),
                field(
                    "span",
                    ValueType::Interval {
                        element: IntervalElement::U64,
                    },
                ),
                field("marker", ValueType::U64),
            ],
        }],
        statements: vec![fd(relation, &[FieldId(0), FieldId(1)])],
    }
    .validate()
    .unwrap();
    let row = |start, end, marker| {
        let mut values = booking(0, start, end);
        values.push(Value::U64(marker));
        (relation, values)
    };
    let added = [row(0, 10, 0), row(0, 10, 1), row(20, 30, 2)];
    for examples_per_statement in [0, 1, 2, 4] {
        let budget = JudgeBudget {
            examples_per_statement,
        };
        let plain = DeltaState::new(&[], &added, &[]);
        let expected = judge_final_state(&schema, &plain, &work(), budget).unwrap();
        let mut shuffled = DeltaState::new(&[], &added, &[]);
        shuffled.ranked_order = Some(RankedOrder {
            ranks: vec![0, u64::MAX, 42],
            full: vec![2, 0, 1],
            group: vec![1, 2, 0],
        });
        let actual = assert_equivalent(&schema, &shuffled, budget);
        assert_eq!(actual, expected);
        let Judgment::Rejected(violations) = actual else {
            panic!("equal spans conflict")
        };
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].examples.len(), examples_per_statement.min(2));
        for cited in &violations[0].examples {
            assert_ne!(
                cited.values[2],
                Value::U64(2),
                "disjoint third row is not an offender"
            );
        }
    }
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

#[test]
fn scalar_key_capacity_overflow_and_cancellation_use_distinct_errors() {
    use super::grouped::ScalarKeyScratch;
    use crate::storage::store::StoreError;
    let context = work();
    let before = crate::alloc_counter::snapshot().absolute.live_bytes;
    // Capacity overflow is deterministic and makes no giant allocation attempt.
    assert!(matches!(
        ScalarKeyScratch::<std::convert::Infallible>::new(&context, usize::MAX, None),
        Err(super::JudgeError::Allocation)
    ));
    assert!(matches!(
        ScalarKeyScratch::new(&context, usize::MAX, Some(super::store_fault)),
        Err(super::JudgeError::State(StoreError::Allocation))
    ));
    context.cancel();
    assert!(matches!(
        ScalarKeyScratch::new(&context, usize::MAX, Some(super::store_fault)),
        Err(super::JudgeError::Work(crate::WorkError::Cancelled))
    ));
    #[cfg(feature = "alloc-counter")]
    assert_eq!(crate::alloc_counter::snapshot().absolute.live_bytes, before);
    let _ = before;
}

#[test]
fn scalar_key_valid_growth_retains_no_good_groups() {
    let schema = theory();
    let parent = [(USER, user(999, "existing")), (USER, user(1000, "removed"))];
    for count in [1, 256] {
        let mut added: Vec<_> = (0..count)
            .map(|id| (USER, user(id, &format!("unique-{id}"))))
            .collect();
        // The producer may over-report a no-op Add, and a replacement may
        // reuse a removed key. Neither is a new final-state competitor.
        added.push(parent[0].clone());
        added.push((USER, user(1000, "replacement")));
        let mut state = DeltaState::new(&parent, &added, &parent[1..]);
        let expected = judge_final_state(&schema, &state, &work(), JudgeBudget::default()).unwrap();
        assert_eq!(expected, Judgment::Admitted);
        state.refuse_stream = true;
        state.row_visits.set(0);
        // Visits scale with added competitors, not the untouched parent.
        let context = work();
        let actual = judge_incremental(
            LawfulParent::established(),
            &schema,
            &state,
            &context,
            JudgeBudget::default(),
            JudgeScratch::disabled(),
        )
        .expect("lawful scalar groups require no retained good-key map");
        assert_eq!(actual, expected);
        assert_eq!(state.row_visits(), 0);
        assert_eq!(state.key_row_visits.get(), 2 * added.len() as u64);
    }
}

#[derive(Clone, Copy, Debug)]
enum GroupedFamily {
    ScalarContainment,
    PointwiseContainment,
    Capacity,
}

/// All scalar variants share one reordered determinant fixture. The complete
/// reference first verifies the parent is lawful; indexed runs later prohibit
/// full scans. Pointwise parents cover a source through two adjacent spans.
fn grouped_family_schema(family: GroupedFamily) -> Schema {
    let source = RelationId(0);
    let target = RelationId(1);
    let extras = [
        field("flag", ValueType::Bool),
        field("unsigned", ValueType::U64),
        field("signed", ValueType::I64),
        field("float", ValueType::F64),
        field("uuid", ValueType::Uuid),
        field("bytes", ValueType::FixedBytes { len: 3 }),
    ];
    let mut source_fields = vec![
        field("group", ValueType::String),
        field("id", ValueType::U64),
        field("partition", ValueType::String),
    ];
    source_fields.extend_from_slice(&extras);
    let mut target_fields = vec![
        field("partition", ValueType::String),
        field("group", ValueType::String),
    ];
    target_fields.extend_from_slice(&extras);
    let mut source_projection = vec![FieldId(2), FieldId(0)];
    source_projection.extend((3..9).map(FieldId));
    let mut target_projection: Vec<_> = (0..8).map(FieldId).collect();
    let pointwise = matches!(family, GroupedFamily::PointwiseContainment);
    if pointwise {
        let span = field(
            "span",
            ValueType::Interval {
                element: IntervalElement::U64,
            },
        );
        source_fields.push(span.clone());
        target_fields.push(span);
        source_projection.push(FieldId(9));
        target_projection.push(FieldId(8));
    }
    let source_side = side(source, &source_projection);
    let target_side = side(target, &target_projection);
    let statement = match family {
        GroupedFamily::Capacity => capacity(source_side, 0, Some(2), target_side),
        _ => containment(source_side, target_side),
    };
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "Source".into(),
                fields: source_fields,
                extension: None,
            },
            RelationDescriptor {
                name: "Target".into(),
                fields: target_fields,
                extension: None,
            },
        ],
        statements: vec![fd(target, &target_projection), statement],
    }
    .validate()
    .unwrap()
}

fn grouped_family_fixture(
    family: GroupedFamily,
    groups: u64,
    text_bytes: usize,
) -> (Schema, DeltaState) {
    let schema = grouped_family_schema(family);
    let source = RelationId(0);
    let target = RelationId(1);
    let pointwise = matches!(family, GroupedFamily::PointwiseContainment);
    let extra_values = [
        Value::Bool(true),
        Value::U64(u64::MAX),
        Value::I64(i64::MIN),
        Value::F64(crate::F64::NAN),
        Value::Uuid(crate::Uuid::from_bytes([0x5a; 16])),
        Value::FixedBytes(Box::new([1, 2, 3])),
    ];

    let mut parent = Vec::new();
    let mut added = Vec::new();
    let mut removed = Vec::new();
    // Logical partition order and canonical source row order disagree.
    // Target rank is also independent of spilled fingerprint key order.
    for group in (0..groups).rev() {
        let text = Value::String(format!("g-{group:03}-{}", "x".repeat(text_bytes)).into());
        let partition = Value::String(format!("p-{:03}", groups - group).into());
        let mut source_row = vec![text.clone(), Value::U64(0), partition.clone()];
        source_row.extend_from_slice(&extra_values);
        let mut target_row = vec![partition, text];
        target_row.extend_from_slice(&extra_values);
        if pointwise {
            source_row.push(Value::IntervalU64(Interval::new(0, 10).unwrap()));
            target_row.push(Value::IntervalU64(Interval::new(0, 4).unwrap()));
        }
        parent.push((source, source_row.clone()));
        parent.push((target, target_row.clone()));
        source_row[1] = Value::U64(1);
        added.push((source, source_row.clone()));
        match family {
            GroupedFamily::ScalarContainment => removed.push((target, target_row)),
            GroupedFamily::PointwiseContainment => {
                target_row[8] = Value::IntervalU64(Interval::new(4, 10).unwrap());
                parent.push((target, target_row.clone()));
                removed.push((target, target_row));
            }
            GroupedFamily::Capacity => {
                // Most affected groups remain valid. Two groups violate;
                // group zero has the last target rank AND the larger total.
                if group == 0 || group + 1 == groups {
                    source_row[1] = Value::U64(2);
                    added.push((source, source_row.clone()));
                }
                if group == 0 {
                    source_row[1] = Value::U64(3);
                    added.push((source, source_row));
                }
            }
        }
    }
    let budget = JudgeBudget {
        examples_per_statement: 0,
    };
    assert_eq!(
        judge_final_state(
            &schema,
            &DeltaState::new(&parent, &[], &[]),
            &work(),
            budget
        )
        .unwrap(),
        Judgment::Admitted,
    );
    (schema, DeltaState::new(&parent, &added, &removed))
}

#[test]
fn indexed_grouped_verdicts_and_canonical_citations_match_with_either_error_channel() {
    for family in [
        GroupedFamily::ScalarContainment,
        GroupedFamily::PointwiseContainment,
        GroupedFamily::Capacity,
    ] {
        let (schema, mut state) = grouped_family_fixture(family, 40, 512);
        schema.compiled_theory().unwrap();
        for examples_per_statement in [0, 2] {
            let budget = JudgeBudget {
                examples_per_statement,
            };
            state.refuse_stream = false;
            let expected = judge_final_state(&schema, &state, &work(), budget).unwrap();
            let Judgment::Rejected(violations) = &expected else {
                panic!("the fixture must violate its grouped statement");
            };
            assert_eq!(violations.len(), 1);
            assert_eq!(violations[0].statement, StatementId(1));
            assert_eq!(violations[0].examples.len(), examples_per_statement);
            assert!(violations[0].examples_truncated);
            if matches!(family, GroupedFamily::Capacity) {
                assert_eq!(violations[0].measure, Some(4));
            }
            state.refuse_stream = true;
            state.row_visits.set(0);
            for channel in [
                None,
                Some(
                    (|fault| panic!("unexpected scratch fault: {fault:?}"))
                        as fn(super::ScratchFault) -> std::convert::Infallible,
                ),
            ] {
                let context = work();
                let before = crate::alloc_counter::snapshot().absolute.live_bytes;
                let actual = judge_incremental(
                    LawfulParent::established(),
                    &schema,
                    &state,
                    &context,
                    budget,
                    JudgeScratch { channel },
                )
                .unwrap();
                assert_eq!(actual, expected, "{family:?}");
                drop(actual);
                #[cfg(feature = "alloc-counter")]
                assert_eq!(crate::alloc_counter::snapshot().absolute.live_bytes, before);
                let _ = before;
                assert_eq!(state.row_visits(), 0);
            }
        }
    }
}

#[test]
fn oversized_determinant_preserves_exact_judgment_without_a_hidden_allowance() {
    let (schema, state) = grouped_family_fixture(GroupedFamily::ScalarContainment, 1, 32_768);
    let budget = JudgeBudget {
        examples_per_statement: 0,
    };
    let expected = judge_final_state(&schema, &state, &work(), budget).unwrap();
    let state = state.refusing_streams();
    state.row_visits.set(0);
    let context = work();
    let actual = judge_incremental(
        LawfulParent::established(),
        &schema,
        &state,
        &context,
        budget,
        JudgeScratch::channel(|fault| panic!("unexpected scratch fault: {fault:?}")),
    )
    .unwrap();
    assert_eq!(actual, expected);
    assert_eq!(state.row_visits(), 0);
}

#[test]
fn scalar_key_bad_groups_probe_two_then_cite_once_in_canonical_order() {
    let schema = theory();
    let parent = [(USER, user(100, "z")), (USER, user(101, "a"))];
    // Reverse encounter order puts the smallest canonical citations last.
    // Many additions to each bad group must not repeat its full scan.
    let added: Vec<_> = (0..32)
        .rev()
        .map(|id| (USER, user(id, if id % 2 == 0 { "z" } else { "a" })))
        .collect();
    for examples_per_statement in [0, 1, 4, 34, 35] {
        let budget = JudgeBudget {
            examples_per_statement,
        };
        let state = DeltaState::new(&parent, &added, &[]);
        let expected = judge_final_state(&schema, &state, &work(), budget).unwrap();
        let indexed = DeltaState::new(&parent, &added, &[]).refusing_streams();
        let actual = judge_incremental(
            LawfulParent::established(),
            &schema,
            &indexed,
            &work(),
            budget,
            JudgeScratch::disabled(),
        )
        .unwrap();
        assert_eq!(actual, expected);
        let Judgment::Rejected(violations) = actual else {
            panic!("email keys violated")
        };
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].statement, USER_EMAIL_KEY);
        assert_eq!(violations[0].examples.len(), examples_per_statement.min(34));
        assert_eq!(
            violations[0].examples_truncated,
            examples_per_statement < 34
        );
        // Unique id probes: 32. Email probes: two per bad group, then all
        // 34 offending rows exactly once. No repeated full enumeration.
        assert_eq!(indexed.key_row_visits.get(), 32 + 4 + 34);
    }
}

#[test]
fn scalar_key_late_unindexed_group_discards_provisional_citations() {
    let schema = theory();
    let parent = [(USER, user(100, "z")), (USER, user(101, "a"))];
    let added = [
        (USER, user(9, "valid")),
        (USER, user(8, "z")),
        (USER, user(0, "a")),
    ];
    // Decline either the initial probe or the second (citation) visit,
    // after an earlier good group and provisional bad-group citations.
    for visits_before_decline in [0, 1] {
        for examples_per_statement in [0, 1, 4] {
            let mut state = DeltaState::new(&parent, &added, &[]);
            state.unindexed_key_group = Some((USER_EMAIL_KEY, vec![Value::String("a".into())]));
            state
                .indexed_matches_before_decline
                .set(visits_before_decline);
            assert_late_index_fallback(
                &schema,
                &state,
                JudgeBudget {
                    examples_per_statement,
                },
            );
        }
    }
}

#[test]
fn scalar_key_bad_group_state_releases_memory_on_success_and_mid_probe_cancellation() {
    let schema = theory();
    schema.compiled_theory().unwrap();
    let parent: Vec<_> = (0..128)
        .map(|id| (USER, user(id, &format!("group-{id:03}"))))
        .collect();
    let added: Vec<_> = (0..128)
        .map(|id| (USER, user(id + 128, &format!("group-{id:03}"))))
        .collect();
    let mut state = DeltaState::new(&parent, &added, &[]);
    let budget = JudgeBudget {
        examples_per_statement: 1,
    };
    let expected = judge_final_state(&schema, &state, &work(), budget).unwrap();
    assert!(matches!(expected, Judgment::Rejected(_)));
    for cancelled in [false, true] {
        let context = work();
        if cancelled {
            state.cancel_on_key_row = Some((state.key_row_visits.get() + 2, context.clone()));
        }
        let before = crate::alloc_counter::snapshot().absolute.live_bytes;
        let actual = judge_incremental(
            LawfulParent::established(),
            &schema,
            &state,
            &context,
            budget,
            JudgeScratch::disabled(),
        );
        if cancelled {
            assert!(matches!(
                actual,
                Err(super::JudgeError::Work(crate::WorkError::Cancelled))
            ));
        } else {
            assert_eq!(actual.unwrap(), expected);
        }
        #[cfg(feature = "alloc-counter")]
        assert_eq!(crate::alloc_counter::snapshot().absolute.live_bytes, before);
        let _ = before;
    }
}

#[test]
fn scalar_key_reordered_large_text_projection_matches_complete() {
    let schema = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Entry".into(),
            fields: vec![
                field("name", ValueType::String),
                field("id", ValueType::U64),
                field("group", ValueType::String),
            ],
        }],
        statements: vec![fd(USER, &[FieldId(2), FieldId(0)])],
    }
    .validate()
    .unwrap();
    let entry = |name: &str, id, group: &str| {
        (
            USER,
            vec![
                Value::String(name.into()),
                Value::U64(id),
                Value::String(group.into()),
            ],
        )
    };
    let parent = [entry("a", 0, "x"), entry("b", 1, "y")];
    let added = [entry("a", 2, "y"), entry("a", 3, "x"), entry("b", 4, "y")];
    let mut state = DeltaState::new(&parent, &added, &[]);
    state.keys = &[(StatementId(0), USER, &[2, 0])];
    let budget = JudgeBudget {
        examples_per_statement: 3,
    };
    let expected = judge_final_state(&schema, &state, &work(), budget).unwrap();
    assert!(matches!(expected, Judgment::Rejected(_)));
    state.refuse_stream = true;
    assert_eq!(
        judge_incremental(
            LawfulParent::established(),
            &schema,
            &state,
            &work(),
            budget,
            JudgeScratch::disabled(),
        )
        .unwrap(),
        expected,
    );

    let large = "wide".repeat(2048);
    let mut state = DeltaState::new(&[], &[entry(&large, 0, "x")], &[]).refusing_streams();
    state.keys = &[(StatementId(0), USER, &[2, 0])];
    let context = work();
    assert_eq!(
        judge_incremental(
            LawfulParent::established(),
            &schema,
            &state,
            &context,
            budget,
            JudgeScratch::channel(|fault| panic!("unexpected scratch fault: {fault:?}")),
        )
        .unwrap(),
        Judgment::Admitted
    );
    assert_eq!(
        state.key_row_visits.get(),
        1,
        "large determinant is probed exactly once"
    );
}

#[test]
fn scalar_key_second_competitor_cancellation_is_not_a_verdict() {
    let context = work();
    let mut state =
        DeltaState::new(&[(USER, user(1, "a"))], &[(USER, user(1, "b"))], &[]).refusing_streams();
    state.cancel_on_key_row = Some((2, context.clone()));
    assert!(matches!(
        judge_incremental(
            LawfulParent::established(),
            &theory(),
            &state,
            &context,
            JudgeBudget::default(),
            JudgeScratch::disabled(),
        ),
        Err(super::JudgeError::Work(crate::WorkError::Cancelled))
    ));
    assert_eq!(state.key_row_visits.get(), 2);
    assert_eq!(
        state.group_visits(),
        1,
        "no citation pass after cancellation"
    );
}

fn scalar_containment_theory() -> Schema {
    let source = RelationId(0);
    let target = RelationId(1);
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Source".into(),
                fields: vec![field("group", ValueType::U64), field("id", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Target".into(),
                fields: vec![
                    field("group", ValueType::U64),
                    field("id", ValueType::U64),
                    field("active", ValueType::Bool),
                ],
            },
        ],
        statements: vec![
            fd(target, &[FieldId(0)]),
            containment(
                side(source, &[FieldId(0)]),
                side_where(target, &[FieldId(0)], vec![(FieldId(2), Value::Bool(true))]),
            ),
        ],
    }
    .validate()
    .unwrap()
}

#[test]
fn scalar_containment_witness_skips_existing_source_fanout() {
    let source = RelationId(0);
    let target = RelationId(1);
    let schema = scalar_containment_theory();
    let mut parent: Vec<_> = (0..1024)
        .map(|id| (source, vec![Value::U64(0), Value::U64(id)]))
        .collect();
    // The unique group-zero target witnesses all existing sources. Other
    // targets belong to different groups, preserving the lawful parent.
    parent.extend((0..1024).map(|id| {
        (
            target,
            vec![Value::U64(id), Value::U64(id), Value::Bool(true)],
        )
    }));
    let added = [(source, vec![Value::U64(0), Value::U64(1024)])];
    let mut state = DeltaState::new(&parent, &added, &[]).refusing_streams();
    state.keys = &[(StatementId(0), RelationId(1), &[0])];
    let verdict = judge_incremental(
        LawfulParent::established(),
        &schema,
        &state,
        &work(),
        JudgeBudget::default(),
        JudgeScratch::disabled(),
    )
    .unwrap();
    assert_eq!(verdict, Judgment::Admitted);
    assert!(
        state.compiled_row_visits.get() <= 4,
        "existing fan-out must not be revisited after an existential witness: {} visits",
        state.compiled_row_visits.get()
    );

    // Replacing the selected group-zero target with an unselected target
    // must not be mistaken for an existential witness. The optimized path
    // must cite unsatisfied sources exactly as complete judgment does.
    let removed: Vec<_> = parent
        .iter()
        .filter(|(relation, row)| *relation == target && row[0] == Value::U64(0))
        .cloned()
        .collect();
    let mut replacement = added.to_vec();
    replacement.push((
        target,
        vec![Value::U64(0), Value::U64(0), Value::Bool(false)],
    ));
    let mut missing = DeltaState::new(&parent, &replacement, &removed);
    missing.keys = state.keys;
    assert!(matches!(
        assert_equivalent(&schema, &missing, JudgeBudget::default()),
        Judgment::Rejected(_)
    ));
}

#[test]
fn scalar_containment_late_unindexed_group_discards_provisional_citations() {
    let source = RelationId(0);
    let target = RelationId(1);
    let schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Source".into(),
                fields: vec![
                    field("group", ValueType::String),
                    field("id", ValueType::U64),
                    field("partition", ValueType::String),
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Target".into(),
                fields: vec![
                    field("partition", ValueType::String),
                    field("group", ValueType::String),
                    field("active", ValueType::Bool),
                ],
            },
        ],
        statements: vec![
            fd(target, &[FieldId(1), FieldId(0)]),
            containment(
                side(source, &[FieldId(2), FieldId(0)]),
                side_where(
                    target,
                    &[FieldId(0), FieldId(1)],
                    vec![(FieldId(2), Value::Bool(true))],
                ),
            ),
        ],
    }
    .validate()
    .unwrap();
    let group = |id| Value::String(format!("group-{id}-{}", "text".repeat(32)).into());
    let partition = |at| {
        Value::String(
            if at == 0 {
                "z-partition"
            } else {
                "a-partition"
            }
            .into(),
        )
    };
    let source_row = |at, id| (source, vec![group(at), Value::U64(id), partition(at)]);
    let target_row = |at| (target, vec![partition(at), group(at), Value::Bool(true)]);
    let parent = vec![
        source_row(0, 0),
        source_row(1, 0),
        target_row(0),
        target_row(1),
    ];
    // Logical partition order visits group one first, independent of delta order.
    // The later group zero has the canonically smallest offending facts.
    // Repeated groups, an over-reported duplicate Add, and removed targets
    // share logical (partition, group) keys despite differing physical order.
    let added = vec![
        source_row(1, 1),
        source_row(1, 2),
        source_row(0, 1),
        source_row(0, 1),
    ];
    let removed = parent[2..].to_vec();
    for unavailable_relation in [source, target] {
        for examples in [0, 1, 4] {
            let budget = JudgeBudget {
                examples_per_statement: examples,
            };
            let mut state = DeltaState::new(&parent, &added, &removed);
            state.keys = &[(StatementId(0), RelationId(1), &[1, 0])];
            let physical_group = vec![group(0), partition(0)];
            state.unindexed_group = Some((unavailable_relation, physical_group));
            assert_late_index_fallback(&schema, &state, budget);
        }
    }
}

fn assert_late_index_fallback(schema: &Schema, state: &DeltaState, budget: JudgeBudget) {
    let expected = judge_final_state(schema, state, &work(), budget).unwrap();
    state.row_visits.set(0);
    let context = work();
    let actual = judge_incremental(
        LawfulParent::established(),
        schema,
        state,
        &context,
        budget,
        JudgeScratch::disabled(),
    )
    .unwrap();
    assert_eq!(actual, expected);
    assert!(matches!(actual, Judgment::Rejected(_)));
    assert_eq!(
        state.declined_groups.get(),
        1,
        "late index fallback must be exercised"
    );
    assert!(
        state.row_visits() > 0,
        "the fallback must complete its relation scan"
    );
}

#[test]
fn pointwise_containment_late_unindexed_group_discards_provisional_citations() {
    let source = RelationId(0);
    let target = RelationId(1);
    let span_type = ValueType::Interval {
        element: IntervalElement::U64,
    };
    let schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Source".into(),
                fields: vec![
                    field("group", ValueType::U64),
                    field("id", ValueType::U64),
                    field("span", span_type),
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Target".into(),
                fields: vec![field("group", ValueType::U64), field("span", span_type)],
            },
        ],
        statements: vec![
            fd(target, &[FieldId(0), FieldId(1)]),
            containment(
                side(source, &[FieldId(0), FieldId(2)]),
                side(target, &[FieldId(0), FieldId(1)]),
            ),
        ],
    }
    .validate()
    .unwrap();
    let source_row = |group, id| {
        (
            source,
            vec![
                Value::U64(group),
                Value::U64(id),
                Value::IntervalU64(Interval::new(0, 10).unwrap()),
            ],
        )
    };
    let parent = vec![
        source_row(0, 0),
        source_row(1, 0),
        (target, booking(0, 0, 10)),
        (target, booking(1, 0, 10)),
    ];
    let added = vec![source_row(1, 1), source_row(0, 1)];
    let removed = parent[2..].to_vec();
    for unavailable_relation in [source, target] {
        for examples in [0, 1, 4] {
            let mut state = DeltaState::new(&parent, &added, &removed);
            state.keys = &[(StatementId(0), RelationId(1), &[0])];
            state.unindexed_group = Some((unavailable_relation, vec![Value::U64(1)]));
            assert_late_index_fallback(
                &schema,
                &state,
                JudgeBudget {
                    examples_per_statement: examples,
                },
            );
        }
    }
}

#[test]
fn capacity_admission_remains_index_only() {
    let schema = theory();
    let parent = [(ROOM, room(0)), (BOOKING, booking(0, 0, 1))];
    let added = [(BOOKING, booking(0, 1, 2))];
    let state = DeltaState::new(&parent, &added, &[]).refusing_streams();
    let verdict = judge_incremental(
        LawfulParent::established(),
        &schema,
        &state,
        &work(),
        JudgeBudget::default(),
        JudgeScratch::disabled(),
    )
    .unwrap();
    assert_eq!(verdict, Judgment::Admitted);
    assert!(state.group_visits() > 0);
    assert_eq!(state.row_visits(), 0);
}

#[test]
fn capacity_measure_uses_global_rank_without_full_scans_for_equal_or_differing_totals() {
    let schema = theory();
    let parent = [
        (ROOM, room(1)),
        (BOOKING, booking(0, 0, 1)),
        (ROOM, room(0)),
        (BOOKING, booking(1, 0, 1)),
    ];
    let mut added = vec![
        (BOOKING, booking(0, 1, 2)),
        (BOOKING, booking(0, 2, 3)),
        (BOOKING, booking(1, 1, 2)),
        (BOOKING, booking(1, 2, 3)),
    ];
    for differing in [false, true] {
        if differing {
            added.push((BOOKING, booking(1, 3, 4)));
        }
        for examples in [0, 1, 4] {
            let budget = JudgeBudget {
                examples_per_statement: examples,
            };
            let state = DeltaState::new(&parent, &added, &[]);
            let expected = judge_final_state(&schema, &state, &work(), budget).unwrap();
            let Judgment::Rejected(violations) = &expected else {
                panic!("capacity violated")
            };
            assert_eq!(violations.len(), 1);
            assert_eq!(
                violations[0].measure,
                Some(3),
                "room0 has the last target rank, not the largest total"
            );
            for shuffled in [false, true] {
                let mut indexed = DeltaState::new(&parent, &added, &[]);
                if shuffled {
                    let length = indexed.rows.len();
                    let mut full: Vec<usize> = (0..length).rev().collect();
                    full.rotate_left(3);
                    let mut group: Vec<usize> = (0..length).collect();
                    group.rotate_left(1);
                    indexed.ranked_order = Some(RankedOrder {
                        ranks: (0..length).map(|index| 7 + index as u64 * 101).collect(),
                        full,
                        group,
                    });
                    assert_eq!(
                        judge_final_state(&schema, &indexed, &work(), budget).unwrap(),
                        expected
                    );
                }
                indexed.row_visits.set(0);
                let indexed = indexed.refusing_streams();
                let context = work();
                let actual = judge_incremental(
                    LawfulParent::established(),
                    &schema,
                    &indexed,
                    &context,
                    budget,
                    JudgeScratch::disabled(),
                )
                .unwrap();
                assert_eq!(actual, expected);
                assert!(indexed.group_visits() > 0);
                assert_eq!(indexed.row_visits(), 0);
            }
            // A provider may implement the older exact group capability
            // without relation-wide ranks. Its source index still runs,
            // but the target must decline to the complete affected walk.
            let mut unranked = DeltaState::new(&parent, &added, &[]);
            unranked.ranked_available = false;
            let context = work();
            let actual = judge_incremental(
                LawfulParent::established(),
                &schema,
                &unranked,
                &context,
                budget,
                JudgeScratch::disabled(),
            )
            .unwrap();
            assert_eq!(actual, expected);
            assert!(unranked.group_visits() > 0);
            assert!(unranked.row_visits() > 0);
        }
    }
}

#[test]
fn capacity_late_unindexed_group_discards_provisional_citations_and_measure() {
    let source = RelationId(0);
    let target = RelationId(1);
    let schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Source".into(),
                fields: vec![field("group", ValueType::U64), field("id", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Target".into(),
                fields: vec![field("group", ValueType::U64)],
            },
        ],
        statements: vec![
            fd(target, &[FieldId(0)]),
            capacity(
                side(source, &[FieldId(0)]),
                0,
                Some(1),
                side(target, &[FieldId(0)]),
            ),
        ],
    }
    .validate()
    .unwrap();
    let source_row = |group, id| (source, vec![Value::U64(group), Value::U64(id)]);
    let parent = vec![
        source_row(0, 0),
        source_row(1, 0),
        (target, room(0)),
        (target, room(1)),
    ];
    let added = vec![source_row(1, 1), source_row(1, 2), source_row(0, 1)];
    // Unequal totals also ensure fallback replaces the indexed pass's
    // last measure with the complete target traversal's exact measure.
    // Target decline follows the first group's target evidence. Source
    // decline must wait until the second visit: the first computes totals,
    // and the second cites sources after both target violations are known.
    for (unavailable_relation, permitted_visits) in [(target, 0), (source, 1)] {
        for examples in [0, 1, 4] {
            let mut state = DeltaState::new(&parent, &added, &[]);
            state.keys = &[(StatementId(0), RelationId(1), &[0])];
            state.unindexed_group = Some((unavailable_relation, vec![Value::U64(1)]));
            state.indexed_matches_before_decline.set(permitted_visits);
            assert_late_index_fallback(
                &schema,
                &state,
                JudgeBudget {
                    examples_per_statement: examples,
                },
            );
            assert_eq!(state.indexed_matches_before_decline.get(), 0);
        }
    }
}

// The affected key is the sole retained determinant representation. Exercise
// its borrowed logical-coordinate decoder in both physical tiers, including
// oversized exact keys, another map spilling inside the callback, and every
// visitor exit. These are mechanism checks, not a second relation oracle.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "One fixture preserves the exact key oracle across nested spill, visitor failure, cancellation, and cleanup"
)]
fn affected_determinants_decode_reordered_oversized_keys_and_release_scratch() {
    use super::grouped::{GroupedMap, ScalarKeyScratch};
    use std::convert::Infallible;

    let fields = [
        field("group", ValueType::String),
        field("id", ValueType::U64),
        field("partition", ValueType::String),
    ];
    let projection = [FieldId(2), FieldId(0)];
    let rows: Vec<_> = (0..32)
        .map(|id| {
            vec![
                Value::String(format!("{id:02}-{}", "payload".repeat(80)).into()),
                Value::U64(id),
                Value::String("partition".into()),
            ]
        })
        .collect();
    let expected: Vec<_> = rows
        .iter()
        .map(|row| vec![row[2].clone(), row[0].clone()])
        .collect();
    for spill in [false, true] {
        let context = work();
        let channel: Option<fn(super::ScratchFault) -> Infallible> =
            Some(|fault| panic!("unexpected scratch fault: {fault:?}"));
        let mut affected = GroupedMap::new(&context, channel);
        {
            let mut key = ScalarKeyScratch::new(&context, 0, channel).unwrap();
            for row in rows.iter().rev() {
                let key = key.encode_projection(row, &projection).unwrap();
                assert!(affected.insert_if_absent(key).unwrap());
                assert!(!affected.insert_if_absent(key).unwrap());
            }
        }
        if spill {
            affected.force_spill().unwrap();
        }
        let path = affected.scratch_path();
        assert_eq!(path.is_some(), spill);
        let mut mirror = GroupedMap::new(&context, channel);
        let mut actual = Vec::new();
        affected
            .for_each_determinant(&fields, &projection, &context, |key, determinant| {
                assert!(mirror.insert_if_absent(key)?);
                if spill && mirror.len() == 1 {
                    mirror.force_spill()?;
                }
                actual.push(determinant.to_vec());
                Ok(true)
            })
            .unwrap();
        assert_eq!(actual.len(), expected.len());
        assert!(expected.iter().all(|row| actual.contains(row)));
        assert_eq!(mirror.len(), affected.len());
        drop(mirror);

        let baseline = crate::alloc_counter::snapshot().absolute.live_bytes;
        let error = super::JudgeError::UndefinedDuration {
            statement: StatementId(91),
        };
        assert_eq!(
            affected.for_each_determinant(&fields, &projection, &context, |_, _| {
                Err(super::JudgeError::UndefinedDuration {
                    statement: StatementId(91),
                })
            }),
            Err(error),
        );
        #[cfg(feature = "alloc-counter")]
        assert_eq!(
            crate::alloc_counter::snapshot().absolute.live_bytes,
            baseline
        );
        let mut visits = 0;
        affected
            .for_each_determinant(&fields, &projection, &context, |_, _| {
                visits += 1;
                Ok(false)
            })
            .unwrap();
        assert_eq!(visits, 1);
        #[cfg(feature = "alloc-counter")]
        assert_eq!(
            crate::alloc_counter::snapshot().absolute.live_bytes,
            baseline
        );

        let mut visits = 0;
        assert_eq!(
            affected.for_each_determinant(&fields, &projection, &context, |_, _| {
                visits += 1;
                context.cancel();
                Ok(true)
            }),
            Err(super::JudgeError::Work(crate::WorkError::Cancelled)),
        );
        assert_eq!(visits, 1);
        #[cfg(feature = "alloc-counter")]
        assert_eq!(
            crate::alloc_counter::snapshot().absolute.live_bytes,
            baseline
        );
        drop(affected);
        assert!(path.as_ref().is_none_or(|path| !path.exists()));
        let _ = baseline;
    }
}

#[test]
fn empty_scalar_projection_is_one_valid_determinant_not_an_empty_set() {
    let context = work();
    let channel: Option<fn(super::ScratchFault) -> std::convert::Infallible> =
        Some(|fault| panic!("unexpected scratch fault: {fault:?}"));
    let fields = [field("group", ValueType::String)];
    for disk in [false, true] {
        let mut empty = super::grouped::GroupedMap::new(&context, channel);
        assert!(empty.insert_if_absent(&[]).unwrap());
        if disk {
            empty.force_spill().unwrap();
        }
        let mut visits = 0;
        empty
            .for_each_determinant(&fields, &[], &context, |key, values| {
                visits += 1;
                assert!(key.is_empty());
                assert!(values.is_empty());
                Ok(true)
            })
            .unwrap();
        assert_eq!(visits, 1);
    }
}
