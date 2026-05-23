//! Simple in-memory reference model for supported typed query IR.

use std::collections::{BTreeMap, BTreeSet};

use bumbledb_core::encoding::{DecimalRaw, TimestampMicros};
use bumbledb_core::query_ir::{
    ComparisonOperator, Literal, TypedClause, TypedComparison, TypedFindTerm, TypedLiteral,
    TypedOperand, TypedQuery, TypedTerm,
};
use bumbledb_core::schema::ValueType;
use bumbledb_lmdb::{
    Error, ExecuteError, Fact, InputBindings, InternalError, QueryError, Result, Value,
};

/// In-memory reference database.
#[derive(Clone, Debug, Default)]
pub struct ReferenceDb {
    facts: BTreeMap<String, Vec<Fact>>,
}

impl ReferenceDb {
    /// Builds a reference DB from logical facts.
    pub fn from_facts(facts: impl IntoIterator<Item = Fact>) -> Self {
        let mut by_relation: BTreeMap<String, BTreeSet<Fact>> = BTreeMap::new();
        for fact in facts {
            by_relation
                .entry(fact.relation().to_owned())
                .or_default()
                .insert(fact);
        }
        Self {
            facts: by_relation
                .into_iter()
                .map(|(relation, facts)| (relation, facts.into_iter().collect()))
                .collect(),
        }
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
        for fact in self.facts.get(&atom.relation).into_iter().flatten() {
            let Some(next) = match_atom(atom, query, inputs, &binding, fact)? else {
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
    fact: &Fact,
) -> Result<Option<Binding>> {
    let mut next = binding.clone();
    for field in &atom.fields {
        let Some(fact_value) = fact.value(&field.field) else {
            return Ok(None);
        };
        match &field.term {
            TypedTerm::Variable(variable) => {
                if !next.bind(*variable, fact_value.clone()) {
                    return Ok(None);
                }
            }
            TypedTerm::Input(input) => {
                let input_value = input_value(query, inputs, *input)?;
                if input_value != fact_value {
                    return Ok(None);
                }
            }
            TypedTerm::Literal(literal) => {
                if literal_to_value(literal)? != *fact_value {
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
    let mut set = BTreeSet::new();
    for binding in bindings {
        let mut fact = Vec::new();
        for term in &query.find {
            let TypedFindTerm::Variable { variable } = term;
            fact.push(bound_variable(binding, *variable)?.clone());
        }
        set.insert(fact);
    }
    Ok(set.into_iter().collect())
}

fn bound_variable(binding: &Binding, variable: usize) -> Result<&Value> {
    binding.get(variable).ok_or_else(|| {
        Error::Internal(InternalError::Invariant {
            message: format!("variable {variable} is unbound"),
        })
    })
}

fn literal_to_value(literal: &TypedLiteral) -> Result<Value> {
    Ok(match (&literal.literal, &literal.value_type) {
        (Literal::Bool(value), ValueType::Bool) => Value::Bool(*value),
        (Literal::String(value), ValueType::String) => Value::String(value.clone()),
        (Literal::Integer(value), ValueType::U64) => Value::U64(*value as u64),
        (Literal::Integer(value), ValueType::I64) => Value::I64(*value as i64),
        (Literal::Integer(value), ValueType::Enum { .. }) => Value::Enum(*value as u8),
        (Literal::Integer(value), ValueType::Serial { .. }) => Value::Serial(*value as u64),
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
            | (Value::Serial(_), ValueType::Serial { .. })
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
        ValueType::Serial {
            type_name,
            owning_relation,
        } => format!("{type_name}@{owning_relation}"),
    }
}

fn value_kind_name(value: &Value) -> &'static str {
    match value {
        Value::Bool(_) => "bool",
        Value::U64(_) => "u64",
        Value::I64(_) => "i64",
        Value::Serial(_) => "serial",
        Value::Timestamp(_) => "timestamp",
        Value::Decimal(_) => "decimal",
        Value::Enum(_) => "enum",
        Value::String(_) => "string",
        Value::Bytes(_) => "bytes",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumbledb_core::query_builder::{QueryBuildResult, QueryBuilder};
    use bumbledb_core::schema::{FieldDescriptor, RelationDescriptor, SchemaDescriptor};

    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

    fn item_schema() -> SchemaDescriptor {
        SchemaDescriptor::new(
            "ReferenceTestDb",
            vec![RelationDescriptor::new(
                "Item",
                vec![FieldDescriptor::new("id", ValueType::U64)],
            )],
        )
    }

    fn item_query(
        build: impl FnOnce(&mut QueryBuilder<'_>) -> QueryBuildResult<()>,
    ) -> QueryBuildResult<TypedQuery> {
        let schema = item_schema();
        let mut query = QueryBuilder::new(&schema);
        build(&mut query)?;
        query.finish()
    }

    #[test]
    fn from_facts_deduplicates_projection_input_facts() -> TestResult {
        let db = ReferenceDb::from_facts([
            Fact::new("Item", [("id", Value::U64(1))]),
            Fact::new("Item", [("id", Value::U64(1))]),
        ]);
        let query = item_query(|query| {
            query.rel("Item")?.var("id", "id")?.done();
            query.find_var("id")?;
            Ok(())
        })?;

        assert_eq!(
            db.execute(&query, &InputBindings::new())?,
            vec![vec![Value::U64(1)]]
        );
        Ok(())
    }
}
