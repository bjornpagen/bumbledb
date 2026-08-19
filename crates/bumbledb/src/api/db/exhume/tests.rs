use crate::error::{CorruptionError, Error};
use crate::ir::Value;
use crate::schema::ValidateDescriptor as _;
use crate::storage::env::{Environment, FORMAT_VERSION, StoreKind};
use crate::testutil::TempDir;
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
        Value::String(title.into()),
        Value::FixedBytes(Box::from(&b"abcd"[..])),
        Value::IntervalU64(Interval::<u64>::new(2, 5).expect("interval")),
        Value::U64(status),
    ]
}

/// Builds a populated store and drops the handle (the advisory lock must
/// release before exhume re-opens the path). Two commits, one fact each:
/// row ids follow commit order, so the scan order below is pinned.
fn build_store(dir: &TempDir) {
    let db = Db::create(dir.path(), theory())
        .expect("create")
        .expect("accepted");
    db.write(|tx| tx.insert_dyn(NOTE, [&note(1, "alpha", 0)]).map(|_| ()))
        .expect("write")
        .unwrap();
    db.write(|tx| tx.insert_dyn(NOTE, [&note(2, "beta", 1)]).map(|_| ()))
        .expect("write")
        .unwrap();
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
fn a_missing_descriptor_is_meta_missing_on_every_open_surface() {
    let dir = TempDir::new("exhume-missing-descriptor");
    build_store(&dir);
    let schema = theory().validate().expect("valid fixture");
    let env = Environment::open(dir.path(), &schema).expect("raw open");
    env.strip_schema_descriptor_for_tests().expect("strip");
    drop(env);

    // Format 8: the descriptor is a required `_meta` key. No adoption
    // back-fill remains — that was the format-7 decoder.
    for err in [
        exhume(dir.path()).map(|_| ()).unwrap_err(),
        Db::open(dir.path(), theory()).map(|_| ()).unwrap_err(),
        Environment::open(dir.path(), &schema)
            .map(|_| ())
            .unwrap_err(),
    ] {
        assert!(
            matches!(err, Error::Corruption(CorruptionError::MetaMissing)),
            "{err:?}"
        );
    }
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

    // Format 8 open reads the descriptor: hash disagreement is typed
    // corruption on every surface. Open never rewrites the bytes.
    for err in [
        exhume(dir.path()).map(|_| ()).unwrap_err(),
        Db::open(dir.path(), theory()).map(|_| ()).unwrap_err(),
        Environment::open(dir.path(), &schema)
            .map(|_| ())
            .unwrap_err(),
    ] {
        assert!(
            matches!(
                err,
                Error::Corruption(CorruptionError::DescriptorFingerprintDesync { .. })
            ),
            "{err:?}"
        );
    }
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
        Err(Error::FormatMismatch { mismatch }) => {
            assert_eq!(mismatch.witnessed, FORMAT_VERSION + 1);
            assert_eq!(mismatch.required, FORMAT_VERSION);
        }
        other => panic!("expected FormatMismatch, got {other:?}"),
    }
}

#[test]
fn an_ephemeral_store_exhumes_too_and_reports_its_kind() {
    let dir = TempDir::new("exhume-ephemeral");
    {
        let db = Db::ephemeral(dir.path(), theory())
            .expect("ephemeral")
            .expect("accepted");
        db.write(|tx| tx.insert_dyn(NOTE, [&note(7, "gamma", 0)]).map(|_| ()))
            .expect("write")
            .unwrap();
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
                            Value::String(Box::from("alpha")),
                            Value::String(Box::from("beta")),
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
    drop(
        Db::create(dir.path(), declared.clone())
            .expect("create")
            .expect("accepted"),
    );
    let exhumed = exhume(dir.path()).expect("exhume");
    assert_eq!(*exhumed.descriptor(), declared);
}
