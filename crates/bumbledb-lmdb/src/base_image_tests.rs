use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use std::collections::BTreeSet;

use bumbledb_core::encoding::{InternId, encode_intern_id, encode_u64};
use bumbledb_core::query_ir::{TypedFindTerm, TypedVariable};
use bumbledb_core::schema::{FieldDescriptor, RelationDescriptor, SchemaDescriptor, ValueType};

use super::field_scope_for_plan;
use crate::diagnostics::{
    allocation_delta, allocation_snapshot, with_allocation_tracking_for_test,
};
use crate::query::free_join::{FjNode, FjPlan, FjSubatom};
use crate::query::model::{
    AtomOccurrence, AtomOccurrenceId, NormalizedFieldBinding, NormalizedQuery, NormalizedTerm,
};
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
            txn.insert(&schema, person(id, &format!("person-{id}"), &[id as u8]))?;
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
        alloc_calls < (cells * 2) as u64,
        "base-image load still looks like key plus per-cell value allocations: {alloc_calls} calls"
    );
    Ok(())
}

#[test]
fn base_image_deleting_row_removes_it_from_new_images() -> Result<()> {
    let (env, schema) = env_and_schema("delete-row")?;
    insert_people(&env, &schema)?;

    env.write(|txn| txn.delete(&schema, person(1, "alice", b"a")))?;

    env.read(|txn| {
        let image = txn.relation_base_image(&schema, "Person", [0])?;
        assert_eq!(image.row_handles.len(), 1);
        Ok(())
    })
}

#[test]
fn base_image_read_snapshot_stays_stable_across_write() -> Result<()> {
    let (env, schema) = env_and_schema("snapshot")?;
    env.write(|txn| txn.insert(&schema, person(1, "alice", b"a")))?;

    env.read(|read| {
        let before = read.relation_base_image(&schema, "Person", [0])?;
        env.write(|write| write.insert(&schema, person(2, "bob", b"b")))?;
        let after = read.relation_base_image(&schema, "Person", [0])?;
        assert!(Arc::ptr_eq(&before, &after));
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
        assert!(Arc::ptr_eq(&first, &second));
        Ok(())
    })
}

#[test]
fn base_image_cache_misses_for_changed_tx_or_scope() -> Result<()> {
    let (env, schema) = env_and_schema("cache-miss")?;
    env.write(|txn| txn.insert(&schema, person(1, "alice", b"a")))?;
    let first = env.read(|txn| txn.relation_base_image(&schema, "Person", [0]))?;
    let different_scope = env.read(|txn| txn.relation_base_image(&schema, "Person", [0, 1]))?;
    env.write(|txn| txn.insert(&schema, person(2, "bob", b"b")))?;
    let changed_tx = env.read(|txn| txn.relation_base_image(&schema, "Person", [0]))?;

    assert!(!Arc::ptr_eq(&first, &different_scope));
    assert!(!Arc::ptr_eq(&first, &changed_tx));
    Ok(())
}

#[test]
fn base_image_scope_can_be_derived_from_validated_plan() -> Result<()> {
    let plan = FjPlan {
        query_variables: 2,
        nodes: vec![FjNode {
            id: 0,
            subatoms: vec![FjSubatom {
                atom: AtomOccurrenceId(0),
                vars: vec![0, 1],
                field_ids: vec![0, 1],
            }],
        }],
    };
    let validated = plan
        .validate(&query_from_atoms([vec![0, 1]]))
        .map_err(|error| Error::invalid_query(error.to_string()))?;
    let scope = field_scope_for_plan(&validated);

    assert_eq!(scope[&AtomOccurrenceId(0)], [0, 1].into_iter().collect());
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

fn insert_people(env: &Environment, schema: &StorageSchema) -> Result<()> {
    env.write(|txn| {
        assert_eq!(
            txn.insert(schema, person(1, "alice", b"a"))?,
            InsertOutcome::Inserted
        );
        assert_eq!(
            txn.insert(schema, person(2, "bob", b"b"))?,
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
