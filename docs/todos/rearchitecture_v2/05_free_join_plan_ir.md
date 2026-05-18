# 05: Free Join Plan IR

**Goal**
- Introduce a single physical plan IR that can express binary-style joins, Generic Join/LFTJ, and hybrid Free Join plans.

**Plan Type**
```rust
pub struct FreeJoinPlan {
    pub nodes: Vec<PlanNode>,
    pub output: OutputPlan,
    pub estimates: PlanEstimates,
}

pub struct PlanNode {
    pub id: NodeId,
    pub bind_vars: Vec<VarId>,
    pub subatoms: Vec<SubAtom>,
    pub implementation: NodeImpl,
    pub payload: PayloadDemand,
}
```

**Node Implementations**
```rust
pub enum NodeImpl {
    SortedLeapfrog,
    HashProbe,
    Hybrid,
    VectorLoop,
    ExistenceCheck,
    Product,
    AggregateSink,
}
```

**SubAtom**
```rust
pub struct SubAtom {
    pub atom_id: AtomId,
    pub relation: RelationId,
    pub fields: Vec<FieldId>,
    pub vars: Vec<VarId>,
    pub access: AccessId,
}
```

**Payload Demand**
```rust
pub struct PayloadDemand {
    pub projected_vars: BitSet<VarId>,
    pub aggregate_vars: BitSet<VarId>,
    pub existence_only_relations: BitSet<RelationId>,
    pub row_id_demands: BitSet<RelationId>,
}
```

**Examples**
Triangle count:
```text
node0 SortedLeapfrog bind=[a] subatoms=[EdgeAB(a), EdgeAC(a)]
node1 SortedLeapfrog bind=[b] subatoms=[EdgeAB(b), EdgeBC(b)]
node2 SortedLeapfrog bind=[c] subatoms=[EdgeAC(c), EdgeBC(c)]
node3 AggregateSink count(a)
```

Tag lookup:
```text
node0 HashProbe bind=[posting] subatoms=[PostingTag(tag=$tag, posting), Posting(id=posting)]
node1 VectorLoop bind=[account] subatoms=[Posting(account)]
output posting, account
```

**Planner Contract**
- Each atom is partitioned into subatoms across nodes.
- A variable can be bound once and reused in later nodes.
- A relation can be existence-only if no projected/aggregate payload depends on it.
- A node may bind multiple variables when that is cheaper than variable-at-a-time LFTJ.
- Output plan owns final set semantics or aggregate sink semantics.

**Executor Contract**
```rust
pub trait PlanNodeExecutor {
    fn execute(
        &mut self,
        input: BindingStream<'_>,
        output: &mut dyn BindingSink,
    ) -> Result<()>;
}
```

**Binding Stream**
```rust
pub enum BindingStream<'a> {
    One(BindingFrame<'a>),
    Rows(Box<dyn Iterator<Item = BindingFrame<'a>> + 'a>),
}
```

**Explain Output**
- Node order.
- Node implementation.
- Bound variables per node.
- Subatoms per node.
- Access paths per subatom.
- Estimated rows and actual rows per node.

**Tests**
- Build Free Join plans manually for simple query shapes.
- Validate subatom partitioning invariants.
- Validate variable binding dependencies.
- Validate explain output for a plan.
- Execute a manually built binary-style plan and LFTJ-style plan with identical output.

**Passing Criteria**
- Free Join plan IR exists and is used by executor entry point.
- Existing query semantics are preserved.
- Pure LFTJ is expressible as a Free Join plan.
- Binary-style/probe plans are expressible without a second executor architecture.

**Non-Goals**
- Do not implement a full cost optimizer in this PRD.
- Do not add code generation yet.
