use std::collections::BTreeMap;
use std::sync::Arc;

use bumbledb_core::encoding::encode_u64;
use bumbledb_core::query_ir::{TypedFindTerm, TypedVariable};
use bumbledb_core::schema::ValueType;

use super::{ColtSource, tuple_schemas_for_atom};
use crate::base_image::{ColumnImage, RelationBaseImage, RelationStats};
use crate::query::free_join::{FjNode, FjPlan, FjSubatom};
use crate::query::model::{
    AtomOccurrence, AtomOccurrenceId, NormalizedFieldBinding, NormalizedQuery, NormalizedTerm,
};
use crate::storage_format::FactHandle;
use crate::tuple::{
    EncodedTuple, GhtSource, KeyCountEstimate, TupleError, TupleField, TupleSchema,
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

    assert_eq!(colt.iter().len(), 3);
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

    assert!(colt.get(&tuple_x(1)?).is_some());
    assert_eq!(colt.counters().nodes_forced, 1);
    assert_eq!(colt.counters().hash_maps_built, 1);
    Ok(())
}

#[test]
fn colt_repeated_get_does_not_force_again() -> TestResult {
    let colt = clover_s_colt(vec![TupleSchema::new(vec![field(0, 0)?])]);
    let key = tuple_x(1)?;

    assert!(colt.get(&key).is_some());
    assert!(colt.get(&key).is_some());
    assert_eq!(colt.counters().nodes_forced, 1);
    Ok(())
}

#[test]
fn colt_lookup_miss_returns_none_and_counts_miss() -> TestResult {
    let colt = clover_s_colt(vec![TupleSchema::new(vec![field(0, 0)?])]);

    assert!(colt.get(&tuple_x(99)?).is_none());
    assert_eq!(colt.counters().misses, 1);
    Ok(())
}

#[test]
fn colt_second_level_lookup_forces_only_selected_child() -> TestResult {
    let colt = clover_s_colt(vec![
        TupleSchema::new(vec![field(0, 0)?]),
        TupleSchema::new(vec![field(2, 1)?]),
    ]);
    let Some(child) = colt.get(&tuple_x(1)?) else {
        return Err("x=1 child missing".into());
    };

    assert!(child.get(&tuple_b(10)?).is_some());
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

    assert!(colt.iter().is_empty());
    assert!(colt.get(&tuple_x(1)?).is_none());
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
    assert!(colt.get(&tuple_x(1)?).is_some());
    Ok(())
}

#[test]
fn colt_output_equals_eager_grouping_for_small_relation() -> TestResult {
    let colt = clover_s_colt(vec![TupleSchema::new(vec![field(0, 0)?, field(2, 1)?])]);

    assert_eq!(
        colt.iter(),
        vec![tuple_xb(1, 10)?, tuple_xb(1, 11)?, tuple_xb(2, 20)?]
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

fn clover_s_colt(schemas: Vec<TupleSchema>) -> ColtSource {
    ColtSource::new(AtomOccurrenceId(1), Arc::new(clover_s_image()), schemas)
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
                field: "x".to_owned(),
                values: Vec::new(),
            },
        )]),
        stats: RelationStats { row_count: 0 },
    }
}

fn column<const N: usize>(field: &str, values: [u64; N]) -> ColumnImage {
    ColumnImage {
        field_id: if field == "x" { 0 } else { 1 },
        field: field.to_owned(),
        values: values
            .into_iter()
            .map(|value| encode_u64(value).to_vec())
            .collect(),
    }
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
