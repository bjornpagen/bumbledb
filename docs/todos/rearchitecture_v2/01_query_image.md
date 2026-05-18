# 01: QueryImage

**Goal**
- Introduce immutable snapshot-local `QueryImage` as the hot query execution representation built from LMDB state.

**Why This Exists**
- Query execution needs reusable, CPU/cache-friendly structures.
- LMDB remains the durable substrate, but hot joins should not repeatedly walk LMDB prefix iterators.
- A `QueryImage` is the unit that sorted tries, hash tries, stats, and plan execution consume.

**Core Types**
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct QueryImageKey {
    pub schema: SchemaFingerprint,
    pub tx_id: u64,
}

pub struct QueryImage {
    key: QueryImageKey,
    relations: Vec<RelationImage>,
    relation_by_name: BTreeMap<String, RelationId>,
    stats: QueryImageStats,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RelationId(pub u16);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FieldId(pub u16);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RowId(pub u32);
```

**Cache Interface**
```rust
pub struct QueryImageCache {
    images: parking_lot::RwLock<BTreeMap<QueryImageKey, Arc<QueryImage>>>,
}

impl QueryImageCache {
    pub fn get_or_build(
        &self,
        txn: &ReadTxn<'_>,
        schema: &StorageSchema,
    ) -> Result<Arc<QueryImage>> {
        let key = QueryImageKey {
            schema: schema.descriptor().fingerprint(),
            tx_id: txn.last_committed_tx_id()?,
        };
        if let Some(image) = self.images.read().get(&key).cloned() {
            return Ok(image);
        }
        let image = Arc::new(QueryImageBuilder::new(txn, schema).build()?);
        self.images.write().insert(key, image.clone());
        Ok(image)
    }
}
```

**Builder Responsibilities**
- Read relation rows from durable LMDB snapshot.
- Build `RelationImage` for every relation in schema order.
- Build encoded columns in field declaration order.
- Build declared sorted trie indexes.
- Build declared hash trie indexes when requested by physical design.
- Build relation/index/prefix stats.
- Attach schema fingerprint and tx id.

**Builder Skeleton**
```rust
pub struct QueryImageBuilder<'a, 'txn> {
    txn: &'a ReadTxn<'txn>,
    schema: &'a StorageSchema,
}

impl<'a, 'txn> QueryImageBuilder<'a, 'txn> {
    pub fn build(self) -> Result<QueryImage> {
        let mut relations = Vec::new();
        for (relation_id, relation) in self.schema.descriptor().relations.iter().enumerate() {
            let relation_id = RelationId(relation_id as u16);
            let image = RelationImageBuilder::new(self.txn, self.schema, relation_id, relation)
                .build()?;
            relations.push(image);
        }
        Ok(QueryImage::new(self.schema, self.txn.last_committed_tx_id()?, relations))
    }
}
```

**Snapshot Invariants**
- A `QueryImage` is immutable after build.
- All relation images correspond to the same LMDB read snapshot.
- A read transaction may use only a query image with `tx_id <= read_snapshot_tx_id` and matching schema fingerprint.
- QueryImage data structures contain encoded values only unless explicitly marked as decoded cache.
- QueryImage can be dropped independently of LMDB transaction after build.

**Memory Accounting**
```rust
pub struct QueryImageStats {
    pub relation_count: usize,
    pub row_count: usize,
    pub encoded_column_bytes: usize,
    pub sorted_trie_bytes: usize,
    pub hash_trie_bytes: usize,
    pub build_micros: u128,
}
```

**Explain Additions**
- Query image tx id.
- Query image cache hit or miss.
- Query image build time.
- Relation image row counts.
- Index image count and memory usage.

**Tests**
- Build QueryImage from a seeded LMDB database.
- Verify row counts match storage diagnostics.
- Verify encoded columns decode to the same logical rows as public scans.
- Verify cache hit for repeated read on unchanged tx id.
- Verify cache miss after write commit changes tx id.
- Verify schema fingerprint mismatch cannot reuse an image.

**Passing Criteria**
- `cargo test --workspace` passes.
- QueryImage build is deterministic for the same snapshot.
- QueryImage contains every relation and every row in the LMDB snapshot.
- No query execution behavior changes yet unless explicitly wired in a later PRD.
- Benchmark harness can optionally print QueryImage build/cache stats without changing query results.

**Non-Goals**
- Do not optimize all indexes in this stage.
- Do not replace executor in this stage.
- Do not persist QueryImage yet.
