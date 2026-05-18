# 06: Hash Trie And Hybrid Nodes

**Goal**
- Add hash trie indexes and hybrid Free Join nodes for point/probe-heavy query shapes.

**Hash Trie Types**
```rust
pub struct HashTrieIndex {
    pub relation: RelationId,
    pub name: String,
    pub fields: Vec<FieldId>,
    pub root: HashNode,
    pub stats: HashTrieStats,
}

pub enum HashNode {
    Inner(hashbrown::HashMap<EncodedOwned, HashNode>),
    Leaf(RowSet),
    CountOnly(u32),
}

pub enum RowSet {
    Empty,
    One(RowId),
    Small(smallvec::SmallVec<[RowId; 4]>),
    Many(Vec<RowId>),
    Range(RowRange),
}
```

**Probe Interface**
```rust
pub trait PrefixProbe {
    fn exists(&self, prefix: &[EncodedRef<'_>]) -> bool;
    fn count(&self, prefix: &[EncodedRef<'_>]) -> usize;
    fn rows<'a>(&'a self, prefix: &[EncodedRef<'_>]) -> RowSetRef<'a>;
}
```

**Specialized Leaves**
- Use `CountOnly` when relation is existence-only.
- Use `One` for unique/primary keys.
- Use `SmallVec` for common low fanout.
- Use `Vec` for high fanout.
- Use `Range` when sorted input gives contiguous row runs.

**Hybrid Node**
```rust
pub struct HybridNodeExecutor<'a> {
    driver: DriverSource<'a>,
    probes: Vec<HashProbeSpec>,
    sorted_checks: Vec<SortedCheckSpec>,
    payload: PayloadDemand,
}

pub enum DriverSource<'a> {
    RowRange(&'a RelationImage, RowRange),
    RowSet(RowSetRef<'a>),
    TrieDistinct(Box<dyn TrieIter + 'a>),
}
```

**Execution Sketch**
```rust
impl<'a> HybridNodeExecutor<'a> {
    pub fn execute(&mut self, input: &BindingFrame<'a>, sink: &mut dyn BindingSink<'a>) -> Result<()> {
        for row in self.driver.rows(input) {
            let mut frame = input.clone_shallow();
            self.bind_driver_payload(row, &mut frame)?;
            if self.probes.iter().all(|probe| probe.matches(&frame))
                && self.sorted_checks.iter_mut().all(|check| check.matches(&frame))
            {
                sink.push(frame)?;
            }
        }
        Ok(())
    }
}
```

**Use Cases**
- Exact primary lookup after binding a foreign key.
- Static predicate lookup such as `tag=$tag` or `nation=$nation`.
- Existence-only relation such as `T(x)` in clover query.
- Intermediate result probing in Free Join nodes.

**Tests**
- Hash trie build over primary key.
- Hash trie build over non-unique equality field.
- Existence-only index stores count/marker without row ids.
- Hybrid node probes hash trie and emits correct bindings.
- Hybrid plan and sorted plan produce the same results.

**Passing Criteria**
- Point/probe-heavy benchmark queries reduce seek/scan counters materially.
- `tag_lookup_join` no longer performs one scan per output posting for primary lookup.
- `supplier_nation_orders` and `red_boat_sailors` can use hash/probe nodes when cheaper.
- No second executor architecture is introduced.

**Non-Goals**
- Do not remove sorted trie indexes.
- Do not implement all optimizer rules in this PRD.
