//! The containment source-side judgment (PRD 08 criteria): scalar guard
//! probes with and without target selections, conditional sources (σ
//! gating both the `R` writes and the probes), the coverage-walk matrix,
//! and the `==` pair's insert-time totality.
//!
//! Own fixture — richer than the shared one: a `==` pair, a selected
//! scalar target, a pointwise target with selected and unselected
//! coverage statements, and a selected source.

use crate::encoding::{encode_u64, ValueRef};
use crate::error::{Direction, Error, Result};
use crate::schema::{
    FieldId, RelationDescriptor, RelationId, Schema, SchemaDescriptor, StatementDescriptor,
    StatementId, ValueType,
};
use crate::storage::env::Environment;
use crate::storage::keys;
use crate::testutil::TempDir;
use crate::value::Value;

use super::{apply_delta, committed_data, fact, field, interval, key, selected, side};

const PARENT: RelationId = RelationId(0);
const CHILD: RelationId = RelationId(1);
const ACCOUNT: RelationId = RelationId(2);
const TRANSFER: RelationId = RelationId(3);
const SHIFT: RelationId = RelationId(4);
const SESSION: RelationId = RelationId(5);
const REST: RelationId = RelationId(6);
const REPORT: RelationId = RelationId(7);

/// Declared statement order (no fresh fields, so no auto-keys).
const TOTALITY: StatementId = StatementId(4);
const ARM: StatementId = StatementId(5);
const TRANSFER_ACCOUNT: StatementId = StatementId(6);
const SESSION_COVER: StatementId = StatementId(7);
const REST_COVER: StatementId = StatementId(8);
const REPORT_ACCOUNT: StatementId = StatementId(9);

/// Parent(id; key id) == Child(parent; key parent), lowered to the two
/// statements [`TOTALITY`] and [`ARM`]. Transfer(account) <=
/// Account(id | active == true): a selected scalar target.
/// Session(worker, span) <= Shift(worker, span) and Rest(worker, span) <=
/// Shift(worker, span | rested == true): coverage against a pointwise
/// key, unselected and selected. Report(subject | urgent == true) <=
/// Account(id): a conditional source.
#[allow(clippy::too_many_lines)] // one fixture: eight relations, ten statements
fn schema() -> Schema {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "Parent".into(),
                fields: vec![field("id", ValueType::U64)],
            },
            RelationDescriptor {
                name: "Child".into(),
                fields: vec![field("parent", ValueType::U64)],
            },
            RelationDescriptor {
                name: "Account".into(),
                fields: vec![
                    field("id", ValueType::U64),
                    field("active", ValueType::Bool),
                ],
            },
            RelationDescriptor {
                name: "Transfer".into(),
                fields: vec![field("account", ValueType::U64)],
            },
            RelationDescriptor {
                name: "Shift".into(),
                fields: vec![
                    field("worker", ValueType::U64),
                    field("span", interval()),
                    field("rested", ValueType::Bool),
                ],
            },
            RelationDescriptor {
                name: "Session".into(),
                fields: vec![field("worker", ValueType::U64), field("span", interval())],
            },
            RelationDescriptor {
                name: "Rest".into(),
                fields: vec![field("worker", ValueType::U64), field("span", interval())],
            },
            RelationDescriptor {
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
                source: side(PARENT, &[0]),
                target: side(CHILD, &[0]),
            },
            StatementDescriptor::Containment {
                source: side(CHILD, &[0]),
                target: side(PARENT, &[0]),
            },
            StatementDescriptor::Containment {
                source: side(TRANSFER, &[0]),
                target: selected(ACCOUNT, &[0], &[(1, Value::Bool(true))]),
            },
            StatementDescriptor::Containment {
                source: side(SESSION, &[0, 1]),
                target: side(SHIFT, &[0, 1]),
            },
            StatementDescriptor::Containment {
                source: side(REST, &[0, 1]),
                target: selected(SHIFT, &[0, 1], &[(2, Value::Bool(true))]),
            },
            StatementDescriptor::Containment {
                source: selected(REPORT, &[0], &[(1, Value::Bool(true))]),
                target: side(ACCOUNT, &[0]),
            },
        ],
    }
    .validate()
    .expect("valid fixture")
}

fn parent(schema: &Schema, id: u64) -> Vec<u8> {
    fact(schema, PARENT, &[ValueRef::U64(id)])
}

fn child(schema: &Schema, parent: u64) -> Vec<u8> {
    fact(schema, CHILD, &[ValueRef::U64(parent)])
}

fn account(schema: &Schema, id: u64, active: bool) -> Vec<u8> {
    fact(
        schema,
        ACCOUNT,
        &[ValueRef::U64(id), ValueRef::Bool(active)],
    )
}

fn transfer(schema: &Schema, account: u64) -> Vec<u8> {
    fact(schema, TRANSFER, &[ValueRef::U64(account)])
}

fn shift(schema: &Schema, worker: u64, start: u64, end: u64, rested: bool) -> Vec<u8> {
    fact(
        schema,
        SHIFT,
        &[
            ValueRef::U64(worker),
            ValueRef::IntervalU64(start, end),
            ValueRef::Bool(rested),
        ],
    )
}

fn session(schema: &Schema, worker: u64, start: u64, end: u64) -> Vec<u8> {
    fact(
        schema,
        SESSION,
        &[ValueRef::U64(worker), ValueRef::IntervalU64(start, end)],
    )
}

fn rest(schema: &Schema, worker: u64, start: u64, end: u64) -> Vec<u8> {
    fact(
        schema,
        REST,
        &[ValueRef::U64(worker), ValueRef::IntervalU64(start, end)],
    )
}

fn report(schema: &Schema, subject: u64, urgent: bool) -> Vec<u8> {
    fact(
        schema,
        REPORT,
        &[ValueRef::U64(subject), ValueRef::Bool(urgent)],
    )
}

/// Inserts all facts in one delta and commits.
fn insert_all(env: &Environment, schema: &Schema, facts: &[(RelationId, Vec<u8>)]) -> Result<()> {
    apply_delta(env, schema, &[], facts)
}

/// Commits `base`, then inserts `facts` in a second delta; on an abort,
/// asserts the base state survived untouched.
fn base_then_insert(
    name: &str,
    base: &[(RelationId, Vec<u8>)],
    facts: &[(RelationId, Vec<u8>)],
) -> Result<()> {
    let dir = TempDir::new(name);
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    if !base.is_empty() {
        insert_all(&env, &schema, base).expect("base commit");
    }
    let before = committed_data(&env);
    let result = insert_all(&env, &schema, facts);
    if result.is_err() {
        assert_eq!(committed_data(&env), before);
    }
    result
}

fn assert_source_violation(result: Result<()>, statement: StatementId, source_fact: &[u8]) {
    let err = result.unwrap_err();
    let Error::ContainmentViolation {
        statement: named,
        direction,
        fact,
    } = &err
    else {
        panic!("expected a containment violation, got {err:?}");
    };
    assert_eq!(*named, statement);
    assert_eq!(*direction, Direction::SourceUnsatisfied);
    assert_eq!(**fact, *source_fact, "the violation names the source fact");
}

/// Every committed `R` key of one statement (direct LMDB inspection).
fn reverse_entries(env: &Environment, statement: StatementId) -> Vec<Vec<u8>> {
    let prefix = key(|b| keys::reverse_prefix(b, statement, &[]));
    committed_data(env)
        .into_iter()
        .map(|(k, _)| k)
        .filter(|k| k.starts_with(&prefix))
        .collect()
}

// ---------- scalar containment ----------

#[test]
fn scalar_source_without_target_aborts() {
    let schema = schema();
    let t = transfer(&schema, 9);
    assert_source_violation(
        base_then_insert("judg-scalar-missing", &[], &[(TRANSFER, t.clone())]),
        TRANSFER_ACCOUNT,
        &t,
    );
}

#[test]
fn scalar_target_and_source_in_one_delta_commit() {
    base_then_insert(
        "judg-scalar-same-delta",
        &[],
        &[
            (ACCOUNT, account(&schema(), 9, true)),
            (TRANSFER, transfer(&schema(), 9)),
        ],
    )
    .expect("target and source land together");
}

#[test]
fn scalar_source_with_pre_committed_target_commits() {
    let schema = schema();
    base_then_insert(
        "judg-scalar-cross-delta",
        &[(ACCOUNT, account(&schema, 9, true))],
        &[(TRANSFER, transfer(&schema, 9))],
    )
    .expect("the base target satisfies the probe");
}

#[test]
fn scalar_target_failing_the_target_selection_aborts() {
    // The guard hit alone is not the proof: the found account is outside
    // the statement's target σ (active == true).
    let schema = schema();
    let t = transfer(&schema, 9);
    assert_source_violation(
        base_then_insert(
            "judg-scalar-target-selection",
            &[],
            &[(ACCOUNT, account(&schema, 9, false)), (TRANSFER, t.clone())],
        ),
        TRANSFER_ACCOUNT,
        &t,
    );
}

// ---------- conditional sources (σ on the source side) ----------

#[test]
fn out_of_sigma_source_commits_without_a_target_and_writes_no_reverse_edge() {
    // A non-urgent report is outside the statement's source σ: no target
    // required, and — asserted by direct inspection of the committed `R`
    // prefix — no reverse edge written.
    let dir = TempDir::new("judg-conditional-outside");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_all(&env, &schema, &[(REPORT, report(&schema, 5, false))])
        .expect("a fact outside σ needs no target");
    assert!(reverse_entries(&env, REPORT_ACCOUNT).is_empty());
}

#[test]
fn in_sigma_source_without_a_target_aborts() {
    let schema = schema();
    let r = report(&schema, 5, true);
    assert_source_violation(
        base_then_insert("judg-conditional-inside", &[], &[(REPORT, r.clone())]),
        REPORT_ACCOUNT,
        &r,
    );
}

#[test]
fn in_sigma_source_writes_its_reverse_edge() {
    let dir = TempDir::new("judg-conditional-edge");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_all(
        &env,
        &schema,
        &[
            (ACCOUNT, account(&schema, 5, true)),
            (REPORT, report(&schema, 5, true)),
        ],
    )
    .expect("commit");
    // Exactly one edge: R | statement | key_bytes | source_rel | source_row.
    let expected = key(|b| keys::reverse_key(b, REPORT_ACCOUNT, &encode_u64(5), REPORT, 0));
    assert_eq!(reverse_entries(&env, REPORT_ACCOUNT), vec![expected]);
}

#[test]
fn deleting_a_source_removes_its_reverse_edge() {
    let dir = TempDir::new("judg-conditional-edge-delete");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let r = report(&schema, 5, true);
    insert_all(
        &env,
        &schema,
        &[(ACCOUNT, account(&schema, 5, true)), (REPORT, r.clone())],
    )
    .expect("commit");
    assert_eq!(reverse_entries(&env, REPORT_ACCOUNT).len(), 1);

    apply_delta(&env, &schema, &[(REPORT, r)], &[]).expect("commit");
    assert!(reverse_entries(&env, REPORT_ACCOUNT).is_empty());
}

// ---------- the coverage walk (unselected target) ----------

#[test]
fn exact_single_segment_covers() {
    let schema = schema();
    base_then_insert(
        "judg-cover-exact",
        &[],
        &[
            (SHIFT, shift(&schema, 1, 10, 20, false)),
            (SESSION, session(&schema, 1, 10, 20)),
        ],
    )
    .expect("an exact segment covers");
}

#[test]
fn abutting_chain_covers() {
    let schema = schema();
    base_then_insert(
        "judg-cover-chain",
        &[
            (SHIFT, shift(&schema, 1, 10, 15, false)),
            (SHIFT, shift(&schema, 1, 15, 20, false)),
        ],
        &[(SESSION, session(&schema, 1, 10, 20))],
    )
    .expect("abutting segments cover jointly");
}

#[test]
fn entry_segment_overhang_covers() {
    // The entry case with no exact-start segment: the group's predecessor
    // [5, 25) is still running at the source's start.
    let schema = schema();
    base_then_insert(
        "judg-cover-overhang",
        &[(SHIFT, shift(&schema, 1, 5, 25, false))],
        &[(SESSION, session(&schema, 1, 10, 20))],
    )
    .expect("a wider running segment covers");
}

#[test]
fn interior_gap_aborts() {
    let schema = schema();
    let s = session(&schema, 1, 10, 20);
    assert_source_violation(
        base_then_insert(
            "judg-cover-gap",
            &[
                (SHIFT, shift(&schema, 1, 10, 14, false)),
                (SHIFT, shift(&schema, 1, 15, 20, false)),
            ],
            &[(SESSION, s.clone())],
        ),
        SESSION_COVER,
        &s,
    );
}

#[test]
fn source_start_before_first_segment_aborts() {
    // The entry gap: no segment is running at s.
    let schema = schema();
    let s = session(&schema, 1, 10, 20);
    assert_source_violation(
        base_then_insert(
            "judg-cover-start-before",
            &[(SHIFT, shift(&schema, 1, 12, 20, false))],
            &[(SESSION, s.clone())],
        ),
        SESSION_COVER,
        &s,
    );
}

#[test]
fn source_end_past_last_segment_aborts() {
    // Prefix exhaustion: the chain runs out below e.
    let schema = schema();
    let s = session(&schema, 1, 10, 20);
    assert_source_violation(
        base_then_insert(
            "judg-cover-end-past",
            &[(SHIFT, shift(&schema, 1, 10, 18, false))],
            &[(SESSION, s.clone())],
        ),
        SESSION_COVER,
        &s,
    );
}

#[test]
fn max_sentinel_segment_covers_a_bounded_source() {
    // The unbounded-end convention writes u64::MAX as the end; ordinary
    // byte comparison judges the coverage — no sentinel-specific path.
    let schema = schema();
    base_then_insert(
        "judg-cover-max-sentinel",
        &[(SHIFT, shift(&schema, 1, 10, u64::MAX, false))],
        &[(SESSION, session(&schema, 1, 15, 1000))],
    )
    .expect("the sentinel segment covers any bounded source above its start");
}

#[test]
fn another_prefix_group_does_not_cover() {
    // The walk never leaves the scalar-prefix group: worker 2's segment
    // proves nothing for worker 1.
    let schema = schema();
    let s = session(&schema, 1, 10, 20);
    assert_source_violation(
        base_then_insert(
            "judg-cover-other-prefix",
            &[(SHIFT, shift(&schema, 2, 10, 20, false))],
            &[(SESSION, s.clone())],
        ),
        SESSION_COVER,
        &s,
    );
}

// ---------- the coverage walk (selected target) ----------

#[test]
fn selected_chain_inside_sigma_commits() {
    let schema = schema();
    base_then_insert(
        "judg-cover-selected-pass",
        &[
            (SHIFT, shift(&schema, 1, 10, 15, true)),
            (SHIFT, shift(&schema, 1, 15, 20, true)),
        ],
        &[(REST, rest(&schema, 1, 10, 20))],
    )
    .expect("every consumed segment satisfies σ");
}

#[test]
fn entry_segment_failing_sigma_aborts() {
    let schema = schema();
    let r = rest(&schema, 1, 10, 20);
    assert_source_violation(
        base_then_insert(
            "judg-cover-selected-entry",
            &[(SHIFT, shift(&schema, 1, 10, 20, false))],
            &[(REST, r.clone())],
        ),
        REST_COVER,
        &r,
    );
}

#[test]
fn mid_chain_segment_failing_sigma_aborts() {
    // The second consumed segment is outside σ: the walk pays one F get
    // per segment and aborts there.
    let schema = schema();
    let r = rest(&schema, 1, 10, 20);
    assert_source_violation(
        base_then_insert(
            "judg-cover-selected-mid",
            &[
                (SHIFT, shift(&schema, 1, 10, 15, true)),
                (SHIFT, shift(&schema, 1, 15, 20, false)),
            ],
            &[(REST, r.clone())],
        ),
        REST_COVER,
        &r,
    );
}

// ---------- the == pair (two statements, both source-side on insert) ----------

#[test]
fn parent_alone_aborts_on_the_totality_statement() {
    let schema = schema();
    let p = parent(&schema, 1);
    assert_source_violation(
        base_then_insert("judg-pair-parent-alone", &[], &[(PARENT, p.clone())]),
        TOTALITY,
        &p,
    );
}

#[test]
fn child_alone_aborts_on_the_arm_statement() {
    let schema = schema();
    let c = child(&schema, 1);
    assert_source_violation(
        base_then_insert("judg-pair-child-alone", &[], &[(CHILD, c.clone())]),
        ARM,
        &c,
    );
}

#[test]
fn parent_and_child_in_one_delta_commit() {
    let schema = schema();
    base_then_insert(
        "judg-pair-together",
        &[],
        &[(PARENT, parent(&schema, 1)), (CHILD, child(&schema, 1))],
    )
    .expect("the cluster lands whole");
}
