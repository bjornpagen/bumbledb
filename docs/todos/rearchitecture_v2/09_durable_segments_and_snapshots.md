# 09: Durable Segments And Snapshots

**Goal**
- Evolve durable storage to produce QueryImage-friendly encoded column and index segments while preserving safe LMDB durability.

**Segment Metadata**
```rust
pub struct SegmentDescriptor {
    pub relation: RelationId,
    pub segment_id: u64,
    pub tx_start: u64,
    pub tx_end: Option<u64>,
    pub row_count: usize,
    pub columns: Vec<ColumnSegmentDescriptor>,
    pub indexes: Vec<IndexSegmentDescriptor>,
}
```

**Column Segment**
```rust
pub struct ColumnSegmentDescriptor {
    pub field: FieldId,
    pub value_type: ValueType,
    pub width: usize,
    pub lmdb_key: Vec<u8>,
    pub byte_len: usize,
}
```

**Index Segment**
```rust
pub struct IndexSegmentDescriptor {
    pub access: AccessId,
    pub fields: Vec<FieldId>,
    pub kind: IndexKind,
    pub lmdb_key: Vec<u8>,
    pub byte_len: usize,
    pub stats: IndexStatsSummary,
}
```

**Storage Direction**
- Bulk load writes encoded column chunks and index chunks.
- Row-by-row writes append delta segments.
- Read snapshot composes base segments plus visible deltas.
- Background compaction merges segments.
- QueryImage can map/copy segment bytes directly into relation images.

**Snapshot Rules**
- Every segment has visibility interval `[tx_start, tx_end)`.
- A read transaction at `tx_id` sees segments where `tx_start <= tx_id` and `tx_end` is absent or greater than `tx_id`.
- QueryImage key remains `(schema_fingerprint, tx_id)`.
- Commit publishes segment metadata atomically after all data chunks are durable.

**LMDB Namespaces**
```text
segment:meta:<relation_id>:<segment_id>
segment:column:<relation_id>:<segment_id>:<field_id>
segment:index:<relation_id>:<segment_id>:<index_id>
segment:visibility:<tx_id>:<relation_id>:<segment_id>
```

**Write Path Sketch**
```rust
impl WriteTxn<'_> {
    pub fn append_segment(&mut self, relation: RelationId, rows: EncodedRowBatch) -> Result<()> {
        let segment = SegmentBuilder::new(relation, rows).build()?;
        self.put_segment_bytes(&segment)?;
        self.put_segment_metadata(&segment)?;
        self.publish_segment_visibility(&segment)?;
        Ok(())
    }
}
```

**Tests**
- Bulk load writes segment metadata and bytes.
- Reopen rebuilds QueryImage from segments.
- Read snapshot sees correct segment set before and after writes.
- Delete/replace delta semantics are represented correctly.
- Crash tests preserve atomic segment publication.

**Passing Criteria**
- Existing durability tests pass.
- QueryImage can be built without scanning covering index keys.
- Bulk load is no slower than current bulk load by more than an accepted temporary threshold.
- Reopen plus first query uses segment metadata to build QueryImage.

**Non-Goals**
- Do not implement compression yet.
- Do not implement distributed storage.
