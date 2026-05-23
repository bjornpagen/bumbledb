use super::*;

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

    pub(super) fn get(&self, name: &str) -> Option<&Value> {
        self.values.get(name)
    }

    /// Returns a bound input value by name.
    pub fn value(&self, name: &str) -> Option<&Value> {
        self.values.get(name)
    }
}

/// Dense input ID inside a normalized query.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InputId(pub u16);

/// Dense predicate ID inside a normalized query.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PredicateId(pub u16);

/// Executor-friendly normalized typed query IR.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizedQuery {
    /// Dense variables used by this query.
    pub vars: Vec<NormVar>,
    /// Dense inputs used by this query.
    pub inputs: Vec<NormInput>,
    /// Relation atoms in clause order.
    pub atoms: Vec<NormAtom>,
    /// Normalized comparison predicates.
    pub predicates: Vec<NormPredicate>,
    /// Output plan used by sinks.
    pub output: OutputPlan,
    /// Original find-term order after normalization.
    pub find: Vec<NormFindTerm>,
}

/// Normalized variable metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormVar {
    /// Dense variable ID.
    pub id: VarId,
    /// Source variable name without `?`.
    pub name: String,
    /// Logical value type.
    pub value_type: ValueType,
}

/// Normalized input metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormInput {
    /// Dense input ID.
    pub id: InputId,
    /// Source input name without `$`.
    pub name: String,
    /// Logical value type.
    pub value_type: ValueType,
}

/// Normalized relation atom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormAtom {
    /// Dense atom ID in relation-clause order.
    pub id: AtomId,
    /// Dense relation ID in schema declaration order.
    pub relation: crate::RelationId,
    /// Relation name, retained for diagnostics and image lookup.
    pub relation_name: String,
    /// Normalized atom fields.
    pub fields: Vec<NormAtomField>,
}

/// Normalized atom field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormAtomField {
    /// Dense field ID in relation declaration order.
    pub field: FieldId,
    /// Field name, retained for diagnostics and access-path lookup.
    pub field_name: String,
    /// Bound normalized term.
    pub term: NormTerm,
    /// Logical field value type.
    pub value_type: ValueType,
}

/// Normalized atom term.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NormTerm {
    /// Variable reference.
    Var(VarId),
    /// Input reference.
    Input(InputId),
    /// Encoded literal.
    Literal(EncodedOwned),
    /// Wildcard.
    Wildcard,
}

/// Normalized comparison predicate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormPredicate {
    /// Dense predicate ID in comparison-clause order.
    pub id: PredicateId,
    /// Binary operands.
    pub operands: [NormOperand; 2],
    /// Comparison operation.
    pub op: ComparisonOperator,
    /// Logical comparison value type.
    pub value_type: ValueType,
    /// Earliest variable-order depth where this predicate can be evaluated.
    pub earliest_depth: Option<usize>,
}

/// Normalized comparison operand.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NormOperand {
    /// Variable reference.
    Var(VarId),
    /// Input reference.
    Input(InputId),
    /// Encoded literal.
    Literal(EncodedOwned),
}

/// Normalized output term in source find order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NormFindTerm {
    /// Projected variable.
    Variable { variable: VarId },
}

/// One fact in a query result set.
pub type ResultFact = Vec<Value>;

/// Duplicate-free query result set in canonical fact order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryResultSet {
    /// Result columns in projection order.
    pub columns: Vec<ResultColumn>,
    /// Result facts in canonical order.
    pub facts: Vec<ResultFact>,
}

impl QueryResultSet {
    /// Builds a canonical result set from possibly unordered facts.
    pub fn new(columns: Vec<ResultColumn>, mut facts: Vec<ResultFact>) -> Self {
        facts.sort();
        facts.dedup();
        Self { columns, facts }
    }

    /// Number of facts in the set.
    pub fn cardinality(&self) -> usize {
        self.facts.len()
    }
}

/// Query execution output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryOutput {
    /// Duplicate-free result set.
    pub result: QueryResultSet,
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
}

/// Physical query plan summary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryPlan {
    /// Deterministic Free Join variable binding order.
    pub variable_order: Vec<String>,
    /// Query image cache diagnostics after acquiring this query image.
    pub query_image_cache: QueryImageCacheDiagnostics,
    /// Planner statistics cache diagnostics after planning.
    pub planner_stats: PlannerStatsCacheDiagnostics,
    /// Free Join physical plan IR.
    pub free_join: FreeJoinPlan,
    /// Coarse query phase timings.
    pub timings: QueryTimings,
    /// Allocation summary for this query, disabled by default.
    pub allocations: QueryAllocationStats,
    /// Execution counters.
    pub counters: PlanCounters,
}
