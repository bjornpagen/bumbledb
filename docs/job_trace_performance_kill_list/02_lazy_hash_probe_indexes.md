# 02 Lazy HashProbe Index Construction

Priority: P0

Primary affected queries:

- `job_q16_character_title_us`: cold hash build `96.6ms`, `99.2%` of prepare, `501,066` build rows, result `0`.
- `job_q24_voice_keyword_actor`: cold hash build `47.8ms`, `98.3%` of prepare, `317,617` build rows, result `0`.
- `job_q33_linked_series_companies`: cold hash build `26.1ms`, `96.4%` of prepare, `220,043` build rows, result `0`.

## Problem

HashProbe can be extremely fast once its hash tries are hot. In the JOB trace, steady-state hash execution for empty/selective queries is tiny:

- `job_q16`: hash execute `16.4%` of `711us`, about `113us` per sample.
- `job_q24`: hash execute `3.6%` of `778us`, about `27us` per sample.
- `job_q33`: hash execute `1.8%` of `809us`, about `14us` per sample.

Cold prepare is the opposite: the engine eagerly builds every hash trie needed by every possible node before it performs the first probe. For empty/selective queries, many of those indexes are never used because an early miss terminates execution.

## Trace Evidence

| Query | Hash Index Build | Build Rows | Runtime Probes | Output | Biggest Wasted Builds |
|---|---:|---:|---:|---:|---|
| `job_q16_character_title_us` | `96.6ms` | `501,066` | `166` | 0 | `Keyword 134,170`, `CompanyName 200,000`, `Name 100,000` |
| `job_q24_voice_keyword_actor` | `47.8ms` | `317,617` | `2` | 0 | `CharName 200,000`, `Name 100,000` |
| `job_q33_linked_series_companies` | `26.1ms` | `220,043` | `5` | 0 | `CompanyName 200,000` |

The worst example is `job_q24`: execution performs only `2` probes, one hit and one miss, then terminates. Before that, the engine built `7` hash tries, including `CharName` and `Name`, which are downstream of the failed branch.

## Current Technical Cause

`execute_hash_probe` builds every hash atom index up front:

`crates/bumbledb-lmdb/src/query.rs:1349-1408`

```rust
let atom_indexes = {
    let _span = tracing::debug_span!(
        "bumbledb.query.hash.build_indexes",
        atoms = plan.relation_atoms.len()
    )
    .entered();
    build_hash_atom_indexes(image, schema, plan)?
};

...

if !executor.static_atoms_pass()? {
    Ok(())
} else {
    executor.execute(0)
}
```

The ordering is the problem. `build_hash_atom_indexes` runs before:

- `static_atoms_pass`
- the first driver probe
- early prefix misses
- comparison filters
- any branch pruning

`build_hash_atom_indexes` gathers every subatom from every node and builds/fetches a hash trie for each requested access:

`crates/bumbledb-lmdb/src/query.rs:1417-1483`

```rust
let subatoms = plan
    .summary
    .free_join
    .nodes
    .iter()
    .flat_map(|node| {
        node.subatoms.iter().map(move |subatom| {
            (
                node.id.0 as usize,
                subatom.atom_id.0 as usize,
                subatom.access,
                subatom.fields.clone(),
            )
        })
    })
    .collect::<Vec<_>>();

for (node_id, atom_id, access, bind_fields) in subatoms {
    ...
    let cached = image.cached_hash_trie(key, || {
        crate::query_image::build_hash_trie_index(
            relation,
            IndexSpec::new(format!("{}_hash", atom.relation_name), fields.clone()),
        )
    })?;
```

`QueryImage::cached_hash_trie` avoids repeat builds after the miss, but the miss path is still synchronous and full-relation:

`crates/bumbledb-lmdb/src/query_image.rs:221-249`

```rust
if let Some(index) = self.hash_trie_cache.read()?.get(key).cloned() {
    return Ok(CachedHashTrie { index, hit: true });
}

let index = Arc::new(build()?);
...
cache.insert(key.to_owned(), index.clone());
```

Hash trie construction scans every row and stores row IDs by default:

`crates/bumbledb-lmdb/src/hash_trie.rs:31-71`

```rust
for row in 0..relation.row_count {
    let row = RowId(row as u32);
    let keys = spec
        .fields
        .iter()
        .map(|field| {
            relation
                .encoded(row, *field)
                .map(EncodedOwned::from_ref)
                .ok_or_else(|| crate::Error::internal("missing hash trie field value"))
        })
        .collect::<Result<KeyStack>>()?;
    insert_row(&mut root, &keys, row, leaf_mode);
}
```

The hash trie implementation already supports `LeafMode::CountOnly`, but query hash builds always call row-retaining mode:

`crates/bumbledb-lmdb/src/query_image.rs:289-294`

```rust
pub(crate) fn build_hash_trie_index(
    relation: &RelationImage,
    spec: IndexSpec,
) -> Result<HashTrieIndex> {
    HashTrieIndex::build_with_mode(relation, spec, LeafMode::Rows)
}
```

## Desired End State

HashProbe should build only the indexes it actually probes, at the time it probes them.

For an empty query that dies at node 0 or node 1, downstream relation indexes should never be built.

For existence-only atoms, HashProbe should build count-only indexes or use existing index/count metadata instead of row-retaining tries.

## Proposed Technical Solution

Replace eager `Vec<HashAtomIndex>` construction with a lazy `HashIndexProvider` used by `HashProbeExecutor`.

### New Data Structure

```rust
struct HashIndexProvider<'image> {
    image: &'image QueryImage,
    schema: &'image StorageSchema,
    requests: Vec<HashIndexRequest>,
    indexes: Vec<Option<Arc<HashTrieIndex>>>,
}

struct HashIndexRequest {
    node_id: usize,
    atom_id: usize,
    access: AccessId,
    fields: Vec<FieldId>,
    cache_key: String,
    index_name: String,
    leaf_mode: LeafMode,
}
```

`build_hash_atom_indexes` should become `build_hash_index_requests`. It computes metadata only; it must not call `cached_hash_trie` or scan relation rows.

### Lazy Lookup

Change `HashProbeExecutor::hash_index` so it asks the provider for an index:

```rust
fn hash_index(&mut self, depth: usize, atom_id: usize) -> Result<&HashAtomIndex> {
    let request_id = self.request_for(depth, atom_id)?;
    self.index_provider.get_or_build(request_id, &mut self.plan.summary.counters)
}
```

The provider:

1. Checks whether this request has already built an `Arc<HashTrieIndex>`.
2. Checks `QueryImage` hash cache.
3. If miss, builds exactly this index.
4. Records `hash_index_builds` and `hash_index_build_rows`.
5. Returns the index.

### Build Before Probe, Not Before Execution

Existing execution already has good early exits:

`crates/bumbledb-lmdb/src/query.rs:2129-2180`

```rust
let row_count = index.count(&refs);
self.plan.summary.counters.hash_probe_calls += 1;
if row_count == 0 {
    self.plan.summary.counters.hash_probe_misses += 1;
    return Ok(());
}
```

With lazy indexes, this early miss prevents downstream indexes from ever being built.

### Existence-Only And Count-Only Modes

The `free_join::PayloadDemand` type already has an `existence_only_relations` field:

`crates/bumbledb-lmdb/src/free_join.rs:100-110`

```rust
pub struct PayloadDemand {
    pub projected_vars: Vec<VarId>,
    pub aggregate_vars: Vec<VarId>,
    pub existence_only_relations: Vec<RelationId>,
    pub row_id_demands: Vec<RelationId>,
}
```

Use this to select `LeafMode::CountOnly` when:

- The atom is only used for existence.
- No later node needs row IDs from that atom.
- No row-dependent field outside the index prefix must be checked.

Add:

```rust
pub(crate) fn build_hash_trie_index_with_mode(
    relation: &RelationImage,
    spec: IndexSpec,
    mode: LeafMode,
) -> Result<HashTrieIndex> {
    HashTrieIndex::build_with_mode(relation, spec, mode)
}
```

### Prefix-Compatible Sharing

The trace shows duplicate builds for the same relation with one-field and two-field key sets. A longer trie can often answer a shorter prefix `count`/`exists` query.

Add a cache lookup strategy:

- Exact key lookup first.
- If not found, look for a cached trie with the same relation and an index field list whose prefix equals the requested fields.
- Use that trie for `exists` and `count` requests.
- Only require exact trie for row iteration if row order/leaf semantics matter.

This can be implemented in `QueryImage` with a secondary map:

```rust
hash_trie_by_relation_fields: BTreeMap<(RelationId, Vec<FieldId>, LeafMode), Arc<HashTrieIndex>>
```

### Static Atom Ordering

Move `static_atoms_pass` before building unrelated index requests. With lazy indexes this happens naturally: `static_atoms_pass` asks for only the static atom indexes it needs.

## Implementation Plan

1. Introduce `HashIndexRequest` and `HashIndexProvider`.
2. Replace eager `build_hash_atom_indexes` with metadata-only request construction.
3. Update `HashProbeExecutor` to own the provider instead of `Vec<HashAtomIndex>`.
4. Change `hash_index`, `probe_atom_count`, and `atom_has_matching_row` to build on demand.
5. Add `LeafMode` selection for existence-only requests.
6. Add prefix-compatible cache lookup.
7. Update diagnostics to distinguish request count from actual built count.

## Tests

Add unit tests in `query.rs`:

- Query dies at first hash node: only first-node hash indexes are built.
- Query dies at second node: downstream indexes are not built.
- Existing behavior preserved for non-empty joins.
- Repeated execution hits `QueryImage` hash trie cache and does not rebuild.
- Count-only/existence-only atom uses `LeafMode::CountOnly` and cannot return rows.
- Prefix-compatible trie serves shorter prefix `count`.

Add benchmark regression assertions:

- `job_q24_voice_keyword_actor` cold `hash_index_build_rows` drops from `317,617` by at least `80%`.
- `job_q33_linked_series_companies` cold `hash_index_build_rows` drops from `220,043` by at least `80%`.
- `job_q16_character_title_us` cold `hash_index_us` drops from `~96ms` by at least `60%`.

## Acceptance Criteria

- Empty/selective HashProbe queries do not build indexes for unreachable nodes.
- `hash_index_builds` and `hash_index_build_rows` correlate with actually probed atoms.
- Steady-state remains at least as fast as current run.
- Cold prepare time improves dramatically for `q16`, `q24`, and `q33`.

## Risks

- Lazy provider must not borrow `plan` in ways that conflict with mutable counter updates.
- Prefix-compatible trie sharing must respect field order and leaf mode.
- Count-only tries cannot support row iteration; request classification must be exact.

## Rollout Plan

1. Metadata-only requests with exact cache keys, row-retaining mode only.
2. Lazy build on first use.
3. Add count-only mode for pure existence checks.
4. Add prefix-compatible sharing.
5. Re-run full JOB firehose trace and update this kill list with before/after deltas.
