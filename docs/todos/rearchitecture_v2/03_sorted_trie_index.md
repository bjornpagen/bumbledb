# 03: Sorted Trie Index

**Goal**
- Build an in-memory sorted trie index over encoded column data with true trie iterator semantics for LFTJ.

**Why This Exists**
- Current execution scans LMDB prefixes and materializes candidate sets.
- LFTJ requires persistent iterator state with `key`, `next`, `seek`, `open`, and `up`.
- The index must support distinct-key iteration at each trie depth and section cardinality lookups.

**Sorted Trie Shape**
```rust
pub struct SortedTrieIndex {
    pub relation: RelationId,
    pub name: String,
    pub fields: Vec<FieldId>,
    pub order: Vec<RowId>,
    pub levels: Vec<TrieLevel>,
    pub stats: TrieStats,
}

pub struct TrieLevel {
    pub field: FieldId,
    pub keys: Vec<EncodedOwned>,
    pub ranges: Vec<RowRange>,
    pub parent: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EncodedOwned {
    One([u8; 1]),
    Eight([u8; 8]),
    Sixteen([u8; 16]),
}
```

**Build Algorithm**
```rust
impl SortedTrieIndex {
    pub fn build(relation: &RelationImage, spec: &IndexSpec) -> Self {
        let mut order = (0..relation.row_count)
            .map(|row| RowId(row as u32))
            .collect::<Vec<_>>();

        order.sort_by(|left, right| {
            for field in &spec.fields {
                let l = relation.encoded(*left, *field).as_bytes();
                let r = relation.encoded(*right, *field).as_bytes();
                match l.cmp(r) {
                    std::cmp::Ordering::Equal => continue,
                    order => return order,
                }
            }
            left.cmp(right)
        });

        let levels = TrieLevelBuilder::new(relation, &order, &spec.fields).build();
        Self { relation: relation.id, name: spec.name.clone(), fields: spec.fields.clone(), order, levels, stats: TrieStats::default() }
    }
}
```

**Iterator Trait**
```rust
pub trait LinearIter {
    fn key(&self) -> EncodedRef<'_>;
    fn next(&mut self);
    fn seek(&mut self, target: EncodedRef<'_>);
    fn at_end(&self) -> bool;
}

pub trait TrieIter: LinearIter {
    fn open(&mut self);
    fn up(&mut self);
    fn depth(&self) -> usize;
    fn current_range(&self) -> RowRange;
    fn count(&self) -> usize;
}
```

**Concrete Cursor**
```rust
pub struct SortedTrieIter<'a> {
    index: &'a SortedTrieIndex,
    stack: SmallVec<[TrieFrame; 8]>,
}

#[derive(Clone, Copy)]
pub struct TrieFrame {
    pub depth: usize,
    pub begin: usize,
    pub end: usize,
    pub pos: usize,
}
```

**Cursor Semantics**
- At depth `d`, `key()` returns the current distinct encoded value for `fields[d]`.
- `next()` advances to the next distinct value in the current parent range.
- `seek(target)` advances to the first distinct value greater than or equal to `target`.
- `open()` descends into the child row range of the current key.
- `up()` pops one trie frame.
- `count()` returns the number of rows under the current key/range.

**Binary Search With Hints**
```rust
impl<'a> SortedTrieIter<'a> {
    fn seek_in_frame(&mut self, target: EncodedRef<'_>) {
        let frame = self.stack.last_mut().unwrap();
        let keys = &self.index.levels[frame.depth].keys[frame.begin..frame.end];
        let relative = lower_bound_encoded(keys, target, frame.pos - frame.begin);
        frame.pos = frame.begin + relative;
    }
}
```

**Trie Stats**
```rust
pub struct TrieStats {
    pub row_count: usize,
    pub distinct_by_depth: Vec<usize>,
    pub avg_fanout_by_depth: Vec<f64>,
    pub max_fanout_by_depth: Vec<usize>,
    pub build_micros: u128,
}
```

**Invariants**
- `order` is sorted by the index field permutation.
- Level ranges always refer to contiguous ranges in `order`.
- A child range is always contained in its parent range.
- `seek` is monotone within a frame.
- No iterator method performs allocation in the normal path.
- No iterator method calls LMDB.

**Tests**
- Build a trie over one, two, and three fields.
- Verify distinct iteration at each level.
- Verify `seek` lands on least upper bound.
- Verify `open/up` preserves parent cursor state.
- Verify `current_range` maps to the expected rows.
- Verify duplicate keys collapse at trie levels while row ranges retain all rows.

**Benchmark Counters**
- `trie_open`
- `trie_up`
- `trie_next`
- `trie_seek`
- `trie_key_reads`
- `trie_ranges_read`
- `trie_allocations`

**Passing Criteria**
- Iterator unit tests pass on deterministic fixtures.
- `seek`, `next`, `open`, and `up` do not allocate.
- Query execution is not yet required to use this index in this PRD.
- Microbenchmark shows iteration over distinct values does not scan duplicate row payloads.

**Non-Goals**
- Do not implement Free Join planning here.
- Do not implement hash indexes here.
- Do not persist the trie here.
