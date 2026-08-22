use crate::encoding::ValueRef;
use crate::error::{Admission, Direction, Result, Violation};
use crate::schema::ValidateDescriptor as _;
use crate::schema::{ContainmentId, Enforcement, KeyId, Schema};
use crate::storage::env::Environment;
use crate::storage::keys;
use crate::testutil::TempDir;
use crate::testutil::expect_rejected;
use bumbledb_theory::Value;
use bumbledb_theory::schema::{
    FieldId, RelationDescriptor, RelationId, Row, SchemaDescriptor, StatementDescriptor,
    StatementId, ValueType,
};

use super::{apply_delta, committed_data, fact, field, key, selected, side};

const SEVERITY: RelationId = RelationId(0);
const ALERT: RelationId = RelationId(1);
const ESCALATION: RelationId = RelationId(2);
const HANDLER: RelationId = RelationId(3);

const HANDLER_KEY: StatementId = StatementId(1);
const ALERT_SEVERITY: StatementId = StatementId(2);
const ESCALATION_SEVERITY: StatementId = StatementId(3);
const SEVERITY_HANDLED: StatementId = StatementId(4);

/// Severity(id) <= Handler(severity): domain quantification — every severity
/// has a handler, at all times after the handlers land.
fn schema() -> Schema {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: Some(Box::new([
                    Row {
                        handle: "Low".into(),
                        values: Box::new([Value::Bool(false)]),
                    },
                    Row {
                        handle: "Med".into(),
                        values: Box::new([Value::Bool(true)]),
                    },
                    Row {
                        handle: "High".into(),
                        values: Box::new([Value::Bool(true)]),
                    },
                ])),
                name: "Severity".into(),
                fields: vec![field("pages", ValueType::Bool)],
            },
            RelationDescriptor {
                extension: None,
                name: "Alert".into(),
                fields: vec![field("severity", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Escalation".into(),
                fields: vec![field("severity", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Handler".into(),
                fields: vec![
                    field("severity", ValueType::U64),
                    field("priority", ValueType::U64),
                ],
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: HANDLER,
                projection: Box::new([FieldId(0)]),
            },
            StatementDescriptor::Containment {
                source: side(ALERT, &[0]),
                target: side(SEVERITY, &[0]),
            },
            StatementDescriptor::Containment {
                source: side(ESCALATION, &[0]),
                target: selected(SEVERITY, &[0], &[(1, Value::Bool(true))]),
            },
            StatementDescriptor::Containment {
                source: side(SEVERITY, &[0]),
                target: side(HANDLER, &[0]),
            },
        ],
    }
    .validate()
    .expect("valid fixture")
}

fn alert(schema: &Schema, severity: u64) -> Vec<u8> {
    fact(schema, ALERT, &[ValueRef::U64(severity)])
}

fn escalation(schema: &Schema, severity: u64) -> Vec<u8> {
    fact(schema, ESCALATION, &[ValueRef::U64(severity)])
}

fn handler(schema: &Schema, severity: u64, priority: u64) -> Vec<u8> {
    fact(
        schema,
        HANDLER,
        &[ValueRef::U64(severity), ValueRef::U64(priority)],
    )
}

fn base_then(
    name: &str,
    base: &[(RelationId, Vec<u8>)],
    deletes: &[(RelationId, Vec<u8>)],
    inserts: &[(RelationId, Vec<u8>)],
) -> Result<Admission<()>> {
    let dir = TempDir::new(name);
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    if !base.is_empty() {
        apply_delta(&env, &schema, &[], base)
            .expect("base commit")
            .expect("admitted");
    }
    let before = committed_data(&env);
    let result = apply_delta(&env, &schema, deletes, inserts);
    if matches!(&result, Ok(Admission::Rejected(_)) | Err(_)) {
        assert_eq!(committed_data(&env), before);
    }
    result
}

fn assert_violation(
    result: Result<Admission<()>>,
    statement: StatementId,
    direction: Direction,
    named_fact: &[u8],
) {
    let violations = expect_rejected(result);
    let [
        (
            Violation::Containment {
                statement: slot,
                direction: dir,
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
    assert_eq!(*dir, direction);
    assert_eq!(**fact, *named_fact, "the violation names the source fact");
}

#[test]
fn closed_reference_inside_the_extension_commits() {
    base_then("closed-ref-ok", &[], &[], &[(ALERT, alert(&schema(), 2))])
        .expect("row id 2 is a ground axiom")
        .unwrap();
}

#[test]
fn closed_reference_beyond_the_extension_aborts() {
    let schema = schema();
    let a = alert(&schema, 3);
    assert_violation(
        base_then("closed-ref-dangling", &[], &[], &[(ALERT, a.clone())]),
        ALERT_SEVERITY,
        Direction::SourceUnsatisfied,
        &a,
    );
}

#[test]
fn closed_reference_beyond_the_roster_cap_aborts() {
    let schema = schema();
    let a = alert(&schema, 300);
    assert_violation(
        base_then("closed-ref-out-of-range", &[], &[], &[(ALERT, a.clone())]),
        ALERT_SEVERITY,
        Direction::SourceUnsatisfied,
        &a,
    );
}

#[test]
fn subset_member_commits() {
    base_then(
        "closed-subset-ok",
        &[],
        &[],
        &[(ESCALATION, escalation(&schema(), 1))],
    )
    .expect("Med pages — inside ψ")
    .unwrap();
}

#[test]
fn subset_nonmember_aborts() {
    let schema = schema();
    let e = escalation(&schema, 0);
    assert_violation(
        base_then("closed-subset-miss", &[], &[], &[(ESCALATION, e.clone())]),
        ESCALATION_SEVERITY,
        Direction::SourceUnsatisfied,
        &e,
    );
}

#[test]
fn subset_out_of_range_aborts() {
    let schema = schema();
    let e = escalation(&schema, 300);
    assert_violation(
        base_then("closed-subset-oob", &[], &[], &[(ESCALATION, e.clone())]),
        ESCALATION_SEVERITY,
        Direction::SourceUnsatisfied,
        &e,
    );
}

#[test]
fn closed_target_statements_write_no_reverse_edges() {
    let dir = TempDir::new("closed-no-r");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    apply_delta(
        &env,
        &schema,
        &[],
        &[
            (ALERT, alert(&schema, 2)),
            (ESCALATION, escalation(&schema, 2)),
        ],
    )
    .expect("commit")
    .expect("admitted");
    for statement in [ALERT_SEVERITY, ESCALATION_SEVERITY] {
        let prefix = key(|b| keys::reverse_prefix(b, statement, &[]));
        let entries: Vec<_> = committed_data(&env)
            .into_iter()
            .filter(|(k, _)| k.starts_with(&prefix))
            .collect();
        assert!(entries.is_empty(), "closed-target statement wrote R keys");
    }
}

fn all_handlers(schema: &Schema) -> Vec<(RelationId, Vec<u8>)> {
    (0..3)
        .map(|severity| (HANDLER, handler(schema, severity, 10)))
        .collect()
}

#[test]
fn deleting_the_last_handler_for_a_severity_aborts() {
    let schema = schema();
    let severity_high = {
        let rows = schema
            .relation(SEVERITY)
            .body()
            .closed_rows()
            .expect("closed");
        rows[2].fact.to_vec()
    };
    assert_violation(
        base_then(
            "closed-domain-abort",
            &all_handlers(&schema),
            &[(HANDLER, handler(&schema, 2, 10))],
            &[],
        ),
        SEVERITY_HANDLED,
        Direction::TargetRequired,
        &severity_high,
    );
}

/// Replacing a handler in the same commit re-establishes the tuple — the plain
/// set difference drops the check before any survivor scan runs.
#[test]
fn replacing_a_handler_in_one_commit_commits() {
    let schema = schema();
    base_then(
        "closed-domain-replace",
        &all_handlers(&schema),
        &[(HANDLER, handler(&schema, 2, 10))],
        &[(HANDLER, handler(&schema, 2, 99))],
    )
    .expect("the severity-2 key tuple re-lands in phase 2")
    .unwrap();
}

#[test]
fn the_domain_statement_resolved_the_handler_key() {
    let schema = schema();
    let Enforcement::ScalarProbe { target_key, .. } =
        &schema.containment(ContainmentId(2)).enforcement
    else {
        panic!("domain quantification resolves against the ordinary target key");
    };
    assert_eq!(*target_key, KeyId(1));
    assert_eq!(schema.key(*target_key).id, HANDLER_KEY);
}
