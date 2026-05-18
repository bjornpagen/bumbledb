use std::collections::{BTreeMap, BTreeSet};

use bumbledb_core::datalog::{
    AggregateFunction, ComparisonOperator, Literal, TypedClause, TypedComparison, TypedFindTerm,
    TypedLiteral, TypedOperand, TypedQuery, TypedRelationAtom, TypedTerm,
};
use bumbledb_core::encoding::{DecimalRaw, TimestampMicros};
use bumbledb_core::schema::{IndexKind, ValueType};

use crate::{Error, FieldValues, IndexScan, ReadTxn, Result, Row, StorageSchema, Value};

/// Query input bindings keyed by input name without `$`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InputBindings {
    values: BTreeMap<String, Value>,
}

impl InputBindings {
    /// Creates empty input bindings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates input bindings from key/value pairs.
    pub fn from_values(values: impl IntoIterator<Item = (impl Into<String>, Value)>) -> Self {
        Self {
            values: values
                .into_iter()
                .map(|(name, value)| (name.into(), value))
                .collect(),
        }
    }

    fn get(&self, name: &str) -> Option<&Value> {
        self.values.get(name)
    }
}

/// Query execution output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryOutput {
    /// Result columns in projection order.
    pub columns: Vec<ResultColumn>,
    /// Result rows in unspecified order.
    pub rows: Vec<Vec<Value>>,
    /// Physical plan and counters.
    pub plan: QueryPlan,
}

/// Result column metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResultColumn {
    /// Projected variable.
    Variable(String),
    /// Aggregate over a variable.
    Aggregate {
        /// Aggregate function.
        function: AggregateFunction,
        /// Variable name.
        variable: String,
    },
}

/// Physical query plan summary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryPlan {
    /// Deterministic variable ordering heuristic output.
    pub variable_order: Vec<String>,
    /// Planned relation atoms in execution order.
    pub atoms: Vec<PlannedAtom>,
    /// Execution counters.
    pub counters: PlanCounters,
    /// True when multiple relation atoms are evaluated as one indexed multiway search.
    pub uses_indexed_multiway_join: bool,
}

/// Planned relation atom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedAtom {
    /// Relation name.
    pub relation: String,
    /// Chosen index name.
    pub index: String,
    /// Chosen index kind.
    pub kind: IndexKind,
    /// Prefix fields expected to be bound when this atom runs.
    pub prefix_fields: Vec<String>,
}

/// Execution counters for stage-06 explain metadata.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlanCounters {
    /// Number of cursor-backed row candidates scanned.
    pub rows_scanned: u64,
    /// Number of candidate rows accepted by relation atom matching.
    pub rows_matched: u64,
    /// Number of complete bindings yielded before projection/aggregation.
    pub bindings_yielded: u64,
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

#[derive(Clone, Debug)]
struct ExecutionPlan<'query> {
    atom_indices: Vec<usize>,
    relation_atoms: Vec<&'query TypedRelationAtom>,
    comparisons: Vec<&'query TypedComparison>,
    summary: QueryPlan,
}

impl<'env> ReadTxn<'env> {
    /// Executes a typed positive Datalog query against current indexes.
    pub fn execute_query(
        &self,
        schema: &StorageSchema,
        query: &TypedQuery,
        inputs: &InputBindings,
    ) -> Result<QueryOutput> {
        validate_inputs(query, inputs)?;

        let mut plan = plan_query(schema, query, inputs)?;
        let mut bindings = Vec::new();
        let initial = Binding::new(query.variables.len());

        self.execute_atoms(schema, query, inputs, &mut plan, 0, initial, &mut bindings)?;

        let columns = result_columns(query);
        let rows = project_results(query, &bindings)?;
        Ok(QueryOutput {
            columns,
            rows,
            plan: plan.summary,
        })
    }

    fn execute_atoms(
        &self,
        schema: &StorageSchema,
        query: &TypedQuery,
        inputs: &InputBindings,
        plan: &mut ExecutionPlan<'_>,
        depth: usize,
        binding: Binding,
        output: &mut Vec<Binding>,
    ) -> Result<()> {
        if depth == plan.atom_indices.len() {
            if comparisons_pass(&plan.comparisons, query, inputs, &binding)? {
                plan.summary.counters.bindings_yielded += 1;
                output.push(binding);
            }
            return Ok(());
        }

        let atom = plan.relation_atoms[plan.atom_indices[depth]];
        let access = choose_access_path(schema, atom, query, inputs, &binding)?;
        plan.summary.atoms[depth] = access.summary.clone();

        let scan = open_scan(self, schema, atom, &access)?;
        for item in scan {
            plan.summary.counters.rows_scanned += 1;
            let item = item?;
            let Some(next_binding) = match_atom(atom, query, inputs, &binding, &item.row)? else {
                continue;
            };
            if !comparisons_pass(&plan.comparisons, query, inputs, &next_binding)? {
                continue;
            }
            plan.summary.counters.rows_matched += 1;
            self.execute_atoms(schema, query, inputs, plan, depth + 1, next_binding, output)?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug)]
struct ChosenAccess {
    summary: PlannedAtom,
    prefix: Option<FieldValues>,
    range: Option<(Option<Value>, Option<Value>)>,
}

fn plan_query<'query>(
    schema: &StorageSchema,
    query: &'query TypedQuery,
    inputs: &InputBindings,
) -> Result<ExecutionPlan<'query>> {
    let relation_atoms = query
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

    let mut variable_degree = vec![0usize; query.variables.len()];
    for atom in &relation_atoms {
        for field in &atom.fields {
            if let TypedTerm::Variable(variable) = field.term {
                variable_degree[variable] += 1;
            }
        }
    }

    let mut atom_indices = (0..relation_atoms.len()).collect::<Vec<_>>();
    atom_indices.sort_by_key(|index| {
        let atom = relation_atoms[*index];
        let static_bound = atom
            .fields
            .iter()
            .filter(|field| matches!(field.term, TypedTerm::Input(_) | TypedTerm::Literal(_)))
            .count();
        let degree = atom
            .fields
            .iter()
            .map(|field| match field.term {
                TypedTerm::Variable(variable) => variable_degree[variable],
                _ => 0,
            })
            .sum::<usize>();
        (
            std::cmp::Reverse(static_bound),
            std::cmp::Reverse(degree),
            *index,
        )
    });

    let mut atoms = Vec::new();
    for index in &atom_indices {
        let access = choose_access_path(
            schema,
            relation_atoms[*index],
            query,
            inputs,
            &Binding::new(query.variables.len()),
        )?;
        atoms.push(access.summary);
    }

    Ok(ExecutionPlan {
        atom_indices,
        relation_atoms,
        comparisons,
        summary: QueryPlan {
            variable_order: variable_order(query, &variable_degree),
            atoms,
            counters: PlanCounters::default(),
            uses_indexed_multiway_join: query
                .clauses
                .iter()
                .filter(|clause| matches!(clause, TypedClause::Relation(_)))
                .count()
                > 1,
        },
    })
}

fn variable_order(query: &TypedQuery, degree: &[usize]) -> Vec<String> {
    let mut ids = (0..query.variables.len()).collect::<Vec<_>>();
    ids.sort_by_key(|id| {
        (
            std::cmp::Reverse(degree[*id]),
            query.variables[*id].name.clone(),
        )
    });
    ids.into_iter()
        .map(|id| query.variables[id].name.clone())
        .collect()
}

fn choose_access_path(
    schema: &StorageSchema,
    atom: &TypedRelationAtom,
    query: &TypedQuery,
    inputs: &InputBindings,
    binding: &Binding,
) -> Result<ChosenAccess> {
    let paths = schema.access_paths(&atom.relation)?;
    let mut best: Option<(usize, usize, &crate::AccessPathDescriptor, FieldValues)> = None;

    for path in &paths {
        let mut prefix_values = Vec::new();
        for leading in &path.leading_fields {
            let Some(field) = atom.fields.iter().find(|field| &field.field == leading) else {
                break;
            };
            let Some(value) = term_bound_value(&field.term, query, inputs, binding)? else {
                break;
            };
            prefix_values.push((
                leading.clone(),
                normalize_value_for_type(&value, &field.value_type),
            ));
        }

        let score = prefix_values.len();
        let kind_rank = match path.kind {
            IndexKind::Unique => 4,
            IndexKind::Primary => 3,
            IndexKind::Ref => 2,
            IndexKind::Range => 1,
        };
        let candidate = (
            score,
            kind_rank,
            path,
            FieldValues::new(&atom.relation, prefix_values),
        );
        if best
            .as_ref()
            .is_none_or(|best| (candidate.0, candidate.1) > (best.0, best.1))
        {
            best = Some(candidate);
        }
    }

    let (score, _, path, prefix) =
        best.ok_or_else(|| Error::Internal("relation has no access paths".to_owned()))?;
    if score > 0 {
        return Ok(ChosenAccess {
            summary: PlannedAtom {
                relation: atom.relation.clone(),
                index: path.index_name.clone(),
                kind: path.kind,
                prefix_fields: path.leading_fields[..score].to_vec(),
            },
            prefix: Some(prefix),
            range: None,
        });
    }

    if let Some((path, start, end)) = range_access(schema, atom, query, inputs, binding)? {
        return Ok(ChosenAccess {
            summary: PlannedAtom {
                relation: atom.relation.clone(),
                index: path.index_name.clone(),
                kind: path.kind,
                prefix_fields: path.leading_fields.clone(),
            },
            prefix: None,
            range: Some((start, end)),
        });
    }

    let primary = paths
        .iter()
        .find(|path| path.kind == IndexKind::Primary)
        .ok_or_else(|| Error::Internal(format!("missing primary index for {}", atom.relation)))?;
    Ok(ChosenAccess {
        summary: PlannedAtom {
            relation: atom.relation.clone(),
            index: primary.index_name.clone(),
            kind: primary.kind,
            prefix_fields: Vec::new(),
        },
        prefix: None,
        range: None,
    })
}

fn range_access(
    schema: &StorageSchema,
    atom: &TypedRelationAtom,
    query: &TypedQuery,
    inputs: &InputBindings,
    binding: &Binding,
) -> Result<Option<(crate::AccessPathDescriptor, Option<Value>, Option<Value>)>> {
    for path in schema.access_paths(&atom.relation)? {
        if path.kind != IndexKind::Range || path.leading_fields.len() != 1 {
            continue;
        }
        let field_name = &path.leading_fields[0];
        let Some(field) = atom.fields.iter().find(|field| &field.field == field_name) else {
            continue;
        };
        let TypedTerm::Variable(variable) = field.term else {
            continue;
        };
        let mut start = None;
        let mut end = None;
        for clause in &query.clauses {
            let TypedClause::Comparison(comparison) = clause else {
                continue;
            };
            if let Some((bound_start, bound_end)) =
                comparison_bound(comparison, variable, query, inputs, binding)?
            {
                start = bound_start.or(start);
                end = bound_end.or(end);
            }
        }
        if start.is_some() || end.is_some() {
            return Ok(Some((path, start, end)));
        }
    }
    Ok(None)
}

fn comparison_bound(
    comparison: &TypedComparison,
    variable: usize,
    query: &TypedQuery,
    inputs: &InputBindings,
    binding: &Binding,
) -> Result<Option<(Option<Value>, Option<Value>)>> {
    let left_is_var = matches!(comparison.left, TypedOperand::Variable(id) if id == variable);
    let right_is_var = matches!(comparison.right, TypedOperand::Variable(id) if id == variable);
    if left_is_var {
        let Some(value) = operand_value(&comparison.right, query, inputs, binding)? else {
            return Ok(None);
        };
        return Ok(match comparison.operator {
            ComparisonOperator::Gt | ComparisonOperator::Gte => Some((
                Some(normalize_value_for_type(&value, &comparison.value_type)),
                None,
            )),
            ComparisonOperator::Lt | ComparisonOperator::Lte => Some((
                None,
                Some(normalize_value_for_type(&value, &comparison.value_type)),
            )),
            _ => None,
        });
    }
    if right_is_var {
        let Some(value) = operand_value(&comparison.left, query, inputs, binding)? else {
            return Ok(None);
        };
        return Ok(match comparison.operator {
            ComparisonOperator::Lt | ComparisonOperator::Lte => Some((
                Some(normalize_value_for_type(&value, &comparison.value_type)),
                None,
            )),
            ComparisonOperator::Gt | ComparisonOperator::Gte => Some((
                None,
                Some(normalize_value_for_type(&value, &comparison.value_type)),
            )),
            _ => None,
        });
    }
    Ok(None)
}

fn open_scan<'borrow, 'env, 'schema>(
    txn: &'borrow ReadTxn<'env>,
    schema: &'schema StorageSchema,
    atom: &TypedRelationAtom,
    access: &ChosenAccess,
) -> Result<IndexScan<'borrow, 'env, 'schema>> {
    if let Some((start, end)) = &access.range {
        txn.scan_range(
            schema,
            &atom.relation,
            &access.summary.index,
            start.clone(),
            end.clone(),
        )
    } else if let Some(prefix) = &access.prefix {
        txn.scan_prefix(schema, &atom.relation, &access.summary.index, prefix)
    } else {
        txn.scan_relation(schema, &atom.relation)
    }
}

fn match_atom(
    atom: &TypedRelationAtom,
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
                let normalized =
                    normalize_value_for_type(row_value, &query.variables[*variable].value_type);
                if !next.bind(*variable, normalized) {
                    return Ok(None);
                }
            }
            TypedTerm::Input(input) => {
                let input_value = input_value(query, inputs, *input)?;
                let normalized =
                    normalize_value_for_type(row_value, &query.inputs[*input].value_type);
                if input_value != &normalized {
                    return Ok(None);
                }
            }
            TypedTerm::Literal(literal) => {
                let normalized = normalize_value_for_type(row_value, &literal.value_type);
                if literal_to_value(literal)? != normalized {
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
        let left = normalize_value_for_type(&left, &comparison.value_type);
        let right = normalize_value_for_type(&right, &comparison.value_type);
        if !compare_values(&left, comparison.operator, &right) {
            return Ok(false);
        }
    }
    Ok(true)
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

fn term_bound_value(
    term: &TypedTerm,
    query: &TypedQuery,
    inputs: &InputBindings,
    binding: &Binding,
) -> Result<Option<Value>> {
    Ok(match term {
        TypedTerm::Variable(variable) => binding.get(*variable).cloned(),
        TypedTerm::Input(input) => Some(input_value(query, inputs, *input)?.clone()),
        TypedTerm::Literal(literal) => Some(literal_to_value(literal)?),
        TypedTerm::Wildcard => None,
    })
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
    let value = inputs.get(&input.name).ok_or_else(|| Error::MissingInput {
        input: input.name.clone(),
    })?;
    if !value_matches_type(value, &input.value_type) {
        return Err(Error::QueryInputTypeMismatch {
            input: input.name.clone(),
            expected: value_type_name(&input.value_type),
            actual: value.kind_name(),
        });
    }
    Ok(value)
}

fn validate_inputs(query: &TypedQuery, inputs: &InputBindings) -> Result<()> {
    for input in &query.inputs {
        input_value(query, inputs, input.id)?;
    }
    Ok(())
}

fn value_matches_type(value: &Value, value_type: &ValueType) -> bool {
    matches!(
        (value, value_type),
        (Value::Bool(_), ValueType::Bool)
            | (Value::U64(_), ValueType::U64)
            | (Value::I64(_), ValueType::I64)
            | (Value::Id(_), ValueType::Id { .. })
            | (Value::Ref(_), ValueType::Ref { .. })
            | (Value::Timestamp(_), ValueType::TimestampMicros)
            | (Value::Decimal(_), ValueType::Decimal { .. })
            | (Value::Uuid(_), ValueType::Uuid)
            | (Value::Symbol(_), ValueType::Symbol { .. })
            | (Value::String(_), ValueType::String)
            | (Value::Bytes(_), ValueType::Bytes)
    )
}

fn normalize_value_for_type(value: &Value, value_type: &ValueType) -> Value {
    match (value, value_type) {
        (Value::Ref(raw), ValueType::Id { .. }) => Value::Id(*raw),
        (Value::Id(raw), ValueType::Ref { .. }) => Value::Ref(*raw),
        _ => value.clone(),
    }
}

fn literal_to_value(literal: &TypedLiteral) -> Result<Value> {
    let value = match (&literal.literal, &literal.value_type) {
        (Literal::Bool(value), ValueType::Bool) => Value::Bool(*value),
        (Literal::String(value), ValueType::String) => Value::String(value.clone()),
        (Literal::Integer(value), ValueType::U64) => Value::U64(*value as u64),
        (Literal::Integer(value), ValueType::I64) => Value::I64(*value as i64),
        (Literal::Integer(value), ValueType::Id { .. }) => Value::Id(*value as u64),
        (Literal::Integer(value), ValueType::Ref { .. }) => Value::Ref(*value as u64),
        (Literal::Integer(value), ValueType::Symbol { .. }) => Value::Symbol(*value as u64),
        (Literal::Integer(value), ValueType::TimestampMicros) => {
            Value::Timestamp(TimestampMicros(*value as i64))
        }
        (Literal::Integer(value), ValueType::Decimal { .. }) => Value::Decimal(DecimalRaw(*value)),
        _ => {
            return Err(Error::Internal(
                "typed literal does not match literal value".to_owned(),
            ));
        }
    };
    Ok(value)
}

fn result_columns(query: &TypedQuery) -> Vec<ResultColumn> {
    query
        .find
        .iter()
        .map(|term| match term {
            TypedFindTerm::Variable { variable } => {
                ResultColumn::Variable(query.variables[*variable].name.clone())
            }
            TypedFindTerm::Aggregate {
                function, variable, ..
            } => ResultColumn::Aggregate {
                function: *function,
                variable: query.variables[*variable].name.clone(),
            },
        })
        .collect()
}

fn project_results(query: &TypedQuery, bindings: &[Binding]) -> Result<Vec<Vec<Value>>> {
    let has_aggregate = query
        .find
        .iter()
        .any(|term| matches!(term, TypedFindTerm::Aggregate { .. }));
    if has_aggregate {
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
        let mut row = Vec::new();
        let mut key_iter = key.into_iter();
        let mut state_iter = states.into_iter();
        for term in &query.find {
            match term {
                TypedFindTerm::Variable { .. } => row.push(key_iter.next().unwrap()),
                TypedFindTerm::Aggregate { .. } => row.push(state_iter.next().unwrap().finish()?),
            }
        }
        rows.push(row);
    }
    rows.sort();
    Ok(rows)
}

fn bound_variable(binding: &Binding, variable: usize) -> Result<&Value> {
    binding
        .get(variable)
        .ok_or_else(|| Error::Internal(format!("variable {variable} is unbound at projection")))
}

#[derive(Clone, Debug)]
enum AggregateState {
    Count(u64),
    SumU64(u64),
    SumI64(i64),
    SumDecimal(i128),
    Min(Option<Value>),
    Max(Option<Value>),
}

impl AggregateState {
    fn new(function: AggregateFunction, value_type: ValueType) -> Self {
        match (function, value_type) {
            (AggregateFunction::Count, _) => AggregateState::Count(0),
            (AggregateFunction::Sum, ValueType::U64) => AggregateState::SumU64(0),
            (AggregateFunction::Sum, ValueType::I64) => AggregateState::SumI64(0),
            (AggregateFunction::Sum, ValueType::Decimal { .. }) => AggregateState::SumDecimal(0),
            (AggregateFunction::Min, _) => AggregateState::Min(None),
            (AggregateFunction::Max, _) => AggregateState::Max(None),
            _ => AggregateState::Count(0),
        }
    }

    fn apply(&mut self, value: &Value) -> Result<()> {
        match self {
            AggregateState::Count(count) => {
                *count = count
                    .checked_add(1)
                    .ok_or(Error::IntegerOverflow { operation: "count" })?;
            }
            AggregateState::SumU64(sum) => {
                let Value::U64(value) = value else {
                    return Err(Error::Internal("sum(u64) received non-u64".to_owned()));
                };
                *sum = sum
                    .checked_add(*value)
                    .ok_or(Error::IntegerOverflow { operation: "sum" })?;
            }
            AggregateState::SumI64(sum) => {
                let Value::I64(value) = value else {
                    return Err(Error::Internal("sum(i64) received non-i64".to_owned()));
                };
                *sum = sum
                    .checked_add(*value)
                    .ok_or(Error::IntegerOverflow { operation: "sum" })?;
            }
            AggregateState::SumDecimal(sum) => {
                let Value::Decimal(DecimalRaw(value)) = value else {
                    return Err(Error::Internal(
                        "sum(decimal) received non-decimal".to_owned(),
                    ));
                };
                *sum = sum
                    .checked_add(*value)
                    .ok_or(Error::DecimalOverflow { operation: "sum" })?;
            }
            AggregateState::Min(current) => match current {
                Some(existing) if &*existing <= value => {}
                _ => *current = Some(value.clone()),
            },
            AggregateState::Max(current) => match current {
                Some(existing) if &*existing >= value => {}
                _ => *current = Some(value.clone()),
            },
        }
        Ok(())
    }

    fn finish(self) -> Result<Value> {
        Ok(match self {
            AggregateState::Count(count) => Value::U64(count),
            AggregateState::SumU64(sum) => Value::U64(sum),
            AggregateState::SumI64(sum) => Value::I64(sum),
            AggregateState::SumDecimal(sum) => Value::Decimal(DecimalRaw(sum)),
            AggregateState::Min(Some(value)) | AggregateState::Max(Some(value)) => value,
            AggregateState::Min(None) | AggregateState::Max(None) => Value::U64(0),
        })
    }
}

fn value_type_name(value_type: &ValueType) -> String {
    match value_type {
        ValueType::Bool => "bool".to_owned(),
        ValueType::U64 => "u64".to_owned(),
        ValueType::I64 => "i64".to_owned(),
        ValueType::Id { name, .. } => name.clone(),
        ValueType::Ref { name, .. } => name.clone(),
        ValueType::TimestampMicros => "timestamp".to_owned(),
        ValueType::Decimal { scale } => format!("decimal(scale={scale})"),
        ValueType::Uuid => "uuid".to_owned(),
        ValueType::Symbol { name } => name.clone(),
        ValueType::String => "string".to_owned(),
        ValueType::Bytes => "bytes".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Environment;
    use bumbledb_core::datalog::parse_and_typecheck;
    use bumbledb_core::schema::{
        FieldDescriptor, PrimaryKeyDescriptor, RelationDescriptor, RelationKind,
    };

    #[test]
    fn executes_single_relation_query() {
        let (env, schema) = seeded_db();
        let query = parse_and_typecheck(
            schema.descriptor(),
            "find ?account where Account(id: ?account, holder: $holder)",
        )
        .unwrap();

        let output = env
            .read(|txn| {
                txn.execute_query(
                    &schema,
                    &query,
                    &InputBindings::from_values([("holder", Value::Ref(1))]),
                )
            })
            .unwrap();

        assert_eq!(output.rows, vec![vec![Value::Id(1)], vec![Value::Id(2)]]);
        assert_eq!(output.plan.atoms[0].index, "by_holder");
    }

    #[test]
    fn executes_two_relation_join() {
        let (env, schema) = seeded_db();
        let query = parse_and_typecheck(
            schema.descriptor(),
            r#"
            find ?account ?holder_name
            where
              Account(id: ?account, holder: ?holder)
              Holder(id: ?holder, name: ?holder_name)
            "#,
        )
        .unwrap();

        let output = env
            .read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))
            .unwrap();
        assert!(output.plan.uses_indexed_multiway_join);
        assert_same_rows(
            &output.rows,
            &[
                vec![Value::Id(1), Value::String("Alice".to_owned())],
                vec![Value::Id(2), Value::String("Alice".to_owned())],
                vec![Value::Id(3), Value::String("Bob".to_owned())],
            ],
        );
    }

    #[test]
    fn executes_many_relation_join_and_range_filter() {
        let (env, schema) = seeded_db();
        let query = parse_and_typecheck(
            schema.descriptor(),
            r#"
            find ?posting ?account ?holder_name
            where
              Posting(id: ?posting, account: ?account, amount: ?amount, at: ?t)
              Account(id: ?account, holder: ?holder)
              Holder(id: ?holder, name: ?holder_name)
              ?t >= $start
              ?t < $end
            "#,
        )
        .unwrap();

        let output = env
            .read(|txn| {
                txn.execute_query(
                    &schema,
                    &query,
                    &InputBindings::from_values([
                        ("start", Value::Timestamp(TimestampMicros(15))),
                        ("end", Value::Timestamp(TimestampMicros(35))),
                    ]),
                )
            })
            .unwrap();

        assert!(output.plan.atoms.iter().any(|atom| atom.index == "by_at"));
        assert_same_rows(
            &output.rows,
            &[
                vec![
                    Value::Id(2),
                    Value::Id(1),
                    Value::String("Alice".to_owned()),
                ],
                vec![
                    Value::Id(3),
                    Value::Id(2),
                    Value::String("Alice".to_owned()),
                ],
            ],
        );
    }

    #[test]
    fn projection_uses_set_semantics() {
        let (env, schema) = seeded_db();
        let query = parse_and_typecheck(
            schema.descriptor(),
            "find ?holder where Account(id: ?account, holder: ?holder)",
        )
        .unwrap();

        let output = env
            .read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))
            .unwrap();
        assert_eq!(output.rows, vec![vec![Value::Ref(1)], vec![Value::Ref(2)]]);
    }

    #[test]
    fn aggregation_groups_and_sums_decimal_values() {
        let (env, schema) = seeded_db();
        let query = parse_and_typecheck(
            schema.descriptor(),
            r#"
            find ?account sum(?amount) count(?posting) min(?t) max(?t)
            where
              Posting(id: ?posting, account: ?account, amount: ?amount, at: ?t)
            "#,
        )
        .unwrap();

        let output = env
            .read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))
            .unwrap();

        assert_same_rows(
            &output.rows,
            &[
                vec![
                    Value::Ref(1),
                    Value::Decimal(DecimalRaw(300)),
                    Value::U64(2),
                    Value::Timestamp(TimestampMicros(10)),
                    Value::Timestamp(TimestampMicros(20)),
                ],
                vec![
                    Value::Ref(2),
                    Value::Decimal(DecimalRaw(300)),
                    Value::U64(1),
                    Value::Timestamp(TimestampMicros(30)),
                    Value::Timestamp(TimestampMicros(30)),
                ],
            ],
        );
    }

    #[test]
    fn detects_integer_and_decimal_aggregation_overflow() {
        let dir = tempfile::tempdir().unwrap();
        let env = Environment::open(dir.path()).unwrap();
        let schema = StorageSchema::new(overflow_schema(), env.max_key_size()).unwrap();
        env.write(|txn| {
            txn.insert(&schema, number_row(1, i64::MAX, i128::MAX))?;
            txn.insert(&schema, number_row(2, 1, 1))?;
            Ok::<(), Error>(())
        })
        .unwrap();

        let int_query =
            parse_and_typecheck(schema.descriptor(), "find sum(?n) where Number(n: ?n)").unwrap();
        let int_error = env
            .read(|txn| txn.execute_query(&schema, &int_query, &InputBindings::new()))
            .unwrap_err();
        assert!(matches!(int_error, Error::IntegerOverflow { .. }));

        let decimal_query =
            parse_and_typecheck(schema.descriptor(), "find sum(?d) where Number(d: ?d)").unwrap();
        let decimal_error = env
            .read(|txn| txn.execute_query(&schema, &decimal_query, &InputBindings::new()))
            .unwrap_err();
        assert!(matches!(decimal_error, Error::DecimalOverflow { .. }));
    }

    #[test]
    fn input_type_mismatch_is_rejected_at_execution() {
        let (env, schema) = seeded_db();
        let query = parse_and_typecheck(
            schema.descriptor(),
            "find ?account where Account(id: ?account, holder: $holder)",
        )
        .unwrap();
        let error = env
            .read(|txn| {
                txn.execute_query(
                    &schema,
                    &query,
                    &InputBindings::from_values([("holder", Value::String("bad".to_owned()))]),
                )
            })
            .unwrap_err();
        assert!(matches!(error, Error::QueryInputTypeMismatch { .. }));
    }

    fn seeded_db() -> (Environment, StorageSchema) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.keep();
        let env = Environment::open(&path).unwrap();
        let schema = StorageSchema::new(ledger_schema(), env.max_key_size()).unwrap();
        env.write(|txn| {
            txn.insert(&schema, holder_row(1, "Alice"))?;
            txn.insert(&schema, holder_row(2, "Bob"))?;
            txn.insert(&schema, account_row(1, 1, 840))?;
            txn.insert(&schema, account_row(2, 1, 978))?;
            txn.insert(&schema, account_row(3, 2, 840))?;
            txn.insert(&schema, posting_row(1, 1, 100, 10))?;
            txn.insert(&schema, posting_row(2, 1, 200, 20))?;
            txn.insert(&schema, posting_row(3, 2, 300, 30))?;
            Ok::<(), Error>(())
        })
        .unwrap();
        (env, schema)
    }

    fn ledger_schema() -> bumbledb_core::schema::SchemaDescriptor {
        bumbledb_core::schema::SchemaDescriptor::new(
            "LedgerDb",
            vec![
                RelationDescriptor::new(
                    "Holder",
                    RelationKind::Entity,
                    vec![
                        FieldDescriptor::new(
                            "id",
                            ValueType::Id {
                                name: "HolderId".to_owned(),
                                relation: "Holder".to_owned(),
                            },
                        ),
                        FieldDescriptor::new("name", ValueType::String),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                )
                .with_generated_id(bumbledb_core::schema::GeneratedIdDescriptor::new("id")),
                RelationDescriptor::new(
                    "Account",
                    RelationKind::Entity,
                    vec![
                        FieldDescriptor::new(
                            "id",
                            ValueType::Id {
                                name: "AccountId".to_owned(),
                                relation: "Account".to_owned(),
                            },
                        ),
                        FieldDescriptor::new(
                            "holder",
                            ValueType::Ref {
                                name: "HolderId".to_owned(),
                                target_relation: "Holder".to_owned(),
                            },
                        ),
                        FieldDescriptor::new(
                            "currency",
                            ValueType::Symbol {
                                name: "Currency".to_owned(),
                            },
                        ),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                )
                .with_generated_id(bumbledb_core::schema::GeneratedIdDescriptor::new("id")),
                RelationDescriptor::new(
                    "Posting",
                    RelationKind::Event,
                    vec![
                        FieldDescriptor::new(
                            "id",
                            ValueType::Id {
                                name: "PostingId".to_owned(),
                                relation: "Posting".to_owned(),
                            },
                        ),
                        FieldDescriptor::new(
                            "account",
                            ValueType::Ref {
                                name: "AccountId".to_owned(),
                                target_relation: "Account".to_owned(),
                            },
                        ),
                        FieldDescriptor::new("amount", ValueType::Decimal { scale: 4 }),
                        FieldDescriptor::new("at", ValueType::TimestampMicros).range_indexed(),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                )
                .with_generated_id(bumbledb_core::schema::GeneratedIdDescriptor::new("id")),
            ],
        )
    }

    fn overflow_schema() -> bumbledb_core::schema::SchemaDescriptor {
        bumbledb_core::schema::SchemaDescriptor::new(
            "OverflowDb",
            vec![RelationDescriptor::new(
                "Number",
                RelationKind::Entity,
                vec![
                    FieldDescriptor::new(
                        "id",
                        ValueType::Id {
                            name: "NumberId".to_owned(),
                            relation: "Number".to_owned(),
                        },
                    ),
                    FieldDescriptor::new("n", ValueType::I64),
                    FieldDescriptor::new("d", ValueType::Decimal { scale: 0 }),
                ],
                PrimaryKeyDescriptor::new(["id"]),
            )],
        )
    }

    fn holder_row(id: u64, name: &str) -> Row {
        Row::new(
            "Holder",
            [
                ("id", Value::Id(id)),
                ("name", Value::String(name.to_owned())),
            ],
        )
    }

    fn account_row(id: u64, holder: u64, currency: u64) -> Row {
        Row::new(
            "Account",
            [
                ("id", Value::Id(id)),
                ("holder", Value::Ref(holder)),
                ("currency", Value::Symbol(currency)),
            ],
        )
    }

    fn posting_row(id: u64, account: u64, amount: i128, at: i64) -> Row {
        Row::new(
            "Posting",
            [
                ("id", Value::Id(id)),
                ("account", Value::Ref(account)),
                ("amount", Value::Decimal(DecimalRaw(amount))),
                ("at", Value::Timestamp(TimestampMicros(at))),
            ],
        )
    }

    fn number_row(id: u64, n: i64, d: i128) -> Row {
        Row::new(
            "Number",
            [
                ("id", Value::Id(id)),
                ("n", Value::I64(n)),
                ("d", Value::Decimal(DecimalRaw(d))),
            ],
        )
    }

    fn assert_same_rows(actual: &[Vec<Value>], expected: &[Vec<Value>]) {
        let mut actual = actual.to_vec();
        let mut expected = expected.to_vec();
        actual.sort();
        expected.sort();
        assert_eq!(actual, expected);
    }
}
