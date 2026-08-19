use super::{Db, complete_admit_empty};
use crate::api::db::InstanceBuilder;
use crate::error::{Admission, Direction, Error, Mismatch, Violation};
use crate::ir::Value;
use crate::schema::ValidateDescriptor as _;
use crate::schema::fingerprint::fingerprint;
use crate::schema::tests::{closed, containment, fd, field, row, side};
use crate::storage::catalog::{
    Bounds, CatalogMap, CatalogRead, LmdbReadCatalog, OrderedRead, ReadCursor,
};
use crate::storage::env::{Environment, FORMAT_VERSION};
use crate::testutil::TempDir;
use bumbledb_theory::schema::{
    FieldId, RelationDescriptor, RelationId, SchemaDescriptor, ValueType,
};

fn empty_holds() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Item".into(),
            fields: vec![field("id", ValueType::U64)],
        }],
        statements: vec![fd(RelationId(0), &[FieldId(0)])],
    }
}

fn empty_does_not_hold() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            closed(
                "Kind",
                vec![],
                vec![row("Soft", vec![]), row("Hard", vec![])],
            ),
            RelationDescriptor {
                extension: None,
                name: "Bucket".into(),
                fields: vec![field("id", ValueType::U64)],
            },
        ],
        statements: vec![
            fd(RelationId(1), &[FieldId(0)]),
            containment(
                side(RelationId(0), &[FieldId(0)]),
                side(RelationId(1), &[FieldId(0)]),
            ),
        ],
    }
}

#[test]
fn a_new_empty_create_is_format_8() {
    assert_eq!(FORMAT_VERSION, 8);
    let dir = TempDir::new("db-create-format-8");
    let db = Db::create(dir.path(), empty_holds())
        .expect("create")
        .expect("accepted");
    drop(db);
    Environment::open(dir.path(), &empty_holds().validate().expect("valid"))
        .expect("format 8 opens");
}

#[test]
fn opening_a_format_7_store_fails_closed() {
    let dir = TempDir::new("db-open-format-7");
    let db = Db::create(dir.path(), empty_holds())
        .expect("create")
        .expect("accepted");
    db.env()
        .force_format_version_for_tests(7)
        .expect("backdate");
    drop(db);
    let err = Db::open(dir.path(), empty_holds()).map(|_| ()).unwrap_err();
    assert!(
        matches!(
            err,
            Error::FormatMismatch {
                mismatch: Mismatch {
                    witnessed: 7,
                    required: 8,
                },
            }
        ),
        "{err:?}"
    );
    let err = Db::ephemeral(dir.path(), empty_holds())
        .map(|_| ())
        .unwrap_err();
    assert!(
        matches!(
            err,
            Error::FormatMismatch {
                mismatch: Mismatch {
                    witnessed: 7,
                    required: 8,
                },
            }
        ),
        "{err:?}"
    );
}

#[test]
fn empty_that_does_not_hold_is_violations_and_mints_no_lease() {
    let schema = empty_does_not_hold()
        .validate()
        .expect("closed source against ordinary target validates");
    let Admission::Rejected(violations) = complete_admit_empty(&schema).expect("admit") else {
        panic!("complete roster must reject empty; incremental would accept");
    };
    assert!(
        violations.iter().any(|v| matches!(
            v,
            Violation::Containment {
                direction: Direction::SourceUnsatisfied,
                ..
            }
        )),
        "{violations}"
    );

    let dir = TempDir::new("db-create-unsat-empty");
    let path = dir.path().join("store");
    match Db::create(&path, empty_does_not_hold()).expect("create") {
        Admission::Rejected(create_violations) => {
            assert_eq!(create_violations, violations);
        }
        Admission::Accepted(_) => panic!("unsatisfiable empty must not mint a lease"),
    }
    assert!(
        !path.exists(),
        "complete-admit runs before any directory is created"
    );

    let ephemeral_path = dir.path().join("ephemeral");
    match Db::ephemeral(&ephemeral_path, empty_does_not_hold()).expect("ephemeral") {
        Admission::Rejected(ephemeral_violations) => {
            assert_eq!(ephemeral_violations, violations);
        }
        Admission::Accepted(_) => panic!("unsatisfiable empty must not mint an ephemeral lease"),
    }
    assert!(
        !ephemeral_path.exists(),
        "rejected empty ephemeral creates no directory"
    );
}

fn dump_map(catalog: &impl OrderedRead, map: CatalogMap) -> Vec<(Vec<u8>, Vec<u8>)> {
    let mut range = catalog.range(map, Bounds::all()).expect("range");
    let mut out = Vec::new();
    while let Some(entry) = range.next().expect("next") {
        out.push((entry.key.to_vec(), entry.value.to_vec()));
    }
    out
}

#[test]
fn from_instance_writes_format_8_and_reopens() {
    let mut builder = InstanceBuilder::new(empty_holds()).expect("valid");
    builder
        .load_dyn(RelationId(0), [&[Value::U64(7)]])
        .expect("load");
    let instance = builder.admit().expect("admit").expect("accepted");

    let dir = TempDir::new("db-from-instance-format-8");
    let path = dir.path().join("store");
    let db = Db::from_instance(&path, &instance).expect("publish");
    drop(db);

    let reopened = Db::open(&path, empty_holds()).expect("reopen format 8");
    drop(reopened);

    assert_eq!(FORMAT_VERSION, 8);
    let schema = empty_holds().validate().expect("valid");
    let env = Environment::open(&path, &schema).expect("env open");
    let rtxn = env.read_txn().expect("read");
    assert_eq!(rtxn.generation().expect("generation").value(), 0);
    assert_eq!(
        rtxn.stored_fingerprint().expect("fingerprint"),
        fingerprint(&schema).0
    );
    assert_eq!(
        rtxn.dict_next_id().expect("dict next"),
        instance.catalog().dict_next_id().expect("dict next").raw()
    );

    let live = LmdbReadCatalog::new(&rtxn);
    assert_eq!(
        dump_map(instance.catalog(), CatalogMap::Data),
        dump_map(&live, CatalogMap::Data),
        "raw _data copy"
    );
    assert_eq!(
        dump_map(instance.catalog(), CatalogMap::Dictionary),
        dump_map(&live, CatalogMap::Dictionary),
        "raw _dict copy"
    );
}

#[test]
fn from_instance_refuses_an_occupied_path() {
    let instance = InstanceBuilder::new(empty_holds())
        .expect("valid")
        .admit()
        .expect("admit")
        .expect("accepted");
    let dir = TempDir::new("db-from-instance-occupied");
    let path = dir.path().join("store");
    std::fs::create_dir_all(&path).expect("mkdir");
    let err = Db::from_instance(&path, &instance).map(|_| ()).unwrap_err();
    assert!(matches!(err, Error::DestinationExists { .. }), "{err:?}");
}

#[test]
fn from_instance_does_not_rejudge_empty() {
    let mut builder = InstanceBuilder::new(empty_does_not_hold()).expect("valid");
    builder
        .load_dyn(RelationId(1), [&[Value::U64(0)], &[Value::U64(1)]])
        .expect("cover closed source");
    let instance = builder.admit().expect("admit").expect("accepted");

    let dir = TempDir::new("db-from-instance-no-rejudge");
    let path = dir.path().join("store");
    let db = Db::from_instance(&path, &instance).expect("raw copy of a known-admitted instance");
    drop(db);
    Db::open(&path, empty_does_not_hold()).expect("reopen");
}
