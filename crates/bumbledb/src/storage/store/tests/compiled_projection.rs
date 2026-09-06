//! Exact-bounded vs fingerprint projection encoding (chapter 40 table).

use super::*;
use crate::schema::compiled::{CompiledTheory, KeyEncoding};
use crate::schema::{FieldDescriptor, RelationDescriptor, SchemaDescriptor, StatementDescriptor};
use crate::storage::store::RowId;
use crate::storage::store::det_index::determinant_bytes;
use crate::storage::store::keys::TAG_DETERMINANT;
use bumbledb_theory::schema::{FieldId, RelationId, StatementId};

#[test]
fn noninterned_or_foreign_relation_projection_refuses_and_rolls_back() {
    struct InvalidSecond {
        projection: ProjectionId,
        calls: std::cell::Cell<usize>,
    }
    impl RowIndexer for InvalidSecond {
        fn index_row(
            &self,
            _relation: RelationId,
            _row: &[u8],
            _work: &WorkContext,
            emit: super::super::ProjectionEmitter<'_>,
        ) -> StoreResult<()> {
            self.calls.set(self.calls.get() + 1);
            if self.calls.get() == 2 {
                emit(self.projection, &0u64.to_be_bytes())?;
            }
            Ok(())
        }
    }
    let (_dir, path) = store_dir("projection-refusal");
    let store = create_default(&path);
    for (projection, relation, second) in [
        (ProjectionId(1), NOTE, note(2, "second")),
        (ProjectionId(256), NOTE, note(2, "second")),
        (ProjectionId(u16::MAX), NOTE, note(2, "second")),
        (NOTE_KEY, TAG, vec![Value::String("tag".into())]),
    ] {
        let context = work();
        let mut owner = store.writer(&context).expect("writer");
        let changes = change_set(
            &schema(),
            &[(NOTE, note(1, "first")), (relation, second)],
            &[],
        );
        let indexer = InvalidSecond {
            projection,
            calls: std::cell::Cell::new(0),
        };
        assert!(matches!(
            owner.prepare(&changes, &indexer, &AdmitAll),
            Err(StoreError::ForeignSchema)
        ));
        assert_eq!(indexer.calls.get(), 2, "a prior row was already applied");
        drop(owner);
        let snapshot = store.snapshot(&context).expect("snapshot");
        assert_eq!(snapshot.row_count(NOTE).expect("count"), 0);
        assert_eq!(snapshot.row_count(TAG).expect("count"), 0);
        assert!(
            store
                .inner
                .data
                .is_empty(snapshot.read_txn())
                .expect("all indexes rolled back")
        );
    }
}

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
    let snapshot = store.snapshot(&work()).expect("snap");
    let proj = snapshot
        .compiled()
        .projection(crate::schema::ProjectionId(0))
        .expect("proj");
    assert!(matches!(
        proj.encoding,
        KeyEncoding::ExactBounded { scalar_width: 8 }
    ));
    let projected = determinant_bytes(proj, &[Value::U64(42)], &work()).expect("project");
    assert_eq!(projected.len(), 8, "exact u64 routing is 8 bytes");
    let key = store
        .inner
        .keys
        .determinant_key(proj.id, &projected, RowId(7))
        .expect("key");
    assert_eq!(
        key.len(),
        snapshot.physical_key_widths().determinant_overhead + 8
    );
    assert_eq!(key.len(), 18, "one-byte schema projection ordinal");
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
    let snapshot = store.snapshot(&work()).expect("snap");
    let proj = snapshot
        .compiled()
        .projection(crate::schema::ProjectionId(0))
        .expect("proj");
    assert_eq!(proj.encoding, KeyEncoding::FingerprintBucket);
    let projected =
        determinant_bytes(proj, &[Value::String("hello".into())], &work()).expect("project");
    assert_eq!(
        projected.as_slice(),
        &[&[0, 1, 4][..], &5u64.to_be_bytes(), b"hello"].concat()
    );
    let routing = crate::storage::store::rows::routing_for_projected(
        store.snapshot(&work()).expect("snap").store_inner(),
        proj.id,
        &projected,
    )
    .expect("route");
    assert_eq!(routing.len(), 16, "16-byte fingerprint routing");
    assert_eq!(
        store
            .inner
            .keys
            .determinant_key(proj.id, &routing, RowId(1))
            .expect("key")
            .len(),
        store.inner.keys.widths().determinant_overhead + 16,
        "minimum key framing plus fingerprint routing"
    );
    assert_eq!(TAG_DETERMINANT, 0x03);
}

#[test]
fn compiled_theory_table_matches_chapter_40() {
    let theory = CompiledTheory::compile(
        &SchemaDescriptor {
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
        .expect("valid"),
    )
    .expect("compile");
    let proj = theory
        .projection(crate::schema::ProjectionId(0))
        .expect("one");
    // The compiled schema retains a conservative framing bound; the store
    // codec chooses the actual schema-fixed ordinal width at open.
    let det_key = proj.complete_key_width();
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
        statements: vec![StatementDescriptor::Functionality {
            relation: RelationId(0),
            projection: Box::from([FieldId(0)]),
        }],
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
        store
            .snapshot(&work())
            .expect("snap")
            .compiled()
            .projections()
            .len(),
        1,
        "one declared key, one interned projection"
    );
    let id0 = store
        .snapshot(&work())
        .expect("snap")
        .compiled()
        .projection_of_statement(StatementId(0))
        .expect("stmt 0")
        .id;
    let theory = schema.shared_compiled_theory().expect("compiled schema");
    let snapshot = store.snapshot(&work()).expect("snap");
    assert!(std::ptr::eq(theory.as_ref(), snapshot.compiled()));
    assert_eq!(
        theory.projection_of_statement(StatementId(0)).unwrap().id,
        id0
    );
}
