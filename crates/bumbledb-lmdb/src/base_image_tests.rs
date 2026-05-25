use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use std::collections::BTreeSet;

use bumbledb_core::encoding::{InternId, decode_u64, encode_intern_id, encode_u64};
use bumbledb_core::query_ir::{TypedFindTerm, TypedVariable};
use bumbledb_core::schema::{FieldDescriptor, RelationDescriptor, SchemaDescriptor, ValueType};

use super::field_scope_for_plan;
use crate::colt::{KeyOwned, SourceFilter, SourceFilterOp};
use crate::diagnostics::{
    allocation_delta, allocation_snapshot, with_allocation_tracking_for_test,
};
use crate::query::free_join::{FjNode, FjPlan, FjSubatom};
use crate::query::model::{
    AtomOccurrence, AtomOccurrenceId, NormalizedFieldBinding, NormalizedQuery, NormalizedTerm,
};
use crate::query::trace::{QueryTrace, TracePhase};
use crate::storage_format::{RowId, column_key};
use crate::{Environment, Error, Fact, InsertOutcome, Result, StorageSchema, Value};

static NEXT_TEST_ID: AtomicU64 = AtomicU64::new(0);

#[test]
fn base_image_row_count_equals_relation_count() -> Result<()> {
    let (env, schema) = env_and_schema("row-count")?;
    insert_people(&env, &schema)?;

    env.read(|txn| {
        let image = txn.relation_base_image(&schema, "Person", [0, 1])?;
        assert_eq!(
            image.stats.row_count as u64,
            txn.relation_fact_count(&schema, "Person")?
        );
        Ok(())
    })
}

#[test]
fn base_image_columns_align_with_row_handles() -> Result<()> {
    let (env, schema) = env_and_schema("columns-align")?;
    insert_people(&env, &schema)?;

    env.read(|txn| {
        let image = txn.relation_base_image(&schema, "Person", [0, 1, 2])?;
        for column in image.columns.values() {
            assert_eq!(column.row_count(), image.row_handles.len());
        }
        Ok(())
    })
}

#[test]
fn base_image_contiguous_columns_return_values_by_offset() -> Result<()> {
    let (env, schema) = env_and_schema("contiguous-values")?;
    insert_people(&env, &schema)?;

    env.read(|txn| {
        let image = txn.relation_base_image(&schema, "Person", [0])?;
        let column = &image.columns[&0];
        let observed = (0..column.row_count())
            .filter_map(|offset| column.value_at(offset).map(<[u8]>::to_vec))
            .collect::<BTreeSet<_>>();

        assert_eq!(column.field_id, 0);
        assert_eq!(column.width, 8);
        assert_eq!(column.values.len(), column.row_count() * column.width);
        assert_eq!(
            observed,
            [encode_u64(1).to_vec(), encode_u64(2).to_vec()]
                .into_iter()
                .collect()
        );
        assert_eq!(column.value_at(column.row_count()), None);
        Ok(())
    })
}

#[test]
fn base_image_string_and_bytes_columns_use_dictionary_ids() -> Result<()> {
    let (env, schema) = env_and_schema("dictionary-columns")?;
    insert_people(&env, &schema)?;

    env.read(|txn| {
        let image = txn.relation_base_image(&schema, "Person", [1, 2])?;
        let name_column = &image.columns[&1];
        let blob_column = &image.columns[&2];
        let first_intern = encode_intern_id(InternId(1));

        assert!((0..name_column.row_count()).all(|offset| {
            name_column
                .value_at(offset)
                .is_some_and(|value| value.len() == 8)
        }));
        assert!((0..blob_column.row_count()).all(|offset| {
            blob_column
                .value_at(offset)
                .is_some_and(|value| value.len() == 8)
        }));
        assert!(
            (0..name_column.row_count())
                .any(|offset| name_column.value_at(offset) == Some(first_intern.as_slice()))
        );
        let facts = txn.debug_relation_facts(&schema, "Person")?;
        assert!(
            facts
                .iter()
                .any(|fact| fact.value("name") == Some(&Value::String("alice".to_owned())))
        );
        Ok(())
    })
}

#[test]
fn base_image_load_allocations_are_below_per_cell_value_allocation_pattern() -> Result<()> {
    let (env, schema) = env_and_schema("allocation-profile")?;
    env.write(|txn| {
        for id in 0..256 {
            txn.insert(&schema, &person(id, &format!("person-{id}"), &[id as u8]))?;
        }
        Ok::<(), Error>(())
    })?;

    let cells = 256 * 3;
    let alloc_calls = with_allocation_tracking_for_test(|| {
        let start = allocation_snapshot();
        env.read(|txn| {
            let image = txn.relation_base_image(&schema, "Person", [0, 1, 2])?;
            assert_eq!(image.row_handles.len(), 256);
            Ok::<_, Error>(())
        })?;
        Ok::<_, Error>(allocation_delta(start, allocation_snapshot()).alloc_calls)
    })?;

    assert!(
        alloc_calls < (cells * 4) as u64,
        "base-image load still looks like key plus per-cell value allocations: {alloc_calls} calls"
    );
    Ok(())
}

#[test]
fn base_image_prefix_scan_preserves_multi_column_alignment() -> Result<()> {
    let (env, schema) = number_env_and_schema("prefix-align")?;
    env.write(|txn| {
        txn.insert(&schema, &number(1, 10, 100))?;
        txn.insert(&schema, &number(2, 20, 200))?;
        txn.insert(&schema, &number(3, 30, 300))?;
        Ok::<(), Error>(())
    })?;

    env.read(|txn| {
        let image = txn.relation_base_image(&schema, "Number", [0, 1, 2])?;
        let a = &image.columns[&0];
        let b = &image.columns[&1];
        let c = &image.columns[&2];

        assert_eq!(image.row_handles.len(), 3);
        for offset in 0..image.row_handles.len() {
            let av = decode_u64(
                a.value_at(offset)
                    .ok_or_else(|| Error::corrupt("missing a"))?,
            )
            .map_err(|error| Error::corrupt(error.to_string()))?;
            let bv = decode_u64(
                b.value_at(offset)
                    .ok_or_else(|| Error::corrupt("missing b"))?,
            )
            .map_err(|error| Error::corrupt(error.to_string()))?;
            let cv = decode_u64(
                c.value_at(offset)
                    .ok_or_else(|| Error::corrupt("missing c"))?,
            )
            .map_err(|error| Error::corrupt(error.to_string()))?;
            assert_eq!(bv, av * 10);
            assert_eq!(cv, av * 100);
        }
        Ok::<(), Error>(())
    })
}

#[test]
fn base_image_prefix_scan_rejects_missing_column_entry() -> Result<()> {
    let (env, schema) = number_env_and_schema("missing-column")?;
    let row_id = insert_one_number_and_row_id(&env, &schema)?;
    env.write(|txn| {
        txn.dbs
            .data
            .delete(&mut txn.txn, &column_key(0, 1, row_id))?;
        Ok::<(), Error>(())
    })?;

    let result = env.read(|txn| txn.relation_base_image(&schema, "Number", [0, 1]));

    assert!(matches!(result, Err(Error::Corrupt { .. })));
    Ok(())
}

#[test]
fn base_image_prefix_scan_rejects_extra_column_entry() -> Result<()> {
    let (env, schema) = number_env_and_schema("extra-column")?;
    insert_one_number_and_row_id(&env, &schema)?;
    env.write(|txn| {
        txn.dbs.data.put(
            &mut txn.txn,
            &column_key(0, 1, RowId(255)),
            &encode_u64(999),
        )?;
        Ok::<(), Error>(())
    })?;

    let result = env.read(|txn| txn.relation_base_image(&schema, "Number", [1]));

    assert!(matches!(result, Err(Error::Corrupt { .. })));
    Ok(())
}

#[test]
fn base_image_prefix_scan_rejects_wrong_column_width() -> Result<()> {
    let (env, schema) = number_env_and_schema("wrong-width")?;
    let row_id = insert_one_number_and_row_id(&env, &schema)?;
    env.write(|txn| {
        txn.dbs
            .data
            .put(&mut txn.txn, &column_key(0, 1, row_id), &[1, 2, 3])?;
        Ok::<(), Error>(())
    })?;

    let result = env.read(|txn| txn.relation_base_image(&schema, "Number", [1]));

    assert!(matches!(result, Err(Error::Corrupt { .. })));
    Ok(())
}

#[test]
fn base_image_filtered_prunes_zero_survivors_before_plan_columns() -> Result<()> {
    let (env, schema) = number_env_and_schema("filtered-zero")?;
    env.write(|txn| {
        txn.insert(&schema, &number(1, 10, 100))?;
        txn.insert(&schema, &number(2, 20, 200))?;
        txn.insert(&schema, &number(3, 30, 300))?;
        Ok::<(), Error>(())
    })?;
    let filters = vec![SourceFilter::Compare {
        field_id: 1,
        op: SourceFilterOp::Eq,
        value: KeyOwned::from_slice(&encode_u64(999)),
    }];

    env.read(|txn| {
        let mut trace = QueryTrace::new();
        let image = super::relation_base_image_filtered_with_trace(
            txn,
            &schema,
            "Number",
            [0],
            &filters,
            &mut trace,
        )?;

        assert!(image.row_handles.is_empty());
        assert_eq!(image.columns[&0].row_count(), 0);
        let load = trace
            .spans
            .iter()
            .find(|span| span.phase == TracePhase::BaseImageLoad)
            .ok_or_else(|| Error::corrupt("missing base-image load span"))?;
        assert_eq!(load.counters.source_filter_rows_tested, 3);
        assert_eq!(load.counters.source_filter_survivors, 0);
        assert_eq!(load.counters.column_values_loaded, 0);
        assert_eq!(load.counters.loaded_bytes, 0);
        Ok::<(), Error>(())
    })
}

#[test]
fn base_image_filtered_preserves_survivor_column_alignment() -> Result<()> {
    let (env, schema) = number_env_and_schema("filtered-align")?;
    env.write(|txn| {
        txn.insert(&schema, &number(1, 10, 100))?;
        txn.insert(&schema, &number(2, 20, 200))?;
        txn.insert(&schema, &number(3, 30, 300))?;
        Ok::<(), Error>(())
    })?;
    let filters = vec![SourceFilter::Compare {
        field_id: 1,
        op: SourceFilterOp::Gt,
        value: KeyOwned::from_slice(&encode_u64(10)),
    }];

    env.read(|txn| {
        let mut trace = QueryTrace::new();
        let image = super::relation_base_image_filtered_with_trace(
            txn,
            &schema,
            "Number",
            [0, 2],
            &filters,
            &mut trace,
        )?;
        let a = &image.columns[&0];
        let c = &image.columns[&2];

        assert_eq!(image.row_handles.len(), 2);
        for offset in 0..image.row_handles.len() {
            let av = decode_u64(
                a.value_at(offset)
                    .ok_or_else(|| Error::corrupt("missing a"))?,
            )
            .map_err(|error| Error::corrupt(error.to_string()))?;
            let cv = decode_u64(
                c.value_at(offset)
                    .ok_or_else(|| Error::corrupt("missing c"))?,
            )
            .map_err(|error| Error::corrupt(error.to_string()))?;
            assert_eq!(cv, av * 100);
        }
        let load = trace
            .spans
            .iter()
            .find(|span| span.phase == TracePhase::BaseImageLoad)
            .ok_or_else(|| Error::corrupt("missing base-image load span"))?;
        assert_eq!(load.counters.source_filter_rows_tested, 3);
        assert_eq!(load.counters.source_filter_survivors, 2);
        assert_eq!(load.counters.column_values_loaded, 4);
        Ok::<(), Error>(())
    })
}
#[test]
#[rustfmt::skip]
fn base_image_sparse_and_dense_survivor_views_choose_expected_column_shape() -> Result<()> {
    let (env, schema) = number_env_and_schema("survivor-view-shapes")?;
    env.write(|txn| { for id in 0..10 { txn.insert(&schema, &number(id, id * 10, id * 100))?; } Ok::<(), Error>(()) })?;

    env.read(|txn| {
        let mut trace = QueryTrace::new();
        let sparse = super::relation_base_image_filtered_with_trace(txn, &schema, "Number", [0], &[SourceFilter::Compare { field_id: 1, op: SourceFilterOp::Lt, value: KeyOwned::from_slice(&encode_u64(10)) }], &mut trace)?;
        let dense = super::relation_base_image_filtered_with_trace(txn, &schema, "Number", [0], &[SourceFilter::Compare { field_id: 1, op: SourceFilterOp::Gte, value: KeyOwned::from_slice(&encode_u64(50)) }], &mut trace)?;
        assert_eq!(sparse.row_handles.len(), 1);
        assert_eq!(sparse.columns[&0].values.len(), 8);
        assert!(sparse.columns[&0].row_offsets.is_none());
        assert_eq!(dense.row_handles.len(), 5);
        assert_eq!(dense.columns[&0].values.len(), 80);
        assert_eq!(dense.columns[&0].row_offsets.as_ref().map(|offsets| offsets.len()), Some(5));
        let observed = (0..dense.columns[&0].row_count())
            .filter_map(|offset| dense.columns[&0].value_at(offset).map(<[u8]>::to_vec))
            .collect::<BTreeSet<_>>();
        assert_eq!(observed, (5..10).map(|value| encode_u64(value).to_vec()).collect());
        Ok::<(), Error>(())
    })
}

#[test]
fn base_image_deleting_row_removes_it_from_new_images() -> Result<()> {
    let (env, schema) = env_and_schema("delete-row")?;
    insert_people(&env, &schema)?;

    env.write(|txn| txn.delete(&schema, &person(1, "alice", b"a")))?;

    env.read(|txn| {
        let image = txn.relation_base_image(&schema, "Person", [0])?;
        assert_eq!(image.row_handles.len(), 1);
        Ok(())
    })
}

#[test]
fn base_image_read_snapshot_stays_stable_across_write() -> Result<()> {
    let (env, schema) = env_and_schema("snapshot")?;
    env.write(|txn| txn.insert(&schema, &person(1, "alice", b"a")))?;

    env.read(|read| {
        let before = read.relation_base_image(&schema, "Person", [0])?;
        env.write(|write| write.insert(&schema, &person(2, "bob", b"b")))?;
        let after = read.relation_base_image(&schema, "Person", [0])?;
        assert!(Rc::ptr_eq(&before, &after));
        assert_eq!(after.row_handles.len(), 1);
        Ok::<(), Error>(())
    })?;
    env.read(|txn| {
        assert_eq!(
            txn.relation_base_image(&schema, "Person", [0])?
                .row_handles
                .len(),
            2
        );
        Ok(())
    })
}

#[test]
fn base_image_cache_hits_for_same_tx_and_scope() -> Result<()> {
    let (env, schema) = env_and_schema("cache-hit")?;
    insert_people(&env, &schema)?;

    env.read(|txn| {
        let first = txn.relation_base_image(&schema, "Person", [0, 1])?;
        let second = txn.relation_base_image(&schema, "Person", [1, 0])?;
        assert!(Rc::ptr_eq(&first, &second));
        Ok(())
    })
}

#[test]
fn base_image_reuses_physical_columns_for_overlapping_scopes() -> Result<()> {
    let (env, schema) = number_env_and_schema("overlap-cache")?;
    env.write(|txn| {
        txn.insert(&schema, &number(1, 10, 100))?;
        txn.insert(&schema, &number(2, 20, 200))?;
        Ok::<(), Error>(())
    })?;

    env.read(|txn| {
        let mut trace = QueryTrace::new();
        let first = super::relation_base_image_with_trace(txn, &schema, "Number", [0], &mut trace)?;
        let second =
            super::relation_base_image_with_trace(txn, &schema, "Number", [0, 1], &mut trace)?;

        assert_eq!(first.columns[&0], second.columns[&0]);
        assert!(
            trace.counters.base_image_cache_hits >= 2,
            "overlapping scope should reuse row handles and field 0 column"
        );
        Ok::<(), Error>(())
    })
}

#[test]
fn base_image_physical_cache_is_read_transaction_local() -> Result<()> {
    let (env, schema) = number_env_and_schema("tx-local-cache")?;
    env.write(|txn| txn.insert(&schema, &number(1, 10, 100)))?;

    env.read(|txn| {
        let mut trace = QueryTrace::new();
        let _ = super::relation_base_image_with_trace(txn, &schema, "Number", [0], &mut trace)?;
        assert_eq!(trace.counters.base_image_cache_hits, 0);
        Ok::<(), Error>(())
    })?;
    env.read(|txn| {
        let mut trace = QueryTrace::new();
        let _ = super::relation_base_image_with_trace(txn, &schema, "Number", [0], &mut trace)?;
        assert_eq!(trace.counters.base_image_cache_hits, 0);
        Ok::<(), Error>(())
    })
}

#[test]
fn base_image_filtered_and_unfiltered_views_coexist() -> Result<()> {
    let (env, schema) = number_env_and_schema("filtered-unfiltered")?;
    env.write(|txn| {
        txn.insert(&schema, &number(1, 10, 100))?;
        txn.insert(&schema, &number(2, 20, 200))?;
        txn.insert(&schema, &number(3, 30, 300))?;
        Ok::<(), Error>(())
    })?;
    let filters = vec![SourceFilter::Compare {
        field_id: 1,
        op: SourceFilterOp::Gt,
        value: KeyOwned::from_slice(&encode_u64(10)),
    }];

    env.read(|txn| {
        let mut trace = QueryTrace::new();
        let filtered = super::relation_base_image_filtered_with_trace(
            txn,
            &schema,
            "Number",
            [0],
            &filters,
            &mut trace,
        )?;
        let unfiltered =
            super::relation_base_image_with_trace(txn, &schema, "Number", [0], &mut trace)?;

        assert_eq!(filtered.row_handles.len(), 2);
        assert_eq!(unfiltered.row_handles.len(), 3);
        assert!(trace.counters.base_image_cache_hits > 0);
        Ok::<(), Error>(())
    })
}

#[test]
fn base_image_cache_misses_for_changed_tx_or_scope() -> Result<()> {
    let (env, schema) = env_and_schema("cache-miss")?;
    env.write(|txn| txn.insert(&schema, &person(1, "alice", b"a")))?;
    let first = env.read(|txn| txn.relation_base_image(&schema, "Person", [0]))?;
    let different_scope = env.read(|txn| txn.relation_base_image(&schema, "Person", [0, 1]))?;
    env.write(|txn| txn.insert(&schema, &person(2, "bob", b"b")))?;
    let changed_tx = env.read(|txn| txn.relation_base_image(&schema, "Person", [0]))?;

    assert!(!Rc::ptr_eq(&first, &different_scope));
    assert!(!Rc::ptr_eq(&first, &changed_tx));
    Ok(())
}

#[test]
fn base_image_scope_can_be_derived_from_validated_plan() -> Result<()> {
    let plan = FjPlan {
        query_variables: 2,
        nodes: [FjNode {
            id: 0,
            subatoms: [FjSubatom {
                atom: AtomOccurrenceId(0),
                vars: [0, 1].into_iter().collect(),
                field_ids: [0, 1].into_iter().collect(),
            }]
            .into_iter()
            .collect(),
        }]
        .into_iter()
        .collect(),
    };
    let validated = plan
        .validate(&query_from_atoms([vec![0, 1]]))
        .map_err(|error| Error::invalid_query(error.to_string()))?;
    let scope = field_scope_for_plan(&validated);

    assert_eq!(
        scope[&AtomOccurrenceId(0)].iter().collect::<Vec<_>>(),
        vec![0, 1]
    );
    Ok(())
}

fn env_and_schema(name: &str) -> Result<(Environment, StorageSchema)> {
    let id = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
    let path =
        std::env::temp_dir().join(format!("bumbledb-prd09-{name}-{}-{id}", std::process::id()));
    if path.exists() {
        std::fs::remove_dir_all(&path)?;
    }
    let schema = StorageSchema::new(schema(), 511)?;
    let env = Environment::open_with_schema(path, &schema)?;
    Ok((env, schema))
}

fn number_env_and_schema(name: &str) -> Result<(Environment, StorageSchema)> {
    let id = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "bumbledb-prd09-number-{name}-{}-{id}",
        std::process::id()
    ));
    if path.exists() {
        std::fs::remove_dir_all(&path)?;
    }
    let schema = StorageSchema::new(number_schema(), 511)?;
    let env = Environment::open_with_schema(path, &schema)?;
    Ok((env, schema))
}

fn schema() -> SchemaDescriptor {
    SchemaDescriptor::new(
        "BaseImage",
        vec![RelationDescriptor::new(
            "Person",
            vec![
                FieldDescriptor::new("id", ValueType::U64),
                FieldDescriptor::new("name", ValueType::String),
                FieldDescriptor::new("blob", ValueType::Bytes),
            ],
        )],
    )
}

fn number_schema() -> SchemaDescriptor {
    SchemaDescriptor::new(
        "BaseImageNumber",
        vec![RelationDescriptor::new(
            "Number",
            vec![
                FieldDescriptor::new("a", ValueType::U64),
                FieldDescriptor::new("b", ValueType::U64),
                FieldDescriptor::new("c", ValueType::U64),
            ],
        )],
    )
}

fn insert_people(env: &Environment, schema: &StorageSchema) -> Result<()> {
    env.write(|txn| {
        assert_eq!(
            txn.insert(schema, &person(1, "alice", b"a"))?,
            InsertOutcome::Inserted
        );
        assert_eq!(
            txn.insert(schema, &person(2, "bob", b"b"))?,
            InsertOutcome::Inserted
        );
        Ok(())
    })
}

fn person(id: u64, name: &str, blob: &[u8]) -> Fact {
    Fact::new(
        "Person",
        [
            ("id", Value::U64(id)),
            ("name", Value::String(name.to_owned())),
            ("blob", Value::Bytes(blob.to_vec())),
        ],
    )
}

fn number(a: u64, b: u64, c: u64) -> Fact {
    Fact::new(
        "Number",
        [
            ("a", Value::U64(a)),
            ("b", Value::U64(b)),
            ("c", Value::U64(c)),
        ],
    )
}

fn insert_one_number_and_row_id(env: &Environment, schema: &StorageSchema) -> Result<RowId> {
    env.write(|txn| txn.insert(schema, &number(1, 10, 100)))?;
    Ok(RowId(1))
}

fn query_from_atoms<const N: usize>(atom_vars: [Vec<usize>; N]) -> NormalizedQuery {
    let query_variables = atom_vars
        .iter()
        .flat_map(|vars| vars.iter().copied())
        .max()
        .map_or(0, |max| max + 1);
    NormalizedQuery {
        variables: (0..query_variables)
            .map(|id| TypedVariable {
                id,
                name: format!("v{id}"),
                value_type: ValueType::U64,
            })
            .collect(),
        inputs: Vec::new(),
        find: vec![TypedFindTerm::Variable { variable: 0 }],
        atoms: atom_vars
            .into_iter()
            .enumerate()
            .map(|(id, vars)| atom(id, vars))
            .collect(),
        comparisons: Vec::new(),
    }
}

fn atom(id: usize, vars: Vec<usize>) -> AtomOccurrence {
    AtomOccurrence {
        id: AtomOccurrenceId(id),
        relation_id: id,
        relation: format!("R{id}"),
        fields: vars
            .iter()
            .enumerate()
            .map(|(field_id, variable)| NormalizedFieldBinding {
                field_id,
                field: format!("f{field_id}"),
                value_type: ValueType::U64,
                term: NormalizedTerm::Variable(*variable),
            })
            .collect(),
        variable_tuple: vars,
        source_predicates: Vec::new(),
    }
}
