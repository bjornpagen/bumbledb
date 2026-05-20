# PRD 13: LFTJ Durable Index Trie Source

## Status

Proposed.

## Motivation

After PRDs 04 and 05, cold LFTJ atom builds should allocate far less. But they still perform unnecessary work in many cases:

```text
durable sorted index bytes -> scan/copy/filter -> temporary RelationImage -> SortedTrieIndex::build -> LFTJ iterator
```

If a durable relation index is already sorted by the atom's bound prefix and remaining LFTJ variable order, the engine should not rebuild a temporary sorted trie. It should expose that durable index range as a trie-like iterator.

This PRD introduces a trie source abstraction over durable index images.

## Evidence

| Evidence | Anchor |
|---|---|
| Relation index images contain sorted encoded index bytes and component offsets | `crates/bumbledb-lmdb/src/query_image.rs:504-552` |
| Prefix iterators already stream matching durable index entries | `crates/bumbledb-lmdb/src/query_image.rs:554-610` |
| LFTJ currently uses durable index only as source to rebuild temp tries | `crates/bumbledb-lmdb/src/query.rs:5599-5678` |
| `SortedTrieIndex` exposes trie iterator API used by LFTJ | `crates/bumbledb-lmdb/src/sorted_trie.rs:64-126` |
| q09 cold build spends 115.6 ms in sorted-trie wrapper and 506.8 ms scan/filter/copy | `docs/job-trace-analysis/04-job_q09_voice_us_actor.md:87-113` |
| q24 sorted trie relation 18 costs 5.7 ms of 6.15 ms sorted-trie wrapper | `docs/job-trace-analysis/06-job_q24_voice_keyword_actor.md:79-95` |

## Goals

- Add an LFTJ atom source abstraction that can be either a cached `SortedTrieIndex` or a direct durable-index adapter.
- Use direct durable-index source when index order exactly satisfies LFTJ variable order after fixed prefix fields.
- Preserve duplicate elimination and trie semantics.
- Avoid temporary relation/trie build for eligible atoms.
- Fall back to PRD 04/05 typed builders when durable index order is not sufficient.

## Non-Goals

- Do not compromise correctness for partial matches.
- Do not change planner variable ordering in this PRD.
- Do not remove `SortedTrieIndex`; it remains necessary for many atoms.
- Do not add a full new storage index format.

## Required Abstraction

Current LFTJ atom plan stores:

```rust
struct LftjAtomPlan {
    variables: Vec<usize>,
    trie: Arc<SortedTrieIndex>,
    row_count: usize,
}
```

Change to:

```rust
enum LftjAtomSource {
    SortedTrie(Arc<SortedTrieIndex>),
    DurableIndex(DurableIndexTrieSource),
}

struct LftjAtomPlan {
    variables: Box<[usize]>,
    source: LftjAtomSource,
    row_count: usize,
}
```

Then create a trait or enum-dispatch iterator:

```rust
trait TrieLikeIndex {
    type Iter<'a>: TrieIter + 'a where Self: 'a;
    fn iter(&self) -> Self::Iter<'_>;
}
```

If trait-object lifetime complexity is too high, use enum dispatch:

```rust
enum LftjAtomIter<'a> {
    Sorted(SortedTrieIter<'a>),
    Durable(DurableIndexTrieIter<'a>),
}
```

Update LFTJ runtime to hold `Vec<LftjAtomIter<'image>>`.

## Eligibility Rules

A durable index can serve directly when all are true:

- The index contains all atom fields needed for variable extraction, literals, inputs, predicates, and output.
- Fixed literal/input terms form a prefix over whole leading index fields.
- After that prefix, the next index components correspond exactly to `variables` in LFTJ atom variable order.
- Duplicate-variable equality can be enforced without reordering ambiguity.
- Local predicates can be evaluated from fields available in the index entry.
- Wildcard fields do not break ordering needed by variables.
- The prefix range is contiguous, which is true for sorted index bytes.

If any condition fails, use the typed builder fallback from PRD 04/05.

## Durable Trie Semantics

LFTJ needs operations:

- `open`
- `up`
- `next`
- `seek`
- `key`
- `at_end`
- `count`
- `current_rows` where needed by existing traits

`DurableIndexTrieIter` can implement these over a sorted entry slice range.

For a prefix range and variable component offsets:

- Depth 0 distinct keys are groups of entries sharing variable 0 bytes.
- Depth 1 distinct keys are groups within the current depth 0 range sharing variable 1 bytes.
- Continue for all variables.

This is the same logical structure `SortedTrieIndex::build_levels` creates, but computed lazily over the durable index byte range.

## Data Structure

```rust
struct DurableIndexTrieSource {
    relation: RelationId,
    access: AccessId,
    index: Arc<RelationIndexImage>, // or borrowed from RelationImage if lifetime permits
    prefix: Box<[u8]>,
    variable_components: Box<[RelationIndexComponent]>,
    range: std::ops::Range<usize>,
}
```

If `RelationIndexImage` stays owned inside `RelationImage`, avoid `Arc` and store relation/access IDs plus borrow in `LftjAtomSource`. The exact ownership depends on `QueryImage` structure after PRD 12.

## Implementation Plan

### Step 1: Add Durable Index Prefix Range API

Depends on PRD 06. Use `prefix_range` to avoid scanning outside matching prefix.

### Step 2: Add Eligibility Function

Add near indexed-prefix selection:

```rust
fn durable_index_lftj_source(
    source: &RelationImage,
    query: &NormalizedQuery,
    inputs: &EncodedInputs,
    atom: &NormAtom,
    variables: &[usize],
) -> Result<Option<DurableIndexTrieSource>>;
```

This function can share prefix selection logic from PRD 05, but must additionally verify post-prefix variable component order.

### Step 3: Implement `DurableIndexTrieIter`

The iterator frame needs:

```rust
struct DurableFrame {
    depth: usize,
    start: usize,
    end: usize,
    pos: usize,
}
```

At each depth:

- `open` initializes first distinct key group inside parent range.
- `next` advances to next distinct key group.
- `seek(target)` finds the first distinct key >= target inside current parent range.
- `key` returns encoded bytes for current component.
- `count` returns number of rows under current frame or suffix count.

Use binary search for `seek` where possible; linear group advance is acceptable initially if ranges are small, but q09 may have large ranges. Prefer partition-point over entries by component bytes.

### Step 4: Integrate With LFTJ Runtime

Update:

- `LftjAtomPlan` construction at `query.rs:5430-5486`.
- `execute_lftj` iterator creation at `query.rs:4925-4931`.
- `LftjPrefixProbe` iterator storage at `query.rs:5042-5069`.
- `LeapfrogState` key/seek logic if it assumes `SortedTrieIter` concrete type.

### Step 5: Counters And Explain

Add counters:

- `durable_index_trie_sources`
- `durable_index_trie_rows`
- `durable_index_trie_seeks`

Or reuse existing trie counters but add explain metadata to distinguish direct durable source vs built sorted trie.

## Acceptance Criteria

- Eligible LFTJ atoms avoid `build_lftj_sorted_trie` and `SortedTrieIndex::build`.
- q09/q24 build spans drop for atoms with matching durable indexes.
- Fallback typed builder path still works for ineligible atoms.
- Outputs match before/after.

## Tests

### Unit Tests

- Durable index source produces same key sequence as `SortedTrieIndex` for a one-variable atom.
- Durable index source produces same nested traversal as `SortedTrieIndex` for two variables.
- Prefix-constrained durable source restricts to matching rows.
- `seek` lands on least upper bound.
- Duplicate-variable atom is rejected or handled correctly.
- Ineligible index order falls back to sorted trie build.

### Integration Tests

- Add a query where an existing durable index order exactly matches LFTJ variable order.
- Add a query where order does not match and verify fallback.

Run:

```sh
cargo test -p bumbledb-lmdb sorted_trie
cargo test -p bumbledb-lmdb query
cargo test --workspace --all-features
```

## Benchmark Gates

Run q09/q24:

```sh
cargo run -p bumbledb-bench --release --features alloc-profile -- \
  --preset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --query job_q09_voice_us_actor \
  --query job_q24_voice_keyword_actor
```

Expected:

- Fewer `sorted_trie_builds` for eligible atoms.
- Lower first-run `lftj_build_us`.
- Lower `lftj_atom_scan_micros` and `lftj_atom_sort_micros`.
- Sample time unchanged or improved.

## Risks

- Trie semantics over durable bytes are subtle. Incorrect grouping breaks join results.
- Existing `SortedTrieIter` returns row ranges over temporary relation row IDs. Durable index source may not have equivalent row IDs. LFTJ mostly needs keys/counts, but any row-dependent code must be audited.
- Local predicates and duplicate variables must be enforced before treating durable source as equivalent.
- Lifetimes can get complex if source borrows `RelationIndexImage`. Prefer enum dispatch with borrows tied to `QueryImage` lifetime.

## Definition Of Done

- LFTJ can use durable sorted index bytes directly for eligible atoms.
- Correctness is proven against `SortedTrieIndex` fixture comparisons.
- q09/q24 cold build work drops when eligible durable indexes exist.
- Fallback path remains correct for non-eligible atoms.
