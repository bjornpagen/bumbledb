# 12: Query Normalization And Runtime Specialization

**Goal**
- Normalize typed Datalog into an executor-friendly IR and prepare the runtime for future generated/specialized execution without committing to a codegen backend immediately.

**Why This Exists**
- The executor should not reason directly over parser/typechecker data structures.
- Rewrites for constants, repeated variables, comparisons, aliases, projections, and aggregate demands must happen before physical planning.
- Future specialization should remove dynamic dispatch and field/type branching from hot loops.

**Normalized Query IR**
```rust
pub struct NormalizedQuery {
    pub vars: Vec<NormVar>,
    pub inputs: Vec<NormInput>,
    pub atoms: Vec<NormAtom>,
    pub predicates: Vec<NormPredicate>,
    pub output: OutputPlan,
}

pub struct NormVar {
    pub id: VarId,
    pub name: String,
    pub value_type: ValueType,
}

pub struct NormAtom {
    pub id: AtomId,
    pub relation: RelationId,
    pub fields: Vec<NormAtomField>,
}

pub struct NormAtomField {
    pub field: FieldId,
    pub term: NormTerm,
    pub value_type: ValueType,
}

pub enum NormTerm {
    Var(VarId),
    Input(InputId),
    Literal(EncodedOwned),
    Wildcard,
}
```

**Predicate IR**
```rust
pub struct NormPredicate {
    pub id: PredicateId,
    pub operands: [NormOperand; 2],
    pub op: ComparisonOperator,
    pub value_type: ValueType,
    pub earliest_depth: Option<usize>,
}

pub enum NormOperand {
    Var(VarId),
    Input(InputId),
    Literal(EncodedOwned),
}
```

**Normalization Rules**
- Resolve relation names to `RelationId`.
- Resolve field names to `FieldId`.
- Encode literals and inputs where possible.
- Normalize `Id` and `Ref` equality domains.
- Rewrite repeated variables in one atom into equality predicates if needed.
- Attach every predicate to the earliest variable depth after planning.
- Compute output payload demand and aggregate demand.

**Runtime Specialization Boundary**
```rust
pub trait ExecutablePlan {
    fn execute<'a>(
        &mut self,
        image: &'a QueryImage,
        inputs: &'a EncodedInputs,
        sink: &mut dyn TupleSink<'a>,
    ) -> Result<ExecutionCounters>;
}
```

**Interpreted First, Specialized Later**
```rust
pub enum CompiledPlan {
    Interpreted(InterpretedFreeJoinPlan),
    Specialized(Box<dyn ExecutablePlan + Send + Sync>),
}
```

**Specialization Targets**
- Replace dynamic `FieldId` lookups with generated column offsets.
- Replace dynamic `ValueType` matches with monomorphic operations.
- Replace boxed iterators with concrete plan-node structs.
- Inline predicate checks.
- Inline aggregate state updates.
- Use small fixed-size arrays for common low-arity joins.

**Generated Rust Sketch**
```rust
pub struct QRedBoatSailors<'a> {
    boat_by_color: &'a HashTrieIndex,
    reserve_by_boat: &'a SortedTrieIndex,
    sailor_primary: &'a HashTrieIndex,
}

impl<'a> ExecutablePlan for QRedBoatSailors<'a> {
    fn execute<'b>(
        &mut self,
        image: &'b QueryImage,
        inputs: &'b EncodedInputs,
        sink: &mut dyn TupleSink<'b>,
    ) -> Result<ExecutionCounters> {
        let color = inputs.get("color")?;
        for boat_row in self.boat_by_color.rows(&[color]) {
            let boat = image.field(boat_row, field!(Boat.id));
            for reserve_row in self.reserve_by_boat.rows(&[boat]) {
                let sailor = image.field(reserve_row, field!(Reserve.sailor));
                if let Some(sailor_row) = self.sailor_primary.one(&[sailor]) {
                    sink.emit_values(&[sailor, image.field(sailor_row, field!(Sailor.rating))])?;
                }
            }
        }
        Ok(ExecutionCounters::default())
    }
}
```

**Tests**
- Normalized query preserves typed query semantics.
- Repeated-variable rewrite is correct.
- Literal/input encoding errors preserve user-facing query errors.
- Predicate earliest-depth assignment is deterministic.
- Interpreted and specialized mock plans return same rows for fixtures.

**Passing Criteria**
- Planner consumes `NormalizedQuery`, not raw `TypedQuery`.
- Executor consumes `FreeJoinPlan` plus `NormalizedQuery`, not parser structures.
- Hot path has a clear future route to monomorphic specialization.
- Existing parser/typechecker tests still pass unchanged.

**Non-Goals**
- Do not implement a production codegen backend in this PRD.
- Do not introduce SQL or non-Datalog semantics.
