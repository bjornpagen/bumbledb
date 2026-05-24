use bumbledb_core::query_ir::{
    Literal, TypedClause, TypedFieldBinding, TypedFindTerm, TypedInput, TypedLiteral, TypedQuery,
    TypedRelationAtom, TypedTerm, TypedVariable,
};
use bumbledb_core::schema::{FieldDescriptor, RelationDescriptor, SchemaDescriptor, ValueType};

use super::model::{AtomOccurrenceId, NormalizedTerm, SourcePredicate};
use super::normalize::normalize_query;
use crate::{Environment, Error, InputBindings, QueryResultSet, Result, StorageSchema, Value};

#[test]
fn query_normalization_rejects_duplicate_field_binding() {
    let schema = schema();
    let query = query_with_pair_fields([
        field(0, "left", TypedTerm::Variable(0)),
        field(0, "left", TypedTerm::Variable(1)),
    ]);

    let result = normalize_query(&schema, &query);

    assert!(matches!(result, Err(Error::InvalidQuery { .. })));
}

#[test]
fn query_normalization_rejects_same_atom_repeated_variable() {
    let schema = schema();
    let query = query_with_pair_fields([
        field(0, "left", TypedTerm::Variable(0)),
        field(1, "right", TypedTerm::Variable(0)),
    ]);

    let result = normalize_query(&schema, &query);

    assert!(matches!(result, Err(Error::InvalidQuery { .. })));
}

#[test]
fn query_normalization_assigns_distinct_self_join_occurrences() -> Result<()> {
    let schema = schema();
    let query = TypedQuery {
        variables: variables(),
        inputs: Vec::new(),
        find: vec![TypedFindTerm::Variable { variable: 0 }],
        clauses: vec![
            TypedClause::Relation(pair_atom([
                field(0, "left", TypedTerm::Variable(0)),
                field(1, "right", TypedTerm::Variable(1)),
            ])),
            TypedClause::Relation(pair_atom([
                field(0, "left", TypedTerm::Variable(1)),
                field(1, "right", TypedTerm::Variable(2)),
            ])),
        ],
    };

    let normalized = normalize_query(&schema, &query)?;

    assert_eq!(normalized.atoms.len(), 2);
    assert_eq!(normalized.atoms[0].id, AtomOccurrenceId(0));
    assert_eq!(normalized.atoms[1].id, AtomOccurrenceId(1));
    assert_eq!(
        normalized.atoms[0].relation_id,
        normalized.atoms[1].relation_id
    );
    Ok(())
}

#[test]
fn query_self_join_projection_result_set_is_duplicate_free() {
    let result = QueryResultSet::new(
        vec![crate::ResultColumn::Variable("x".to_owned())],
        vec![vec![Value::U64(1)], vec![Value::U64(1)]],
    );

    assert_eq!(result.cardinality(), 1);
}

#[test]
fn query_normalization_preserves_literal_input_wildcard_and_omitted_terms() -> Result<()> {
    let schema = schema();
    let literal = TypedLiteral {
        literal: Literal::Integer(7),
        value_type: ValueType::U64,
    };
    let query = TypedQuery {
        variables: variables(),
        inputs: vec![TypedInput {
            id: 0,
            name: "runtime".to_owned(),
            value_type: ValueType::U64,
        }],
        find: vec![TypedFindTerm::Variable { variable: 0 }],
        clauses: vec![TypedClause::Relation(TypedRelationAtom {
            relation_id: 1,
            relation: "Mixed".to_owned(),
            fields: vec![
                mixed_field(0, "a", TypedTerm::Variable(0)),
                mixed_field(1, "b", TypedTerm::Input(0)),
                mixed_field(2, "c", TypedTerm::Literal(literal.clone())),
                mixed_field(3, "d", TypedTerm::Wildcard),
            ],
        })],
    };

    let normalized = normalize_query(&schema, &query)?;
    let atom = &normalized.atoms[0];

    assert_eq!(atom.variable_tuple, vec![0]);
    assert_eq!(atom.fields[0].term, NormalizedTerm::Variable(0));
    assert_eq!(atom.fields[1].term, NormalizedTerm::Input(0));
    assert_eq!(
        atom.fields[2].term,
        NormalizedTerm::Literal(literal.clone())
    );
    assert_eq!(atom.fields[3].term, NormalizedTerm::Wildcard);
    assert_eq!(atom.fields[4].term, NormalizedTerm::Omitted);
    assert_eq!(
        atom.source_predicates,
        vec![
            SourcePredicate::InputEq {
                field_id: 1,
                input: 0,
            },
            SourcePredicate::LiteralEq {
                field_id: 2,
                literal,
            },
        ]
    );
    Ok(())
}

#[test]
fn query_execution_boundary_rejects_invalid_public_ir() -> Result<()> {
    let path = std::env::temp_dir().join("bumbledb-prd02-invalid-boundary");
    if path.exists() {
        std::fs::remove_dir_all(&path)?;
    }
    let env = Environment::open(&path)?;
    let storage_schema = StorageSchema::new(schema(), 511)?;
    let query = query_with_pair_fields([
        field(0, "left", TypedTerm::Variable(0)),
        field(0, "left", TypedTerm::Variable(1)),
    ]);

    let result: Result<QueryResultSet> =
        env.read(|txn| txn.execute_query(&storage_schema, &query, &InputBindings::new()));

    if path.exists() {
        std::fs::remove_dir_all(&path)?;
    }
    assert!(matches!(result, Err(Error::InvalidQuery { .. })));
    Ok(())
}

#[test]
fn query_normalization_retains_unprojected_binding_variables() -> Result<()> {
    let schema = schema();
    let query = query_with_pair_fields([
        field(0, "left", TypedTerm::Variable(0)),
        field(1, "right", TypedTerm::Variable(1)),
    ]);

    let normalized = normalize_query(&schema, &query)?;

    assert_eq!(
        normalized.find,
        vec![TypedFindTerm::Variable { variable: 0 }]
    );
    assert_eq!(normalized.variables.len(), 3);
    assert_eq!(normalized.atoms[0].variable_tuple, vec![0, 1]);
    Ok(())
}

fn schema() -> SchemaDescriptor {
    SchemaDescriptor::new(
        "Prd02",
        vec![
            RelationDescriptor::new(
                "Pair",
                vec![
                    FieldDescriptor::new("left", ValueType::U64),
                    FieldDescriptor::new("right", ValueType::U64),
                ],
            ),
            RelationDescriptor::new(
                "Mixed",
                vec![
                    FieldDescriptor::new("a", ValueType::U64),
                    FieldDescriptor::new("b", ValueType::U64),
                    FieldDescriptor::new("c", ValueType::U64),
                    FieldDescriptor::new("d", ValueType::U64),
                    FieldDescriptor::new("e", ValueType::U64),
                ],
            ),
        ],
    )
}

fn variables() -> Vec<TypedVariable> {
    (0..3)
        .map(|id| TypedVariable {
            id,
            name: format!("v{id}"),
            value_type: ValueType::U64,
        })
        .collect()
}

fn query_with_pair_fields<const N: usize>(fields: [TypedFieldBinding; N]) -> TypedQuery {
    TypedQuery {
        variables: variables(),
        inputs: Vec::new(),
        find: vec![TypedFindTerm::Variable { variable: 0 }],
        clauses: vec![TypedClause::Relation(pair_atom(fields))],
    }
}

fn pair_atom(fields: impl IntoIterator<Item = TypedFieldBinding>) -> TypedRelationAtom {
    TypedRelationAtom {
        relation_id: 0,
        relation: "Pair".to_owned(),
        fields: fields.into_iter().collect(),
    }
}

fn field(field_id: usize, field: &str, term: TypedTerm) -> TypedFieldBinding {
    TypedFieldBinding {
        field_id,
        field: field.to_owned(),
        value_type: ValueType::U64,
        term,
    }
}

fn mixed_field(field_id: usize, field_name: &str, term: TypedTerm) -> TypedFieldBinding {
    field(field_id, field_name, term)
}
