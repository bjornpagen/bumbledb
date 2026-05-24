use std::collections::BTreeSet;

use bumbledb_core::query_ir::{
    ComparisonOperator, TypedClause, TypedComparison, TypedFieldBinding, TypedFindTerm,
    TypedLiteral, TypedOperand, TypedQuery, TypedRelationAtom, TypedTerm,
};
use bumbledb_core::schema::{RelationDescriptor, SchemaDescriptor, ValueType};

use crate::query::model::{
    AtomOccurrence, AtomOccurrenceId, NormalizedFieldBinding, NormalizedQuery, NormalizedTerm,
    SourcePredicate,
};
use crate::{Error, Result};

/// Normalizes and validates typed query IR before planning.
pub(crate) fn normalize_query(
    schema: &SchemaDescriptor,
    query: &TypedQuery,
) -> Result<NormalizedQuery> {
    validate_dense_metadata(query)?;

    let mut atoms = Vec::new();
    let mut comparisons = Vec::new();
    for clause in &query.clauses {
        match clause {
            TypedClause::Relation(atom) => {
                let id = AtomOccurrenceId(atoms.len());
                atoms.push(normalize_atom(schema, query, id, atom)?);
            }
            TypedClause::Comparison(comparison) => {
                validate_comparison(query, comparison)?;
                comparisons.push(comparison.clone());
            }
        }
    }

    Ok(NormalizedQuery {
        variables: query.variables.clone(),
        inputs: query.inputs.clone(),
        find: query.find.clone(),
        atoms,
        comparisons,
    })
}

fn validate_dense_metadata(query: &TypedQuery) -> Result<()> {
    for (expected, variable) in query.variables.iter().enumerate() {
        if variable.id != expected {
            return invalid(format!(
                "variable {} has id {}, expected {expected}",
                variable.name, variable.id
            ));
        }
    }
    for (expected, input) in query.inputs.iter().enumerate() {
        if input.id != expected {
            return invalid(format!(
                "input {} has id {}, expected {expected}",
                input.name, input.id
            ));
        }
    }
    for find in &query.find {
        match find {
            TypedFindTerm::Variable { variable } => {
                variable_type(query, *variable)?;
            }
        }
    }
    Ok(())
}

fn normalize_atom(
    schema: &SchemaDescriptor,
    query: &TypedQuery,
    id: AtomOccurrenceId,
    atom: &TypedRelationAtom,
) -> Result<AtomOccurrence> {
    let relation = schema
        .relations
        .get(atom.relation_id)
        .ok_or_else(|| Error::invalid_query(format!("unknown relation id {}", atom.relation_id)))?;
    if relation.name != atom.relation {
        return invalid(format!(
            "relation id {} is {}, not {}",
            atom.relation_id, relation.name, atom.relation
        ));
    }

    let mut fields: Vec<_> = relation
        .fields
        .iter()
        .enumerate()
        .map(|(field_id, field)| NormalizedFieldBinding {
            field_id,
            field: field.name.clone(),
            value_type: field.value_type.clone(),
            term: NormalizedTerm::Omitted,
        })
        .collect();
    let mut seen_field_ids = BTreeSet::new();
    let mut seen_field_names = BTreeSet::new();
    let mut seen_variables = BTreeSet::new();
    let mut variable_tuple = Vec::new();
    let mut source_predicates = Vec::new();

    for binding in &atom.fields {
        let schema_field = relation_field(relation, binding)?;
        if !seen_field_ids.insert(binding.field_id) {
            return invalid(format!(
                "duplicate field binding {}.{}",
                relation.name, schema_field.name
            ));
        }
        if !seen_field_names.insert(binding.field.clone()) {
            return invalid(format!(
                "duplicate field binding {}.{}",
                relation.name, binding.field
            ));
        }

        let term = normalize_term(query, binding, &schema_field.value_type)?;
        match &term {
            NormalizedTerm::Variable(variable) => {
                if !seen_variables.insert(*variable) {
                    return invalid(format!(
                        "same-atom repeated variable {} is unsupported",
                        query.variables[*variable].name
                    ));
                }
                variable_tuple.push(*variable);
            }
            NormalizedTerm::Input(input) => source_predicates.push(SourcePredicate::InputEq {
                field_id: binding.field_id,
                input: *input,
            }),
            NormalizedTerm::Literal(literal) => {
                source_predicates.push(SourcePredicate::LiteralEq {
                    field_id: binding.field_id,
                    literal: literal.clone(),
                });
            }
            NormalizedTerm::Wildcard | NormalizedTerm::Omitted => {}
        }
        fields[binding.field_id].term = term;
    }

    Ok(AtomOccurrence {
        id,
        relation_id: atom.relation_id,
        relation: atom.relation.clone(),
        fields,
        variable_tuple,
        source_predicates,
    })
}

fn relation_field<'relation>(
    relation: &'relation RelationDescriptor,
    binding: &TypedFieldBinding,
) -> Result<&'relation bumbledb_core::schema::FieldDescriptor> {
    let field = relation.fields.get(binding.field_id).ok_or_else(|| {
        Error::invalid_query(format!(
            "unknown field id {} in relation {}",
            binding.field_id, relation.name
        ))
    })?;
    if field.name != binding.field {
        return invalid(format!(
            "field id {} in relation {} is {}, not {}",
            binding.field_id, relation.name, field.name, binding.field
        ));
    }
    if field.value_type != binding.value_type {
        return invalid(format!(
            "field {}.{} has type {}, not {}",
            relation.name, field.name, field.value_type, binding.value_type
        ));
    }
    Ok(field)
}

fn normalize_term(
    query: &TypedQuery,
    binding: &TypedFieldBinding,
    expected: &ValueType,
) -> Result<NormalizedTerm> {
    match &binding.term {
        TypedTerm::Variable(variable) => {
            let value_type = variable_type(query, *variable)?;
            validate_type(expected, value_type, "variable")?;
            Ok(NormalizedTerm::Variable(*variable))
        }
        TypedTerm::Input(input) => {
            let value_type = input_type(query, *input)?;
            validate_type(expected, value_type, "input")?;
            Ok(NormalizedTerm::Input(*input))
        }
        TypedTerm::Wildcard => Ok(NormalizedTerm::Wildcard),
        TypedTerm::Literal(literal) => {
            validate_literal(expected, literal)?;
            Ok(NormalizedTerm::Literal(literal.clone()))
        }
    }
}

fn validate_comparison(query: &TypedQuery, comparison: &TypedComparison) -> Result<()> {
    validate_operand(query, &comparison.left, &comparison.value_type)?;
    validate_operand(query, &comparison.right, &comparison.value_type)?;
    if matches!(
        comparison.operator,
        ComparisonOperator::Lt
            | ComparisonOperator::Lte
            | ComparisonOperator::Gt
            | ComparisonOperator::Gte
    ) && !is_orderable(&comparison.value_type)
    {
        return invalid(format!(
            "comparison operator requires orderable type, got {}",
            comparison.value_type
        ));
    }
    Ok(())
}

fn validate_operand(
    query: &TypedQuery,
    operand: &TypedOperand,
    expected: &ValueType,
) -> Result<()> {
    match operand {
        TypedOperand::Variable(variable) => validate_type(
            expected,
            variable_type(query, *variable)?,
            "comparison variable",
        ),
        TypedOperand::Input(input) => {
            validate_type(expected, input_type(query, *input)?, "comparison input")
        }
        TypedOperand::Literal(literal) => validate_literal(expected, literal),
    }
}

fn variable_type(query: &TypedQuery, variable: usize) -> Result<&ValueType> {
    query
        .variables
        .get(variable)
        .map(|variable| &variable.value_type)
        .ok_or_else(|| Error::invalid_query(format!("unknown variable id {variable}")))
}

fn input_type(query: &TypedQuery, input: usize) -> Result<&ValueType> {
    query
        .inputs
        .get(input)
        .map(|input| &input.value_type)
        .ok_or_else(|| Error::invalid_query(format!("unknown input id {input}")))
}

fn validate_literal(expected: &ValueType, literal: &TypedLiteral) -> Result<()> {
    validate_type(expected, &literal.value_type, "literal")
}

fn validate_type(expected: &ValueType, actual: &ValueType, label: &str) -> Result<()> {
    if expected == actual {
        Ok(())
    } else {
        invalid(format!("{label} has type {actual}, expected {expected}"))
    }
}

fn is_orderable(value_type: &ValueType) -> bool {
    matches!(
        value_type,
        ValueType::U64 | ValueType::I64 | ValueType::Serial { .. }
    )
}

fn invalid<T>(reason: impl Into<String>) -> Result<T> {
    Err(Error::invalid_query(reason))
}
