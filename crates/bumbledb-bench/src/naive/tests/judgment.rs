//! Judgment goldens: the engine's commit fixtures — the pointwise-key
//! matrix, the source-side judgment, and the target-side judgment —
//! re-expressed against the naive model, table-driven. Verdict
//! and violator must match the hand-computed expectation; these cases
//! double as the engine-agreement seed corpus.

use bumbledb::schema::{
    FieldDescriptor, FieldId, Generation, IntervalElement, RelationDescriptor, SchemaDescriptor,
    Side, StatementDescriptor, ValueType,
};
use bumbledb::{Direction, RelationId, StatementId, Value};

use crate::fixture::{field, side};
use crate::naive::{Delta, NaiveDb, Violation};

fn interval() -> ValueType {
    ValueType::Interval {
        element: IntervalElement::U64,
    }
}

fn selected(relation: RelationId, projection: &[u16], field: u16, literal: bool) -> Side {
    Side {
        relation,
        projection: projection.iter().map(|&f| FieldId(f)).collect(),
        selection: Box::new([(FieldId(field), Value::Bool(literal))]),
    }
}

fn functionality(statement: u16) -> Violation {
    Violation::Functionality {
        statement: StatementId(statement),
    }
}

fn source_unsatisfied(statement: u16) -> Violation {
    Violation::Containment {
        statement: StatementId(statement),
        direction: Direction::SourceUnsatisfied,
    }
}

fn target_required(statement: u16) -> Violation {
    Violation::Containment {
        statement: StatementId(statement),
        direction: Direction::TargetRequired,
    }
}

type Facts = Vec<(RelationId, Vec<Value>)>;

struct Case {
    name: &'static str,
    base: Facts,
    deletes: Facts,
    inserts: Facts,
    verdict: Result<(), Violation>,
}

fn run(schema: &SchemaDescriptor, cases: Vec<Case>) {
    for case in cases {
        let mut db = NaiveDb::new(schema);
        db.apply(&Delta {
            deletes: vec![],
            inserts: case.base.clone(),
        })
        .unwrap_or_else(|violation| panic!("{}: base commit refused: {violation:?}", case.name));
        let before = db.clone();
        let got = db.apply(&Delta {
            deletes: case.deletes.clone(),
            inserts: case.inserts.clone(),
        });
        assert_eq!(got, case.verdict, "{}", case.name);
        if got.is_err() {
            assert_eq!(db, before, "{}: an abort must not apply", case.name);
        }
    }
}

// ---------- functionality — the pointwise-key matrix ----------
//
// The engine fixture: Target(id fresh) + Keyed(x, y; key x) +
// Booking(room, during, tag; key (room, during)) + Claim(holder) <=
// Target(id). Materialized order: Target's fresh auto-key first.

const TARGET: RelationId = RelationId(0);
const KEYED: RelationId = RelationId(1);
const BOOKING: RelationId = RelationId(2);
const CLAIM: RelationId = RelationId(3);
const KEYED_KEY: u16 = 1;
const BOOKING_KEY: u16 = 2;
const CLAIM_TARGET: u16 = 3;

fn matrix_schema() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Target".into(),
                fields: vec![FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                    generation: Generation::Fresh,
                }],
            },
            RelationDescriptor {
                extension: None,
                name: "Keyed".into(),
                fields: vec![field("x", ValueType::U64), field("y", ValueType::I64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Booking".into(),
                fields: vec![
                    field("room", ValueType::U64),
                    field("during", interval()),
                    field("tag", ValueType::U64),
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Claim".into(),
                fields: vec![field("holder", ValueType::U64)],
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: KEYED,
                projection: Box::new([FieldId(0)]),
            },
            StatementDescriptor::Functionality {
                relation: BOOKING,
                projection: Box::new([FieldId(0), FieldId(1)]),
            },
            StatementDescriptor::Containment {
                source: side(CLAIM, &[0], &[]),
                target: side(TARGET, &[0], &[]),
            },
        ],
    }
}

fn booking(room: u64, start: u64, end: u64, tag: u64) -> (RelationId, Vec<Value>) {
    (
        BOOKING,
        vec![
            Value::U64(room),
            Value::IntervalU64(start, end),
            Value::U64(tag),
        ],
    )
}

fn keyed(x: u64, y: i64) -> (RelationId, Vec<Value>) {
    (KEYED, vec![Value::U64(x), Value::I64(y)])
}

/// Each pointwise cell twice: both facts in one delta, and the contender
/// against a committed incumbent — the model judges final states, so both
/// forms reduce to the same brute-force test.
fn matrix_cell(
    name_in: &'static str,
    name_cross: &'static str,
    contender: (RelationId, Vec<Value>),
    verdict: Result<(), Violation>,
) -> Vec<Case> {
    let incumbent = booking(1, 10, 20, 0);
    vec![
        Case {
            name: name_in,
            base: vec![],
            deletes: vec![],
            inserts: vec![incumbent.clone(), contender.clone()],
            verdict,
        },
        Case {
            name: name_cross,
            base: vec![incumbent],
            deletes: vec![],
            inserts: vec![contender],
            verdict,
        },
    ]
}

#[test]
fn pointwise_key_matrix() {
    let mut cases = Vec::new();
    cases.extend(matrix_cell(
        "overlap left in delta",
        "overlap left cross delta",
        booking(1, 5, 15, 1),
        Err(functionality(BOOKING_KEY)),
    ));
    cases.extend(matrix_cell(
        "overlap right in delta",
        "overlap right cross delta",
        booking(1, 15, 25, 1),
        Err(functionality(BOOKING_KEY)),
    ));
    cases.extend(matrix_cell(
        "containment in delta",
        "containment cross delta",
        booking(1, 12, 18, 1),
        Err(functionality(BOOKING_KEY)),
    ));
    cases.extend(matrix_cell(
        "exact duplicate interval in delta",
        "exact duplicate interval cross delta",
        booking(1, 10, 20, 1),
        Err(functionality(BOOKING_KEY)),
    ));
    cases.extend(matrix_cell(
        "adjacent left in delta",
        "adjacent left cross delta",
        booking(1, 5, 10, 1),
        Ok(()),
    ));
    cases.extend(matrix_cell(
        "adjacent right in delta",
        "adjacent right cross delta",
        booking(1, 20, 25, 1),
        Ok(()),
    ));
    cases.extend(matrix_cell(
        "disjoint in delta",
        "disjoint cross delta",
        booking(1, 30, 40, 1),
        Ok(()),
    ));
    cases.extend(matrix_cell(
        "same interval different prefix in delta",
        "same interval different prefix cross delta",
        booking(2, 10, 20, 1),
        Ok(()),
    ));
    cases.push(Case {
        name: "delete then reinsert overlapping in one delta",
        base: vec![booking(1, 10, 20, 0)],
        deletes: vec![booking(1, 10, 20, 0)],
        inserts: vec![booking(1, 15, 25, 1)],
        verdict: Ok(()),
    });
    cases.push(Case {
        name: "two open-ended intervals in one group abort",
        base: vec![booking(1, 5, u64::MAX, 0)],
        deletes: vec![],
        inserts: vec![booking(1, 9, u64::MAX, 1)],
        verdict: Err(functionality(BOOKING_KEY)),
    });
    cases.push(Case {
        name: "bounded interval adjacent to open-ended passes",
        base: vec![booking(1, 5, 9, 0)],
        deletes: vec![],
        inserts: vec![booking(1, 9, u64::MAX, 1)],
        verdict: Ok(()),
    });
    run(&matrix_schema(), cases);
}

#[test]
fn scalar_key_conflicts() {
    run(
        &matrix_schema(),
        vec![
            Case {
                name: "scalar key conflict in one delta",
                base: vec![],
                deletes: vec![],
                inserts: vec![keyed(1, 10), keyed(1, 20)],
                verdict: Err(functionality(KEYED_KEY)),
            },
            Case {
                name: "scalar key conflict across deltas",
                base: vec![keyed(1, 10)],
                deletes: vec![],
                inserts: vec![keyed(1, 20)],
                verdict: Err(functionality(KEYED_KEY)),
            },
            Case {
                name: "distinct scalar keys coexist",
                base: vec![keyed(1, 10)],
                deletes: vec![],
                inserts: vec![keyed(2, 10)],
                verdict: Ok(()),
            },
            Case {
                name: "claim without target aborts",
                base: vec![],
                deletes: vec![],
                inserts: vec![(CLAIM, vec![Value::U64(5)])],
                verdict: Err(source_unsatisfied(CLAIM_TARGET)),
            },
            Case {
                name: "deleting a claimed target aborts",
                base: vec![(TARGET, vec![Value::U64(5)]), (CLAIM, vec![Value::U64(5)])],
                deletes: vec![(TARGET, vec![Value::U64(5)])],
                inserts: vec![],
                verdict: Err(target_required(CLAIM_TARGET)),
            },
        ],
    );
}

// ---------- containment, source side ----------
//
// The engine's judgment fixture: Parent == Child (lowered to TOTALITY and
// ARM), Transfer(account) <= Account(id | active == true), Session <=
// Shift (unselected coverage), Rest <= Shift(… | rested == true)
// (selected coverage), Report(subject | urgent == true) <= Account(id).

mod source_side {
    use super::{
        Case, FieldId, RelationDescriptor, RelationId, SchemaDescriptor, StatementDescriptor,
        Value, ValueType, field, interval, run, selected, side, source_unsatisfied,
    };

    const PARENT: RelationId = RelationId(0);
    const CHILD: RelationId = RelationId(1);
    const ACCOUNT: RelationId = RelationId(2);
    const TRANSFER: RelationId = RelationId(3);
    const SHIFT: RelationId = RelationId(4);
    const SESSION: RelationId = RelationId(5);
    const REST: RelationId = RelationId(6);
    const REPORT: RelationId = RelationId(7);

    const TOTALITY: u16 = 4;
    const ARM: u16 = 5;
    const TRANSFER_ACCOUNT: u16 = 6;
    const SESSION_COVER: u16 = 7;
    const REST_COVER: u16 = 8;
    const REPORT_ACCOUNT: u16 = 9;

    fn schema() -> SchemaDescriptor {
        SchemaDescriptor {
            relations: vec![
                RelationDescriptor {
                    extension: None,
                    name: "Parent".into(),
                    fields: vec![field("id", ValueType::U64)],
                },
                RelationDescriptor {
                    extension: None,
                    name: "Child".into(),
                    fields: vec![field("parent", ValueType::U64)],
                },
                RelationDescriptor {
                    extension: None,
                    name: "Account".into(),
                    fields: vec![
                        field("id", ValueType::U64),
                        field("active", ValueType::Bool),
                    ],
                },
                RelationDescriptor {
                    extension: None,
                    name: "Transfer".into(),
                    fields: vec![field("account", ValueType::U64)],
                },
                RelationDescriptor {
                    extension: None,
                    name: "Shift".into(),
                    fields: vec![
                        field("worker", ValueType::U64),
                        field("span", interval()),
                        field("rested", ValueType::Bool),
                    ],
                },
                RelationDescriptor {
                    extension: None,
                    name: "Session".into(),
                    fields: vec![field("worker", ValueType::U64), field("span", interval())],
                },
                RelationDescriptor {
                    extension: None,
                    name: "Rest".into(),
                    fields: vec![field("worker", ValueType::U64), field("span", interval())],
                },
                RelationDescriptor {
                    extension: None,
                    name: "Report".into(),
                    fields: vec![
                        field("subject", ValueType::U64),
                        field("urgent", ValueType::Bool),
                    ],
                },
            ],
            statements: vec![
                StatementDescriptor::Functionality {
                    relation: PARENT,
                    projection: Box::new([FieldId(0)]),
                },
                StatementDescriptor::Functionality {
                    relation: CHILD,
                    projection: Box::new([FieldId(0)]),
                },
                StatementDescriptor::Functionality {
                    relation: ACCOUNT,
                    projection: Box::new([FieldId(0)]),
                },
                StatementDescriptor::Functionality {
                    relation: SHIFT,
                    projection: Box::new([FieldId(0), FieldId(1)]),
                },
                StatementDescriptor::Containment {
                    source: side(PARENT, &[0], &[]),
                    target: side(CHILD, &[0], &[]),
                },
                StatementDescriptor::Containment {
                    source: side(CHILD, &[0], &[]),
                    target: side(PARENT, &[0], &[]),
                },
                StatementDescriptor::Containment {
                    source: side(TRANSFER, &[0], &[]),
                    target: selected(ACCOUNT, &[0], 1, true),
                },
                StatementDescriptor::Containment {
                    source: side(SESSION, &[0, 1], &[]),
                    target: side(SHIFT, &[0, 1], &[]),
                },
                StatementDescriptor::Containment {
                    source: side(REST, &[0, 1], &[]),
                    target: selected(SHIFT, &[0, 1], 2, true),
                },
                StatementDescriptor::Containment {
                    source: selected(REPORT, &[0], 1, true),
                    target: side(ACCOUNT, &[0], &[]),
                },
            ],
        }
    }

    fn account(id: u64, active: bool) -> (RelationId, Vec<Value>) {
        (ACCOUNT, vec![Value::U64(id), Value::Bool(active)])
    }

    fn shift(worker: u64, start: u64, end: u64, rested: bool) -> (RelationId, Vec<Value>) {
        (
            SHIFT,
            vec![
                Value::U64(worker),
                Value::IntervalU64(start, end),
                Value::Bool(rested),
            ],
        )
    }

    fn span(rel: RelationId, worker: u64, start: u64, end: u64) -> (RelationId, Vec<Value>) {
        (
            rel,
            vec![Value::U64(worker), Value::IntervalU64(start, end)],
        )
    }

    #[test]
    fn scalar_and_conditional_sources() {
        run(
            &schema(),
            vec![
                Case {
                    name: "scalar source without target aborts",
                    base: vec![],
                    deletes: vec![],
                    inserts: vec![(TRANSFER, vec![Value::U64(9)])],
                    verdict: Err(source_unsatisfied(TRANSFER_ACCOUNT)),
                },
                Case {
                    name: "scalar target and source in one delta commit",
                    base: vec![],
                    deletes: vec![],
                    inserts: vec![account(9, true), (TRANSFER, vec![Value::U64(9)])],
                    verdict: Ok(()),
                },
                Case {
                    name: "scalar source with pre-committed target commits",
                    base: vec![account(9, true)],
                    deletes: vec![],
                    inserts: vec![(TRANSFER, vec![Value::U64(9)])],
                    verdict: Ok(()),
                },
                Case {
                    name: "scalar target failing the target selection aborts",
                    base: vec![],
                    deletes: vec![],
                    inserts: vec![account(9, false), (TRANSFER, vec![Value::U64(9)])],
                    verdict: Err(source_unsatisfied(TRANSFER_ACCOUNT)),
                },
                Case {
                    name: "out-of-sigma source commits without a target",
                    base: vec![],
                    deletes: vec![],
                    inserts: vec![(REPORT, vec![Value::U64(5), Value::Bool(false)])],
                    verdict: Ok(()),
                },
                Case {
                    name: "in-sigma source without a target aborts",
                    base: vec![],
                    deletes: vec![],
                    inserts: vec![(REPORT, vec![Value::U64(5), Value::Bool(true)])],
                    verdict: Err(source_unsatisfied(REPORT_ACCOUNT)),
                },
                Case {
                    name: "in-sigma source with its target commits",
                    base: vec![account(5, true)],
                    deletes: vec![],
                    inserts: vec![(REPORT, vec![Value::U64(5), Value::Bool(true)])],
                    verdict: Ok(()),
                },
            ],
        );
    }

    #[test]
    fn coverage_walk() {
        run(
            &schema(),
            vec![
                Case {
                    name: "exact single segment covers",
                    base: vec![],
                    deletes: vec![],
                    inserts: vec![shift(1, 10, 20, false), span(SESSION, 1, 10, 20)],
                    verdict: Ok(()),
                },
                Case {
                    name: "abutting chain covers",
                    base: vec![shift(1, 10, 15, false), shift(1, 15, 20, false)],
                    deletes: vec![],
                    inserts: vec![span(SESSION, 1, 10, 20)],
                    verdict: Ok(()),
                },
                Case {
                    name: "entry segment overhang covers",
                    base: vec![shift(1, 5, 25, false)],
                    deletes: vec![],
                    inserts: vec![span(SESSION, 1, 10, 20)],
                    verdict: Ok(()),
                },
                Case {
                    name: "interior gap aborts",
                    base: vec![shift(1, 10, 14, false), shift(1, 15, 20, false)],
                    deletes: vec![],
                    inserts: vec![span(SESSION, 1, 10, 20)],
                    verdict: Err(source_unsatisfied(SESSION_COVER)),
                },
                Case {
                    name: "source start before first segment aborts",
                    base: vec![shift(1, 12, 20, false)],
                    deletes: vec![],
                    inserts: vec![span(SESSION, 1, 10, 20)],
                    verdict: Err(source_unsatisfied(SESSION_COVER)),
                },
                Case {
                    name: "source end past last segment aborts",
                    base: vec![shift(1, 10, 18, false)],
                    deletes: vec![],
                    inserts: vec![span(SESSION, 1, 10, 20)],
                    verdict: Err(source_unsatisfied(SESSION_COVER)),
                },
                Case {
                    name: "ray target covers a bounded source",
                    base: vec![shift(1, 10, u64::MAX, false)],
                    deletes: vec![],
                    inserts: vec![span(SESSION, 1, 15, 1000)],
                    verdict: Ok(()),
                },
                Case {
                    name: "ray source not covered by bounded targets",
                    base: vec![shift(1, 10, 1_000_000, false)],
                    deletes: vec![],
                    inserts: vec![span(SESSION, 1, 15, u64::MAX)],
                    verdict: Err(source_unsatisfied(SESSION_COVER)),
                },
                Case {
                    name: "ray source covered by a ray target",
                    base: vec![shift(1, 10, u64::MAX, false)],
                    deletes: vec![],
                    inserts: vec![span(SESSION, 1, 15, u64::MAX)],
                    verdict: Ok(()),
                },
                Case {
                    name: "another prefix group does not cover",
                    base: vec![shift(2, 10, 20, false)],
                    deletes: vec![],
                    inserts: vec![span(SESSION, 1, 10, 20)],
                    verdict: Err(source_unsatisfied(SESSION_COVER)),
                },
                Case {
                    name: "selected chain inside sigma commits",
                    base: vec![shift(1, 10, 15, true), shift(1, 15, 20, true)],
                    deletes: vec![],
                    inserts: vec![span(REST, 1, 10, 20)],
                    verdict: Ok(()),
                },
                Case {
                    name: "entry segment failing sigma aborts",
                    base: vec![shift(1, 10, 20, false)],
                    deletes: vec![],
                    inserts: vec![span(REST, 1, 10, 20)],
                    verdict: Err(source_unsatisfied(REST_COVER)),
                },
                Case {
                    name: "mid-chain segment failing sigma aborts",
                    base: vec![shift(1, 10, 15, true), shift(1, 15, 20, false)],
                    deletes: vec![],
                    inserts: vec![span(REST, 1, 10, 20)],
                    verdict: Err(source_unsatisfied(REST_COVER)),
                },
            ],
        );
    }

    /// The merged-union rule, isolated: a target relation carrying NO key
    /// at all holds overlapping segments [10,17) and [14,20), and the
    /// model must still judge [10,20) covered — it collects, sorts, and
    /// merges every matching segment rather than assuming the engine's
    /// acceptance gate kept the target disjoint.
    #[test]
    fn overlapping_target_segments_cover_jointly() {
        let schema = SchemaDescriptor {
            relations: vec![
                RelationDescriptor {
                    extension: None,
                    name: "Cover".into(),
                    fields: vec![field("who", ValueType::U64), field("span", interval())],
                },
                RelationDescriptor {
                    extension: None,
                    name: "Need".into(),
                    fields: vec![field("who", ValueType::U64), field("span", interval())],
                },
            ],
            statements: vec![StatementDescriptor::Containment {
                source: side(RelationId(1), &[0, 1], &[]),
                target: side(RelationId(0), &[0, 1], &[]),
            }],
        };
        run(
            &schema,
            vec![
                Case {
                    name: "overlapping segments cover jointly",
                    base: vec![
                        span(RelationId(0), 1, 10, 17),
                        span(RelationId(0), 1, 14, 20),
                    ],
                    deletes: vec![],
                    inserts: vec![span(RelationId(1), 1, 10, 20)],
                    verdict: Ok(()),
                },
                Case {
                    name: "overlapping segments still leave a gap visible",
                    base: vec![
                        span(RelationId(0), 1, 10, 17),
                        span(RelationId(0), 1, 14, 20),
                        span(RelationId(0), 1, 25, 30),
                    ],
                    deletes: vec![],
                    inserts: vec![span(RelationId(1), 1, 10, 26)],
                    verdict: Err(source_unsatisfied(0)),
                },
            ],
        );
    }

    #[test]
    fn equality_pair_on_insert() {
        run(
            &schema(),
            vec![
                Case {
                    name: "parent alone aborts on the totality statement",
                    base: vec![],
                    deletes: vec![],
                    inserts: vec![(PARENT, vec![Value::U64(1)])],
                    verdict: Err(source_unsatisfied(TOTALITY)),
                },
                Case {
                    name: "child alone aborts on the arm statement",
                    base: vec![],
                    deletes: vec![],
                    inserts: vec![(CHILD, vec![Value::U64(1)])],
                    verdict: Err(source_unsatisfied(ARM)),
                },
                Case {
                    name: "parent and child in one delta commit",
                    base: vec![],
                    deletes: vec![],
                    inserts: vec![(PARENT, vec![Value::U64(1)]), (CHILD, vec![Value::U64(1)])],
                    verdict: Ok(()),
                },
            ],
        );
    }
}

// ---------- containment, target side ----------
//
// The engine's target fixture: two scalar containments sharing one target
// key, coverage over a pointwise key, the == pair on delete, and
// ψ-qualified re-establishment in both forms.

mod target_side {
    use super::{
        Case, FieldId, RelationDescriptor, RelationId, SchemaDescriptor, StatementDescriptor,
        Value, ValueType, field, interval, run, selected, side, target_required,
    };

    const TARGET2: RelationId = RelationId(0);
    const CLAIM_A: RelationId = RelationId(1);
    const CLAIM_B: RelationId = RelationId(2);
    const SHIFT: RelationId = RelationId(3);
    const SESSION: RelationId = RelationId(4);
    const PARENT: RelationId = RelationId(5);
    const CHILD: RelationId = RelationId(6);
    const ACCOUNT: RelationId = RelationId(7);
    const TRANSFER: RelationId = RelationId(8);
    const ROSTER: RelationId = RelationId(9);
    const REST: RelationId = RelationId(10);

    const CLAIM_A_TARGET: u16 = 4;
    const CLAIM_B_TARGET: u16 = 5;
    const SESSION_COVER: u16 = 6;
    const TOTALITY: u16 = 7;
    const ARM: u16 = 8;
    const TRANSFER_ACCOUNT: u16 = 11;
    const REST_COVER: u16 = 12;

    #[expect(
        clippy::too_many_lines,
        reason = "the linear table or protocol is clearer kept together"
    )] // one fixture: eleven relations, thirteen statements
    fn schema() -> SchemaDescriptor {
        SchemaDescriptor {
            relations: vec![
                RelationDescriptor {
                    extension: None,
                    name: "Target".into(),
                    fields: vec![field("id", ValueType::U64), field("note", ValueType::U64)],
                },
                RelationDescriptor {
                    extension: None,
                    name: "ClaimA".into(),
                    fields: vec![field("t", ValueType::U64)],
                },
                RelationDescriptor {
                    extension: None,
                    name: "ClaimB".into(),
                    fields: vec![field("t", ValueType::U64)],
                },
                RelationDescriptor {
                    extension: None,
                    name: "Shift".into(),
                    fields: vec![field("worker", ValueType::U64), field("span", interval())],
                },
                RelationDescriptor {
                    extension: None,
                    name: "Session".into(),
                    fields: vec![field("worker", ValueType::U64), field("span", interval())],
                },
                RelationDescriptor {
                    extension: None,
                    name: "Parent".into(),
                    fields: vec![field("id", ValueType::U64)],
                },
                RelationDescriptor {
                    extension: None,
                    name: "Child".into(),
                    fields: vec![field("parent", ValueType::U64)],
                },
                RelationDescriptor {
                    extension: None,
                    name: "Account".into(),
                    fields: vec![
                        field("id", ValueType::U64),
                        field("active", ValueType::Bool),
                        field("note", ValueType::U64),
                    ],
                },
                RelationDescriptor {
                    extension: None,
                    name: "Transfer".into(),
                    fields: vec![field("account", ValueType::U64)],
                },
                RelationDescriptor {
                    extension: None,
                    name: "Roster".into(),
                    fields: vec![
                        field("worker", ValueType::U64),
                        field("span", interval()),
                        field("rested", ValueType::Bool),
                    ],
                },
                RelationDescriptor {
                    extension: None,
                    name: "Rest".into(),
                    fields: vec![field("worker", ValueType::U64), field("span", interval())],
                },
            ],
            statements: vec![
                StatementDescriptor::Functionality {
                    relation: TARGET2,
                    projection: Box::new([FieldId(0)]),
                },
                StatementDescriptor::Functionality {
                    relation: SHIFT,
                    projection: Box::new([FieldId(0), FieldId(1)]),
                },
                StatementDescriptor::Functionality {
                    relation: PARENT,
                    projection: Box::new([FieldId(0)]),
                },
                StatementDescriptor::Functionality {
                    relation: CHILD,
                    projection: Box::new([FieldId(0)]),
                },
                StatementDescriptor::Containment {
                    source: side(CLAIM_A, &[0], &[]),
                    target: side(TARGET2, &[0], &[]),
                },
                StatementDescriptor::Containment {
                    source: side(CLAIM_B, &[0], &[]),
                    target: side(TARGET2, &[0], &[]),
                },
                StatementDescriptor::Containment {
                    source: side(SESSION, &[0, 1], &[]),
                    target: side(SHIFT, &[0, 1], &[]),
                },
                StatementDescriptor::Containment {
                    source: side(PARENT, &[0], &[]),
                    target: side(CHILD, &[0], &[]),
                },
                StatementDescriptor::Containment {
                    source: side(CHILD, &[0], &[]),
                    target: side(PARENT, &[0], &[]),
                },
                StatementDescriptor::Functionality {
                    relation: ACCOUNT,
                    projection: Box::new([FieldId(0)]),
                },
                StatementDescriptor::Functionality {
                    relation: ROSTER,
                    projection: Box::new([FieldId(0), FieldId(1)]),
                },
                StatementDescriptor::Containment {
                    source: side(TRANSFER, &[0], &[]),
                    target: selected(ACCOUNT, &[0], 1, true),
                },
                StatementDescriptor::Containment {
                    source: side(REST, &[0, 1], &[]),
                    target: selected(ROSTER, &[0, 1], 2, true),
                },
            ],
        }
    }

    fn target(id: u64, note: u64) -> (RelationId, Vec<Value>) {
        (TARGET2, vec![Value::U64(id), Value::U64(note)])
    }

    fn span(rel: RelationId, worker: u64, start: u64, end: u64) -> (RelationId, Vec<Value>) {
        (
            rel,
            vec![Value::U64(worker), Value::IntervalU64(start, end)],
        )
    }

    fn account(id: u64, active: bool, note: u64) -> (RelationId, Vec<Value>) {
        (
            ACCOUNT,
            vec![Value::U64(id), Value::Bool(active), Value::U64(note)],
        )
    }

    fn roster(worker: u64, start: u64, end: u64, rested: bool) -> (RelationId, Vec<Value>) {
        (
            ROSTER,
            vec![
                Value::U64(worker),
                Value::IntervalU64(start, end),
                Value::Bool(rested),
            ],
        )
    }

    #[test]
    fn scalar_form() {
        run(
            &schema(),
            vec![
                Case {
                    name: "deleting a referenced target alone aborts",
                    base: vec![target(5, 0), (CLAIM_A, vec![Value::U64(5)])],
                    deletes: vec![target(5, 0)],
                    inserts: vec![],
                    verdict: Err(target_required(CLAIM_A_TARGET)),
                },
                Case {
                    name: "cluster demolition commits",
                    base: vec![target(5, 0), (CLAIM_A, vec![Value::U64(5)])],
                    deletes: vec![target(5, 0), (CLAIM_A, vec![Value::U64(5)])],
                    inserts: vec![],
                    verdict: Ok(()),
                },
                Case {
                    name: "surviving source of the other statement convicts its own id",
                    base: vec![
                        target(5, 0),
                        (CLAIM_A, vec![Value::U64(5)]),
                        (CLAIM_B, vec![Value::U64(5)]),
                    ],
                    deletes: vec![target(5, 0), (CLAIM_A, vec![Value::U64(5)])],
                    inserts: vec![],
                    verdict: Err(target_required(CLAIM_B_TARGET)),
                },
                Case {
                    name: "delete and re-establish by a different fact commits",
                    base: vec![target(5, 0), (CLAIM_A, vec![Value::U64(5)])],
                    deletes: vec![target(5, 0)],
                    inserts: vec![target(5, 1)],
                    verdict: Ok(()),
                },
            ],
        );
    }

    #[test]
    fn interval_form() {
        run(
            &schema(),
            vec![
                Case {
                    name: "shrink under a covered source aborts",
                    base: vec![span(SHIFT, 1, 0, 10), span(SESSION, 1, 5, 9)],
                    deletes: vec![span(SHIFT, 1, 0, 10)],
                    inserts: vec![span(SHIFT, 1, 0, 7)],
                    verdict: Err(target_required(SESSION_COVER)),
                },
                Case {
                    name: "shrink outside the source commits",
                    base: vec![span(SHIFT, 1, 0, 10), span(SESSION, 1, 2, 6)],
                    deletes: vec![span(SHIFT, 1, 0, 10)],
                    inserts: vec![span(SHIFT, 1, 0, 7)],
                    verdict: Ok(()),
                },
                Case {
                    name: "deleting one segment of a covering chain aborts",
                    base: vec![
                        span(SHIFT, 1, 0, 5),
                        span(SHIFT, 1, 5, 10),
                        span(SESSION, 1, 2, 9),
                    ],
                    deletes: vec![span(SHIFT, 1, 5, 10)],
                    inserts: vec![],
                    verdict: Err(target_required(SESSION_COVER)),
                },
                Case {
                    name: "delete plus replacement covering the hole commits",
                    base: vec![
                        span(SHIFT, 1, 0, 5),
                        span(SHIFT, 1, 5, 10),
                        span(SESSION, 1, 2, 9),
                    ],
                    deletes: vec![span(SHIFT, 1, 5, 10)],
                    inserts: vec![span(SHIFT, 1, 5, 9)],
                    verdict: Ok(()),
                },
                Case {
                    name: "whole chain replaced in one delta commits",
                    base: vec![
                        span(SHIFT, 1, 0, 5),
                        span(SHIFT, 1, 5, 10),
                        span(SESSION, 1, 2, 9),
                    ],
                    deletes: vec![span(SHIFT, 1, 0, 5), span(SHIFT, 1, 5, 10)],
                    inserts: vec![span(SHIFT, 1, 0, 6), span(SHIFT, 1, 6, 9)],
                    verdict: Ok(()),
                },
                Case {
                    name: "segment outside every source deletes freely",
                    base: vec![
                        span(SHIFT, 1, 0, 10),
                        span(SHIFT, 1, 20, 30),
                        span(SESSION, 1, 2, 9),
                    ],
                    deletes: vec![span(SHIFT, 1, 20, 30)],
                    inserts: vec![],
                    verdict: Ok(()),
                },
            ],
        );
    }

    #[test]
    fn psi_qualified_reestablishment() {
        run(
            &schema(),
            vec![
                Case {
                    name: "re-establishment outside psi aborts",
                    base: vec![account(9, true, 0), (TRANSFER, vec![Value::U64(9)])],
                    deletes: vec![account(9, true, 0)],
                    inserts: vec![account(9, false, 0)],
                    verdict: Err(target_required(TRANSFER_ACCOUNT)),
                },
                Case {
                    name: "re-establishment inside psi commits",
                    base: vec![account(9, true, 0), (TRANSFER, vec![Value::U64(9)])],
                    deletes: vec![account(9, true, 0)],
                    inserts: vec![account(9, true, 1)],
                    verdict: Ok(()),
                },
                Case {
                    name: "interval re-establishment outside psi aborts",
                    base: vec![roster(1, 0, 10, true), span(REST, 1, 2, 6)],
                    deletes: vec![roster(1, 0, 10, true)],
                    inserts: vec![roster(1, 0, 10, false)],
                    verdict: Err(target_required(REST_COVER)),
                },
            ],
        );
    }

    #[test]
    fn equality_pair_on_delete() {
        run(
            &schema(),
            vec![
                Case {
                    name: "parent and child deleted together commit",
                    base: vec![(PARENT, vec![Value::U64(1)]), (CHILD, vec![Value::U64(1)])],
                    deletes: vec![(PARENT, vec![Value::U64(1)]), (CHILD, vec![Value::U64(1)])],
                    inserts: vec![],
                    verdict: Ok(()),
                },
                Case {
                    name: "child alone deleted aborts on the totality direction",
                    base: vec![(PARENT, vec![Value::U64(1)]), (CHILD, vec![Value::U64(1)])],
                    deletes: vec![(CHILD, vec![Value::U64(1)])],
                    inserts: vec![],
                    verdict: Err(target_required(TOTALITY)),
                },
                Case {
                    name: "parent alone deleted aborts on the arm direction",
                    base: vec![(PARENT, vec![Value::U64(1)]), (CHILD, vec![Value::U64(1)])],
                    deletes: vec![(PARENT, vec![Value::U64(1)])],
                    inserts: vec![],
                    verdict: Err(target_required(ARM)),
                },
            ],
        );
    }
}

// ---------- the multi-violation citation set ----------
//
// One delta breaking SEVERAL statements at once (the ops fuzz target's
// first finding, docs/prd-crucible/12-fuzz-ops.md § conflict): the
// citation ORDER among simultaneous violations is unpinned — the engine
// cites per affected tuple, the model per statement id — so the model
// exposes the COMPLETE set (`violations`), whose head is `apply`'s
// verdict, one derivation.
mod citation_set {
    use super::*;

    const P1: RelationId = RelationId(0);
    const P2: RelationId = RelationId(1);
    const C: RelationId = RelationId(2);

    /// P1(id), P2(id), C(x, y); C(x) <= P1(id) is statement 0 and
    /// C(y) <= P2(id) statement 1 (no fresh fields, so the declared
    /// statements open the materialized order).
    fn schema() -> SchemaDescriptor {
        SchemaDescriptor {
            relations: vec![
                RelationDescriptor {
                    extension: None,
                    name: "P1".into(),
                    fields: vec![field("id", ValueType::U64)],
                },
                RelationDescriptor {
                    extension: None,
                    name: "P2".into(),
                    fields: vec![field("id", ValueType::U64)],
                },
                RelationDescriptor {
                    extension: None,
                    name: "C".into(),
                    fields: vec![field("x", ValueType::U64), field("y", ValueType::U64)],
                },
            ],
            statements: vec![
                StatementDescriptor::Containment {
                    source: side(C, &[0], &[]),
                    target: side(P1, &[0], &[]),
                },
                StatementDescriptor::Containment {
                    source: side(C, &[0], &[]),
                    target: side(P2, &[0], &[]),
                },
            ],
        }
    }

    #[test]
    fn the_complete_set_lists_every_simultaneous_violation_in_statement_order() {
        let mut db = NaiveDb::new(&schema());
        db.apply(&Delta {
            deletes: vec![],
            inserts: vec![
                (P1, vec![Value::U64(0)]),
                (P2, vec![Value::U64(0)]),
                (C, vec![Value::U64(0), Value::U64(0)]),
            ],
        })
        .expect("the base world commits");
        let both = Delta {
            deletes: vec![(P2, vec![Value::U64(0)]), (P1, vec![Value::U64(0)])],
            inserts: vec![],
        };
        assert_eq!(
            db.violations(&both),
            vec![target_required(0), target_required(1)],
            "both broken statements, statement order — regardless of delete order"
        );
        // `apply`'s verdict is the set's head: one derivation.
        assert_eq!(db.clone().apply(&both), Err(target_required(0)));
        // A single-violation delta degenerates to the singleton set.
        let one = Delta {
            deletes: vec![(P2, vec![Value::U64(0)])],
            inserts: vec![],
        };
        assert_eq!(db.violations(&one), vec![target_required(1)]);
        assert_eq!(db.apply(&one), Err(target_required(1)));
        // A committing delta has the empty set.
        let clean = Delta {
            deletes: vec![
                (C, vec![Value::U64(0), Value::U64(0)]),
                (P1, vec![Value::U64(0)]),
            ],
            inserts: vec![],
        };
        assert_eq!(db.violations(&clean), vec![]);
    }
}
