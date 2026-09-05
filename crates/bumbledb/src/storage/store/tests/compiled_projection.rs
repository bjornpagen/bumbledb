//! Exact-bounded vs fingerprint projection encoding (chapter 40 table).

use super::*;
use crate::schema::compiled::{CompiledTheory, KeyEncoding};
use crate::schema::{FieldDescriptor, RelationDescriptor, SchemaDescriptor, StatementDescriptor};
use crate::storage::store::det_index::determinant_bytes;
use crate::storage::store::keys::{DETERMINANT_KEY_MIN_LEN, TAG_DETERMINANT};
use bumbledb_theory::schema::{FieldId, RelationId, StatementId};

#[test]
fn u64_key_uses_exact_bounded_routing_bytes() {
    let schema = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "T".into(),
            fields: vec![FieldDescriptor {
                name: "id".into(),
                value_type: ValueType::U64,
            }],
            extension: None,
        }],
        statements: vec![StatementDescriptor::Functionality {
            relation: RelationId(0),
            projection: Box::from([FieldId(0)]),
        }],
    }
    .validate()
    .expect("valid");
    let store = Store::create(
        &TempDir::new("exact-u64").path().join("store"),
        &schema,
        MapPolicy::default(),
    )
    .expect("create")
    .0;
    let proj = store
        .snapshot(&work())
        .expect("snap")
        .compiled()
        .projection(crate::schema::ProjectionId(0))
        .expect("proj");
    assert!(matches!(
        proj.encoding,
        KeyEncoding::ExactBounded { scalar_width: 8 }
    ));
    let projected = determinant_bytes(proj, &[Value::U64(42)], &work()).expect("project");
    assert_eq!(projected.len(), 8, "exact u64 routing is 8 bytes");
    let key_len = 1 + 2 + projected.len() + 8;
    assert_eq!(key_len, 19, "raw determinant key matches chapter 40 table");
}

#[test]
fn text_key_uses_fingerprint_routing() {
    let schema = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "T".into(),
            fields: vec![FieldDescriptor {
                name: "text".into(),
                value_type: ValueType::String,
            }],
            extension: None,
        }],
        statements: vec![StatementDescriptor::Functionality {
            relation: RelationId(0),
            projection: Box::from([FieldId(0)]),
        }],
    }
    .validate()
    .expect("valid");
    let store = Store::create(
        &TempDir::new("fp-text").path().join("store"),
        &schema,
        MapPolicy::default(),
    )
    .expect("create")
    .0;
    let proj = store
        .snapshot(&work())
        .expect("snap")
        .compiled()
        .projection(crate::schema::ProjectionId(0))
        .expect("proj");
    assert_eq!(proj.encoding, KeyEncoding::FingerprintBucket);
    let projected =
        determinant_bytes(proj, &[Value::String("hello".into())], &work()).expect("project");
    assert!(projected.len() > 16, "canonical row encoding for hash input");
    let routing = super::rows::routing_for_projected(
        store.snapshot(&work()).expect("snap").store_inner(),
        proj.id,
        &projected,
    )
    .expect("route");
    assert_eq!(routing.len(), 16, "16-byte fingerprint routing");
    assert_eq!(
        1 + 2 + routing.len() + 8,
        DETERMINANT_KEY_MIN_LEN,
        "fingerprint determinant key width"
    );
    assert_eq!(TAG_DETERMINANT, 0x03);
}

#[test]
fn compiled_theory_table_matches_chapter_40() {
    let theory = CompiledTheory::compile(&SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "T".into(),
            fields: vec![FieldDescriptor {
                name: "id".into(),
                value_type: ValueType::U64,
            }],
            extension: None,
        }],
        statements: vec![StatementDescriptor::Functionality {
            relation: RelationId(0),
            projection: Box::from([FieldId(0)]),
        }],
    }
    .validate()
    .expect("valid"))
    .expect("compile");
    let proj = theory.projection(crate::schema::ProjectionId(0)).expect("one");
    // Row 13 + membership 29 + determinant exact u64 19 = 61 raw key bytes
    // (one fact, one key) per chapter 40 worked example structure.
    let det_key = 1 + 2 + proj.encoding.routing_width() + 8;
    assert_eq!(det_key, 19);
}

#[test]
fn store_shares_schema_compiled_theory() {
    let schema = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "T".into(),
            fields: vec![FieldDescriptor {
                name: "id".into(),
                value_type: ValueType::U64,
            }],
            extension: None,
        }],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: RelationId(0),
                projection: Box::from([FieldId(0)]),
            },
            StatementDescriptor::Functionality {
                relation: RelationId(0),
                projection: Box::from([FieldId(0)]),
            },
        ],
    }
    .validate()
    .expect("valid");
    let store = Store::create(
        &TempDir::new("shared-theory").path().join("store"),
        &schema,
        MapPolicy::default(),
    )
    .expect("create")
    .0;
    assert_eq!(
        store.snapshot(&work()).expect("snap").compiled().projections().len(),
        1,
        "one interned projection shared across statements"
    );
    let id0 = store
        .snapshot(&work())
        .expect("snap")
        .compiled()
        .projection_of_statement(StatementId(0))
        .expect("stmt 0")
        .id;
    let id1 = store
        .snapshot(&work())
        .expect("snap")
        .compiled()
        .projection_of_statement(StatementId(1))
        .expect("stmt 1")
        .id;
    assert_eq!(id0, id1);
}
