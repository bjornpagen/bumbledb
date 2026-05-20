//! Simple in-memory reference model for supported typed query IR.

use std::collections::{BTreeMap, BTreeSet};

use bumbledb_core::encoding::{DecimalRaw, TimestampMicros};
use bumbledb_core::query_ir::{
    AggregateFunction, ComparisonOperator, Literal, TypedClause, TypedComparison, TypedFindTerm,
    TypedLiteral, TypedOperand, TypedQuery, TypedTerm,
};
use bumbledb_core::schema::{IdentityAllocation, ValueType};
use bumbledb_lmdb::{
    AggregateError, Error, ExecuteError, IdentityValue, InputBindings, InternalError, QueryError,
    Result, Row, Value,
};

/// In-memory reference database.
#[derive(Clone, Debug, Default)]
pub struct ReferenceDb {
    rows: BTreeMap<String, Vec<Row>>,
}

impl ReferenceDb {
    /// Builds a reference DB from logical rows.
    pub fn from_rows(rows: impl IntoIterator<Item = Row>) -> Self {
        let mut by_relation: BTreeMap<String, Vec<Row>> = BTreeMap::new();
        for row in rows {
            by_relation
                .entry(row.relation().to_owned())
                .or_default()
                .push(row);
        }
        Self { rows: by_relation }
    }

    /// Executes a typed positive query IR.
    pub fn execute(&self, query: &TypedQuery, inputs: &InputBindings) -> Result<Vec<Vec<Value>>> {
        validate_inputs(query, inputs)?;
        let atoms = query
            .clauses
            .iter()
            .filter_map(|clause| match clause {
                TypedClause::Relation(atom) => Some(atom),
                TypedClause::Comparison(_) => None,
            })
            .collect::<Vec<_>>();
        let comparisons = query
            .clauses
            .iter()
            .filter_map(|clause| match clause {
                TypedClause::Comparison(comparison) => Some(comparison),
                TypedClause::Relation(_) => None,
            })
            .collect::<Vec<_>>();

        let mut bindings = Vec::new();
        self.recurse(
            query,
            inputs,
            &atoms,
            &comparisons,
            0,
            Binding::new(query.variables.len()),
            &mut bindings,
        )?;
        project_results(query, &bindings)
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "reference recursion carries explicit query state"
    )]
    fn recurse(
        &self,
        query: &TypedQuery,
        inputs: &InputBindings,
        atoms: &[&bumbledb_core::query_ir::TypedRelationAtom],
        comparisons: &[&TypedComparison],
        depth: usize,
        binding: Binding,
        output: &mut Vec<Binding>,
    ) -> Result<()> {
        if depth == atoms.len() {
            if comparisons_pass(comparisons, query, inputs, &binding)? {
                output.push(binding);
            }
            return Ok(());
        }

        let atom = atoms[depth];
        for row in self.rows.get(&atom.relation).into_iter().flatten() {
            let Some(next) = match_atom(atom, query, inputs, &binding, row)? else {
                continue;
            };
            if comparisons_pass(comparisons, query, inputs, &next)? {
                self.recurse(query, inputs, atoms, comparisons, depth + 1, next, output)?;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct Binding {
    values: Vec<Option<Value>>,
}

impl Binding {
    fn new(variable_count: usize) -> Self {
        Self {
            values: vec![None; variable_count],
        }
    }

    fn get(&self, variable: usize) -> Option<&Value> {
        self.values[variable].as_ref()
    }

    fn bind(&mut self, variable: usize, value: Value) -> bool {
        match &self.values[variable] {
            Some(existing) => existing == &value,
            None => {
                self.values[variable] = Some(value);
                true
            }
        }
    }
}

fn match_atom(
    atom: &bumbledb_core::query_ir::TypedRelationAtom,
    query: &TypedQuery,
    inputs: &InputBindings,
    binding: &Binding,
    row: &Row,
) -> Result<Option<Binding>> {
    let mut next = binding.clone();
    for field in &atom.fields {
        let Some(row_value) = row.value(&field.field) else {
            return Ok(None);
        };
        match &field.term {
            TypedTerm::Variable(variable) => {
                if !next.bind(*variable, row_value.clone()) {
                    return Ok(None);
                }
            }
            TypedTerm::Input(input) => {
                let input_value = input_value(query, inputs, *input)?;
                if input_value != row_value {
                    return Ok(None);
                }
            }
            TypedTerm::Literal(literal) => {
                if literal_to_value(literal)? != *row_value {
                    return Ok(None);
                }
            }
            TypedTerm::Wildcard => {}
        }
    }
    Ok(Some(next))
}

fn comparisons_pass(
    comparisons: &[&TypedComparison],
    query: &TypedQuery,
    inputs: &InputBindings,
    binding: &Binding,
) -> Result<bool> {
    for comparison in comparisons {
        let Some(left) = operand_value(&comparison.left, query, inputs, binding)? else {
            continue;
        };
        let Some(right) = operand_value(&comparison.right, query, inputs, binding)? else {
            continue;
        };
        if !compare_values(&left, comparison.operator, &right) {
            return Ok(false);
        }
    }
    Ok(true)
}

fn operand_value(
    operand: &TypedOperand,
    query: &TypedQuery,
    inputs: &InputBindings,
    binding: &Binding,
) -> Result<Option<Value>> {
    Ok(match operand {
        TypedOperand::Variable(variable) => binding.get(*variable).cloned(),
        TypedOperand::Input(input) => Some(input_value(query, inputs, *input)?.clone()),
        TypedOperand::Literal(literal) => Some(literal_to_value(literal)?),
    })
}

fn input_value<'a>(
    query: &'a TypedQuery,
    inputs: &'a InputBindings,
    input: usize,
) -> Result<&'a Value> {
    let input = &query.inputs[input];
    let value = inputs.value(&input.name).ok_or_else(|| {
        Error::Query(QueryError::Execute(ExecuteError::MissingInput {
            input: input.name.clone(),
        }))
    })?;
    if !value_matches_type(value, &input.value_type) {
        return Err(Error::Query(QueryError::Execute(
            ExecuteError::InputTypeMismatch {
                input: input.name.clone(),
                expected: value_type_name(&input.value_type),
                actual: value_kind_name(value),
            },
        )));
    }
    Ok(value)
}

fn validate_inputs(query: &TypedQuery, inputs: &InputBindings) -> Result<()> {
    for input in &query.inputs {
        input_value(query, inputs, input.id)?;
    }
    Ok(())
}

fn project_results(query: &TypedQuery, bindings: &[Binding]) -> Result<Vec<Vec<Value>>> {
    if query
        .find
        .iter()
        .any(|term| matches!(term, TypedFindTerm::Aggregate { .. }))
    {
        project_aggregates(query, bindings)
    } else {
        let mut set = BTreeSet::new();
        for binding in bindings {
            let mut row = Vec::new();
            for term in &query.find {
                let TypedFindTerm::Variable { variable } = term else {
                    continue;
                };
                row.push(bound_variable(binding, *variable)?.clone());
            }
            set.insert(row);
        }
        Ok(set.into_iter().collect())
    }
}

fn project_aggregates(query: &TypedQuery, bindings: &[Binding]) -> Result<Vec<Vec<Value>>> {
    let group_terms = query
        .find
        .iter()
        .filter_map(|term| match term {
            TypedFindTerm::Variable { variable } => Some(*variable),
            TypedFindTerm::Aggregate { .. } => None,
        })
        .collect::<Vec<_>>();
    let aggregate_terms = query
        .find
        .iter()
        .filter_map(|term| match term {
            TypedFindTerm::Aggregate {
                function,
                variable,
                value_type,
            } => Some((*function, *variable, value_type.clone())),
            TypedFindTerm::Variable { .. } => None,
        })
        .collect::<Vec<_>>();
    let mut groups: BTreeMap<Vec<Value>, Vec<AggregateState>> = BTreeMap::new();
    for binding in bindings {
        let key = group_terms
            .iter()
            .map(|variable| bound_variable(binding, *variable).cloned())
            .collect::<Result<Vec<_>>>()?;
        let states = groups.entry(key).or_insert_with(|| {
            aggregate_terms
                .iter()
                .map(|(function, _, value_type)| AggregateState::new(*function, value_type.clone()))
                .collect()
        });
        for (state, (_, variable, _)) in states.iter_mut().zip(&aggregate_terms) {
            state.apply(bound_variable(binding, *variable)?)?;
        }
    }
    let mut rows = Vec::new();
    for (key, states) in groups {
        let mut key_iter = key.into_iter();
        let mut state_iter = states.into_iter();
        let mut row = Vec::new();
        for term in &query.find {
            match term {
                TypedFindTerm::Variable { .. } => row.push(key_iter.next().ok_or_else(|| {
                    Error::Internal(InternalError::Invariant {
                        message: "missing aggregate group key".to_owned(),
                    })
                })?),
                TypedFindTerm::Aggregate { .. } => {
                    let state = state_iter.next().ok_or_else(|| {
                        Error::Internal(InternalError::Invariant {
                            message: "missing aggregate state".to_owned(),
                        })
                    })?;
                    row.push(state.finish()?);
                }
            }
        }
        rows.push(row);
    }
    rows.sort();
    Ok(rows)
}

fn bound_variable(binding: &Binding, variable: usize) -> Result<&Value> {
    binding.get(variable).ok_or_else(|| {
        Error::Internal(InternalError::Invariant {
            message: format!("variable {variable} is unbound"),
        })
    })
}

#[derive(Clone, Debug)]
enum AggregateState {
    Count(u64),
    SumI64(i64),
    SumDecimal(i128),
    Min(Option<Value>),
    Max(Option<Value>),
}

impl AggregateState {
    fn new(function: AggregateFunction, value_type: ValueType) -> Self {
        match (function, value_type) {
            (AggregateFunction::Count, _) => Self::Count(0),
            (AggregateFunction::Sum, ValueType::I64) => Self::SumI64(0),
            (AggregateFunction::Sum, ValueType::Decimal { .. }) => Self::SumDecimal(0),
            (AggregateFunction::Min, _) => Self::Min(None),
            (AggregateFunction::Max, _) => Self::Max(None),
            _ => Self::Count(0),
        }
    }

    fn apply(&mut self, value: &Value) -> Result<()> {
        match self {
            Self::Count(count) => {
                *count = count.checked_add(1).ok_or_else(|| {
                    Error::Query(QueryError::Aggregate(AggregateError::IntegerOverflow {
                        operation: "count",
                    }))
                })?
            }
            Self::SumI64(sum) => {
                let Value::I64(value) = value else {
                    return Err(Error::Internal(InternalError::Invariant {
                        message: "sum(i64) type mismatch".to_owned(),
                    }));
                };
                *sum = sum.checked_add(*value).ok_or_else(|| {
                    Error::Query(QueryError::Aggregate(AggregateError::IntegerOverflow {
                        operation: "sum",
                    }))
                })?;
            }
            Self::SumDecimal(sum) => {
                let Value::Decimal(DecimalRaw(value)) = value else {
                    return Err(Error::Internal(InternalError::Invariant {
                        message: "sum(decimal) type mismatch".to_owned(),
                    }));
                };
                *sum = sum.checked_add(*value).ok_or_else(|| {
                    Error::Query(QueryError::Aggregate(AggregateError::DecimalOverflow {
                        operation: "sum",
                    }))
                })?;
            }
            Self::Min(current) => match current {
                Some(existing) if &*existing <= value => {}
                _ => *current = Some(value.clone()),
            },
            Self::Max(current) => match current {
                Some(existing) if &*existing >= value => {}
                _ => *current = Some(value.clone()),
            },
        }
        Ok(())
    }

    fn finish(self) -> Result<Value> {
        Ok(match self {
            Self::Count(value) => Value::U64(value),
            Self::SumI64(value) => Value::I64(value),
            Self::SumDecimal(value) => Value::Decimal(DecimalRaw(value)),
            Self::Min(Some(value)) | Self::Max(Some(value)) => value,
            Self::Min(None) | Self::Max(None) => Value::U64(0),
        })
    }
}

fn literal_to_value(literal: &TypedLiteral) -> Result<Value> {
    Ok(match (&literal.literal, &literal.value_type) {
        (Literal::Bool(value), ValueType::Bool) => Value::Bool(*value),
        (Literal::String(value), ValueType::String) => Value::String(value.clone()),
        (Literal::Integer(value), ValueType::U64) => Value::U64(*value as u64),
        (Literal::Integer(value), ValueType::I64) => Value::I64(*value as i64),
        (Literal::Integer(value), ValueType::Enum { .. }) => Value::Enum(*value as u64),
        (
            Literal::Integer(value),
            ValueType::Identity {
                allocation: IdentityAllocation::Serial,
                ..
            },
        ) => Value::Identity(IdentityValue::Serial(*value as u64)),
        (
            Literal::Integer(value),
            ValueType::Identity {
                allocation: IdentityAllocation::Application,
                ..
            },
        ) => Value::Identity(IdentityValue::Application(*value as u64)),
        (Literal::Integer(value), ValueType::TimestampMicros) => {
            Value::Timestamp(TimestampMicros(*value as i64))
        }
        (Literal::Integer(value), ValueType::Decimal { .. }) => Value::Decimal(DecimalRaw(*value)),
        _ => {
            return Err(Error::Internal(InternalError::Invariant {
                message: "typed literal mismatch".to_owned(),
            }));
        }
    })
}

fn compare_values(left: &Value, operator: ComparisonOperator, right: &Value) -> bool {
    match operator {
        ComparisonOperator::Eq => left == right,
        ComparisonOperator::NotEq => left != right,
        ComparisonOperator::Lt => left < right,
        ComparisonOperator::Lte => left <= right,
        ComparisonOperator::Gt => left > right,
        ComparisonOperator::Gte => left >= right,
    }
}

fn value_matches_type(value: &Value, value_type: &ValueType) -> bool {
    matches!(
        (value, value_type),
        (Value::Bool(_), ValueType::Bool)
            | (Value::U64(_), ValueType::U64)
            | (Value::I64(_), ValueType::I64)
            | (
                Value::Identity(IdentityValue::Serial(_)),
                ValueType::Identity {
                    allocation: IdentityAllocation::Serial,
                    ..
                }
            )
            | (
                Value::Identity(IdentityValue::Application(_)),
                ValueType::Identity {
                    allocation: IdentityAllocation::Application,
                    ..
                }
            )
            | (Value::Timestamp(_), ValueType::TimestampMicros)
            | (Value::Decimal(_), ValueType::Decimal { .. })
            | (Value::Enum(_), ValueType::Enum { .. })
            | (Value::String(_), ValueType::String)
            | (Value::Bytes(_), ValueType::Bytes)
    )
}

fn value_type_name(value_type: &ValueType) -> String {
    match value_type {
        ValueType::Bool => "bool".to_owned(),
        ValueType::U64 => "u64".to_owned(),
        ValueType::I64 => "i64".to_owned(),
        ValueType::TimestampMicros => "timestamp".to_owned(),
        ValueType::Decimal { scale } => format!("decimal(scale={scale})"),
        ValueType::Enum { name } => name.clone(),
        ValueType::String => "string".to_owned(),
        ValueType::Bytes => "bytes".to_owned(),
        ValueType::Identity {
            type_name,
            owning_relation,
            ..
        } => format!("{type_name}@{owning_relation}"),
    }
}

fn value_kind_name(value: &Value) -> &'static str {
    match value {
        Value::Bool(_) => "bool",
        Value::U64(_) => "u64",
        Value::I64(_) => "i64",
        Value::Identity(_) => "identity",
        Value::Timestamp(_) => "timestamp",
        Value::Decimal(_) => "decimal",
        Value::Enum(_) => "enum",
        Value::String(_) => "string",
        Value::Bytes(_) => "bytes",
    }
}
