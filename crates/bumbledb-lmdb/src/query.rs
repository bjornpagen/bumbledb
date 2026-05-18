use std::collections::{BTreeMap, BTreeSet};

use bumbledb_core::datalog::{
    AggregateFunction, ComparisonOperator, Literal, TypedClause, TypedComparison, TypedFindTerm,
    TypedLiteral, TypedOperand, TypedQuery, TypedRelationAtom, TypedTerm,
};
use bumbledb_core::encoding::{DecimalRaw, TimestampMicros};
use bumbledb_core::schema::{IndexComponent, IndexKind, ValueType};

use crate::storage::EncodedIndexItem;
use crate::{AccessPathDescriptor, Error, ReadTxn, Result, StorageSchema, Value};

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

    /// Returns a bound input value by name.
    pub fn value(&self, name: &str) -> Option<&Value> {
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

impl QueryOutput {
    /// Renders a human-readable explain plan for this executed query.
    pub fn explain(&self) -> String {
        self.plan.explain()
    }
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
    /// Deterministic variable ordering optimizer output.
    pub variable_order: Vec<String>,
    /// Estimated work for variables in execution order.
    pub variable_estimates: Vec<VariableEstimate>,
    /// Planned relation atoms in execution order.
    pub atoms: Vec<PlannedAtom>,
    /// Physical index recommendations for predicates not served by leading indexes.
    pub missing_indexes: Vec<MissingIndexRecommendation>,
    /// Execution counters.
    pub counters: PlanCounters,
    /// True when multiple relation atoms are evaluated as one indexed multiway search.
    pub uses_indexed_multiway_join: bool,
}

impl QueryPlan {
    /// Renders this physical plan and its current execution counters.
    pub fn explain(&self) -> String {
        let mut out = String::new();
        out.push_str("QueryPlan\n");
        out.push_str(&format!("variable_order: {:?}\n", self.variable_order));
        out.push_str(&format!(
            "uses_indexed_multiway_join: {}\n",
            self.uses_indexed_multiway_join
        ));
        out.push_str("variable_estimates:\n");
        for estimate in &self.variable_estimates {
            out.push_str(&format!(
                "  variable_estimate name={} estimated_candidates={} static_constraints={} bound_constraints={} relation_constraints={}\n",
                estimate.variable,
                estimate.estimated_candidates,
                estimate.static_constraints,
                estimate.bound_constraints,
                estimate.relation_constraints
            ));
        }
        out.push_str("atoms:\n");
        for atom in &self.atoms {
            out.push_str(&format!(
                "  relation={} index={} kind={:?} prefix_fields={:?}\n",
                atom.relation, atom.index, atom.kind, atom.prefix_fields
            ));
        }
        if !self.missing_indexes.is_empty() {
            out.push_str("missing_indexes:\n");
            for missing in &self.missing_indexes {
                out.push_str(&format!(
                    "  missing_index relation={} fields={:?} reason={}\n",
                    missing.relation, missing.fields, missing.reason
                ));
            }
        }
        out.push_str("counters:\n");
        out.push_str(&format!("  cursor_seeks: {}\n", self.counters.cursor_seeks));
        out.push_str(&format!("  rows_scanned: {}\n", self.counters.rows_scanned));
        out.push_str(&format!("  rows_matched: {}\n", self.counters.rows_matched));
        out.push_str(&format!(
            "  bindings_yielded: {}\n",
            self.counters.bindings_yielded
        ));
        out.push_str(&format!(
            "  comparisons_evaluated: {}\n",
            self.counters.comparisons_evaluated
        ));
        out.push_str(&format!(
            "  comparisons_failed: {}\n",
            self.counters.comparisons_failed
        ));
        out.push_str(&format!(
            "  aggregate_groups: {}\n",
            self.counters.aggregate_groups
        ));
        out.push_str(&format!(
            "  trie_intersections: {}\n",
            self.counters.trie_intersections
        ));
        out.push_str(&format!(
            "  variable_candidates: {}\n",
            self.counters.variable_candidates
        ));
        out.push_str(&format!(
            "  decoded_values: {}\n",
            self.counters.decoded_values
        ));
        out.push_str(&format!(
            "  dictionary_reverse_lookups: {}\n",
            self.counters.dictionary_reverse_lookups
        ));
        out.push_str(&format!(
            "  encoded_comparisons_evaluated: {}\n",
            self.counters.encoded_comparisons_evaluated
        ));
        out.push_str(&format!(
            "  decoded_comparisons_evaluated: {}\n",
            self.counters.decoded_comparisons_evaluated
        ));
        out.push_str(&format!(
            "  materialized_output_values: {}\n",
            self.counters.materialized_output_values
        ));
        out.push_str(&format!("  output_rows: {}\n", self.counters.output_rows));
        out
    }
}

/// Optimizer estimate for one variable in execution order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VariableEstimate {
    /// Variable name without `?`.
    pub variable: String,
    /// Estimated candidate domain size at the point this variable is bound.
    pub estimated_candidates: u64,
    /// Input/literal/comparison constraints available before binding this variable.
    pub static_constraints: usize,
    /// Already-bound variable constraints available before binding this variable.
    pub bound_constraints: usize,
    /// Number of relation atoms constraining this variable.
    pub relation_constraints: usize,
}

/// Physical index recommendation emitted by the planner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissingIndexRecommendation {
    /// Relation name.
    pub relation: String,
    /// Suggested leading fields.
    pub fields: Vec<String>,
    /// Why the planner wants this index.
    pub reason: String,
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

/// Execution counters for the encoded trie/WCOJ executor.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlanCounters {
    /// Number of encoded index scan openings.
    pub cursor_seeks: u64,
    /// Number of encoded index entries inspected.
    pub rows_scanned: u64,
    /// Number of encoded index entries accepted by currently bound constraints.
    pub rows_matched: u64,
    /// Number of complete encoded bindings yielded before projection/aggregation.
    pub bindings_yielded: u64,
    /// Number of comparison predicates evaluated.
    pub comparisons_evaluated: u64,
    /// Number of comparison predicate failures.
    pub comparisons_failed: u64,
    /// Number of aggregate groups produced.
    pub aggregate_groups: u64,
    /// Number of final output rows.
    pub output_rows: u64,
    /// Number of variable-domain intersections performed.
    pub trie_intersections: u64,
    /// Number of candidate variable values produced after intersection.
    pub variable_candidates: u64,
    /// Number of logical values decoded for comparisons/projection/aggregation.
    pub decoded_values: u64,
    /// Number of string/bytes dictionary reverse lookups caused by decoding.
    pub dictionary_reverse_lookups: u64,
    /// Number of comparison predicates evaluated directly on encoded bytes.
    pub encoded_comparisons_evaluated: u64,
    /// Number of comparison predicates evaluated after logical decoding.
    pub decoded_comparisons_evaluated: u64,
    /// Number of final logical output values materialized.
    pub materialized_output_values: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct EncodedValue {
    value_type: ValueType,
    bytes: Vec<u8>,
}

impl EncodedValue {
    fn new(value_type: ValueType, bytes: Vec<u8>) -> Self {
        Self { value_type, bytes }
    }
}

#[derive(Clone, Debug)]
struct EncodedBinding {
    values: Vec<Option<EncodedValue>>,
}

impl EncodedBinding {
    fn new(variable_count: usize) -> Self {
        Self {
            values: vec![None; variable_count],
        }
    }

    fn get(&self, variable: usize) -> Option<&EncodedValue> {
        self.values[variable].as_ref()
    }

    fn bind(&mut self, variable: usize, value: EncodedValue) -> bool {
        match &self.values[variable] {
            Some(existing) => existing.bytes == value.bytes,
            None => {
                self.values[variable] = Some(value);
                true
            }
        }
    }

    fn unbind(&mut self, variable: usize) {
        self.values[variable] = None;
    }
}

#[derive(Clone, Debug)]
struct ExecutionPlan<'query> {
    variable_order_ids: Vec<usize>,
    relation_atoms: Vec<&'query TypedRelationAtom>,
    comparisons: Vec<&'query TypedComparison>,
    summary: QueryPlan,
}

#[derive(Clone, Debug)]
struct ScanAccess {
    summary: PlannedAtom,
    prefix: Vec<u8>,
    components: Vec<IndexComponent>,
}

type ScanAccessCandidate = (
    usize,
    bool,
    usize,
    AccessPathDescriptor,
    Vec<u8>,
    Vec<String>,
);

#[derive(Clone, Debug)]
struct PlannerStats {
    relation_rows: BTreeMap<String, u64>,
    index_entries: BTreeMap<(String, String), u64>,
}

impl PlannerStats {
    fn collect(
        txn: &ReadTxn<'_>,
        schema: &StorageSchema,
        atoms: &[&TypedRelationAtom],
    ) -> Result<Self> {
        let mut relation_rows = BTreeMap::new();
        let index_entries = BTreeMap::new();
        for atom in atoms {
            if relation_rows.contains_key(&atom.relation) {
                continue;
            }
            relation_rows.insert(
                atom.relation.clone(),
                txn.relation_row_count(schema, &atom.relation)?,
            );
        }
        Ok(Self {
            relation_rows,
            index_entries,
        })
    }

    fn relation_rows(&self, relation: &str) -> u64 {
        self.relation_rows
            .get(relation)
            .copied()
            .unwrap_or(1)
            .max(1)
    }

    fn index_entries(&self, relation: &str, index: &str) -> u64 {
        self.index_entries
            .get(&(relation.to_owned(), index.to_owned()))
            .copied()
            .unwrap_or_else(|| self.relation_rows(relation))
            .max(1)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct VariableCost {
    variable: usize,
    estimated_candidates: u64,
    static_constraints: usize,
    bound_constraints: usize,
    relation_constraints: usize,
    degree: usize,
}

impl<'env> ReadTxn<'env> {
    /// Executes a typed positive Datalog query against current indexes.
    #[tracing::instrument(name = "bumbledb.query.execute", skip_all, fields(vars = query.variables.len(), clauses = query.clauses.len(), inputs = query.inputs.len()))]
    pub fn execute_query(
        &self,
        schema: &StorageSchema,
        query: &TypedQuery,
        inputs: &InputBindings,
    ) -> Result<QueryOutput> {
        validate_inputs(query, inputs)?;

        let mut plan = plan_query(self, schema, query, inputs)?;
        tracing::debug!(variable_order = ?plan.summary.variable_order, atoms = plan.summary.atoms.len(), "wcoj query planned");
        let mut bindings = Vec::new();
        let mut current = EncodedBinding::new(query.variables.len());

        if self.constant_atoms_exist(
            schema,
            query,
            inputs,
            &plan.relation_atoms,
            &mut plan.summary.counters,
        )? {
            self.execute_variables(
                schema,
                query,
                inputs,
                &mut plan,
                0,
                &mut current,
                &mut bindings,
            )?;
        }

        let columns = result_columns(query);
        let rows = project_results(self, query, &bindings, &mut plan.summary.counters)?;
        plan.summary.counters.output_rows = rows.len() as u64;
        if query
            .find
            .iter()
            .any(|term| matches!(term, TypedFindTerm::Aggregate { .. }))
        {
            plan.summary.counters.aggregate_groups = rows.len() as u64;
        }
        tracing::debug!(?plan.summary.counters, "wcoj query executed");
        Ok(QueryOutput {
            columns,
            rows,
            plan: plan.summary,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn execute_variables(
        &self,
        schema: &StorageSchema,
        query: &TypedQuery,
        inputs: &InputBindings,
        plan: &mut ExecutionPlan<'_>,
        depth: usize,
        binding: &mut EncodedBinding,
        output: &mut Vec<EncodedBinding>,
    ) -> Result<()> {
        if depth == plan.variable_order_ids.len() {
            if comparisons_ready_pass(
                self,
                &plan.comparisons,
                query,
                inputs,
                binding,
                &mut plan.summary.counters,
            )? {
                plan.summary.counters.bindings_yielded += 1;
                output.push(binding.clone());
            }
            return Ok(());
        }

        let variable = plan.variable_order_ids[depth];
        let candidates = self.candidate_values_for_variable(
            schema,
            query,
            inputs,
            &plan.relation_atoms,
            variable,
            binding,
            &mut plan.summary.counters,
        )?;
        plan.summary.counters.variable_candidates += candidates.len() as u64;

        for candidate in candidates {
            if !binding.bind(variable, candidate) {
                continue;
            }
            let keep = comparisons_ready_pass(
                self,
                &plan.comparisons,
                query,
                inputs,
                binding,
                &mut plan.summary.counters,
            )?;
            if keep {
                self.execute_variables(schema, query, inputs, plan, depth + 1, binding, output)?;
            }
            binding.unbind(variable);
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn candidate_values_for_variable(
        &self,
        schema: &StorageSchema,
        query: &TypedQuery,
        inputs: &InputBindings,
        atoms: &[&TypedRelationAtom],
        variable: usize,
        binding: &EncodedBinding,
        counters: &mut PlanCounters,
    ) -> Result<BTreeSet<EncodedValue>> {
        let atom_infos = atoms
            .iter()
            .copied()
            .filter(|atom| atom_contains_variable(atom, variable))
            .map(|atom| {
                (
                    atom,
                    atom_constraint_strength(atom, variable, binding),
                    atom_has_unbound_other_variable(atom, variable, binding),
                )
            })
            .collect::<Vec<_>>();
        let has_constrained_stream = atom_infos.iter().any(|(_, strength, _)| *strength > 0);

        let mut sets = Vec::new();
        for (atom, strength, has_unbound_other) in atom_infos {
            if has_constrained_stream && strength == 0 && has_unbound_other {
                continue;
            }
            sets.push(self.collect_atom_candidates(
                schema, query, inputs, atom, variable, binding, counters,
            )?);
        }
        let mut sets = sets.into_iter();
        let Some(mut intersection) = sets.next() else {
            return Err(Error::internal(format!(
                "variable {} is not constrained by any relation atom",
                query.variables[variable].name
            )));
        };
        for set in sets {
            counters.trie_intersections += 1;
            intersection = intersection.intersection(&set).cloned().collect();
            if intersection.is_empty() {
                break;
            }
        }
        Ok(intersection)
    }

    #[allow(clippy::too_many_arguments)]
    fn collect_atom_candidates(
        &self,
        schema: &StorageSchema,
        query: &TypedQuery,
        inputs: &InputBindings,
        atom: &TypedRelationAtom,
        variable: usize,
        binding: &EncodedBinding,
        counters: &mut PlanCounters,
    ) -> Result<BTreeSet<EncodedValue>> {
        let access =
            choose_scan_access(self, schema, atom, query, inputs, binding, Some(variable))?;
        counters.cursor_seeks += 1;
        let scan = self.scan_encoded_index_prefix(
            schema,
            &atom.relation,
            &access.summary.index,
            &access.prefix,
        )?;
        let mut values = BTreeSet::new();
        for item in scan {
            counters.rows_scanned += 1;
            let item = item?;
            if let Some(value) = atom_candidate_from_item(
                self, schema, query, inputs, atom, variable, binding, &access, &item,
            )? {
                counters.rows_matched += 1;
                values.insert(value);
            }
        }
        Ok(values)
    }

    fn constant_atoms_exist(
        &self,
        schema: &StorageSchema,
        query: &TypedQuery,
        inputs: &InputBindings,
        atoms: &[&TypedRelationAtom],
        counters: &mut PlanCounters,
    ) -> Result<bool> {
        let binding = EncodedBinding::new(query.variables.len());
        for atom in atoms {
            if atom
                .fields
                .iter()
                .any(|field| matches!(field.term, TypedTerm::Variable(_)))
            {
                continue;
            }
            let access = choose_scan_access(self, schema, atom, query, inputs, &binding, None)?;
            counters.cursor_seeks += 1;
            let scan = self.scan_encoded_index_prefix(
                schema,
                &atom.relation,
                &access.summary.index,
                &access.prefix,
            )?;
            let mut found = false;
            for item in scan {
                counters.rows_scanned += 1;
                let item = item?;
                if atom_matches_item(self, schema, query, inputs, atom, &binding, &access, &item)? {
                    counters.rows_matched += 1;
                    found = true;
                    break;
                }
            }
            if !found {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

fn plan_query<'query>(
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    query: &'query TypedQuery,
    inputs: &InputBindings,
) -> Result<ExecutionPlan<'query>> {
    let _span = tracing::debug_span!("bumbledb.query.plan").entered();
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

    let stats = PlannerStats::collect(txn, schema, &relation_atoms)?;
    let (variable_order_ids, variable_costs) =
        choose_variable_order(schema, query, &relation_atoms, &comparisons, &stats)?;
    let variable_order = variable_order_ids
        .iter()
        .map(|id| query.variables[*id].name.clone())
        .collect::<Vec<_>>();
    let variable_estimates = variable_costs
        .iter()
        .map(|cost| VariableEstimate {
            variable: query.variables[cost.variable].name.clone(),
            estimated_candidates: cost.estimated_candidates,
            static_constraints: cost.static_constraints,
            bound_constraints: cost.bound_constraints,
            relation_constraints: cost.relation_constraints,
        })
        .collect::<Vec<_>>();
    let missing_indexes = missing_index_recommendations(schema, &relation_atoms)?;

    let empty = EncodedBinding::new(query.variables.len());
    let mut atoms = Vec::new();
    for atom in &relation_atoms {
        atoms.push(choose_summary_access(schema, atom, query, inputs, &empty)?);
    }

    let uses_indexed_multiway_join = relation_atoms.len() > 1;
    Ok(ExecutionPlan {
        variable_order_ids,
        relation_atoms,
        comparisons,
        summary: QueryPlan {
            variable_order,
            variable_estimates,
            atoms,
            missing_indexes,
            counters: PlanCounters::default(),
            uses_indexed_multiway_join,
        },
    })
}

fn choose_variable_order(
    schema: &StorageSchema,
    query: &TypedQuery,
    atoms: &[&TypedRelationAtom],
    comparisons: &[&TypedComparison],
    stats: &PlannerStats,
) -> Result<(Vec<usize>, Vec<VariableCost>)> {
    let mut remaining = (0..query.variables.len()).collect::<BTreeSet<_>>();
    let mut bound = BTreeSet::new();
    let mut order = Vec::new();
    let mut costs = Vec::new();

    while !remaining.is_empty() {
        let mut candidates = remaining
            .iter()
            .map(|variable| {
                estimate_variable_cost(schema, atoms, comparisons, stats, &bound, *variable)
            })
            .collect::<Result<Vec<_>>>()?;
        candidates.sort_by_key(|cost| {
            (
                cost.estimated_candidates,
                std::cmp::Reverse(cost.static_constraints),
                std::cmp::Reverse(cost.bound_constraints),
                std::cmp::Reverse(cost.relation_constraints),
                std::cmp::Reverse(cost.degree),
                query.variables[cost.variable].name.clone(),
            )
        });
        let best = candidates
            .into_iter()
            .next()
            .ok_or_else(|| Error::internal("query has no remaining variables"))?;
        remaining.remove(&best.variable);
        bound.insert(best.variable);
        order.push(best.variable);
        costs.push(best);
    }

    Ok((order, costs))
}

#[allow(clippy::too_many_arguments)]
fn estimate_variable_cost(
    schema: &StorageSchema,
    atoms: &[&TypedRelationAtom],
    comparisons: &[&TypedComparison],
    stats: &PlannerStats,
    bound: &BTreeSet<usize>,
    variable: usize,
) -> Result<VariableCost> {
    let atom_infos = atoms
        .iter()
        .copied()
        .filter(|atom| atom_contains_variable(atom, variable))
        .map(|atom| {
            let relation_constraints = atom_bound_constraint_count(atom, variable, bound);
            let static_constraints = atom_static_constraint_count(atom, variable)
                + comparison_static_constraint_count(comparisons, variable, bound);
            let has_unbound_other = atom_has_unbound_other_variable_id(atom, variable, bound);
            (
                atom,
                relation_constraints + static_constraints,
                has_unbound_other,
            )
        })
        .collect::<Vec<_>>();
    let has_constrained_stream = atom_infos.iter().any(|(_, strength, _)| *strength > 0);
    let mut estimates = Vec::new();
    let mut relation_constraints = 0usize;
    let mut static_constraints = comparison_static_constraint_count(comparisons, variable, bound);
    let mut bound_constraints = comparison_bound_constraint_count(comparisons, variable, bound);

    for (atom, strength, has_unbound_other) in atom_infos {
        relation_constraints += 1;
        static_constraints += atom_static_constraint_count(atom, variable);
        bound_constraints += atom_bound_constraint_count(atom, variable, bound);
        if has_constrained_stream && strength == 0 && has_unbound_other {
            continue;
        }
        estimates.push(estimate_atom_variable_access(
            schema, stats, bound, atom, variable,
        )?);
    }

    let degree = atoms
        .iter()
        .filter(|atom| atom_contains_variable(atom, variable))
        .count();
    let estimated_candidates = estimates.into_iter().min().unwrap_or(u64::MAX / 4).max(1);

    Ok(VariableCost {
        variable,
        estimated_candidates,
        static_constraints,
        bound_constraints,
        relation_constraints,
        degree,
    })
}

fn estimate_atom_variable_access(
    schema: &StorageSchema,
    stats: &PlannerStats,
    bound: &BTreeSet<usize>,
    atom: &TypedRelationAtom,
    variable: usize,
) -> Result<u64> {
    let paths = schema.access_paths(&atom.relation)?;
    let relation_rows = stats.relation_rows(&atom.relation);
    let mut best = relation_rows.saturating_mul(4).max(1);

    for path in paths {
        if !path.components.iter().any(|component| {
            atom.fields.iter().any(|field| {
                field.field == component.field_name
                    && matches!(field.term, TypedTerm::Variable(id) if id == variable)
            })
        }) {
            continue;
        }

        let mut prefix_len = 0usize;
        let mut current_is_next = false;
        for field_name in &path.leading_fields {
            let Some(field) = atom.fields.iter().find(|field| &field.field == field_name) else {
                break;
            };
            if matches!(field.term, TypedTerm::Variable(id) if id == variable) {
                current_is_next = true;
                break;
            }
            if field_is_bound_for_estimate(field, bound) {
                prefix_len += 1;
            } else {
                break;
            }
        }

        let mut estimate = stats.index_entries(&atom.relation, &path.index_name);
        if prefix_len > 0 {
            estimate = divide_ceil(estimate, 16_u64.saturating_pow(prefix_len as u32));
        }
        if !current_is_next {
            estimate = estimate.saturating_mul(2);
        }
        if path.kind == IndexKind::Unique
            && current_is_next
            && prefix_len + 1 == path.leading_fields.len()
        {
            estimate = estimate.min(1);
        }
        best = best.min(estimate.max(1));
    }

    Ok(best.max(1))
}

fn divide_ceil(value: u64, divisor: u64) -> u64 {
    if divisor == 0 {
        value
    } else {
        value.div_ceil(divisor)
    }
}

fn field_is_bound_for_estimate(
    field: &bumbledb_core::datalog::TypedFieldBinding,
    bound: &BTreeSet<usize>,
) -> bool {
    match field.term {
        TypedTerm::Variable(variable) => bound.contains(&variable),
        TypedTerm::Input(_) | TypedTerm::Literal(_) => true,
        TypedTerm::Wildcard => false,
    }
}

fn atom_static_constraint_count(atom: &TypedRelationAtom, variable: usize) -> usize {
    atom.fields
        .iter()
        .filter(|field| {
            !matches!(field.term, TypedTerm::Variable(id) if id == variable)
                && matches!(field.term, TypedTerm::Input(_) | TypedTerm::Literal(_))
        })
        .count()
}

fn atom_bound_constraint_count(
    atom: &TypedRelationAtom,
    variable: usize,
    bound: &BTreeSet<usize>,
) -> usize {
    atom.fields
        .iter()
        .filter(|field| {
            matches!(field.term, TypedTerm::Variable(id) if id != variable && bound.contains(&id))
        })
        .count()
}

fn atom_has_unbound_other_variable_id(
    atom: &TypedRelationAtom,
    variable: usize,
    bound: &BTreeSet<usize>,
) -> bool {
    atom.fields.iter().any(|field| {
        matches!(field.term, TypedTerm::Variable(id) if id != variable && !bound.contains(&id))
    })
}

fn comparison_static_constraint_count(
    comparisons: &[&TypedComparison],
    variable: usize,
    bound: &BTreeSet<usize>,
) -> usize {
    comparisons
        .iter()
        .filter(|comparison| comparison_constrains_variable(comparison, variable, bound, true))
        .count()
}

fn comparison_bound_constraint_count(
    comparisons: &[&TypedComparison],
    variable: usize,
    bound: &BTreeSet<usize>,
) -> usize {
    comparisons
        .iter()
        .filter(|comparison| comparison_constrains_variable(comparison, variable, bound, false))
        .count()
}

fn comparison_constrains_variable(
    comparison: &TypedComparison,
    variable: usize,
    bound: &BTreeSet<usize>,
    static_only: bool,
) -> bool {
    let left_is_var = matches!(comparison.left, TypedOperand::Variable(id) if id == variable);
    let right_is_var = matches!(comparison.right, TypedOperand::Variable(id) if id == variable);
    if left_is_var {
        operand_constrains_for_estimate(&comparison.right, bound, static_only)
    } else if right_is_var {
        operand_constrains_for_estimate(&comparison.left, bound, static_only)
    } else {
        false
    }
}

fn operand_constrains_for_estimate(
    operand: &TypedOperand,
    bound: &BTreeSet<usize>,
    static_only: bool,
) -> bool {
    match operand {
        TypedOperand::Variable(variable) => !static_only && bound.contains(variable),
        TypedOperand::Input(_) | TypedOperand::Literal(_) => static_only,
    }
}

fn missing_index_recommendations(
    schema: &StorageSchema,
    atoms: &[&TypedRelationAtom],
) -> Result<Vec<MissingIndexRecommendation>> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for atom in atoms {
        let (_, relation) = schema.relation(&atom.relation)?;
        for field in &atom.fields {
            if !matches!(field.term, TypedTerm::Input(_) | TypedTerm::Literal(_)) {
                continue;
            }
            if has_leading_index(schema, &atom.relation, &field.field)? {
                continue;
            }
            let fields = recommended_index_fields(relation, &field.field);
            if seen.insert((atom.relation.clone(), fields.clone())) {
                out.push(MissingIndexRecommendation {
                    relation: atom.relation.clone(),
                    fields,
                    reason: "static predicate has no leading index".to_owned(),
                });
            }
        }
    }
    Ok(out)
}

fn has_leading_index(schema: &StorageSchema, relation: &str, field: &str) -> Result<bool> {
    Ok(schema.access_paths(relation)?.iter().any(|path| {
        path.leading_fields
            .first()
            .is_some_and(|leading| leading == field)
    }))
}

fn recommended_index_fields(
    relation: &bumbledb_core::schema::RelationDescriptor,
    field: &str,
) -> Vec<String> {
    let mut fields = vec![field.to_owned()];
    for primary in &relation.primary_key.fields {
        if !fields.iter().any(|field| field == primary) {
            fields.push(primary.clone());
        }
    }
    fields
}

fn choose_summary_access(
    schema: &StorageSchema,
    atom: &TypedRelationAtom,
    query: &TypedQuery,
    inputs: &InputBindings,
    binding: &EncodedBinding,
) -> Result<PlannedAtom> {
    let access = choose_scan_access_summary(schema, atom, query, inputs, binding)?;
    Ok(access.summary)
}

fn choose_scan_access_summary(
    schema: &StorageSchema,
    atom: &TypedRelationAtom,
    query: &TypedQuery,
    inputs: &InputBindings,
    binding: &EncodedBinding,
) -> Result<ScanAccess> {
    let paths = schema.access_paths(&atom.relation)?;
    let mut best: Option<(usize, usize, AccessPathDescriptor)> = None;
    for path in paths {
        let prefix_len = path
            .leading_fields
            .iter()
            .take_while(|field_name| {
                atom.fields
                    .iter()
                    .find(|field| &field.field == *field_name)
                    .is_some_and(|field| {
                        matches!(field.term, TypedTerm::Input(_) | TypedTerm::Literal(_))
                            || matches!(field.term, TypedTerm::Variable(variable) if binding.get(variable).is_some())
                    })
            })
            .count();
        let mut score = prefix_len;
        if score == 0 && range_field_for_atom(&path, atom, query, inputs, binding).is_some() {
            score = 1;
        }
        let candidate = (score, kind_rank(path.kind), path);
        if best
            .as_ref()
            .is_none_or(|best| (candidate.0, candidate.1) > (best.0, best.1))
        {
            best = Some(candidate);
        }
    }
    let (_, _, path) = best.ok_or_else(|| Error::internal("relation has no access paths"))?;
    Ok(ScanAccess {
        summary: PlannedAtom {
            relation: atom.relation.clone(),
            index: path.index_name,
            kind: path.kind,
            prefix_fields: path.leading_fields,
        },
        prefix: Vec::new(),
        components: path.components,
    })
}

fn choose_scan_access(
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    atom: &TypedRelationAtom,
    query: &TypedQuery,
    inputs: &InputBindings,
    binding: &EncodedBinding,
    current_variable: Option<usize>,
) -> Result<ScanAccess> {
    let paths = schema.access_paths(&atom.relation)?;
    let mut best: Option<ScanAccessCandidate> = None;
    for path in paths {
        if let Some(variable) = current_variable
            && !path.components.iter().any(|component| {
                atom.fields.iter().any(|field| {
                    field.field == component.field_name
                        && matches!(field.term, TypedTerm::Variable(id) if id == variable)
                })
            })
        {
            continue;
        }

        let mut prefix = Vec::new();
        let mut prefix_fields = Vec::new();
        let mut current_is_next = false;
        for field_name in &path.leading_fields {
            let Some(field) = atom.fields.iter().find(|field| &field.field == field_name) else {
                break;
            };
            if matches!(field.term, TypedTerm::Variable(variable) if Some(variable) == current_variable)
            {
                current_is_next = true;
                break;
            }
            let Some(bytes) = term_bound_bytes(txn, schema, atom, field, query, inputs, binding)?
            else {
                break;
            };
            prefix.extend_from_slice(&bytes);
            prefix_fields.push(field_name.clone());
        }

        let candidate = (
            prefix_fields.len(),
            current_is_next,
            kind_rank(path.kind),
            path,
            prefix,
            prefix_fields,
        );
        if best
            .as_ref()
            .is_none_or(|best| (candidate.0, candidate.1, candidate.2) > (best.0, best.1, best.2))
        {
            best = Some(candidate);
        }
    }
    let (_, _, _, path, prefix, prefix_fields) =
        best.ok_or_else(|| Error::internal("relation has no usable trie access path"))?;
    Ok(ScanAccess {
        summary: PlannedAtom {
            relation: atom.relation.clone(),
            index: path.index_name,
            kind: path.kind,
            prefix_fields,
        },
        prefix,
        components: path.components,
    })
}

fn range_field_for_atom(
    path: &AccessPathDescriptor,
    atom: &TypedRelationAtom,
    query: &TypedQuery,
    _inputs: &InputBindings,
    binding: &EncodedBinding,
) -> Option<String> {
    if path.kind != IndexKind::Range || path.leading_fields.len() != 1 {
        return None;
    }
    let field_name = &path.leading_fields[0];
    let field = atom
        .fields
        .iter()
        .find(|field| &field.field == field_name)?;
    let TypedTerm::Variable(variable) = field.term else {
        return None;
    };
    query.clauses.iter().find_map(|clause| {
        let TypedClause::Comparison(comparison) = clause else {
            return None;
        };
        if comparison_mentions_bound(comparison, variable, binding) {
            Some(field_name.clone())
        } else {
            None
        }
    })
}

fn comparison_mentions_bound(
    comparison: &TypedComparison,
    variable: usize,
    binding: &EncodedBinding,
) -> bool {
    let left_is_var = matches!(comparison.left, TypedOperand::Variable(id) if id == variable);
    let right_is_var = matches!(comparison.right, TypedOperand::Variable(id) if id == variable);
    if left_is_var {
        operand_is_bound(&comparison.right, binding)
    } else if right_is_var {
        operand_is_bound(&comparison.left, binding)
    } else {
        false
    }
}

fn operand_is_bound(operand: &TypedOperand, binding: &EncodedBinding) -> bool {
    match operand {
        TypedOperand::Variable(variable) => binding.get(*variable).is_some(),
        TypedOperand::Input(_) => true,
        TypedOperand::Literal(_) => true,
    }
}

fn kind_rank(kind: IndexKind) -> usize {
    match kind {
        IndexKind::Unique => 4,
        IndexKind::Primary => 3,
        IndexKind::Ref => 2,
        IndexKind::Range => 1,
    }
}

#[allow(clippy::too_many_arguments)]
fn atom_candidate_from_item(
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    query: &TypedQuery,
    inputs: &InputBindings,
    atom: &TypedRelationAtom,
    variable: usize,
    binding: &EncodedBinding,
    access: &ScanAccess,
    item: &EncodedIndexItem,
) -> Result<Option<EncodedValue>> {
    let mut candidate: Option<EncodedValue> = None;
    for field in &atom.fields {
        let bytes = component_bytes(access, item, &field.field)?;
        match &field.term {
            TypedTerm::Variable(field_variable) if *field_variable == variable => {
                let value = EncodedValue::new(
                    query.variables[*field_variable].value_type.clone(),
                    bytes.to_vec(),
                );
                match &candidate {
                    Some(existing) if existing.bytes != value.bytes => return Ok(None),
                    Some(_) => {}
                    None => candidate = Some(value),
                }
            }
            TypedTerm::Variable(field_variable) => {
                if let Some(bound) = binding.get(*field_variable)
                    && bound.bytes != bytes
                {
                    return Ok(None);
                }
            }
            TypedTerm::Input(_) | TypedTerm::Literal(_) => {
                let Some(expected) =
                    term_bound_bytes(txn, schema, atom, field, query, inputs, binding)?
                else {
                    return Ok(None);
                };
                if expected.as_slice() != bytes {
                    return Ok(None);
                }
            }
            TypedTerm::Wildcard => {}
        }
    }
    Ok(candidate)
}

#[allow(clippy::too_many_arguments)]
fn atom_matches_item(
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    query: &TypedQuery,
    inputs: &InputBindings,
    atom: &TypedRelationAtom,
    binding: &EncodedBinding,
    access: &ScanAccess,
    item: &EncodedIndexItem,
) -> Result<bool> {
    for field in &atom.fields {
        let bytes = component_bytes(access, item, &field.field)?;
        match &field.term {
            TypedTerm::Variable(variable) => {
                if let Some(bound) = binding.get(*variable)
                    && bound.bytes != bytes
                {
                    return Ok(false);
                }
            }
            TypedTerm::Input(_) | TypedTerm::Literal(_) => {
                let Some(expected) =
                    term_bound_bytes(txn, schema, atom, field, query, inputs, binding)?
                else {
                    return Ok(false);
                };
                if expected.as_slice() != bytes {
                    return Ok(false);
                }
            }
            TypedTerm::Wildcard => {}
        }
    }
    Ok(true)
}

fn component_bytes<'a>(
    access: &ScanAccess,
    item: &'a EncodedIndexItem,
    field_name: &str,
) -> Result<&'a [u8]> {
    let index = access
        .components
        .iter()
        .position(|component| component.field_name == field_name)
        .ok_or_else(|| Error::internal(format!("missing component for field {field_name}")))?;
    item.component(&access.components, index)
        .ok_or_else(|| Error::corrupt("encoded index item missing component"))
}

fn term_bound_bytes(
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    atom: &TypedRelationAtom,
    field: &bumbledb_core::datalog::TypedFieldBinding,
    query: &TypedQuery,
    inputs: &InputBindings,
    binding: &EncodedBinding,
) -> Result<Option<Vec<u8>>> {
    match &field.term {
        TypedTerm::Variable(variable) => {
            Ok(binding.get(*variable).map(|value| value.bytes.clone()))
        }
        TypedTerm::Input(input) => {
            let value = input_value(query, inputs, *input)?;
            let normalized = normalize_value_for_type(value, &field.value_type);
            Ok(Some(txn.encode_query_field_value(
                schema,
                &atom.relation,
                &field.field,
                &normalized,
            )?))
        }
        TypedTerm::Literal(literal) => {
            let value = literal_to_value(literal)?;
            let normalized = normalize_value_for_type(&value, &field.value_type);
            Ok(Some(txn.encode_query_field_value(
                schema,
                &atom.relation,
                &field.field,
                &normalized,
            )?))
        }
        TypedTerm::Wildcard => Ok(None),
    }
}

fn atom_contains_variable(atom: &TypedRelationAtom, variable: usize) -> bool {
    atom.fields
        .iter()
        .any(|field| matches!(field.term, TypedTerm::Variable(id) if id == variable))
}

fn atom_constraint_strength(
    atom: &TypedRelationAtom,
    variable: usize,
    binding: &EncodedBinding,
) -> usize {
    atom.fields
        .iter()
        .filter(|field| match field.term {
            TypedTerm::Variable(id) if id == variable => false,
            TypedTerm::Variable(id) => binding.get(id).is_some(),
            TypedTerm::Input(_) | TypedTerm::Literal(_) => true,
            TypedTerm::Wildcard => false,
        })
        .count()
}

fn atom_has_unbound_other_variable(
    atom: &TypedRelationAtom,
    variable: usize,
    binding: &EncodedBinding,
) -> bool {
    atom.fields.iter().any(|field| {
        matches!(field.term, TypedTerm::Variable(id) if id != variable && binding.get(id).is_none())
    })
}

fn comparisons_ready_pass(
    txn: &ReadTxn<'_>,
    comparisons: &[&TypedComparison],
    query: &TypedQuery,
    inputs: &InputBindings,
    binding: &EncodedBinding,
    counters: &mut PlanCounters,
) -> Result<bool> {
    for comparison in comparisons {
        let Some(left_encoded) =
            operand_encoded_value(txn, &comparison.left, comparison, query, inputs, binding)?
        else {
            continue;
        };
        let Some(right_encoded) =
            operand_encoded_value(txn, &comparison.right, comparison, query, inputs, binding)?
        else {
            continue;
        };
        if encoded_comparison_supported(comparison.operator, &comparison.value_type) {
            counters.comparisons_evaluated += 1;
            counters.encoded_comparisons_evaluated += 1;
            if !compare_encoded_values(
                &left_encoded.bytes,
                comparison.operator,
                &right_encoded.bytes,
            ) {
                counters.comparisons_failed += 1;
                return Ok(false);
            }
            continue;
        }

        let Some(left) =
            operand_logical_value(txn, &comparison.left, query, inputs, binding, counters)?
        else {
            continue;
        };
        let Some(right) =
            operand_logical_value(txn, &comparison.right, query, inputs, binding, counters)?
        else {
            continue;
        };
        counters.comparisons_evaluated += 1;
        counters.decoded_comparisons_evaluated += 1;
        let left = normalize_value_for_type(&left, &comparison.value_type);
        let right = normalize_value_for_type(&right, &comparison.value_type);
        if !compare_values(&left, comparison.operator, &right) {
            counters.comparisons_failed += 1;
            return Ok(false);
        }
    }
    Ok(true)
}

fn operand_encoded_value(
    txn: &ReadTxn<'_>,
    operand: &TypedOperand,
    comparison: &TypedComparison,
    query: &TypedQuery,
    inputs: &InputBindings,
    binding: &EncodedBinding,
) -> Result<Option<EncodedValue>> {
    Ok(match operand {
        TypedOperand::Variable(variable) => binding.get(*variable).map(|value| EncodedValue {
            value_type: comparison.value_type.clone(),
            bytes: value.bytes.clone(),
        }),
        TypedOperand::Input(input) => {
            let value = input_value(query, inputs, *input)?;
            let normalized = normalize_value_for_type(value, &comparison.value_type);
            Some(EncodedValue::new(
                comparison.value_type.clone(),
                txn.encode_query_value(&comparison.value_type, &normalized)?,
            ))
        }
        TypedOperand::Literal(literal) => {
            let value = literal_to_value(literal)?;
            let normalized = normalize_value_for_type(&value, &comparison.value_type);
            Some(EncodedValue::new(
                comparison.value_type.clone(),
                txn.encode_query_value(&comparison.value_type, &normalized)?,
            ))
        }
    })
}

fn encoded_comparison_supported(operator: ComparisonOperator, value_type: &ValueType) -> bool {
    match operator {
        ComparisonOperator::Eq | ComparisonOperator::NotEq => true,
        ComparisonOperator::Lt
        | ComparisonOperator::Lte
        | ComparisonOperator::Gt
        | ComparisonOperator::Gte => !matches!(value_type, ValueType::String | ValueType::Bytes),
    }
}

fn compare_encoded_values(left: &[u8], operator: ComparisonOperator, right: &[u8]) -> bool {
    match operator {
        ComparisonOperator::Eq => left == right,
        ComparisonOperator::NotEq => left != right,
        ComparisonOperator::Lt => left < right,
        ComparisonOperator::Lte => left <= right,
        ComparisonOperator::Gt => left > right,
        ComparisonOperator::Gte => left >= right,
    }
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

fn operand_logical_value(
    txn: &ReadTxn<'_>,
    operand: &TypedOperand,
    query: &TypedQuery,
    inputs: &InputBindings,
    binding: &EncodedBinding,
    counters: &mut PlanCounters,
) -> Result<Option<Value>> {
    Ok(match operand {
        TypedOperand::Variable(variable) => binding
            .get(*variable)
            .map(|value| {
                record_decode(&query.variables[*variable].value_type, counters);
                txn.decode_query_value(&query.variables[*variable].value_type, &value.bytes)
            })
            .transpose()?,
        TypedOperand::Input(input) => Some(input_value(query, inputs, *input)?.clone()),
        TypedOperand::Literal(literal) => Some(literal_to_value(literal)?),
    })
}

fn record_decode(value_type: &ValueType, counters: &mut PlanCounters) {
    counters.decoded_values += 1;
    if matches!(value_type, ValueType::String | ValueType::Bytes) {
        counters.dictionary_reverse_lookups += 1;
    }
}

fn input_value<'a>(
    query: &'a TypedQuery,
    inputs: &'a InputBindings,
    input: usize,
) -> Result<&'a Value> {
    let input = &query.inputs[input];
    let value = inputs
        .get(&input.name)
        .ok_or_else(|| Error::missing_input(&input.name))?;
    if !value_matches_type(value, &input.value_type) {
        return Err(Error::query_input_type_mismatch(
            &input.name,
            value_type_name(&input.value_type),
            value.kind_name(),
        ));
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
            return Err(Error::internal(
                "typed literal does not match literal value",
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

fn project_results(
    txn: &ReadTxn<'_>,
    query: &TypedQuery,
    bindings: &[EncodedBinding],
    counters: &mut PlanCounters,
) -> Result<Vec<Vec<Value>>> {
    let _span = tracing::debug_span!("bumbledb.query.project", bindings = bindings.len()).entered();
    let has_aggregate = query
        .find
        .iter()
        .any(|term| matches!(term, TypedFindTerm::Aggregate { .. }));
    if has_aggregate {
        project_aggregates(txn, query, bindings, counters)
    } else {
        let mut set = BTreeSet::new();
        for binding in bindings {
            let mut row = Vec::new();
            for term in &query.find {
                let TypedFindTerm::Variable { variable } = term else {
                    continue;
                };
                row.push(bound_encoded_variable(binding, *variable)?.clone());
            }
            set.insert(row);
        }
        set.into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|value| decode_output_value(txn, value, counters))
                    .collect::<Result<Vec<_>>>()
            })
            .collect()
    }
}

fn project_aggregates(
    txn: &ReadTxn<'_>,
    query: &TypedQuery,
    bindings: &[EncodedBinding],
    counters: &mut PlanCounters,
) -> Result<Vec<Vec<Value>>> {
    let _span =
        tracing::debug_span!("bumbledb.query.aggregate", bindings = bindings.len()).entered();
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

    let mut groups: BTreeMap<Vec<EncodedValue>, Vec<AggregateState>> = BTreeMap::new();
    for binding in bindings {
        let key = group_terms
            .iter()
            .map(|variable| bound_encoded_variable(binding, *variable).cloned())
            .collect::<Result<Vec<_>>>()?;
        let states = groups.entry(key).or_insert_with(|| {
            aggregate_terms
                .iter()
                .map(|(function, _, value_type)| AggregateState::new(*function, value_type.clone()))
                .collect()
        });
        for (state, (function, variable, _)) in states.iter_mut().zip(&aggregate_terms) {
            if *function == AggregateFunction::Count {
                state.apply_count()?;
            } else {
                let value = decode_bound_variable(txn, query, binding, *variable, counters)?;
                state.apply(&value)?;
            }
        }
    }

    let mut rows = Vec::new();
    for (key, states) in groups {
        let mut row = Vec::new();
        let mut key_iter = key.into_iter();
        let mut state_iter = states.into_iter();
        for term in &query.find {
            match term {
                TypedFindTerm::Variable { .. } => {
                    row.push(decode_output_value(
                        txn,
                        key_iter.next().unwrap(),
                        counters,
                    )?);
                }
                TypedFindTerm::Aggregate { .. } => {
                    counters.materialized_output_values += 1;
                    row.push(state_iter.next().unwrap().finish()?);
                }
            }
        }
        rows.push(row);
    }
    rows.sort();
    Ok(rows)
}

fn bound_encoded_variable(binding: &EncodedBinding, variable: usize) -> Result<&EncodedValue> {
    binding
        .get(variable)
        .ok_or_else(|| Error::internal(format!("variable {variable} is unbound at projection")))
}

fn decode_bound_variable(
    txn: &ReadTxn<'_>,
    query: &TypedQuery,
    binding: &EncodedBinding,
    variable: usize,
    counters: &mut PlanCounters,
) -> Result<Value> {
    let value = bound_encoded_variable(binding, variable)?;
    record_decode(&query.variables[variable].value_type, counters);
    txn.decode_query_value(&query.variables[variable].value_type, &value.bytes)
}

fn decode_output_value(
    txn: &ReadTxn<'_>,
    value: EncodedValue,
    counters: &mut PlanCounters,
) -> Result<Value> {
    counters.materialized_output_values += 1;
    record_decode(&value.value_type, counters);
    txn.decode_query_value(&value.value_type, &value.bytes)
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

    fn apply_count(&mut self) -> Result<()> {
        let AggregateState::Count(count) = self else {
            return Err(Error::internal("count aggregate state mismatch"));
        };
        *count = count
            .checked_add(1)
            .ok_or_else(|| Error::integer_overflow("count"))?;
        Ok(())
    }

    fn apply(&mut self, value: &Value) -> Result<()> {
        match self {
            AggregateState::Count(_) => self.apply_count()?,
            AggregateState::SumU64(sum) => {
                let Value::U64(value) = value else {
                    return Err(Error::aggregate_type_mismatch("sum", value.kind_name()));
                };
                *sum = sum
                    .checked_add(*value)
                    .ok_or_else(|| Error::integer_overflow("sum"))?;
            }
            AggregateState::SumI64(sum) => {
                let Value::I64(value) = value else {
                    return Err(Error::aggregate_type_mismatch("sum", value.kind_name()));
                };
                *sum = sum
                    .checked_add(*value)
                    .ok_or_else(|| Error::integer_overflow("sum"))?;
            }
            AggregateState::SumDecimal(sum) => {
                let Value::Decimal(DecimalRaw(value)) = value else {
                    return Err(Error::aggregate_type_mismatch("sum", value.kind_name()));
                };
                *sum = sum
                    .checked_add(*value)
                    .ok_or_else(|| Error::decimal_overflow("sum"))?;
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
    use crate::{AggregateError, Environment, ExecuteError, QueryError, Row};
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
    fn planner_recommends_missing_static_predicate_index() {
        let (env, schema) = seeded_db();
        let query = parse_and_typecheck(
            schema.descriptor(),
            "find ?account where Account(id: ?account, currency: $currency)",
        )
        .unwrap();

        let output = env
            .read(|txn| {
                txn.execute_query(
                    &schema,
                    &query,
                    &InputBindings::from_values([("currency", Value::Symbol(840))]),
                )
            })
            .unwrap();

        assert_same_rows(&output.rows, &[vec![Value::Id(1)], vec![Value::Id(3)]]);
        let expected_fields = vec!["currency".to_owned(), "id".to_owned()];
        assert!(output.plan.missing_indexes.iter().any(|missing| {
            missing.relation == "Account" && missing.fields == expected_fields
        }));
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
        assert!(matches!(
            int_error,
            Error::Query(QueryError::Aggregate(
                AggregateError::IntegerOverflow { .. }
            ))
        ));

        let decimal_query =
            parse_and_typecheck(schema.descriptor(), "find sum(?d) where Number(d: ?d)").unwrap();
        let decimal_error = env
            .read(|txn| txn.execute_query(&schema, &decimal_query, &InputBindings::new()))
            .unwrap_err();
        assert!(matches!(
            decimal_error,
            Error::Query(QueryError::Aggregate(
                AggregateError::DecimalOverflow { .. }
            ))
        ));
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
        assert!(matches!(
            error,
            Error::Query(QueryError::Execute(ExecuteError::InputTypeMismatch { .. }))
        ));
    }

    #[test]
    fn explain_and_storage_diagnostics_are_available() {
        let (env, schema) = seeded_db();
        let query = parse_and_typecheck(
            schema.descriptor(),
            r#"
            find ?posting ?amount
            where
              Posting(id: ?posting, account: ?account, amount: ?amount, at: ?t)
              Account(id: ?account, holder: $holder)
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
                        ("holder", Value::Ref(1)),
                        ("start", Value::Timestamp(TimestampMicros(0))),
                        ("end", Value::Timestamp(TimestampMicros(100))),
                    ]),
                )
            })
            .unwrap();
        let explain = output.explain();
        assert!(explain.contains("variable_order"));
        assert!(explain.contains("variable_estimate"));
        assert!(explain.contains("index="));
        assert!(explain.contains("cursor_seeks"));
        assert!(explain.contains("rows_scanned"));
        assert!(explain.contains("bindings_yielded"));
        assert!(explain.contains("decoded_values"));
        assert!(explain.contains("encoded_comparisons_evaluated"));
        assert!(explain.contains("materialized_output_values"));
        assert!(explain.contains("output_rows"));

        let diagnostics = env.storage_diagnostics(&schema).unwrap();
        assert_eq!(diagnostics.storage_tx_id, 1);
        assert!(diagnostics.lmdb_map_size > 0);
        assert!(diagnostics.dictionary_entries > 0);
        assert!(
            diagnostics
                .relations
                .iter()
                .any(|relation| relation.relation == "Account" && relation.row_count == 3)
        );
        assert_eq!(
            diagnostics.schema_fingerprint,
            schema.descriptor().fingerprint().to_string()
        );
    }

    #[test]
    fn differential_reference_evaluator_matches_lmdb() {
        let (env, schema) = seeded_db();
        let reference = ReferenceDb::from_rows(seeded_rows());
        let cases = [
            (
                "find ?account where Account(id: ?account, holder: $holder)",
                InputBindings::from_values([("holder", Value::Ref(1))]),
            ),
            (
                r#"
                find ?account ?holder_name
                where
                  Account(id: ?account, holder: ?holder)
                  Holder(id: ?holder, name: ?holder_name)
                "#,
                InputBindings::new(),
            ),
            (
                r#"
                find ?account sum(?amount) count(?posting)
                where
                  Posting(id: ?posting, account: ?account, amount: ?amount, at: ?t)
                  ?t >= $start
                  ?t < $end
                "#,
                InputBindings::from_values([
                    ("start", Value::Timestamp(TimestampMicros(0))),
                    ("end", Value::Timestamp(TimestampMicros(100))),
                ]),
            ),
        ];

        for (source, inputs) in cases {
            let query = parse_and_typecheck(schema.descriptor(), source).unwrap();
            let lmdb_rows = env
                .read(|txn| txn.execute_query(&schema, &query, &inputs))
                .unwrap()
                .rows;
            let reference_rows = reference.execute(&query, &inputs).unwrap();
            assert_same_rows(&lmdb_rows, &reference_rows);
        }
    }

    fn seeded_db() -> (Environment, StorageSchema) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.keep();
        let env = Environment::open(&path).unwrap();
        let schema = StorageSchema::new(ledger_schema(), env.max_key_size()).unwrap();
        let rows = seeded_rows();
        env.write(|txn| {
            for row in &rows {
                txn.insert(&schema, row.clone())?;
            }
            Ok::<(), Error>(())
        })
        .unwrap();
        (env, schema)
    }

    fn seeded_rows() -> Vec<Row> {
        vec![
            holder_row(1, "Alice"),
            holder_row(2, "Bob"),
            account_row(1, 1, 840),
            account_row(2, 1, 978),
            account_row(3, 2, 840),
            posting_row(1, 1, 100, 10),
            posting_row(2, 1, 200, 20),
            posting_row(3, 2, 300, 30),
        ]
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

    struct ReferenceDb {
        rows: BTreeMap<String, Vec<Row>>,
    }

    #[derive(Clone, Debug)]
    struct ReferenceBinding {
        values: Vec<Option<Value>>,
    }

    impl ReferenceBinding {
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

    impl ReferenceDb {
        fn from_rows(rows: Vec<Row>) -> Self {
            let mut by_relation: BTreeMap<String, Vec<Row>> = BTreeMap::new();
            for row in rows {
                by_relation
                    .entry(row.relation().to_owned())
                    .or_default()
                    .push(row);
            }
            Self { rows: by_relation }
        }

        fn execute(&self, query: &TypedQuery, inputs: &InputBindings) -> Result<Vec<Vec<Value>>> {
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
            let mut output = Vec::new();
            let mut counters = PlanCounters::default();
            self.recurse(
                query,
                inputs,
                &atoms,
                &comparisons,
                0,
                ReferenceBinding::new(query.variables.len()),
                &mut output,
                &mut counters,
            )?;
            reference_project_results(query, &output)
        }

        #[allow(clippy::too_many_arguments)]
        fn recurse(
            &self,
            query: &TypedQuery,
            inputs: &InputBindings,
            atoms: &[&TypedRelationAtom],
            comparisons: &[&TypedComparison],
            depth: usize,
            binding: ReferenceBinding,
            output: &mut Vec<ReferenceBinding>,
            counters: &mut PlanCounters,
        ) -> Result<()> {
            if depth == atoms.len() {
                if reference_comparisons_pass(comparisons, query, inputs, &binding, counters)? {
                    output.push(binding);
                }
                return Ok(());
            }

            let atom = atoms[depth];
            for row in self.rows.get(&atom.relation).into_iter().flatten() {
                let Some(next) = reference_match_atom(atom, query, inputs, &binding, row)? else {
                    continue;
                };
                if reference_comparisons_pass(comparisons, query, inputs, &next, counters)? {
                    self.recurse(
                        query,
                        inputs,
                        atoms,
                        comparisons,
                        depth + 1,
                        next,
                        output,
                        counters,
                    )?;
                }
            }
            Ok(())
        }
    }

    fn reference_match_atom(
        atom: &TypedRelationAtom,
        query: &TypedQuery,
        inputs: &InputBindings,
        binding: &ReferenceBinding,
        row: &Row,
    ) -> Result<Option<ReferenceBinding>> {
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

    fn reference_comparisons_pass(
        comparisons: &[&TypedComparison],
        query: &TypedQuery,
        inputs: &InputBindings,
        binding: &ReferenceBinding,
        counters: &mut PlanCounters,
    ) -> Result<bool> {
        for comparison in comparisons {
            let Some(left) = reference_operand_value(&comparison.left, query, inputs, binding)?
            else {
                continue;
            };
            let Some(right) = reference_operand_value(&comparison.right, query, inputs, binding)?
            else {
                continue;
            };
            counters.comparisons_evaluated += 1;
            let left = normalize_value_for_type(&left, &comparison.value_type);
            let right = normalize_value_for_type(&right, &comparison.value_type);
            if !compare_values(&left, comparison.operator, &right) {
                counters.comparisons_failed += 1;
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn reference_operand_value(
        operand: &TypedOperand,
        query: &TypedQuery,
        inputs: &InputBindings,
        binding: &ReferenceBinding,
    ) -> Result<Option<Value>> {
        Ok(match operand {
            TypedOperand::Variable(variable) => binding.get(*variable).cloned(),
            TypedOperand::Input(input) => Some(input_value(query, inputs, *input)?.clone()),
            TypedOperand::Literal(literal) => Some(literal_to_value(literal)?),
        })
    }

    fn reference_project_results(
        query: &TypedQuery,
        bindings: &[ReferenceBinding],
    ) -> Result<Vec<Vec<Value>>> {
        let has_aggregate = query
            .find
            .iter()
            .any(|term| matches!(term, TypedFindTerm::Aggregate { .. }));
        if has_aggregate {
            reference_project_aggregates(query, bindings)
        } else {
            let mut set = BTreeSet::new();
            for binding in bindings {
                let mut row = Vec::new();
                for term in &query.find {
                    let TypedFindTerm::Variable { variable } = term else {
                        continue;
                    };
                    row.push(reference_bound_variable(binding, *variable)?.clone());
                }
                set.insert(row);
            }
            Ok(set.into_iter().collect())
        }
    }

    fn reference_project_aggregates(
        query: &TypedQuery,
        bindings: &[ReferenceBinding],
    ) -> Result<Vec<Vec<Value>>> {
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
                .map(|variable| reference_bound_variable(binding, *variable).cloned())
                .collect::<Result<Vec<_>>>()?;
            let states = groups.entry(key).or_insert_with(|| {
                aggregate_terms
                    .iter()
                    .map(|(function, _, value_type)| {
                        AggregateState::new(*function, value_type.clone())
                    })
                    .collect()
            });
            for (state, (_, variable, _)) in states.iter_mut().zip(&aggregate_terms) {
                state.apply(reference_bound_variable(binding, *variable)?)?;
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
                    TypedFindTerm::Aggregate { .. } => {
                        row.push(state_iter.next().unwrap().finish()?)
                    }
                }
            }
            rows.push(row);
        }
        rows.sort();
        Ok(rows)
    }

    fn reference_bound_variable(binding: &ReferenceBinding, variable: usize) -> Result<&Value> {
        binding
            .get(variable)
            .ok_or_else(|| Error::internal(format!("variable {variable} is unbound at projection")))
    }
}
