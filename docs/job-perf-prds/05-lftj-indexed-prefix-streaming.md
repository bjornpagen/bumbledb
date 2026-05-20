# PRD 05: LFTJ Indexed-Prefix Streaming

## Status

Proposed.

## Motivation

PRD 04 removes the worst full-scan `Vec<Vec<u8>>` representation from LFTJ atom builds. However, the indexed-prefix path has its own nested intermediate:

```rust
struct IndexedLftjAtomValues {
    rows: Vec<Vec<Vec<u8>>>,
    source_rows_scanned: u64,
}
```

That shape means an index prefix scan still materializes every accepted row as nested heap byte vectors before appending into columns.

The q09 trace shows indexed prefix spans are not theoretical:

- `bumbledb.query.lftj_atom.indexed_prefix[relation=Name]`: 123,000 us in prepare.
- `bumbledb.query.lftj_atom.indexed_prefix[relation=CompanyName]`: 10,300 us in prepare.

The q24 trace has indexed-prefix activity too, although smaller:

- `bumbledb.query.lftj_atom.indexed_prefix[relation=Keyword]`: 1.42 us.

This PRD changes indexed-prefix LFTJ atom construction to stream index entries directly into typed builders.

## Evidence

| Evidence | Anchor |
|---|---|
| Indexed-prefix helper returns nested row vectors | `crates/bumbledb-lmdb/src/query.rs:5599-5678` |
| It calls `atom_index_entry_values`, which allocates `BTreeMap<usize, Vec<u8>>` and clones values | `crates/bumbledb-lmdb/src/query.rs:5681-5729` |
| Prefix iteration over durable index bytes already streams borrowed entry slices | `crates/bumbledb-lmdb/src/query_image.rs:554-610` |
| q09 indexed prefix is a measurable part of cold build | `docs/job-trace-analysis/04-job_q09_voice_us_actor.md:91-100` |
| q24 build is dominated by scan/filter/copy, including indexed and fallback scan paths | `docs/job-trace-analysis/06-job_q24_voice_keyword_actor.md:79-95` |

## Goals

- Delete `IndexedLftjAtomValues` and all nested `Vec<Vec<Vec<u8>>>` indexed-prefix intermediates.
- Stream each matching index entry through dense extraction and local predicate checks.
- Append accepted values directly into `EncodedColumnBuilder`s.
- Preserve best-prefix index selection behavior.
- Preserve `source_rows_scanned`, `rows_retained`, `bytes_copied`, and trace counters.
- Prepare the code for PRD 13, where some indexed-prefix scans can become direct trie sources instead of temporary trie builds.

## Non-Goals

- Do not implement PRD 13 direct durable-index trie source here.
- Do not change variable ordering or planner decisions.
- Do not change sorted trie layout.
- Do not add prefix-count API here unless PRD 06 is done first; streaming can work without exact prefix counts.

## Current Code Shape

The current path is:

```text
indexed_lftj_atom_values
  -> choose best RelationIndexImage prefix
  -> iterate entries_with_prefix(prefix)
  -> atom_index_entry_values(entry) -> Option<Vec<Vec<u8>>>
  -> atom_local_comparisons_pass(values)
  -> rows.push(values)

build_lftj_sorted_trie
  -> for values in indexed.rows
  -> raw_columns[column].push(bytes)
```

Target path:

```text
append_indexed_lftj_atom_values
  -> choose best RelationIndexImage prefix
  -> iterate entries_with_prefix(prefix)
  -> fill dense slots from borrowed entry bytes
  -> check local predicates from slots
  -> append slots into builders
```

## Required API

Replace:

```rust
fn indexed_lftj_atom_values(...) -> Result<Option<IndexedLftjAtomValues>>
```

with:

```rust
struct IndexedPrefixAppendStats {
    source_rows_scanned: u64,
    rows_retained: u64,
    bytes_appended: u64,
}

fn append_indexed_lftj_atom_values(
    builders: &mut [EncodedColumnBuilder],
    source: &RelationImage,
    query: &NormalizedQuery,
    inputs: &EncodedInputs,
    atom: &NormAtom,
    variables: &[usize],
) -> Result<Option<IndexedPrefixAppendStats>>;
```

Return `Ok(None)` when no usable indexed prefix exists. Return `Ok(Some(stats))` when a prefix was used, even if zero rows were retained.

## Implementation Details

### Best Prefix Selection

Preserve current selection logic from `query.rs:5611-5650`:

- Consider only indexes that contain every atom field.
- Build a leading prefix from literal/input fields in index leading-field order.
- Skip indexes where prefix length is zero.
- Choose the index with the largest `prefix_fields`.

This is not the final optimizer, but preserving it keeps this PRD isolated.

### Dense Extraction

Use the dense extraction helper from PRD 04. It must support `RelationIndexImage` entries:

- `index.component_bytes(entry, field.field)` returns borrowed bytes.
- Variable terms fill slots.
- Duplicate variables compare bytes.
- Input/literal terms compare bytes.
- Wildcards are ignored.

Do not allocate `BTreeMap`.
Do not allocate `Vec<u8>` for each field.
Do not allocate `Vec<Vec<u8>>` for each row.

### Local Predicate Checks

Evaluate local comparisons before appending into builders.

If the extraction helper currently appends directly, refactor it into two phases:

- Fill stack/dense slots.
- Check local comparisons.
- Append slots.

This avoids partial appended rows when a later predicate rejects the row.

### Stats

Map old stats to new stats:

- `source_rows_scanned`: increment for each index entry visited.
- `rows_retained`: increment for each row appended.
- `bytes_appended`: sum logical encoded value widths appended to builders.

Keep existing counters in `build_lftj_atom_plan`:

- `counters.lftj_atom_source_rows_scanned`
- `counters.lftj_atom_rows_retained`
- `counters.lftj_atom_bytes_copied`
- `counters.atom_temp_relation_source_rows`
- `counters.atom_temp_relation_rows`

The name `bytes_copied` can remain for now, but semantically it becomes logical bytes appended into builders.

### Trace Span

Keep the span:

```rust
tracing::trace_span!(
    "bumbledb.query.lftj_atom.indexed_prefix",
    relation = %source.name,
    prefix_bytes = prefix.len()
)
```

Add optional fields if useful:

- `access = index.access.0`
- `prefix_fields`

Avoid per-row trace events.

## Deletions

After this PRD:

- Delete `IndexedLftjAtomValues`.
- Delete `indexed.rows` materialization.
- Delete production `atom_index_entry_values` returning `Vec<Vec<u8>>`.
- Delete any indexed-prefix `rows.push(values)` path.

## Acceptance Criteria

- Indexed-prefix LFTJ path streams entries directly into builders.
- `grep` for `Vec<Vec<Vec<u8>>>` in `query.rs` finds no production code.
- `grep` for `IndexedLftjAtomValues` finds nothing.
- q09/q24 output rows and counts are unchanged.
- q09 indexed-prefix spans still exist but allocate far less.

## Tests

### Unit Tests

Add tests that force indexed-prefix path:

- Atom with literal leading field uses an index prefix and returns expected trie keys.
- Atom with input leading field uses an index prefix.
- Atom with duplicate variable fields rejects mismatches.
- Local comparison predicate rejects rows before append.
- No usable prefix falls back to full-scan builder path.
- Prefix path with zero matches returns an empty atom trie, not fallback full scan.

### Differential Tests

Run existing SQLite comparison tests and add a query that uses literal prefix plus join variables.

Commands:

```sh
cargo test -p bumbledb-lmdb query
cargo test -p bumbledb-test-support sqlite_comparison
cargo test --workspace --all-features
```

## Benchmark Plan

Use the same q09/q24 run from PRD 04. Compare after PRD 05 specifically:

- q09 indexed-prefix spans should have lower allocation impact.
- q09 `scan_filter_copy` should drop from 506,779.5 us if indexed prefix intermediate was material.
- q24 `scan_filter_copy` should drop from 25,735.0 us.

Run:

```sh
RUST_LOG="bumbledb_lmdb=debug" \
cargo run -p bumbledb-bench --release --features alloc-profile -- \
  --preset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --query job_q09_voice_us_actor \
  --query job_q24_voice_keyword_actor \
  --trace --trace-format json \
  --trace-output /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/job-indexed-prefix-streaming-trace.jsonl
```

## Risks

- If rows are appended before all checks pass, columns become different lengths. Enforce staged slots first.
- If the chosen index does not include all variable fields, component extraction may silently miss variables. Preserve current `all(|field| index.contains_field(field.field))` condition.
- Prefix length must align to whole encoded fields. Current prefix construction uses complete input/literal encoded values; keep that invariant.
- Empty prefix should not select an index. Preserve `prefix_fields == 0` skip.

## Future Follow-Up

PRD 13 will inspect whether the chosen durable index order can be exposed directly as a trie source. This PRD should leave the code structured so an indexed-prefix scan can choose between:

- Append to temporary builders.
- Return a direct durable-index trie adapter.

Do not implement the adapter here.

## Definition Of Done

- Indexed-prefix path has no nested heap row materialization.
- Full-scan and indexed-prefix LFTJ builders share the same dense slot extraction logic.
- q09/q24 correctness and allocation gates pass.
