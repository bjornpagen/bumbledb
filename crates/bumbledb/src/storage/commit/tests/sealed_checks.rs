//! The staging law at the checker (PRD 08): σ literals seal at validate
//! ([`CompiledCheck`]), the commit path consumes constants, and only
//! interned text resolves per commit. The replay matrix pins the typed
//! verdicts (statement ids included) so the staging move is
//! behavior-preserving by test, not by hope.

use crate::encoding::ValueRef;
use crate::error::{Direction, Error, Result};
use crate::schema::{
    CompiledCheck, FieldId, RelationDescriptor, RelationId, Schema, SchemaDescriptor,
    StatementDescriptor, StatementId, ValueType,
};
use crate::storage::commit::judgment::{SelectionCheck, Selections};
use crate::storage::delta::WriteDelta;
use crate::storage::env::Environment;
use crate::testutil::TempDir;
use crate::value::Value;

use super::{apply_delta, fact, field, selected, side};

const ACCOUNT: RelationId = RelationId(0);
const TRANSFER: RelationId = RelationId(1);
const REPORT: RelationId = RelationId(2);

/// Declared statement order (no fresh fields, no closed relations).
const ACCOUNT_KEY: StatementId = StatementId(0);
const TRANSFER_ACCOUNT: StatementId = StatementId(1);
const REPORT_ACCOUNT: StatementId = StatementId(2);

/// Account(id; key id) — Transfer(account | flagged == true) <=
/// Account(id) carries a sealable bool σ; Report(account | note ==
/// "urgent") <= Account(id) carries the one commit-resolved kind.
fn schema() -> Schema {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Account".into(),
                fields: vec![field("id", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Transfer".into(),
                fields: vec![
                    field("account", ValueType::U64),
                    field("flagged", ValueType::Bool),
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Report".into(),
                fields: vec![
                    field("account", ValueType::U64),
                    field("note", ValueType::String),
                ],
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: ACCOUNT,
                projection: Box::new([FieldId(0)]),
            },
            StatementDescriptor::Containment {
                source: selected(TRANSFER, &[0], &[(1, Value::Bool(true))]),
                target: side(ACCOUNT, &[0]),
            },
            StatementDescriptor::Containment {
                source: selected(
                    REPORT,
                    &[0],
                    &[(1, Value::String("urgent".as_bytes().into()))],
                ),
                target: side(ACCOUNT, &[0]),
            },
        ],
    }
    .validate()
    .expect("valid fixture")
}

fn account(schema: &Schema, id: u64) -> Vec<u8> {
    fact(schema, ACCOUNT, &[ValueRef::U64(id)])
}

fn transfer(schema: &Schema, account: u64, flagged: bool) -> Vec<u8> {
    fact(
        schema,
        TRANSFER,
        &[ValueRef::U64(account), ValueRef::Bool(flagged)],
    )
}

/// The [shape] pin: the bool literal's canonical byte sealed at validate
/// (`Encoded`), the str literal held as text (`Interned`) — encoding work
/// left for the commit path is exactly the dictionary lookup, nothing
/// else.
#[test]
fn sigma_literals_seal_at_validate() {
    let schema = schema();
    let bool_sigma = schema
        .statement(TRANSFER_ACCOUNT)
        .checks
        .as_ref()
        .expect("containment");
    assert_eq!(
        bool_sigma.source.as_ref(),
        &[CompiledCheck::Encoded {
            field: FieldId(1),
            bytes: Box::new([1]),
        }]
    );
    assert!(bool_sigma.target.is_empty());
    let str_sigma = schema
        .statement(REPORT_ACCOUNT)
        .checks
        .as_ref()
        .expect("containment");
    assert_eq!(
        str_sigma.source.as_ref(),
        &[CompiledCheck::Interned {
            field: FieldId(1),
            text: "urgent".into(),
        }]
    );
    assert!(schema.statement(ACCOUNT_KEY).checks.is_none());
}

/// The `Interned`-miss path still yields `Never`: with "urgent" never
/// interned, no stored fact can satisfy the σ — while the sealed bool σ
/// materializes as a plain compare with zero dictionary traffic.
#[test]
fn an_uninterned_sigma_literal_resolves_to_never() {
    let dir = TempDir::new("sealed-never");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let view = env.read_txn().expect("txn");
    let delta = WriteDelta::new(&schema);
    let selections = Selections::encode(&delta, &view).expect("encode");
    assert!(matches!(
        selections.containment(REPORT_ACCOUNT).source,
        SelectionCheck::Never
    ));
    assert!(matches!(
        selections.containment(TRANSFER_ACCOUNT).source,
        SelectionCheck::Compare(_)
    ));
    assert!(matches!(
        selections.containment(TRANSFER_ACCOUNT).target,
        SelectionCheck::Empty
    ));
}

/// The replay matrix: one σ-bearing theory, a hand-built op stream, every
/// verdict typed and statement-id-exact — the staging move changed where
/// literals encode, never what the judgment says.
#[test]
fn a_sigma_bearing_stream_replays_the_same_verdicts() {
    let dir = TempDir::new("sealed-replay");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");

    // In-σ source without its target: the violation names the statement.
    let flagged = transfer(&schema, 9, true);
    let result = apply_delta(&env, &schema, &[], &[(TRANSFER, flagged.clone())]);
    let Err(Error::ContainmentViolation {
        statement,
        direction,
        fact: violating,
    }) = result
    else {
        panic!("expected a containment violation");
    };
    assert_eq!(statement, TRANSFER_ACCOUNT);
    assert_eq!(direction, Direction::SourceUnsatisfied);
    assert_eq!(*violating, *flagged);

    // Out-of-σ source: no edge, no probe, commits against the empty store.
    apply_delta(
        &env,
        &schema,
        &[],
        &[(TRANSFER, transfer(&schema, 9, false))],
    )
    .expect("a fact outside σ has no edge");

    // Target and in-σ source land together: the final state satisfies.
    apply_delta(
        &env,
        &schema,
        &[],
        &[(ACCOUNT, account(&schema, 9)), (TRANSFER, flagged)],
    )
    .expect("target and source in one delta");

    // The Never σ: "urgent" was never interned, so no Report fact can
    // satisfy the selection — an insert with a different interned note
    // commits although its would-be target is absent.
    let noted: Result<Vec<u8>> = (|| {
        let view = env.read_txn()?;
        let mut delta = WriteDelta::new(&schema);
        let note = delta.intern_str(&view, "routine")?;
        let bytes = fact(
            &schema,
            REPORT,
            &[ValueRef::U64(404), ValueRef::String(note)],
        );
        delta.insert(&view, REPORT, &bytes)?;
        drop(view);
        crate::storage::commit::commit(delta, &env)?;
        Ok(bytes)
    })();
    noted.expect("no fact can satisfy an uninterned σ — the edge never derives");
}
