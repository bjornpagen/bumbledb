# 08: Optimizer And Statistics

**Goal**
- Build a statistics-backed optimizer that chooses Free Join plans, node implementations, and physical access paths.

**Stats Model**
```rust
pub struct RelationStats {
    pub rows: usize,
    pub fields: Vec<FieldStats>,
    pub indexes: Vec<IndexStats>,
}

pub struct FieldStats {
    pub field: FieldId,
    pub distinct: usize,
    pub min: Option<EncodedOwned>,
    pub max: Option<EncodedOwned>,
    pub heavy_hitters: Vec<(EncodedOwned, usize)>,
}

pub struct IndexStats {
    pub index: AccessId,
    pub rows: usize,
    pub distinct_by_depth: Vec<usize>,
    pub avg_fanout_by_depth: Vec<f64>,
    pub max_fanout_by_depth: Vec<usize>,
    pub prefix_samples: Vec<PrefixSample>,
}
```

**Plan Estimates**
```rust
pub struct PlanEstimates {
    pub output_rows: f64,
    pub iterator_ops: f64,
    pub hash_build_rows: f64,
    pub hash_probe_rows: f64,
    pub materialized_values: f64,
    pub memory_bytes: usize,
}
```

**Optimizer Interface**
```rust
pub struct Optimizer<'a> {
    image: &'a QueryImage,
    config: OptimizerConfig,
}

impl<'a> Optimizer<'a> {
    pub fn plan(&self, query: &NormalizedQuery) -> Result<FreeJoinPlan> {
        let candidates = self.enumerate_candidates(query)?;
        candidates.into_iter()
            .min_by_key(|plan| self.cost_key(plan))
            .ok_or_else(|| Error::internal("no plan candidates"))
    }
}
```

**Candidate Generation**
- Pure LFTJ plan over all variables.
- Left-deep binary/probe plans seeded from selective predicates.
- Free Join hybrid plans based on partitioned subatoms.
- Aggregate-pushdown variants.
- Existence-only variants.

**Cost Key**
```rust
pub struct CostKey {
    pub estimated_micros: u64,
    pub memory_bytes: usize,
    pub materialization_penalty: u64,
    pub tie_breaker: String,
}
```

**Physical Design Feedback**
```rust
pub struct IndexRecommendation {
    pub relation: RelationId,
    pub fields: Vec<FieldId>,
    pub kind: IndexKind,
    pub reason: IndexReason,
    pub estimated_benefit: f64,
}

pub enum IndexReason {
    StaticPredicate,
    JoinPrefix,
    RangePredicate,
    HybridProbe,
    AggregateGroupKey,
}
```

**Stats Build Requirements**
- Exact distinct counts by trie depth.
- Exact fanout stats for prefix levels.
- Heavy hitter detection for high fanout keys.
- Range min/max for order-preserving fields.
- Row count and payload cardinality estimates.

**Explain Output**
- Candidate plans considered.
- Chosen plan with cost estimates.
- Rejected plans with top-level reason.
- Missing/recommended indexes.
- Actual versus estimated rows per node.

**Tests**
- Deterministic plan selection for fixed stats.
- Selects equality index for static predicates.
- Selects hash probe for exact lookup-heavy node.
- Selects LFTJ for triangle/cyclic query.
- Emits missing index when no access path can support chosen prefix.
- Cost tie-breaking is stable.

**Passing Criteria**
- Benchmark explain includes chosen Free Join plan and estimates.
- TPC-H queries no longer regress due to poor variable order.
- Small selective queries choose low-overhead probe-like plans.
- No hidden heuristic order is used without stats explanation.

**Non-Goals**
- Do not attempt exhaustive global optimality.
- Do not implement learned cardinality estimation.
