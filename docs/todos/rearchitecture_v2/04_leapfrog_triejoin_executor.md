# 04: Leapfrog Triejoin Executor

**Goal**
- Replace candidate-set WCOJ execution with true LFTJ over `TrieIter` implementations.

**Current Path To Delete**
- `candidate_values_for_variable` collecting `BTreeSet<EncodedValue>`.
- `collect_atom_candidates` scanning encoded prefixes into sets.
- `BTreeSet` intersections for variable domains.

**Leapfrog Join**
```rust
pub struct LeapfrogJoin<'a> {
    iters: SmallVec<[&'a mut dyn LinearIter; 8]>,
    p: usize,
    at_end: bool,
}

impl<'a> LeapfrogJoin<'a> {
    pub fn init(&mut self) {
        self.iters.sort_by(|a, b| a.key().as_bytes().cmp(b.key().as_bytes()));
        self.p = 0;
        self.search();
    }

    pub fn search(&mut self) {
        if self.iters.iter().any(|iter| iter.at_end()) {
            self.at_end = true;
            return;
        }
        let mut max = self.iters[(self.p + self.iters.len() - 1) % self.iters.len()].key();
        loop {
            let current = self.iters[self.p].key();
            if current.as_bytes() == max.as_bytes() {
                return;
            }
            self.iters[self.p].seek(max);
            if self.iters[self.p].at_end() {
                self.at_end = true;
                return;
            }
            max = self.iters[self.p].key();
            self.p = (self.p + 1) % self.iters.len();
        }
    }
}
```

**Triejoin State**
```rust
pub struct LeapfrogTrieJoin<'a> {
    variable_order: Vec<VarId>,
    atom_iters: Vec<Box<dyn TrieIter + 'a>>,
    joins: Vec<LeapfrogJoinState>,
    depth: isize,
    binding: BindingFrame<'a>,
    counters: LftjCounters,
}

pub struct LeapfrogJoinState {
    pub atom_iter_ids: SmallVec<[usize; 8]>,
    pub initialized: bool,
}
```

**Open/Up Semantics**
```rust
impl<'a> LeapfrogTrieJoin<'a> {
    pub fn open(&mut self) {
        self.depth += 1;
        let state = &self.joins[self.depth as usize];
        for iter_id in &state.atom_iter_ids {
            self.atom_iters[*iter_id].open();
        }
        self.init_current_join();
    }

    pub fn up(&mut self) {
        let state = &self.joins[self.depth as usize];
        for iter_id in &state.atom_iter_ids {
            self.atom_iters[*iter_id].up();
        }
        self.depth -= 1;
    }
}
```

**Execution Loop**
```rust
pub fn enumerate(&mut self, sink: &mut dyn TupleSink) -> Result<()> {
    self.open();
    loop {
        if self.current_join().at_end() {
            if self.depth == 0 { break; }
            self.up();
            self.current_join_mut().next();
            continue;
        }

        self.bind_current_key()?;
        if !self.ready_predicates_pass()? {
            self.current_join_mut().next();
            continue;
        }

        if self.depth as usize + 1 == self.variable_order.len() {
            sink.emit(&self.binding)?;
            self.current_join_mut().next();
        } else {
            self.open();
        }
    }
    Ok(())
}
```

**Binding Frame**
```rust
pub struct BindingFrame<'a> {
    values: Vec<Option<EncodedRef<'a>>>,
}

impl<'a> BindingFrame<'a> {
    #[inline]
    pub fn bind(&mut self, var: VarId, value: EncodedRef<'a>) {
        self.values[var.0 as usize] = Some(value);
    }
}
```

**Required Rewrite Rules**
- Every atom argument order must be a subsequence of the variable order for the selected index.
- If not, the planner must choose or request another index permutation.
- Constants and inputs are represented as one-element trie iterators or prefix-bound iterators.
- Comparisons are attached to the earliest variable depth where all operands are bound.

**Counters**
```rust
pub struct LftjCounters {
    pub open_count: u64,
    pub up_count: u64,
    pub next_count: u64,
    pub seek_count: u64,
    pub emitted_bindings: u64,
    pub predicate_checks: u64,
    pub predicate_failures: u64,
}
```

**Tests**
- Unary intersection tests for leapfrog join.
- Binary relation triejoin tests.
- Triangle query over three binary relations.
- Empty triangle query where pairwise joins are large.
- Same result set as current executor and reference evaluator.
- Counters show no candidate set materialization.

**Passing Criteria**
- Production query execution uses LFTJ for full conjunctive query bodies.
- No `BTreeSet<EncodedValue>` candidate-domain construction remains in production executor.
- No LMDB scan is opened during variable recursion.
- `triangle_count` counter output reports trie seeks/next/open/up, not prefix scan openings.
- Existing query, property, SQLite comparison, and benchmark row-count tests pass.

**Non-Goals**
- Do not implement Free Join hybrid plans yet.
- Do not optimize aggregation yet.
