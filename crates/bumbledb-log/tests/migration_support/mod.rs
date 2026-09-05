//! Shared fixtures for the migration_* integration lanes. Test support
//! only; nothing here is a production surface. Verification: `NotRun` (F1).
#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use bumbledb::schema::{
    FieldDescriptor, RelationDescriptor, RelationId, SchemaDescriptor, StatementDescriptor,
    ValueType,
};
use bumbledb::{Db, ExecutionPolicy, Id128, WorkContext};

use bumbledb_log::history::command::Limits;
use bumbledb_log::history::{DatabaseId, IncarnationId, OperationId};
use bumbledb_log::migration::manifest::{Manifest, append_entry};
use bumbledb_log::migration::plan::{
    FieldMap, Loss, Operation, Plan, PlanExpr, StepLabel, canonical_plan_bytes, plan_digest,
};
use bumbledb_log::schema_file::schema_id;

pub const LIMITS: Limits = Limits {
    envelope_bytes: 1_000_000,
    change_bytes: 900_000,
    evidence_bytes: 10_000,
    result_bytes: 1_000,
};

pub const CAP: usize = LIMITS.envelope_bytes;

static DIR_SEQ: AtomicU64 = AtomicU64::new(0);

pub fn temp_dir(tag: &str) -> PathBuf {
    let seq = DIR_SEQ.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let path = std::env::temp_dir().join(format!(
        "bdb-log-mig-{tag}-{}-{nanos}-{seq}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("create test root");
    path
}

pub fn policy() -> ExecutionPolicy {
    ExecutionPolicy {
        input_bytes: 10_000_000,
        working_bytes: 10_000_000,
        scratch_bytes: 10_000_000,
        result_bytes: 10_000_000,
        rows: 1_000_000,
        work_units: 100_000_000,
        timeout: Duration::from_secs(120),
    }
}

pub fn work() -> WorkContext {
    policy().start().unwrap()
}

/// A deliberately tiny allowance for exhaustion-mid-execution tests.
pub fn tiny_work() -> WorkContext {
    ExecutionPolicy {
        input_bytes: 4096,
        working_bytes: 4096,
        scratch_bytes: 4096,
        result_bytes: 4096,
        rows: 4,
        work_units: 64,
        timeout: Duration::from_secs(120),
    }
    .start()
    .unwrap()
}

pub fn op(byte: u8) -> OperationId {
    OperationId::from_core(Id128::from_bytes([byte; 16]))
}

pub fn db_id(byte: u8) -> DatabaseId {
    DatabaseId::from_core(Id128::from_bytes([byte; 16]))
}

pub fn incarnation(byte: u8) -> IncarnationId {
    IncarnationId::from_core(Id128::from_bytes([byte; 16]))
}

/// Base schema: `Note(id: u64, body: string)` with a key on `id`.
pub fn base_schema() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "Note".into(),
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
        }],
        statements: vec![StatementDescriptor::Functionality {
            relation: RelationId(0),
            projection: Box::new([bumbledb::FieldId(0)]),
        }],
    }
}

/// Step 1 target: `Note(id, body, pinned: bool)`.
pub fn pinned_schema() -> SchemaDescriptor {
    let mut schema = base_schema();
    schema.relations[0].fields.push(FieldDescriptor {
        name: "pinned".into(),
        value_type: ValueType::Bool,
    });
    schema
}

/// Step 2 target: `Note(id, body, pinned)` plus a new empty `Tag(name)`
/// with a key, seeded with one fixed row.
pub fn tagged_schema() -> SchemaDescriptor {
    let mut schema = pinned_schema();
    schema.relations.push(RelationDescriptor {
        name: "Tag".into(),
        fields: vec![FieldDescriptor {
            name: "name".into(),
            value_type: ValueType::String,
        }],
        extension: None,
    });
    schema.statements.push(StatementDescriptor::Functionality {
        relation: RelationId(1),
        projection: Box::new([bumbledb::FieldId(0)]),
    });
    schema
}

pub fn field(name: &str) -> PlanExpr {
    PlanExpr::Field(name.into())
}

pub fn copy_field(name: &str) -> FieldMap {
    FieldMap {
        target: name.into(),
        expression: field(name),
    }
}

pub fn literal(value: bumbledb::Value) -> PlanExpr {
    PlanExpr::Literal(value)
}

/// Plan 0: base -> pinned (add `pinned` defaulted false).
pub fn plan_pinned() -> Plan {
    Plan {
        sequence: 0,
        label: StepLabel::new("0000-note-pinned").unwrap(),
        from_schema: schema_id(&base_schema()).unwrap(),
        to_schema: schema_id(&pinned_schema()).unwrap(),
        operations: vec![
            Operation::MapRelation {
                source: "Note".into(),
                target: "Note".into(),
                fields: vec![
                    copy_field("id"),
                    copy_field("body"),
                    FieldMap {
                        target: "pinned".into(),
                        expression: literal(bumbledb::Value::Bool(false)),
                    },
                ],
            },
            Operation::ValidateSchema {
                schema: schema_id(&pinned_schema()).unwrap(),
            },
        ],
        destructive: vec![],
    }
}

/// Plan 1: pinned -> tagged (new empty `Tag`, one seed row).
pub fn plan_tagged() -> Plan {
    Plan {
        sequence: 1,
        label: StepLabel::new("0001-note-tags").unwrap(),
        from_schema: schema_id(&pinned_schema()).unwrap(),
        to_schema: schema_id(&tagged_schema()).unwrap(),
        operations: vec![
            Operation::MapRelation {
                source: "Note".into(),
                target: "Note".into(),
                fields: vec![copy_field("id"), copy_field("body"), copy_field("pinned")],
            },
            Operation::EmptyRelation {
                target: "Tag".into(),
            },
            Operation::Seed {
                target: "Tag".into(),
                rows: vec![Box::from([bumbledb::Value::String("seeded".into())])],
            },
            Operation::ValidateSchema {
                schema: schema_id(&tagged_schema()).unwrap(),
            },
        ],
        destructive: vec![],
    }
}

/// The two-entry manifest rooted at the base schema.
pub fn manifest() -> Manifest {
    let mut manifest = Manifest {
        base_schema: schema_id(&base_schema()).unwrap(),
        entries: vec![],
    };
    append_entry(&mut manifest, &plan_pinned(), CAP).unwrap();
    append_entry(&mut manifest, &plan_tagged(), CAP).unwrap();
    manifest
}

pub fn digest_of(plan: &Plan) -> [u8; 32] {
    plan_digest(&canonical_plan_bytes(plan, CAP).unwrap())
}

/// A destructive-intent example: pinned -> base (drops the `pinned` field).
pub fn plan_unpin(acknowledged: bool) -> Plan {
    Plan {
        sequence: 0,
        label: StepLabel::new("0000-unpin").unwrap(),
        from_schema: schema_id(&pinned_schema()).unwrap(),
        to_schema: schema_id(&base_schema()).unwrap(),
        operations: vec![
            Operation::MapRelation {
                source: "Note".into(),
                target: "Note".into(),
                fields: vec![copy_field("id"), copy_field("body")],
            },
            Operation::ValidateSchema {
                schema: schema_id(&base_schema()).unwrap(),
            },
        ],
        destructive: if acknowledged {
            vec![Loss {
                relation: "Note".into(),
                field: Some("pinned".into()),
            }]
        } else {
            vec![]
        },
    }
}

pub fn fresh_source(tag: &str) -> (Arc<Db<SchemaDescriptor>>, PathBuf) {
    let root = temp_dir(tag);
    let dir = root.join("source");
    let db = Arc::new(
        Db::create(&dir, base_schema(), work())
            .expect("create store")
            .expect("empty store admits"),
    );
    (db, root)
}
