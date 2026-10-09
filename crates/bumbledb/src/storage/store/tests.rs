//! Store tests, by area:
//! - `lifecycle`: create, open, format and schema refusal, locks, close.
//! - `candidate`: private candidates, judgment over the final state, seals.
//! - `ceiling`: the fixed virtual map.
//! - `collision`: forced fingerprints through every row and index path.
//! - `indexed`: determinant entries and keyed groups.
//! - `coherence`: snapshot isolation, export order and content digests.
//! - `images`: compacted images and their installation.
//! - `incremental`: incremental against complete judgment.
//! - `crash`: process death around the commit boundary.

use bumbledb_theory::schema::RelationId;

use crate::schema::{
    FieldDescriptor, RelationDescriptor, Schema, SchemaDescriptor, ValidateDescriptor as _,
    ValueType,
};
use crate::testutil::TempDir;
use crate::work::WorkContext;
use crate::{ChangeSet, Value};

use super::candidate::{Candidate, Commit};
use super::error::{StoreError, StoreResult};
use super::format::DatabaseId;
use super::host::{HostChanges, HostRecord};
use super::store_env::{Options, Store};

mod candidate;
mod ceiling;
mod coherence;
mod collision;
mod crash;
mod images;
mod incremental;
mod indexed;
mod lifecycle;

pub(super) const NOTE: RelationId = RelationId(0);
pub(super) const TAG: RelationId = RelationId(1);

/// `note { id: u64, body: str }` keyed on `id`, and `tag { label: str }`.
pub(super) fn schema() -> Schema {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "note".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                    },
                    FieldDescriptor {
                        name: "body".into(),
                        value_type: ValueType::String,
                    },
                ],
                extension: None,
            },
            RelationDescriptor {
                name: "tag".into(),
                fields: vec![FieldDescriptor {
                    name: "label".into(),
                    value_type: ValueType::String,
                }],
                extension: None,
            },
        ],
        statements: vec![
            bumbledb_theory::schema::StatementDescriptor::Functionality {
                relation: NOTE,
                projection: Box::from([bumbledb_theory::schema::FieldId(0)]),
            },
        ],
    }
    .validate()
    .expect("test schema validates")
}

pub(super) fn other_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "unrelated".into(),
            fields: vec![FieldDescriptor {
                name: "n".into(),
                value_type: ValueType::I64,
            }],
            extension: None,
        }],
        statements: vec![],
    }
    .validate()
    .expect("other schema validates")
}

pub(super) fn work() -> WorkContext {
    WorkContext::new()
}

/// A ceiling small enough for deterministic `Full` schedules.
pub(super) const SMALL_CEILING: u64 = 1 << 20;

pub(super) fn note(id: u64, body: &str) -> Vec<Value> {
    vec![Value::U64(id), Value::String(body.into())]
}

pub(super) fn tag(label: &str) -> Vec<Value> {
    vec![Value::String(label.into())]
}

pub(super) fn row_bytes(schema: &Schema, relation: RelationId, values: &[Value]) -> Vec<u8> {
    crate::canonical::CanonicalRow::encode(schema.relation(relation).fields(), values, &work())
        .expect("canonical row")
        .as_bytes()
        .to_vec()
}

pub(super) fn change_set(
    schema: &Schema,
    adds: &[(RelationId, Vec<Value>)],
    removes: &[(RelationId, Vec<Value>)],
) -> ChangeSet {
    let mut builder = ChangeSet::builder(schema, work());
    for (relation, values) in removes {
        builder.delete(*relation, values).expect("stage delete");
    }
    for (relation, values) in adds {
        builder.insert(*relation, values).expect("stage insert");
    }
    builder.finish().expect("sealed change set")
}

/// Apply, seal and commit one change set unjudged.
pub(super) fn commit_changes(store: &Store, changes: &ChangeSet) -> Commit {
    try_commit_changes(store, changes).expect("committed changes")
}

pub(super) fn try_commit_changes(store: &Store, changes: &ChangeSet) -> StoreResult<Commit> {
    let context = work();
    let mut owner = store.writer(&context)?;
    owner
        .prepare_decided(std::slice::from_ref(changes))?
        .seal(HostChanges::NONE)?
        .commit()
}

/// Judge, then commit when admitted; the rejection otherwise.
pub(super) fn judged_commit(
    store: &Store,
    schema: &Schema,
    changes: &ChangeSet,
) -> Result<Commit, Box<[crate::schema::judge::JudgedViolation]>> {
    let context = work();
    let mut owner = store.writer(&context).expect("writer");
    match owner.prepare_judged(schema, changes).expect("prepare") {
        Candidate::Admitted(prepared) => Ok(prepared
            .seal(HostChanges::NONE)
            .expect("seal")
            .commit()
            .expect("commit")),
        Candidate::Rejected(violations) => Err(violations),
    }
}

pub(super) fn host_put<'a>(key: &'a [u8], value: &'a [u8]) -> [HostRecord<'a>; 1] {
    [HostRecord::Put { key, value }]
}

pub(super) fn store_dir(tag: &str) -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new(tag);
    let path = dir.path().join("store");
    std::fs::create_dir_all(dir.path()).expect("test parent dir");
    (dir, path)
}

pub(super) fn open_default(path: &std::path::Path) -> Store {
    Store::open(path, &schema(), Options::default()).expect("open store")
}

pub(super) fn create_default(path: &std::path::Path) -> Store {
    create_with(path, &schema())
}

pub(super) fn create_with(path: &std::path::Path, schema: &Schema) -> Store {
    Store::create(path, schema, DatabaseId::mint(), Options::default()).expect("create store")
}

pub(super) fn small_ceiling() -> Options {
    Options {
        map_ceiling: SMALL_CEILING,
        ..Options::default()
    }
}

pub(super) fn assert_full<T>(result: StoreResult<T>) {
    match result {
        Err(StoreError::Full { .. }) => {}
        Err(other) => panic!("expected Full, got {other:?}"),
        Ok(_) => panic!("expected Full, got success"),
    }
}
