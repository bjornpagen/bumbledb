use std::collections::BTreeMap;
use std::ops::ControlFlow;
use std::sync::Arc;

use bumbledb_core::encoding::encode_u64;
use bumbledb_core::query_ir::{TypedFindTerm, TypedVariable};
use bumbledb_core::schema::ValueType;

use super::{ColtSource, tuple_schemas_for_atom};
use crate::base_image::{ColumnImage, RelationBaseImage, RelationStats};
use crate::diagnostics::{
    allocation_delta, allocation_snapshot, with_allocation_tracking_for_test,
};
use crate::query::free_join::{FjNode, FjPlan, FjSubatom};
use crate::query::model::{
    AtomOccurrence, AtomOccurrenceId, NormalizedFieldBinding, NormalizedQuery, NormalizedTerm,
};
use crate::storage_format::FactHandle;
use crate::tuple::{
    EncodedTuple, GhtSource, KeyCountEstimate, TupleCursor, TupleError, TupleField, TupleSchema,
};

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

#[test]
fn colt_initial_node_contains_all_offsets() -> TestResult {
    let colt = clover_s_colt(vec![TupleSchema::new(vec![field(0, 0)?])]);

    assert!(colt.is_vector());
    assert_eq!(colt.offset_len(), 3);
    Ok(())
}

#[test]
fn colt_cover_only_iteration_avoids_force() -> TestResult {
    let colt = clover_s_colt(vec![TupleSchema::new(vec![field(0, 0)?, field(2, 1)?])]);

    assert_eq!(collect_tuples(&colt).len(), 3);
    assert!(colt.is_vector());
    assert_eq!(colt.counters().hash_maps_built, 0);
    Ok(())
}

#[test]
fn colt_get_forces_root_once_and_finds_child() -> TestResult {
    let colt = clover_s_colt(vec![
        TupleSchema::new(vec![field(0, 0)?]),
        TupleSchema::new(vec![field(2, 1)?]),
    ]);

    assert!(colt.get(tuple_x(1)?.as_ref()).is_some());
    assert_eq!(colt.counters().nodes_forced, 1);
    assert_eq!(colt.counters().hash_maps_built, 1);
    Ok(())
}

#[test]
fn colt_repeated_get_does_not_force_again() -> TestResult {
    let colt = clover_s_colt(vec![TupleSchema::new(vec![field(0, 0)?])]);
    let key = tuple_x(1)?;

    assert!(colt.get(key.as_ref()).is_some());
    assert!(colt.get(key.as_ref()).is_some());
    assert_eq!(colt.counters().nodes_forced, 1);
    Ok(())
}

#[test]
fn colt_lookup_miss_returns_none_and_counts_miss() -> TestResult {
    let colt = clover_s_colt(vec![TupleSchema::new(vec![field(0, 0)?])]);

    assert!(colt.get(tuple_x(99)?.as_ref()).is_none());
    assert_eq!(colt.counters().misses, 1);
    Ok(())
}

#[test]
fn colt_second_level_lookup_forces_only_selected_child() -> TestResult {
    let colt = clover_s_colt(vec![
        TupleSchema::new(vec![field(0, 0)?]),
        TupleSchema::new(vec![field(2, 1)?]),
    ]);
    let key = tuple_x(1)?;
    let Some(child) = colt.get(key.as_ref()) else {
        return Err("x=1 child missing".into());
    };

    assert!(child.get(tuple_b(10)?.as_ref()).is_some());
    assert_eq!(colt.counters().nodes_forced, 2);
    Ok(())
}

#[test]
fn colt_empty_relation_iteration_and_lookup_work() -> TestResult {
    let colt = ColtSource::new(
        AtomOccurrenceId(0),
        Arc::new(empty_image()),
        vec![TupleSchema::new(vec![field(0, 0)?])],
    );

    assert!(collect_tuples(&colt).is_empty());
    assert!(colt.get(tuple_x(1)?.as_ref()).is_none());
    Ok(())
}

#[test]
fn colt_paper_clover_shape_builds_from_plan() -> TestResult {
    let query = clover_query();
    let plan = FjPlan {
        query_variables: 4,
        nodes: vec![
            node(
                0,
                [sub(0, [0, 1], [0, 1]), sub(1, [0], [0]), sub(2, [0], [0])],
            ),
            node(1, [sub(1, [2], [1])]),
            node(2, [sub(2, [3], [1])]),
        ],
    };
    let validated = plan.validate(&query)?;
    let schemas = tuple_schemas_for_atom(&query, &validated, AtomOccurrenceId(1));
    let colt = ColtSource::new(AtomOccurrenceId(1), Arc::new(clover_s_image()), schemas);

    assert_eq!(colt.vars(), &[0]);
    assert!(colt.get(tuple_x(1)?.as_ref()).is_some());
    Ok(())
}

#[test]
fn colt_output_equals_eager_grouping_for_small_relation() -> TestResult {
    let colt = clover_s_colt(vec![TupleSchema::new(vec![field(0, 0)?, field(2, 1)?])]);

    assert_eq!(
        collect_tuples(&colt),
        vec![tuple_xb(1, 10)?, tuple_xb(1, 11)?, tuple_xb(2, 20)?]
    );
    Ok(())
}

#[test]
fn colt_fill_batch_respects_size_without_materializing_all_tuples() -> TestResult {
    let colt = range_colt(7)?;
    let mut cursor = TupleCursor::default();

    let first = colt.fill_batch(&mut cursor, 3);
    let second = colt.fill_batch(&mut cursor, 3);
    let third = colt.fill_batch(&mut cursor, 3);

    assert_eq!(first.tuples.len(), 3);
    assert!(!first.exhausted);
    assert_eq!(second.tuples.len(), 3);
    assert!(!second.exhausted);
    assert_eq!(third.tuples.len(), 1);
    assert!(third.exhausted);
    assert_eq!(colt.counters().hash_maps_built, 0);
    Ok(())
}

#[test]
fn colt_fill_batch_allocation_is_bounded_to_requested_batch() -> TestResult {
    let colt = range_colt(128)?;
    let mut cursor = TupleCursor::default();

    let alloc_calls = with_allocation_tracking_for_test(|| {
        let start = allocation_snapshot();
        let batch = colt.fill_batch(&mut cursor, 4);
        assert_eq!(batch.tuples.len(), 4);
        allocation_delta(start, allocation_snapshot()).alloc_calls
    });

    assert!(
        alloc_calls < 32,
        "batch fill appears to allocate beyond the requested tuple batch: {alloc_calls} calls"
    );
    assert_eq!(cursor.position, 4);
    Ok(())
}

#[test]
fn dynamic_cover_counting_unforced_vector_does_not_force() -> TestResult {
    let colt = clover_s_colt(vec![TupleSchema::new(vec![field(0, 0)?])]);

    assert_eq!(colt.key_count(), KeyCountEstimate::Estimate(3));
    assert!(colt.is_vector());
    assert_eq!(colt.counters().hash_maps_built, 0);
    Ok(())
}

fn clover_s_colt(schemas: Vec<TupleSchema>) -> ColtSource {
    ColtSource::new(AtomOccurrenceId(1), Arc::new(clover_s_image()), schemas)
}

fn range_colt(rows: usize) -> Result<ColtSource, TupleError> {
    let image = RelationBaseImage {
        relation_id: 0,
        name: "Range".to_owned(),
        row_handles: (0..rows)
            .map(|offset| FactHandle([offset as u8; 16]))
            .collect(),
        columns: BTreeMap::from([(0, u64_column(0, 0..rows as u64))]),
        stats: RelationStats { row_count: rows },
    };
    Ok(ColtSource::new(
        AtomOccurrenceId(0),
        Arc::new(image),
        vec![TupleSchema::new(vec![field(0, 0)?])],
    ))
}

fn clover_s_image() -> RelationBaseImage {
    RelationBaseImage {
        relation_id: 1,
        name: "S".to_owned(),
        row_handles: vec![
            FactHandle([1; 16]),
            FactHandle([2; 16]),
            FactHandle([3; 16]),
        ],
        columns: BTreeMap::from([(0, column("x", [1, 1, 2])), (1, column("b", [10, 11, 20]))]),
        stats: RelationStats { row_count: 3 },
    }
}

fn empty_image() -> RelationBaseImage {
    RelationBaseImage {
        relation_id: 0,
        name: "E".to_owned(),
        row_handles: Vec::new(),
        columns: BTreeMap::from([(
            0,
            ColumnImage {
                field_id: 0,
                width: 8,
                values: Vec::new(),
            },
        )]),
        stats: RelationStats { row_count: 0 },
    }
}

fn column<const N: usize>(field: &str, values: [u64; N]) -> ColumnImage {
    u64_column(if field == "x" { 0 } else { 1 }, values)
}

fn u64_column(field_id: usize, values: impl IntoIterator<Item = u64>) -> ColumnImage {
    let values = values.into_iter().collect::<Vec<_>>();
    let mut bytes = Vec::with_capacity(values.len() * 8);
    for value in values {
        bytes.extend_from_slice(&encode_u64(value));
    }
    ColumnImage {
        field_id,
        width: 8,
        values: bytes,
    }
}

fn collect_tuples(source: &impl GhtSource) -> Vec<EncodedTuple> {
    let mut tuples = Vec::new();
    let result = source.try_for_each_tuple::<(), _>(|tuple| {
        tuples.push(tuple.to_owned_tuple());
        Ok(ControlFlow::Continue(()))
    });
    assert!(result.is_ok());
    tuples
}

fn tuple_x(value: u64) -> Result<EncodedTuple, TupleError> {
    EncodedTuple::new(
        &TupleSchema::new(vec![field(0, 0)?]),
        encode_u64(value).to_vec(),
    )
}

fn tuple_b(value: u64) -> Result<EncodedTuple, TupleError> {
    EncodedTuple::new(
        &TupleSchema::new(vec![field(2, 1)?]),
        encode_u64(value).to_vec(),
    )
}

fn tuple_xb(x: u64, b: u64) -> Result<EncodedTuple, TupleError> {
    let schema = TupleSchema::new(vec![field(0, 0)?, field(2, 1)?]);
    let mut bytes = encode_u64(x).to_vec();
    bytes.extend_from_slice(&encode_u64(b));
    EncodedTuple::new(&schema, bytes)
}

fn field(variable: usize, field_id: usize) -> Result<TupleField, TupleError> {
    TupleField::new(variable, Some(field_id), 8)
}

fn clover_query() -> NormalizedQuery {
    query_from_atoms([vec![0, 1], vec![0, 2], vec![0, 3]])
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

fn node<const N: usize>(id: usize, subatoms: [FjSubatom; N]) -> FjNode {
    FjNode {
        id,
        subatoms: subatoms.into_iter().collect(),
    }
}

fn sub<const V: usize, const F: usize>(
    atom: usize,
    vars: [usize; V],
    field_ids: [usize; F],
) -> FjSubatom {
    FjSubatom {
        atom: AtomOccurrenceId(atom),
        vars: vars.into_iter().collect(),
        field_ids: field_ids.into_iter().collect(),
    }
}
