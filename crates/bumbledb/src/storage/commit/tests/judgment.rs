use crate::encoding::{ValueRef, encode_u64};
use crate::error::{Admission, Direction, Result, Violation};
use crate::schema::Schema;
use crate::schema::ValidateDescriptor as _;
use crate::storage::env::Environment;
use crate::storage::keys;
use crate::testutil::TempDir;
use crate::testutil::expect_rejected;
use bumbledb_theory::Value;
use bumbledb_theory::schema::{
    FieldId, RelationDescriptor, RelationId, SchemaDescriptor, StatementDescriptor, StatementId,
    ValueType,
};

use super::{apply_delta, committed_data, fact, field, interval, key, selected, side};

const PARENT: RelationId = RelationId(0);
const CHILD: RelationId = RelationId(1);
const ACCOUNT: RelationId = RelationId(2);
const TRANSFER: RelationId = RelationId(3);
const SHIFT: RelationId = RelationId(4);
const SESSION: RelationId = RelationId(5);
const REST: RelationId = RelationId(6);
const REPORT: RelationId = RelationId(7);

const TOTALITY: StatementId = StatementId(4);
const ARM: StatementId = StatementId(5);
const TRANSFER_ACCOUNT: StatementId = StatementId(6);
const SESSION_COVER: StatementId = StatementId(7);
const REST_COVER: StatementId = StatementId(8);
const REPORT_ACCOUNT: StatementId = StatementId(9);

fn schema() -> Schema {
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
            ValueRef::IntervalU64(
                bumbledb_theory::Interval::<u64>::new(start, end).expect("nonempty interval"),
            ),
            ValueRef::Bool(rested),
        ],
    )
}

fn session(schema: &Schema, worker: u64, start: u64, end: u64) -> Vec<u8> {
    fact(
        schema,
        SESSION,
        &[
            ValueRef::U64(worker),
            ValueRef::IntervalU64(
                bumbledb_theory::Interval::<u64>::new(start, end).expect("nonempty interval"),
            ),
        ],
    )
}

fn rest(schema: &Schema, worker: u64, start: u64, end: u64) -> Vec<u8> {
    fact(
        schema,
        REST,
        &[
            ValueRef::U64(worker),
            ValueRef::IntervalU64(
                bumbledb_theory::Interval::<u64>::new(start, end).expect("nonempty interval"),
            ),
        ],
    )
}

fn report(schema: &Schema, subject: u64, urgent: bool) -> Vec<u8> {
    fact(
        schema,
        REPORT,
        &[ValueRef::U64(subject), ValueRef::Bool(urgent)],
    )
}

fn insert_all(
    env: &Environment,
    schema: &Schema,
    facts: &[(RelationId, Vec<u8>)],
) -> Result<Admission<()>> {
    apply_delta(env, schema, &[], facts)
}

fn base_then_insert(
    name: &str,
    base: &[(RelationId, Vec<u8>)],
    facts: &[(RelationId, Vec<u8>)],
) -> Result<Admission<()>> {
    let dir = TempDir::new(name);
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    if !base.is_empty() {
        insert_all(&env, &schema, base)
            .expect("base commit")
            .expect("admitted");
    }
    let before = committed_data(&env);
    let result = insert_all(&env, &schema, facts);
    if matches!(&result, Ok(Admission::Rejected(_)) | Err(_)) {
        assert_eq!(committed_data(&env), before);
    }
    result
}

fn assert_source_violation(
    result: Result<Admission<()>>,
    statement: StatementId,
    source_fact: &[u8],
) {
    let violations = expect_rejected(result);
    let [
        (
            Violation::Containment {
                statement: slot,
                direction,
                fact,
                ..
            },
            _,
        ),
    ] = violations.as_slice()
    else {
        panic!("expected one containment citation, got {violations:?}");
    };
    assert_eq!(schema().id_of(*slot), statement);
    assert_eq!(*direction, Direction::SourceUnsatisfied);
    assert_eq!(**fact, *source_fact, "the violation names the source fact");
}

fn reverse_entries(env: &Environment, statement: StatementId) -> Vec<Vec<u8>> {
    let prefix = key(|b| keys::reverse_prefix(b, statement, &[]));
    committed_data(env)
        .into_iter()
        .map(|(k, _)| k)
        .filter(|k| k.starts_with(&prefix))
        .collect()
}

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
fn a_sorted_probe_group_judges_through_the_shared_cursor() {
    let schema = schema();
    let base: Vec<(RelationId, Vec<u8>)> = (0..40)
        .step_by(2)
        .map(|id| (ACCOUNT, account(&schema, id, true)))
        .collect();

    let mixed: Vec<(RelationId, Vec<u8>)> = (0..40)
        .map(|id| (TRANSFER, transfer(&schema, id)))
        .collect();
    assert_source_violation(
        base_then_insert("judg-sorted-walker-mixed", &base, &mixed),
        TRANSFER_ACCOUNT,
        &transfer(&schema, 1),
    );

    let all_present: Vec<(RelationId, Vec<u8>)> = (0..40)
        .step_by(2)
        .map(|id| (TRANSFER, transfer(&schema, id)))
        .collect();
    base_then_insert("judg-sorted-walker-green", &base, &all_present)
        .expect("every probe hits through the walker")
        .unwrap();

    assert_source_violation(
        base_then_insert(
            "judg-sorted-walker-tail",
            &base,
            &[(TRANSFER, transfer(&schema, 999))],
        ),
        TRANSFER_ACCOUNT,
        &transfer(&schema, 999),
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
    .expect("target and source land together")
    .unwrap();
}

#[test]
fn scalar_source_with_pre_committed_target_commits() {
    let schema = schema();
    base_then_insert(
        "judg-scalar-cross-delta",
        &[(ACCOUNT, account(&schema, 9, true))],
        &[(TRANSFER, transfer(&schema, 9))],
    )
    .expect("the base target satisfies the probe")
    .unwrap();
}

#[test]
fn scalar_target_failing_the_target_selection_aborts() {
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

#[test]
fn out_of_sigma_source_commits_without_a_target_and_writes_no_reverse_edge() {
    let dir = TempDir::new("judg-conditional-outside");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_all(&env, &schema, &[(REPORT, report(&schema, 5, false))])
        .expect("a fact outside σ needs no target")
        .unwrap();
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
    .expect("commit")
    .expect("admitted");

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
    .expect("commit")
    .expect("admitted");
    assert_eq!(reverse_entries(&env, REPORT_ACCOUNT).len(), 1);

    apply_delta(&env, &schema, &[(REPORT, r)], &[])
        .expect("commit")
        .expect("admitted");
    assert!(reverse_entries(&env, REPORT_ACCOUNT).is_empty());
}

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
    .expect("an exact segment covers")
    .unwrap();
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
    .expect("abutting segments cover jointly")
    .unwrap();
}

#[test]
fn entry_segment_overhang_covers() {
    let schema = schema();
    base_then_insert(
        "judg-cover-overhang",
        &[(SHIFT, shift(&schema, 1, 5, 25, false))],
        &[(SESSION, session(&schema, 1, 10, 20))],
    )
    .expect("a wider running segment covers")
    .unwrap();
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
fn ray_target_covers_a_bounded_source() {
    let schema = schema();
    base_then_insert(
        "judg-cover-ray-target",
        &[(SHIFT, shift(&schema, 1, 10, u64::MAX, false))],
        &[(SESSION, session(&schema, 1, 15, 1000))],
    )
    .expect("a ray covers any bounded source above its start")
    .unwrap();
}

#[test]
fn ray_source_not_covered_by_bounded_targets() {
    let schema = schema();
    let s = session(&schema, 1, 15, u64::MAX);
    assert_source_violation(
        base_then_insert(
            "judg-cover-ray-source-bounded",
            &[(SHIFT, shift(&schema, 1, 10, 1_000_000, false))],
            &[(SESSION, s.clone())],
        ),
        SESSION_COVER,
        &s,
    );
}

#[test]
fn ray_source_covered_by_ray_target() {
    let schema = schema();
    base_then_insert(
        "judg-cover-ray-source-ray",
        &[(SHIFT, shift(&schema, 1, 10, u64::MAX, false))],
        &[(SESSION, session(&schema, 1, 15, u64::MAX))],
    )
    .expect("a target ray covers a source ray")
    .unwrap();
}

#[test]
fn another_prefix_group_does_not_cover() {
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
    .expect("every consumed segment satisfies σ")
    .unwrap();
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
    .expect("the cluster lands whole")
    .unwrap();
}
