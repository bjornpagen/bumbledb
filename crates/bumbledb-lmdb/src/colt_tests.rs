use std::collections::BTreeMap;
use std::ops::ControlFlow;
use std::rc::Rc;

use bumbledb_core::encoding::encode_u64;
use bumbledb_core::query_ir::{TypedFindTerm, TypedVariable};
use bumbledb_core::schema::ValueType;

use super::{
    ColtSource, KeyOwned, OwnedColtSource, SourceFilter, SourceFilterOp, tuple_schemas_for_atom,
};
use crate::base_image::{ColumnImage, RelationBaseImage, RelationStats};
use crate::diagnostics::{
    allocation_delta, allocation_snapshot, with_allocation_tracking_for_test,
};
use crate::query::free_join::{FjNode, FjPlan, FjSubatom};
use crate::query::model::{
    AtomOccurrence, AtomOccurrenceId, NormalizedFieldBinding, NormalizedQuery, NormalizedTerm,
};
use crate::query::trace::{QueryTrace, TracePhase};
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
    assert_eq!(colt.counters().map_entries_built, 2);
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
        Rc::new(empty_image()),
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
        nodes: [
            node(
                0,
                [sub(0, [0, 1], [0, 1]), sub(1, [0], [0]), sub(2, [0], [0])],
            ),
            node(1, [sub(1, [2], [1])]),
            node(2, [sub(2, [3], [1])]),
        ]
        .into_iter()
        .collect(),
    };
    let validated = plan.validate(&query)?;
    let schemas = tuple_schemas_for_atom(&query, &validated, AtomOccurrenceId(1));
    let colt = ColtSource::new(AtomOccurrenceId(1), Rc::new(clover_s_image()), schemas);

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

    assert_eq!(first.len(), 3);
    assert!(!first.exhausted);
    assert_eq!(second.len(), 3);
    assert!(!second.exhausted);
    assert_eq!(third.len(), 1);
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
        assert_eq!(batch.len(), 4);
        allocation_delta(start, allocation_snapshot()).alloc_calls
    });

    assert!(
        alloc_calls < 128,
        "batch fill appears to allocate as if it materialized every source tuple: {alloc_calls} calls"
    );
    assert_eq!(cursor.position, 4);
    Ok(())
}

#[test]
fn colt_suffix_iteration_allocation_is_bounded_independent_of_rows() -> TestResult {
    let small = range_colt(16)?;
    let large = range_colt(1024)?;

    let (small_count, small_alloc_calls, _small_allocated_bytes) = iteration_allocation(&small);
    let (large_count, large_alloc_calls, large_allocated_bytes) = iteration_allocation(&large);

    assert_eq!(small_count, 16);
    assert_eq!(large_count, 1024);
    assert_eq!(small.counters().hash_maps_built, 0);
    assert_eq!(large.counters().hash_maps_built, 0);
    let _ = small_alloc_calls;
    assert!(
        large_alloc_calls < (large_count as u64 / 2),
        "suffix iteration allocation should stay far below one allocation per row: large={large_alloc_calls} rows={large_count}"
    );
    assert!(
        large_allocated_bytes < 1_000_000,
        "suffix iteration allocated bytes should remain scratch-sized, got {large_allocated_bytes}"
    );
    Ok(())
}

#[test]
fn colt_duplicate_heavy_force_allocates_by_distinct_keys_not_rows() -> TestResult {
    let rows = 1024;
    let distinct_keys = 4;
    let colt = grouped_colt(rows, distinct_keys)?;
    let key = tuple_x(1)?;

    let (found, alloc_calls, allocated_bytes) = with_allocation_tracking_for_test(|| {
        let start = allocation_snapshot();
        let found = colt.get(key.as_ref()).is_some();
        let delta = allocation_delta(start, allocation_snapshot());
        (found, delta.alloc_calls, delta.allocated_bytes)
    });

    assert!(found);
    assert_eq!(colt.counters().offsets_scanned, rows);
    assert_eq!(colt.counters().map_entries_built, distinct_keys);
    assert_eq!(colt.counters().nodes_created, distinct_keys + 1);
    assert!(
        alloc_calls < rows as u64,
        "duplicate-heavy force allocation calls should not scale like rows: {alloc_calls} calls"
    );
    assert!(allocated_bytes > 0);
    Ok(())
}

#[test]
fn colt_distinct_force_fixture_records_distinct_key_allocation() -> TestResult {
    let rows = 64;
    let colt = grouped_colt(rows, rows)?;
    let key = tuple_x(7)?;

    let (found, alloc_calls, allocated_bytes) = with_allocation_tracking_for_test(|| {
        let start = allocation_snapshot();
        let found = colt.get(key.as_ref()).is_some();
        let delta = allocation_delta(start, allocation_snapshot());
        (found, delta.alloc_calls, delta.allocated_bytes)
    });

    assert!(found);
    assert_eq!(colt.counters().offsets_scanned, rows);
    assert_eq!(colt.counters().map_entries_built, rows);
    assert_eq!(colt.counters().nodes_created, rows + 1);
    assert!(alloc_calls > 0);
    assert!(allocated_bytes > 0);
    Ok(())
}

#[test]
fn colt_repeated_probe_after_force_allocates_bounded_constant() -> TestResult {
    let colt = grouped_colt(128, 8)?;
    let key = tuple_x(3)?;
    assert!(colt.get(key.as_ref()).is_some());
    let forced = colt.counters();

    let (hits, alloc_calls, allocated_bytes) = with_allocation_tracking_for_test(|| {
        let start = allocation_snapshot();
        let mut hits = 0usize;
        for _ in 0..1000 {
            if colt.get(key.as_ref()).is_some() {
                hits += 1;
            }
        }
        let delta = allocation_delta(start, allocation_snapshot());
        (hits, delta.alloc_calls, delta.allocated_bytes)
    });

    assert_eq!(hits, 1000);
    assert_eq!(colt.counters().nodes_forced, forced.nodes_forced);
    // Allocation tracking is process-global, so parallel test activity can add
    // noise. Keep the bound generous while still guarding against one heap
    // allocation per probe in the COLT path.
    assert!(
        alloc_calls < 500,
        "repeated probes into an already-forced source should not allocate per probe: {alloc_calls} calls"
    );
    assert!(
        allocated_bytes < 1_000_000,
        "repeated probe allocation should be constant-sized, got {allocated_bytes} bytes"
    );
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

#[test]
fn colt_source_filters_shrink_offsets_before_force() -> TestResult {
    let filter = SourceFilter::Compare {
        field_id: 0,
        op: SourceFilterOp::Eq,
        value: KeyOwned::from_slice(&encode_u64(1)),
    };
    let colt = ColtSource::new_filtered(
        AtomOccurrenceId(1),
        Rc::new(clover_s_image()),
        vec![
            TupleSchema::new(vec![field(0, 0)?]),
            TupleSchema::new(vec![field(2, 1)?]),
        ],
        vec![filter],
    );

    assert_eq!(colt.offset_len(), 2);
    assert!(colt.is_vector());
    assert_eq!(colt.counters().hash_maps_built, 0);

    let key = tuple_x(1)?;
    assert!(colt.get(key.as_ref()).is_some());
    assert_eq!(colt.counters().offsets_scanned, 2);
    Ok(())
}

#[test]
fn colt_trace_distinguishes_build_from_force() -> TestResult {
    let mut trace = QueryTrace::new();
    let colt = ColtSource::new_filtered_traced(
        AtomOccurrenceId(1),
        Rc::new(clover_s_image()),
        vec![
            TupleSchema::new(vec![field(0, 0)?]),
            TupleSchema::new(vec![field(2, 1)?]),
        ],
        Vec::new(),
        &mut trace,
    );
    assert!(
        trace
            .spans
            .iter()
            .any(|span| span.phase == TracePhase::ColtBuild)
    );
    assert!(
        !trace
            .spans
            .iter()
            .any(|span| span.phase == TracePhase::ColtForce)
    );

    let key = tuple_x(1)?;
    let _ = colt.get_traced(key.as_ref(), &mut trace, "test force");

    assert!(
        trace
            .spans
            .iter()
            .any(|span| span.phase == TracePhase::ColtForce)
    );
    assert!(
        trace
            .spans
            .iter()
            .any(|span| span.phase == TracePhase::ColtGet)
    );
    Ok(())
}

fn clover_s_colt(schemas: Vec<TupleSchema>) -> OwnedColtSource {
    ColtSource::new(AtomOccurrenceId(1), Rc::new(clover_s_image()), schemas)
}

fn range_colt(rows: usize) -> Result<OwnedColtSource, TupleError> {
    let image = RelationBaseImage {
        relation_id: 0,
        name: "Range".to_owned(),
        row_handles: Rc::new(
            (0..rows)
                .map(|offset| FactHandle([offset as u8; 16]))
                .collect(),
        ),
        columns: BTreeMap::from([(0, u64_column(0, 0..rows as u64))]),
        stats: RelationStats { row_count: rows },
    };
    Ok(ColtSource::new(
        AtomOccurrenceId(0),
        Rc::new(image),
        vec![TupleSchema::new(vec![field(0, 0)?])],
    ))
}

fn grouped_colt(rows: usize, distinct_keys: usize) -> Result<OwnedColtSource, TupleError> {
    let image = RelationBaseImage {
        relation_id: 0,
        name: "Grouped".to_owned(),
        row_handles: Rc::new(
            (0..rows)
                .map(|offset| FactHandle([offset as u8; 16]))
                .collect(),
        ),
        columns: BTreeMap::from([
            (
                0,
                u64_column(0, (0..rows).map(|offset| (offset % distinct_keys) as u64)),
            ),
            (1, u64_column(1, 0..rows as u64)),
        ]),
        stats: RelationStats { row_count: rows },
    };
    Ok(ColtSource::new(
        AtomOccurrenceId(0),
        Rc::new(image),
        vec![
            TupleSchema::new(vec![field(0, 0)?]),
            TupleSchema::new(vec![field(1, 1)?]),
        ],
    ))
}

fn iteration_allocation(source: &OwnedColtSource) -> (usize, u64, u64) {
    with_allocation_tracking_for_test(|| {
        let start = allocation_snapshot();
        let mut count = 0usize;
        let result = source.try_for_each_tuple::<(), _>(|tuple| {
            assert_eq!(tuple.bytes().len(), 8);
            count += 1;
            Ok(ControlFlow::Continue(()))
        });
        assert!(result.is_ok());
        let delta = allocation_delta(start, allocation_snapshot());
        (count, delta.alloc_calls, delta.allocated_bytes)
    })
}

fn clover_s_image() -> RelationBaseImage {
    RelationBaseImage {
        relation_id: 1,
        name: "S".to_owned(),
        row_handles: Rc::new(vec![
            FactHandle([1; 16]),
            FactHandle([2; 16]),
            FactHandle([3; 16]),
        ]),
        columns: BTreeMap::from([(0, column("x", [1, 1, 2])), (1, column("b", [10, 11, 20]))]),
        stats: RelationStats { row_count: 3 },
    }
}

fn empty_image() -> RelationBaseImage {
    RelationBaseImage {
        relation_id: 0,
        name: "E".to_owned(),
        row_handles: Rc::new(Vec::new()),
        columns: BTreeMap::from([(
            0,
            ColumnImage {
                field_id: 0,
                width: 8,
                values: Rc::new(Vec::new()),
                row_offsets: None,
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
        values: Rc::new(bytes),
        row_offsets: None,
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

fn query_from_atoms<const N: usize, I>(atom_vars: [I; N]) -> NormalizedQuery
where
    I: Clone + IntoIterator<Item = usize>,
{
    let query_variables = atom_vars
        .iter()
        .flat_map(|vars| vars.clone().into_iter())
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

fn atom(id: usize, vars: impl IntoIterator<Item = usize>) -> AtomOccurrence {
    let vars = vars.into_iter().collect::<Vec<_>>();
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
