//! Authored successor-store tests (F1: written, executed only in F3).
//!
//! Gate mapping (chapter 70 / audit 50):
//! - `lifecycle`      → G06 open/lock/close, old-family refusal before
//!   cleanup, format/layout/schema refusals, ENG-008 (E-DURABILITY: no
//!   `NO_SYNC` flag reachable).
//! - `snapshot_coherence` → ENG-003 (E-SNAPSHOT), ENG-006 (E-TEXT),
//!   coherent export under concurrent commits.
//! - `candidate_visibility` → E-VISIBILITY / PROTO-07 substrate, C04
//!   prepare/seal/commit, metadata-only decisions, rejected candidates,
//!   failed-seal-dispatches-nothing (G06 seal fault row).
//! - `collision`      → HASH-02 / Q-COLLISION substrate: forced constant
//!   fingerprints through insert/contains/delete/judgment/export, long
//!   values above the LMDB key bound.
//! - `resize`         → G06 elastic map: growth under load, pinned-reader
//!   blocked resize, growth ceiling refusal, map-full before/after seal.
//! - `crash`          → G06/E-DURABILITY process-death schedules: kill
//!   before/after durable commit, lock release on death. (True power-loss
//!   qualification is a separate authorized hardware gate.)

use std::time::Duration;

use bumbledb_theory::schema::RelationId;

use crate::schema::ProjectionId;

use crate::schema::{
    FieldDescriptor, RelationDescriptor, Schema, SchemaDescriptor, ValidateDescriptor as _,
    ValueType,
};
use crate::testutil::TempDir;
use crate::work::{ExecutionPolicy, WorkContext};
use crate::{ChangeSet, Value};

use super::candidate::{
    CandidateJudge, CandidateState, Judgment, Prepared, RowIndexer, StoreCommit,
};
use super::error::{StoreError, StoreResult};
use super::host::{AttachmentChange, HostChanges, HostRecordChange};
use super::map::MapPolicy;
use super::store_env::Store;

mod admission;
mod compiled_projection;
mod fresh_adoption;
mod candidate_visibility;
mod collision;
mod crash;
mod incremental;
mod judged;
mod large;
mod lifecycle;
mod resize;
mod schema_indexed;
mod snapshot_coherence;

pub(super) const NOTE: RelationId = RelationId(0);
pub(super) const TAG: RelationId = RelationId(1);
pub(super) const NOTE_KEY: ProjectionId = ProjectionId(0);

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
        statements: vec![],
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
    ExecutionPolicy {
        input_bytes: 1 << 30,
        working_bytes: 1 << 30,
        scratch_bytes: 1 << 30,
        result_bytes: 1 << 30,
        rows: 1 << 24,
        work_units: 1 << 40,
        timeout: Duration::from_secs(120),
    }
    .start()
    .expect("work context")
}

pub(super) fn short_work(timeout: Duration) -> WorkContext {
    ExecutionPolicy {
        input_bytes: 1 << 30,
        working_bytes: 1 << 30,
        scratch_bytes: 1 << 30,
        result_bytes: 1 << 30,
        rows: 1 << 24,
        work_units: 1 << 40,
        timeout,
    }
    .start()
    .expect("short work context")
}

/// Tiny-map policy for forced growth schedules.
pub(super) fn tiny_map() -> MapPolicy {
    MapPolicy {
        initial_map_bytes: 1 << 20,
        max_map_bytes: None,
    }
}

/// Indexes nothing: relations with no declared key statements.
pub(super) struct NoIndex;

impl RowIndexer for NoIndex {
    fn index_row(
        &self,
        _relation: RelationId,
        _row: &[u8],
        _work: &WorkContext,
        _        emit: &mut dyn FnMut(ProjectionId, &[u8], Option<&[u8]>) -> StoreResult<()>,
    ) -> StoreResult<()> {
        let _ = emit;
        Ok(())
    }
}

/// Projects the first canonical field of every `note` row as the
/// `NOTE_KEY` determinant, so two notes with one id land in one bucket.
pub(super) struct FirstFieldKey;

impl RowIndexer for FirstFieldKey {
    fn index_row(
        &self,
        relation: RelationId,
        row: &[u8],
        work: &WorkContext,
        emit: &mut dyn FnMut(ProjectionId, &[u8], Option<&[u8]>) -> StoreResult<()>,
    ) -> StoreResult<()> {
        if relation != NOTE {
            return Ok(());
        }
        let decoded = crate::canonical::decode(schema().relation(NOTE).fields(), row, work)?;
        let Value::U64(id) = decoded.values()[0] else {
            panic!("test rows lead with a u64 id");
        };
        emit(NOTE_KEY, &id.to_be_bytes(), None)
    }
}

/// Admits everything; the store-level tests exercise physical behavior.
pub(super) struct AdmitAll;

impl CandidateJudge for AdmitAll {
    type Rejection = std::convert::Infallible;

    fn judge(
        &self,
        _candidate: &CandidateState<'_, '_>,
        _work: &WorkContext,
    ) -> StoreResult<Judgment<Self::Rejection>> {
        Ok(Judgment::Admitted)
    }
}

/// A miniature key law over the proposed FINAL state: for every added note,
/// the `NOTE_KEY` determinant bucket must hold exactly one row. Because the
/// determinant namespace is a multimap, both competing rows are visible to
/// this judge before any decision — the ENG-005 physical precondition.
pub(super) struct UniqueNoteId;

impl CandidateJudge for UniqueNoteId {
    /// The competing local row ids the judge saw, proving evidence for all
    /// conflicting proposals is available (not just "an install failed").
    type Rejection = Vec<super::format::RowId>;

    fn judge(
        &self,
        candidate: &CandidateState<'_, '_>,
        work: &WorkContext,
    ) -> StoreResult<Judgment<Self::Rejection>> {
        let Some(changes) = candidate.changes() else {
            return Ok(Judgment::Admitted);
        };
        for record in changes.records() {
            if record.relation != NOTE || record.kind != crate::changes::ChangeKind::Add {
                continue;
            }
            let decoded =
                crate::canonical::decode(schema().relation(NOTE).fields(), record.row, work)?;
            let Value::U64(id) = decoded.values()[0] else {
                panic!("test rows lead with a u64 id");
            };
            let bucket = candidate.determinant_candidates(NOTE_KEY, &id.to_be_bytes(), work)?;
            // Exact recheck: forced-collision tests put unrelated rows in
            // the same bucket; only true id matches count against the law.
            let mut matching = Vec::new();
            for row_id in bucket {
                let row = candidate
                    .fetch(NOTE, row_id)?
                    .expect("bucket entries resolve");
                let row = crate::canonical::decode(schema().relation(NOTE).fields(), row, work)?;
                if row.values()[0] == Value::U64(id) {
                    matching.push(row_id);
                }
            }
            if matching.len() > 1 {
                return Ok(Judgment::Rejected(matching));
            }
        }
        Ok(Judgment::Admitted)
    }
}

pub(super) fn note(id: u64, body: &str) -> Vec<Value> {
    vec![Value::U64(id), Value::String(body.into())]
}

pub(super) fn tag(label: &str) -> Vec<Value> {
    vec![Value::String(label.into())]
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

pub(super) const NO_HOST: HostChanges<'static> = HostChanges {
    records: &[],
    attachment: AttachmentChange::Keep,
};

/// Prepare/seal/commit one delta through the full private-candidate path.
pub(super) fn commit_changes(store: &Store, changes: &ChangeSet) -> StoreCommit {
    try_commit_changes(store, changes).expect("committed changes")
}

pub(super) fn try_commit_changes(
    store: &Store,
    changes: &ChangeSet,
) -> Result<StoreCommit, StoreError> {
    let context = work();
    let mut owner = store.writer(&context)?;
    match owner.prepare(changes, &FirstFieldKey, &AdmitAll)? {
        Prepared::Admitted(prepared) => prepared.seal(NO_HOST)?.commit(),
        Prepared::Rejected(never) => match never {},
    }
}

/// One host-record put helper for receipt-shaped tests.
pub(super) fn host_put<'a>(key: &'a [u8], value: &'a [u8]) -> [HostRecordChange<'a>; 1] {
    [HostRecordChange::Put { key, value }]
}

pub(super) fn store_dir(tag: &str) -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new(tag);
    let path = dir.path().join("store");
    std::fs::create_dir_all(dir.path()).expect("test parent dir");
    (dir, path)
}

pub(super) fn open_default(path: &std::path::Path) -> Store {
    Store::open(path, &schema(), MapPolicy::default()).expect("open store")
}

pub(super) fn create_default(path: &std::path::Path) -> Store {
    Store::create(path, &schema(), MapPolicy::default())
        .expect("create store")
        .0
}
