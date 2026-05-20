# PRD 04: LFTJ Atom Column Builders

## Status

Proposed.

## Motivation

The second-largest allocation cliff is cold LFTJ atom build. The current implementation scans or prefix-scans relation images, copies retained atom values into nested heap vectors, converts those vectors into `ColumnImage`, then builds a sorted trie from the temporary relation.

The trace shows this is catastrophic:

| Query | Cold `lftj_build` time | `lftj_build` alloc calls | `lftj_build` bytes allocated |
|---|---:|---:|---:|
| `job_q09_voice_us_actor` | 682,770 us | 16,579,615 | 2,649,330,360 |
| `job_q24_voice_keyword_actor` | 33,592 us | 920,431 | 107,683,203 |

The main subphase is `scan_filter_copy`, not trie sorting:

| Query | `scan_filter_copy` | % of prepare query.execute |
|---|---:|---:|
| q09 | 506,779.5 us | 51.554% |
| q24 | 25,735.0 us | 69.180% |

This PRD replaces heap-owned temporary atom rows with width-specialized column builders from PRD 02.

## Evidence

| Evidence | Location |
|---|---|
| LFTJ build starts before LFTJ execution and owns first-run allocations | `crates/bumbledb-lmdb/src/query.rs:4864-4956` |
| Atom plans are cached after build | `crates/bumbledb-lmdb/src/query.rs:5430-5486`, `crates/bumbledb-lmdb/src/query_image.rs:218-307` |
| `build_lftj_sorted_trie` creates `raw_columns: Vec<Vec<Vec<u8>>>` | `crates/bumbledb-lmdb/src/query.rs:5488-5505` |
| Scan path pushes owned `Vec<u8>` values into raw columns | `crates/bumbledb-lmdb/src/query.rs:5512-5538` |
| Column conversion calls `ColumnImage::from_query_image_bytes` | `crates/bumbledb-lmdb/src/query.rs:5549-5559` |
| Indexed-prefix helper returns `Vec<Vec<Vec<u8>>>` | `crates/bumbledb-lmdb/src/query.rs:5599-5678` |
| Row extraction allocates `BTreeMap<usize, Vec<u8>>` | `crates/bumbledb-lmdb/src/query.rs:5681-5729`, `5835-5884` |

## Goals

- Replace LFTJ atom temporary columns with `EncodedColumnBuilder`.
- Append retained atom variable values directly into typed fixed-width vectors.
- Remove `Vec<Vec<u8>>` and `Vec<Vec<Vec<u8>>>` from LFTJ build production paths.
- Replace per-row `BTreeMap<usize, Vec<u8>>` extraction with dense variable slot extraction.
- Keep current sorted-trie build semantics initially.
- Preserve atom cache correctness and duplicate-variable semantics.

## Non-Goals

- Do not directly use durable indexes as trie sources yet. That is PRD 13.
- Do not optimize steady-state LFTJ traversal yet. That is PRD 14.
- Do not rewrite the planner in this PRD.
- Do not change query result semantics.

## Current Build Pipeline

Current cold LFTJ atom build:

```text
RelationImage
  -> atom_row_values or atom_index_entry_values
  -> Option<Vec<Vec<u8>>> per retained row
  -> raw_columns: Vec<Vec<Vec<u8>>>
  -> ColumnImage::from_query_image_bytes
  -> temporary RelationImage
  -> SortedTrieIndex::build
  -> cached Arc<SortedTrieIndex>
```

Target pipeline:

```text
RelationImage or RelationIndexImage
  -> dense fixed-width extracted slots
  -> EncodedColumnBuilder per atom variable
  -> temporary RelationImage
  -> SortedTrieIndex::build
  -> cached Arc<SortedTrieIndex>
```

PRD 13 may later skip the temporary relation/trie for some atoms, but this PRD keeps that boundary and removes the worst heap churn.

## Required Data Structures

### Dense Atom Value Slots

Introduce a small local representation near LFTJ build helpers:

```rust
type AtomValueSlots = smallvec::SmallVec<[Option<EncodedOwned>; 8]>;
```

or, if const initialization is awkward, a small helper struct:

```rust
struct AtomValueSlots {
    values: SmallVec<[Option<EncodedOwned>; 8]>,
}
```

Required behavior:

- Slot index is dense query variable id or dense position in `variables`.
- Duplicate variable fields must compare equal; mismatches reject the row.
- Literals and inputs must compare without allocation.
- Wildcards do nothing.
- Final append order must match `variables` order passed to `build_lftj_sorted_trie`.

### Builder Set

For an atom with variables `[v1, v2, ...]`, create one `EncodedColumnBuilder` per variable:

```rust
let fields = variables_to_field_images(query, variables);
let mut builders = encoded_column_builders(&fields, estimated_capacity)?;
```

Append with:

```rust
builders[column].append_encoded_owned(&value)?;
```

If PRD 02 did not add `append_encoded_owned`, add it here:

```rust
pub(crate) fn append_encoded_owned(&mut self, value: &EncodedOwned) -> Result<()>;
```

## Implementation Plan

### Step 1: Create Dense Extraction Functions

Replace these functions:

- `atom_index_entry_values` at `query.rs:5681-5729`.
- `atom_row_values` at `query.rs:5835-5884`.

with functions that append into builders or fill slots without allocating byte vectors.

Suggested signatures:

```rust
fn append_atom_index_entry_values(
    builders: &mut [EncodedColumnBuilder],
    index: &RelationIndexImage,
    inputs: &EncodedInputs,
    atom: &NormAtom,
    entry: &[u8],
    variables: &[usize],
) -> Result<bool>;

fn append_atom_row_values(
    builders: &mut [EncodedColumnBuilder],
    relation: &RelationImage,
    inputs: &EncodedInputs,
    atom: &NormAtom,
    row: RowId,
    variables: &[usize],
) -> Result<bool>;
```

Returning `true` means a retained row was appended. Returning `false` means duplicate-variable, literal, or input mismatch.

### Step 2: Preserve Local Predicate Filtering

`atom_local_comparisons_pass` currently accepts `values: &[Vec<u8>]` at `query.rs:5732-5772`.

Replace with either:

```rust
fn atom_local_comparisons_pass_slots(
    query: &NormalizedQuery,
    inputs: &EncodedInputs,
    variables: &[usize],
    values: &[EncodedOwned],
) -> Result<bool>
```

or pass a helper that resolves variable position to `&[u8]`.

Do not allocate `Vec<&[u8]>` per predicate if avoidable. A small fixed array for the two operands is enough because current comparisons are binary.

### Step 3: Rewrite Full-Scan Build

In `build_lftj_sorted_trie`:

- Replace `raw_columns` with builders.
- For each source row, fill local slots or append after predicate pass.
- Increment `included_rows` only after all builders append successfully.
- Count `bytes_copied` as logical encoded bytes appended, not heap allocation bytes.

### Step 4: Temporarily Rewrite Indexed-Prefix Path To Use Builders

PRD 05 will make indexed-prefix streaming cleaner. This PRD can either:

- Skip `indexed_lftj_atom_values` and run a streaming prefix loop inline when an indexed prefix exists.
- Or change `indexed_lftj_atom_values` to accept a builder callback.

Do not keep `IndexedLftjAtomValues { rows: Vec<Vec<Vec<u8>>> }` after this PRD.

### Step 5: Build Temporary Relation From Finished Builders

Keep the existing temporary `RelationImage` and `SortedTrieIndex::build` shape:

```rust
let columns = builders.into_iter().map(EncodedColumnBuilder::finish).collect();
let relation = RelationImage { columns, ... };
let trie = build_sorted_trie_index(&relation, IndexSpec::new(...))?;
```

This isolates the change to allocation representation without changing LFTJ iterator semantics.

### Step 6: Delete Old Helpers

Remove production uses of:

- `IndexedLftjAtomValues`.
- `atom_index_entry_values` returning `Vec<Vec<u8>>`.
- `atom_row_values` returning `Vec<Vec<u8>>`.
- `raw_columns: Vec<Vec<Vec<u8>>>`.

## Duplicate Variable Semantics

For atoms like:

```text
Relation(a: ?x, b: ?x)
```

The row is retained only if encoded bytes for `a` and `b` are equal.

Current code enforces this through `BTreeMap<usize, Vec<u8>>` in `atom_row_values` and `atom_index_entry_values`. The new dense slot implementation must preserve it exactly.

Test cases must include duplicate variables for row image and index image paths.

## Capacity Strategy

For full scans, capacity can start as zero or estimated from `source.row_count`. Starting with `source.row_count` for every builder can over-allocate on selective atoms. Starting at zero can reallocate. Prefer:

- If atom has no literal/input/local predicate filters, reserve `source.row_count`.
- If indexed prefix path is used, use `prefix_count` from PRD 06 if available; otherwise reserve zero.
- If filters exist but no index prefix, reserve `source.row_count.min(4096)` or zero.

Do not let capacity strategy block the main allocation win. Per-cell heap removal matters more.

## Acceptance Criteria

- No production `Vec<Vec<Vec<u8>>>` in `query.rs` LFTJ build path.
- No production `BTreeMap::<usize, Vec<u8>>` in LFTJ atom extraction.
- No production `bytes.to_vec()` for every retained LFTJ atom value.
- q09 and q24 outputs remain unchanged.
- q09 and q24 `lftj_build` allocation calls drop at least 80%.

## Tests

### Unit Tests

Add LFTJ atom build tests under `query.rs` tests:

- Full-scan atom build retains same rows as old path for simple atom.
- Literal filter rejects nonmatching rows without allocation-heavy temp values.
- Input filter works.
- Duplicate variable equality works.
- Local comparison predicates work.
- Indexed-prefix path and full-scan path produce identical `SortedTrieIndex` row counts and keys.
- Width 1, 8, and 16 fields all append correctly.

### Differential Tests

Use existing SQLite comparison tests and add targeted queries if needed.

Run:

```sh
cargo test -p bumbledb-lmdb query
cargo test -p bumbledb-test-support sqlite_comparison
cargo test --workspace --all-features
```

## Benchmark Plan

Run q09 and q24 traced allocation:

```sh
RUST_LOG="bumbledb_lmdb=debug" \
cargo run -p bumbledb-bench --release --features alloc-profile -- \
  --preset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --query job_q09_voice_us_actor \
  --query job_q24_voice_keyword_actor \
  --trace --trace-format json \
  --trace-output /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/job-lftj-builders-trace.jsonl
```

Gates:

- q09 `allocations.phases.lftj_build.alloc_calls` below 3.32M.
- q09 `allocations.phases.lftj_build.bytes_allocated` materially below 2.649 GB.
- q24 `allocations.phases.lftj_build.alloc_calls` below 184k.
- q24 `allocations.phases.lftj_build.bytes_allocated` materially below 107.7 MB.
- q09/q24 sample averages no worse than 5%.
- q09/q24 first-run build time materially lower.

## Risks

- Appending directly into builders before local comparisons pass can leave partial rows in columns. Always validate first, then append all columns, or use a reversible staged slot.
- Dense variable slot indexing must not assume variable IDs are contiguous inside one atom unless using full query variable IDs with vector length `query.vars.len()`.
- If `EncodedOwned` is cloned too often in slots, allocation is still gone but copy cost remains. That is acceptable for this PRD; PRD 14 handles traversal copies.
- Reusing builder helpers from `query_image.rs` may require visibility changes. Keep it `pub(crate)`, not public API.

## Definition Of Done

- LFTJ atom build uses typed column builders.
- Old nested byte-vector LFTJ temporary representation is gone.
- Duplicate variable, literal, input, and predicate semantics are tested.
- q09/q24 allocation gates pass.
