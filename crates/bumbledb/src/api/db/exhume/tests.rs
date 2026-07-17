use crate::error::{CorruptionError, Error};
use crate::ir::Value;
use crate::schema::ValidateDescriptor as _;
use crate::storage::env::{Environment, FORMAT_VERSION, StoreKind};
use crate::testutil::TempDir;
use crate::verify_store::StoreFinding;
use crate::{Db, exhume};
use bumbledb_theory::Interval;
use bumbledb_theory::schema::{
    FieldDescriptor, Generation, IntervalElement, LiteralSet, RelationDescriptor, RelationId, Row,
    SchemaDescriptor, Side, StatementDescriptor, ValueType,
};

fn field(name: &str, value_type: ValueType) -> FieldDescriptor {
    FieldDescriptor {
        name: name.into(),
        value_type,
        generation: Generation::None,
    }
}

/// Status { flag: bool } = { On, Off } + Note(id fresh, title str,
/// digest bytes<4>, at interval<u64>, status u64), with
/// `Note(status) <= Status(id)` — every decode lane a scan exercises
/// (str via `_dict`, inline bytes, intervals, bool, u64) plus a closed
/// roster and a real dependency.
fn theory() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "Status".into(),
                fields: vec![field("flag", ValueType::Bool)],
                extension: Some(Box::new([
                    Row {
                        handle: "On".into(),
                        values: Box::new([Value::Bool(true)]),
                    },
                    Row {
                        handle: "Off".into(),
                        values: Box::new([Value::Bool(false)]),
                    },
                ])),
            },
            RelationDescriptor {
                name: "Note".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                        generation: Generation::Fresh,
                    },
                    field("title", ValueType::String),
                    field("digest", ValueType::FixedBytes { len: 4 }),
                    field(
                        "at",
                        ValueType::Interval {
                            element: IntervalElement::U64,
                            width: None,
                        },
                    ),
                    field("status", ValueType::U64),
                ],
                extension: None,
            },
        ],
        statements: vec![StatementDescriptor::Containment {
            source: Side {
                relation: RelationId(1),
                projection: Box::new([bumbledb_theory::schema::FieldId(4)]),
                selection: Box::new([]),
            },
            target: Side {
                relation: RelationId(0),
                projection: Box::new([bumbledb_theory::schema::FieldId(0)]),
                selection: Box::new([]),
            },
        }],
    }
}

const NOTE: RelationId = RelationId(1);

fn note(id: u64, title: &str, status: u64) -> Vec<Value> {
    vec![
        Value::U64(id),
        Value::String(title.as_bytes().into()),
        Value::FixedBytes(Box::from(&b"abcd"[..])),
        Value::IntervalU64(Interval::<u64>::new(2, 5).expect("interval")),
        Value::U64(status),
    ]
}

/// Builds a populated store and drops the handle (the advisory lock must
/// release before exhume re-opens the path). Two commits, one fact each:
/// row ids follow commit order, so the scan order below is pinned.
fn build_store(dir: &TempDir) {
    let db = Db::create(dir.path(), theory()).expect("create");
    db.write(|tx| tx.insert_dyn(NOTE, &note(1, "alpha", 0)).map(|_| ()))
        .expect("write");
    db.write(|tx| tx.insert_dyn(NOTE, &note(2, "beta", 1)).map(|_| ()))
        .expect("write");
}

#[test]
fn create_then_exhume_reads_every_relation_field_and_row_with_no_theory() {
    let dir = TempDir::new("exhume-roundtrip");
    build_store(&dir);

    let exhumed = exhume(dir.path()).expect("exhume");
    // The descriptor IS the declaration: names, types, and the closed
    // roster come back exactly as declared.
    assert_eq!(*exhumed.descriptor(), theory());
    assert_eq!(exhumed.kind(), StoreKind::Durable);
    let names: Vec<&str> = exhumed
        .descriptor()
        .relations
        .iter()
        .map(|relation| relation.name.as_ref())
        .collect();
    assert_eq!(names, ["Status", "Note"]);
    assert_eq!(
        exhumed.descriptor().relations[1]
            .fields
            .iter()
            .map(|f| f.name.as_ref())
            .collect::<Vec<_>>(),
        ["id", "title", "digest", "at", "status"]
    );

    // Every row of every relation, readable with no theory in scope —
    // the ordinary relation from `F` (str resolved through `_dict`), the
    // closed relation from its sealed roster.
    let notes = exhumed
        .read(|snap| {
            snap.scan(exhumed.relation("Note").expect("Note resolves"))?
                .collect::<crate::error::Result<Vec<_>>>()
        })
        .expect("scan Note");
    assert_eq!(notes, vec![note(1, "alpha", 0), note(2, "beta", 1)]);
    let statuses = exhumed
        .read(|snap| {
            snap.scan(exhumed.relation("Status").expect("Status resolves"))?
                .collect::<crate::error::Result<Vec<_>>>()
        })
        .expect("scan Status");
    assert_eq!(
        statuses,
        vec![
            vec![Value::U64(0), Value::Bool(true)],
            vec![Value::U64(1), Value::Bool(false)],
        ]
    );
    assert_eq!(exhumed.relation("Ghost"), None);
}

#[test]
fn a_pre_descriptor_store_refuses_exhume_then_one_open_adopts_it() {
    let dir = TempDir::new("exhume-backfill");
    build_store(&dir);
    // The pre-descriptor fixture: strip the persisted descriptor,
    // reproducing the exact on-disk shape of a store created before
    // descriptors existed.
    let schema = theory().validate().expect("valid fixture");
    let env = Environment::open(dir.path(), &schema).expect("raw open");
    env.strip_schema_descriptor_for_tests().expect("strip");
    drop(env);

    // Not yet adopted: the typed refusal names the remedy.
    match exhume(dir.path()).map(|_| ()) {
        Err(Error::DescriptorMissing) => {}
        other => panic!("expected DescriptorMissing, got {other:?}"),
    }

    // One successful fingerprint-matching open back-fills the
    // descriptor — the adoption path.
    drop(Db::open(dir.path(), theory()).expect("adopting open"));

    // Self-describing forever: exhume now succeeds and the descriptor
    // is the declaration.
    let exhumed = exhume(dir.path()).expect("exhume after adoption");
    assert_eq!(*exhumed.descriptor(), theory());
}

#[test]
fn a_desynced_descriptor_is_an_exhume_corruption_and_a_verify_store_conviction() {
    let dir = TempDir::new("exhume-desync");
    build_store(&dir);
    let schema = theory().validate().expect("valid fixture");
    let env = Environment::open(dir.path(), &schema).expect("raw open");
    env.overwrite_schema_descriptor_for_tests(b"not the canonical bytes")
        .expect("overwrite");
    drop(env);

    // Exhume's integrity gate: hash disagreement is typed corruption.
    match exhume(dir.path()).map(|_| ()) {
        Err(Error::Corruption(CorruptionError::DescriptorFingerprintDesync { .. })) => {}
        other => panic!("expected DescriptorFingerprintDesync, got {other:?}"),
    }

    // The ordinary open still verifies its fingerprint (untouched) and
    // must NOT silently repair the present-but-wrong descriptor; the
    // sweeper convicts it.
    let db = Db::open(dir.path(), theory()).expect("open under the theory");
    let report = db.verify_store().expect("sweep");
    assert!(
        report
            .findings
            .iter()
            .any(|finding| matches!(finding, StoreFinding::DescriptorFingerprintDesync { .. })),
        "expected the descriptor conviction, got {:?}",
        report.findings
    );
}

#[test]
fn exhume_of_a_nonexistent_path_is_the_io_refusal() {
    let dir = TempDir::new("exhume-nonexistent");
    match exhume(&dir.path().join("no-such-store")).map(|_| ()) {
        Err(Error::Io(_)) => {}
        other => panic!("expected Io, got {other:?}"),
    }
}

#[test]
fn exhume_of_a_version_mismatched_store_is_the_format_refusal() {
    let dir = TempDir::new("exhume-version");
    build_store(&dir);
    let schema = theory().validate().expect("valid fixture");
    let env = Environment::open(dir.path(), &schema).expect("raw open");
    env.force_format_version_for_tests(FORMAT_VERSION + 1)
        .expect("force version");
    drop(env);

    match exhume(dir.path()).map(|_| ()) {
        Err(Error::FormatMismatch { found, expected }) => {
            assert_eq!(found, FORMAT_VERSION + 1);
            assert_eq!(expected, FORMAT_VERSION);
        }
        other => panic!("expected FormatMismatch, got {other:?}"),
    }
}

#[test]
fn an_ephemeral_store_exhumes_too_and_reports_its_kind() {
    let dir = TempDir::new("exhume-ephemeral");
    {
        let db = Db::ephemeral(dir.path(), theory()).expect("ephemeral");
        db.write(|tx| tx.insert_dyn(NOTE, &note(7, "gamma", 0)).map(|_| ()))
            .expect("write");
    }
    let exhumed = exhume(dir.path()).expect("exhume");
    assert_eq!(exhumed.kind(), StoreKind::Ephemeral);
    let notes = exhumed
        .read(|snap| {
            snap.scan(exhumed.relation("Note").expect("Note resolves"))?
                .collect::<crate::error::Result<Vec<_>>>()
        })
        .expect("scan");
    assert_eq!(notes, vec![note(7, "gamma", 0)]);
}

#[test]
fn a_selection_carrying_theory_survives_the_exhume_round_trip() {
    // The literal-decode lanes (str, set, interval literals) through a
    // real store: descriptor equality after create → exhume.
    let declared = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "Holder".into(),
            fields: vec![
                FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                    generation: Generation::Fresh,
                },
                field("name", ValueType::String),
                field(
                    "at",
                    ValueType::Interval {
                        element: IntervalElement::U64,
                        width: None,
                    },
                ),
            ],
            extension: None,
        }],
        statements: vec![StatementDescriptor::Containment {
            source: Side {
                relation: RelationId(0),
                projection: Box::new([bumbledb_theory::schema::FieldId(0)]),
                selection: Box::new([
                    (
                        bumbledb_theory::schema::FieldId(1),
                        LiteralSet::Many(Box::new([
                            Value::String(Box::from(&b"alpha"[..])),
                            Value::String(Box::from(&b"beta"[..])),
                        ])),
                    ),
                    (
                        bumbledb_theory::schema::FieldId(2),
                        LiteralSet::One(Value::IntervalU64(
                            Interval::<u64>::new(5, u64::MAX).expect("ray"),
                        )),
                    ),
                ]),
            },
            target: Side {
                relation: RelationId(0),
                projection: Box::new([bumbledb_theory::schema::FieldId(0)]),
                selection: Box::new([]),
            },
        }],
    };
    let dir = TempDir::new("exhume-selections");
    drop(Db::create(dir.path(), declared.clone()).expect("create"));
    let exhumed = exhume(dir.path()).expect("exhume");
    assert_eq!(*exhumed.descriptor(), declared);
}
