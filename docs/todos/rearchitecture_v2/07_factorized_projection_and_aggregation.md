# 07: Factorized Projection And Aggregation

**Goal**
- Move projection and aggregation into the Free Join execution pipeline and avoid full binding materialization where algebraically valid.

**Output Plan**
```rust
pub enum OutputPlan {
    Project(ProjectPlan),
    Aggregate(AggregatePlan),
}

pub struct ProjectPlan {
    pub vars: Vec<VarId>,
    pub set_semantics: bool,
}

pub struct AggregatePlan {
    pub group_vars: Vec<VarId>,
    pub aggregates: Vec<AggregateTerm>,
}

pub struct AggregateTerm {
    pub function: AggregateFunction,
    pub var: VarId,
    pub value_type: ValueType,
}
```

**Tuple Sink Interface**
```rust
pub trait TupleSink<'a> {
    fn emit(&mut self, binding: &BindingFrame<'a>) -> Result<()>;
    fn finish(self: Box<Self>) -> Result<Vec<Vec<Value>>>;
}
```

**Encoded Projection Sink**
```rust
pub struct EncodedProjectSink<'a> {
    vars: Vec<VarId>,
    rows: BTreeSet<Vec<EncodedOwned>>,
    decoder: OutputDecoder<'a>,
}

impl<'a> TupleSink<'a> for EncodedProjectSink<'a> {
    fn emit(&mut self, binding: &BindingFrame<'a>) -> Result<()> {
        let row = self.vars.iter()
            .map(|var| binding.owned(*var))
            .collect::<Result<Vec<_>>>()?;
        self.rows.insert(row);
        Ok(())
    }
}
```

**Aggregate Sink**
```rust
pub struct AggregateSink<'a> {
    group_vars: Vec<VarId>,
    terms: Vec<AggregateTerm>,
    groups: hashbrown::HashMap<Vec<EncodedOwned>, Vec<AggregateState>>,
    decoder: OutputDecoder<'a>,
}

pub enum AggregateState {
    Count(u64),
    SumI64(i64),
    SumU64(u64),
    SumDecimal(i128),
    Min(Option<EncodedOwned>),
    Max(Option<EncodedOwned>),
}
```

**Factorized Count**
```rust
pub trait MultiplicitySource {
    fn multiplicity(&self, binding: &BindingFrame<'_>) -> Option<u64>;
}

impl<'a> AggregateSink<'a> {
    pub fn emit_count_range(&mut self, binding: &BindingFrame<'a>, count: u64) -> Result<()> {
        let key = self.group_key(binding)?;
        let states = self.groups.entry(key).or_insert_with(|| self.initial_states());
        for state in states.iter_mut() {
            if let AggregateState::Count(value) = state {
                *value = value.checked_add(count).ok_or_else(|| Error::integer_overflow("count"))?;
            }
        }
        Ok(())
    }
}
```

**Early Projection Rules**
- Carry only variables demanded by later nodes, output, aggregates, or predicates.
- Existence-only relations carry no row id payload.
- Count-only branches may emit multiplicity without enumerating all child rows.
- Group keys remain encoded until final output.
- String/bytes are decoded only in final output or unsupported semantic comparison.

**Loop-Invariant Aggregation**
For a plan node producing a product of independent payload sets:
```rust
sum_{a in A} sum_{b in B} f(a) = |B| * sum_{a in A} f(a)
```

Represent as:
```rust
pub enum AggregateRewrite {
    Direct,
    MultiplyByCardinality { source: NodeId, factor: CardinalityExpr },
    PreAggregateThenJoin { pre_node: NodeId },
}
```

**Tests**
- Projection deduplicates encoded rows before decoding.
- Count avoids decoding counted variable.
- Count over a range can use multiplicity.
- Sum decodes only aggregate operand values.
- Group key dictionary reverse lookup happens only at final output.
- Results match current aggregate semantics.

**Passing Criteria**
- `triangle_count` uses count sink and emits one output row without storing all triangle bindings.
- Aggregate queries report lower `materialized_output_values` than complete binding count where applicable.
- Existing aggregate overflow behavior is preserved.
- `cargo test --workspace` and differential tests pass.

**Non-Goals**
- Do not implement all FAQ/InsideOut rewrites in this PRD.
- Do not change public aggregate semantics.
