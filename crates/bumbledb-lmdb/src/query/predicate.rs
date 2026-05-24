use bumbledb_core::query_ir::{ComparisonOperator, Literal, TypedLiteral, TypedOperand};
use bumbledb_core::schema::{SchemaDescriptor, ValueType};

use crate::colt::{SourceFilter, SourceFilterOp};
use crate::query::model::{AtomOccurrence, NormalizedQuery, NormalizedTerm, SourcePredicate};
use crate::query::sink::Binding;
use crate::query::trace::{QueryTrace, TraceCounters, TracePhase};
use crate::storage_v5;
use crate::{Error, InputBindings, ReadTxn, Result, Value};

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PredicateMode {
    ResidualOnly,
    Pushdown,
}

pub(crate) fn source_filters_for_atom_with_trace(
    txn: &ReadTxn<'_>,
    schema: &SchemaDescriptor,
    query: &NormalizedQuery,
    atom: &AtomOccurrence,
    inputs: &InputBindings,
    mode: PredicateMode,
    trace: &mut QueryTrace,
) -> Result<Vec<SourceFilter>> {
    let span = crate::query_trace_span!(
        trace,
        TracePhase::SourceFilterEncode,
        "relation={} atom={:?}",
        atom.relation,
        atom.id
    );
    let filters = source_filters_for_atom_inner(txn, schema, query, atom, inputs, mode)?;
    if let Some(span) = span {
        trace.finish_span(
            span,
            TraceCounters {
                source_filters_encoded: filters.len() as u64,
                source_filter_false_decisions: filters
                    .iter()
                    .filter(|filter| matches!(filter, SourceFilter::False))
                    .count() as u64,
                ..TraceCounters::default()
            },
        );
    }
    Ok(filters)
}

fn source_filters_for_atom_inner(
    txn: &ReadTxn<'_>,
    schema: &SchemaDescriptor,
    query: &NormalizedQuery,
    atom: &AtomOccurrence,
    inputs: &InputBindings,
    mode: PredicateMode,
) -> Result<Vec<SourceFilter>> {
    let mut filters = Vec::new();
    for predicate in &atom.source_predicates {
        match predicate {
            SourcePredicate::InputEq { field_id, input } => {
                push_eq_filter(
                    txn,
                    schema,
                    query,
                    atom,
                    inputs,
                    &mut filters,
                    *field_id,
                    *input,
                )?;
            }
            SourcePredicate::LiteralEq { field_id, literal } => {
                let field = &atom.fields[*field_id];
                let value = literal_value(schema, literal)?;
                push_encoded_filter(
                    txn,
                    schema,
                    &field.value_type,
                    &mut filters,
                    *field_id,
                    SourceFilterOp::Eq,
                    &value,
                )?;
            }
        }
    }
    if mode == PredicateMode::Pushdown {
        for comparison in &query.comparisons {
            if let Some((variable, op, operand)) = variable_bound_comparison(comparison) {
                let Some(field_id) = field_id_for_variable(atom, variable) else {
                    continue;
                };
                let field = &atom.fields[field_id];
                let Some(value) = bound_operand_value(schema, query, inputs, operand)? else {
                    filters.push(SourceFilter::False);
                    continue;
                };
                push_encoded_filter(
                    txn,
                    schema,
                    &field.value_type,
                    &mut filters,
                    field_id,
                    op,
                    &value,
                )?;
            }
        }
    }
    Ok(filters)
}

pub(super) fn binding_satisfies(
    txn: &ReadTxn<'_>,
    schema: &SchemaDescriptor,
    query: &NormalizedQuery,
    binding: &Binding,
    inputs: &InputBindings,
) -> Result<bool> {
    for comparison in &query.comparisons {
        let left = operand_bytes(txn, schema, query, binding, inputs, &comparison.left)?;
        let right = operand_bytes(txn, schema, query, binding, inputs, &comparison.right)?;
        let (Some(left), Some(right)) = (left, right) else {
            return Ok(false);
        };
        if !compare_encoded(&left, comparison.operator, &right) {
            return Ok(false);
        }
    }
    Ok(true)
}

#[allow(clippy::too_many_arguments)]
fn push_eq_filter(
    txn: &ReadTxn<'_>,
    schema: &SchemaDescriptor,
    query: &NormalizedQuery,
    atom: &AtomOccurrence,
    inputs: &InputBindings,
    filters: &mut Vec<SourceFilter>,
    field_id: usize,
    input: usize,
) -> Result<()> {
    let field = &atom.fields[field_id];
    let value = input_value(query, inputs, input, &field.value_type)?;
    push_encoded_filter(
        txn,
        schema,
        &field.value_type,
        filters,
        field_id,
        SourceFilterOp::Eq,
        value,
    )
}

fn push_encoded_filter(
    txn: &ReadTxn<'_>,
    schema: &SchemaDescriptor,
    value_type: &ValueType,
    filters: &mut Vec<SourceFilter>,
    field_id: usize,
    op: SourceFilterOp,
    value: &Value,
) -> Result<()> {
    match storage_v5::encode_existing_value(txn, schema, value_type, value)? {
        Some(value) => filters.push(SourceFilter::Compare {
            field_id,
            op,
            value,
        }),
        None => filters.push(SourceFilter::False),
    }
    Ok(())
}

fn operand_bytes(
    txn: &ReadTxn<'_>,
    schema: &SchemaDescriptor,
    query: &NormalizedQuery,
    binding: &Binding,
    inputs: &InputBindings,
    operand: &TypedOperand,
) -> Result<Option<Vec<u8>>> {
    match operand {
        TypedOperand::Variable(variable) => Ok(binding.value(*variable).map(<[u8]>::to_vec)),
        TypedOperand::Input(input) => {
            let value_type = &query.inputs[*input].value_type;
            let value = input_value(query, inputs, *input, value_type)?;
            storage_v5::encode_existing_value(txn, schema, value_type, value)
        }
        TypedOperand::Literal(literal) => {
            let value = literal_value(schema, literal)?;
            storage_v5::encode_existing_value(txn, schema, &literal.value_type, &value)
        }
    }
}

fn input_value<'a>(
    query: &'a NormalizedQuery,
    inputs: &'a InputBindings,
    input: usize,
    value_type: &ValueType,
) -> Result<&'a Value> {
    let input = query
        .inputs
        .get(input)
        .ok_or_else(|| Error::invalid_query(format!("unknown input {input}")))?;
    let value = inputs
        .value(&input.name)
        .ok_or_else(|| Error::invalid_query(format!("missing input {}", input.name)))?;
    if !value.matches_type(value_type) {
        return Err(Error::invalid_query(format!(
            "input {} has wrong type",
            input.name
        )));
    }
    Ok(value)
}

fn literal_value(schema: &SchemaDescriptor, literal: &TypedLiteral) -> Result<Value> {
    Ok(match (&literal.literal, &literal.value_type) {
        (Literal::Bool(value), ValueType::Bool) => Value::Bool(*value),
        (Literal::String(value), ValueType::String) => Value::String(value.clone()),
        (Literal::Integer(value), ValueType::U64) => Value::U64(integer_to_u64(*value)?),
        (Literal::Integer(value), ValueType::Serial { .. }) => {
            Value::Serial(integer_to_u64(*value)?)
        }
        (Literal::Integer(value), ValueType::I64) => Value::I64(integer_to_i64(*value)?),
        (Literal::Integer(value), ValueType::Enum { name }) => {
            let code = integer_to_u8(*value)?;
            if !schema.enum_contains_code(name, code) {
                return Err(Error::invalid_query(format!(
                    "enum {name} does not contain code {code}"
                )));
            }
            Value::Enum(code)
        }
        _ => return Err(Error::invalid_query("literal/type mismatch")),
    })
}

fn bound_operand_value(
    schema: &SchemaDescriptor,
    query: &NormalizedQuery,
    inputs: &InputBindings,
    operand: &TypedOperand,
) -> Result<Option<Value>> {
    match operand {
        TypedOperand::Literal(literal) => literal_value(schema, literal).map(Some),
        TypedOperand::Input(input) => {
            let value_type = &query.inputs[*input].value_type;
            input_value(query, inputs, *input, value_type)
                .cloned()
                .map(Some)
        }
        TypedOperand::Variable(_) => Ok(None),
    }
}

fn variable_bound_comparison(
    comparison: &bumbledb_core::query_ir::TypedComparison,
) -> Option<(usize, SourceFilterOp, &TypedOperand)> {
    match (&comparison.left, &comparison.right) {
        (
            TypedOperand::Variable(variable),
            operand @ (TypedOperand::Literal(_) | TypedOperand::Input(_)),
        ) => Some((*variable, source_op(comparison.operator), operand)),
        (
            operand @ (TypedOperand::Literal(_) | TypedOperand::Input(_)),
            TypedOperand::Variable(variable),
        ) => Some((
            *variable,
            source_op(invert_operator(comparison.operator)),
            operand,
        )),
        _ => None,
    }
}

fn field_id_for_variable(atom: &AtomOccurrence, variable: usize) -> Option<usize> {
    atom.fields.iter().find_map(|field| match field.term {
        NormalizedTerm::Variable(bound) if bound == variable => Some(field.field_id),
        _ => None,
    })
}

fn compare_encoded(left: &[u8], op: ComparisonOperator, right: &[u8]) -> bool {
    match op {
        ComparisonOperator::Eq => left == right,
        ComparisonOperator::NotEq => left != right,
        ComparisonOperator::Lt => left < right,
        ComparisonOperator::Lte => left <= right,
        ComparisonOperator::Gt => left > right,
        ComparisonOperator::Gte => left >= right,
    }
}

fn source_op(operator: ComparisonOperator) -> SourceFilterOp {
    match operator {
        ComparisonOperator::Eq => SourceFilterOp::Eq,
        ComparisonOperator::NotEq => SourceFilterOp::NotEq,
        ComparisonOperator::Lt => SourceFilterOp::Lt,
        ComparisonOperator::Lte => SourceFilterOp::Lte,
        ComparisonOperator::Gt => SourceFilterOp::Gt,
        ComparisonOperator::Gte => SourceFilterOp::Gte,
    }
}

fn invert_operator(operator: ComparisonOperator) -> ComparisonOperator {
    match operator {
        ComparisonOperator::Eq => ComparisonOperator::Eq,
        ComparisonOperator::NotEq => ComparisonOperator::NotEq,
        ComparisonOperator::Lt => ComparisonOperator::Gt,
        ComparisonOperator::Lte => ComparisonOperator::Gte,
        ComparisonOperator::Gt => ComparisonOperator::Lt,
        ComparisonOperator::Gte => ComparisonOperator::Lte,
    }
}

fn integer_to_u64(value: i128) -> Result<u64> {
    value
        .try_into()
        .map_err(|_| Error::invalid_query("integer literal does not fit u64"))
}

fn integer_to_i64(value: i128) -> Result<i64> {
    value
        .try_into()
        .map_err(|_| Error::invalid_query("integer literal does not fit i64"))
}

fn integer_to_u8(value: i128) -> Result<u8> {
    value
        .try_into()
        .map_err(|_| Error::invalid_query("integer literal does not fit u8"))
}
