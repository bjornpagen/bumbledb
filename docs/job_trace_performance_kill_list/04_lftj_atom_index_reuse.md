# 04 LFTJ Atom Index Reuse And Lazy Construction

Priority: P0

Primary affected queries:

- `job_movie_link_bridge`: cold LFTJ build `64.1ms`, `98.4%` of prepare.
- `job_q01_top_production`: cold LFTJ build `4.33ms`, `91.8%` of prepare despite empty result.
- `job_broad_movie_info_star`: cold LFTJ build `32.1ms`, `40.7%` of prepare.
- `job_broad_cast_keyword_company`: LFTJ build `52.6ms`, `19.3%` of prepare.
- `job_q09_voice_us_actor`: LFTJ build `72.7ms`, `30.4%` of prepare in mixed fallback.

## Problem

The LFTJ runtime currently constructs atom-specific sorted tries by scanning relation images, copying encoded bytes into temporary relation images, then sorting/building new tries. This happens before execution starts, for every atom in the query.

This cold work dominates several JOB queries. Some of the built atom tries are unnecessary because execution later discovers an empty branch. Other atom tries duplicate existing access paths that the schema and query image already know how to represent.

## Trace Evidence

| Query | Cold LFTJ Build | Prepare Share | Runtime Work |
|---|---:|---:|---|
| `job_movie_link_bridge` | `64.1ms` | `98.4%` | only `147` candidates, `36` bindings |
| `job_q01_top_production` | `4.33ms` | `91.8%` | only `1` candidate, `0` bindings |
| `job_broad_movie_info_star` | `32.1ms` | `40.7%` | `47,034` bindings |
| `job_broad_cast_keyword_company` | `52.6ms` | `19.3%` | `11,009` bindings |

For `job_movie_link_bridge`, `sorted_trie.build` itself accounts for only `9.47ms` of the `64.1ms` LFTJ build. The majority is temporary atom relation construction: row scans, literal checks, byte cloning, and column construction.

For `job_q01_top_production`, the query is empty because `InfoType(info="top 250 rank")` has no matching loaded branch at this scale. The engine still builds all five atom tries before execution discovers that.

## Current Technical Cause

`execute_lftj` builds every atom plan before it executes:

`crates/bumbledb-lmdb/src/query.rs:2443-2524`

```rust
let atom_plans = {
    let _span = tracing::debug_span!(
        "bumbledb.query.lftj.build",
        atoms = plan.relation_atoms.len()
    )
    .entered();
    build_lftj_atom_plans(
        image,
        query,
        inputs,
        &plan.relation_atoms,
        &plan.variable_order_ids,
        &mut plan.summary.counters,
    )?
};
```

`build_lftj_atom_plans` maps every atom unconditionally:

`crates/bumbledb-lmdb/src/query.rs:2775-2787`

```rust
atoms
    .iter()
    .map(|atom| build_lftj_atom_plan(image, query, inputs, atom, variable_order_ids, counters))
    .collect()
```

Each atom plan scans the full source relation and copies encoded bytes:

`crates/bumbledb-lmdb/src/query.rs:2830-2895`

```rust
let mut raw_columns = vec![Vec::<Vec<u8>>::new(); variables.len()];

for row in 0..source.row_count {
    let row = RowId(row as u32);
    let Some(values) = atom_row_values(source, query, inputs, atom, row, variables)? else {
        continue;
    };
    included_rows += 1;
    for (column, bytes) in values.into_iter().enumerate() {
        raw_columns[column].push(bytes);
    }
}

let columns = fields
    .iter()
    .zip(raw_columns)
    .map(|(field, raw_column)| {
        crate::ColumnImage::from_query_image_bytes(field.id, field.width, raw_column)
    })
    .collect::<Result<Vec<_>>>()?;
```

`atom_row_values` clones variable bytes per row:

`crates/bumbledb-lmdb/src/query.rs:2949-2999`

```rust
if let Some(existing) = values_by_variable.get(&variable) {
    if existing.as_slice() != bytes {
        return Ok(None);
    }
} else {
    values_by_variable.insert(variable, bytes.to_vec());
}
...
variables
    .iter()
    .map(|variable| {
        values_by_variable
            .get(variable)
            .cloned()
            .ok_or_else(|| Error::internal("missing LFTJ variable value"))
    })
```

The temp relation then gets a sorted trie built by sorting all retained rows:

`crates/bumbledb-lmdb/src/sorted_trie.rs:81-118`

```rust
let mut order = (0..relation.row_count)
    .map(|row| RowId(row as u32))
    .collect::<Vec<_>>();

order.sort_by(|left, right| {
    for field in &spec.fields {
        let left = relation.encoded_bytes(*left, *field).unwrap_or(&[]);
        let right = relation.encoded_bytes(*right, *field).unwrap_or(&[]);
        match left.cmp(right) {
            std::cmp::Ordering::Equal => continue,
            ordering => return ordering,
        }
    }
    left.cmp(right)
});

let levels = build_levels(relation, &order, &spec.fields)?;
```

The cache key includes atom id and variable order, which prevents sharing physically equivalent atom indexes across aliases:

`crates/bumbledb-lmdb/src/query.rs:2898-2932`

```rust
let _ = write!(
    key,
    "relation={};atom={};vars={:?};order={:?};fields=",
    atom.relation.0, atom.id.0, variables, variable_order_ids
);
```

## Desired End State

LFTJ should not build a temporary relation/trie when one of these is true:

- The existing relation image or durable segment index already has the needed order.
- A static literal atom is empty and can short-circuit before downstream build work.
- The atom is pure existence and implied by foreign-key constraints.
- The same physical relation/field/literal/index shape was already built for another alias.
- The atom is not needed because an earlier variable node dies.

## Proposed Technical Solution

This item has four parts.

### Part 1: LFTJ Atom Plan Metadata First, Index Later

Replace `Vec<LftjAtomPlan>` with a provider similar to the lazy HashProbe provider:

```rust
struct LftjAtomProvider<'image> {
    image: &'image QueryImage,
    query: &'image NormalizedQuery,
    inputs: &'image EncodedInputs,
    requests: Vec<LftjAtomRequest>,
    plans: Vec<Option<Arc<SortedTrieIndex>>>,
}

struct LftjAtomRequest {
    atom_id: usize,
    relation: RelationId,
    variables: Vec<usize>,
    physical_fields: Vec<FieldId>,
    literal_filters: Vec<AtomFilter>,
    cache_key: LftjCacheKey,
}
```

Only build an atom trie when a variable depth needs that atom as a participant.

This allows `job_q01_top_production` to detect the empty `InfoType` participant before building `Title` and other downstream tries.

### Part 2: Use Existing Indexes As LFTJ Views

If an atom has no literal/input filters and the variable order matches an existing relation access path, use a `SortedTrieIndex` view over the existing relation image/index instead of building a temp relation.

Current query image already stores relation images and durable segment index metadata. The spec should introduce:

```rust
enum LftjAtomIndex {
    ExistingRelationIndex(Arc<SortedTrieIndex>),
    TempFilteredIndex(Arc<SortedTrieIndex>),
}
```

Start with reusing cached sorted tries built directly on full relation images with matching `FieldId` order. Later, use durable segment indexes to avoid rebuilding even the full-relation sorted trie.

### Part 3: Canonicalize Cache Keys

Do not include `atom.id` when the physical content is identical. A canonical key should include:

- Relation ID.
- Ordered output fields/variables as physical field IDs and logical types.
- Literal/input filters by field ID and encoded bytes.
- Repeated-variable equality constraints.
- Whether the index is full relation or filtered temp.

This allows aliases like `Title(id: ?movie1)` and `Title(id: ?movie2)` to share a physical one-field title-id trie where valid.

### Part 4: Empty Atom And Static Literal Prechecks

Before building all LFTJ atom plans, evaluate literal-only or literal-leading atoms with existing indexes. If any required atom is empty, return early.

The existing empty shortcut is too narrow:

`crates/bumbledb-lmdb/src/query.rs:2486-2490`

```rust
if atom_plans
    .iter()
    .any(|atom| atom.variables.is_empty() && atom.row_count == 0)
{
    return Ok(());
}
```

It only handles zero-row atoms with no variables. A variable-bearing atom such as `InfoType(id: ?info_type, info: "top 250 rank")` can be empty and should also terminate the query.

## Implementation Plan

1. Introduce metadata-only `LftjAtomRequest` generation.
2. Add an `LftjAtomProvider` with `get_or_build(atom_id)`.
3. Refactor `LftjRuntime` to hold provider-backed iterators or lazily open iterators.
4. Add precheck for literal-bound relation atoms using existing access paths.
5. Add canonical LFTJ cache keys.
6. Add existing full-relation sorted trie reuse for atoms without filters.
7. Later: replace full-relation trie rebuilds with durable segment index views.

## Tests

- Empty variable-bearing literal atom returns immediately and avoids building unrelated atom tries.
- `job_q01_top_production` fixture records fewer than five LFTJ atom temp builds when `InfoType` is empty.
- Two alias atoms over the same relation and physical fields share a cache key when safe.
- Filtered atom cache keys remain distinct when literal values differ.
- Query outputs match current LFTJ outputs.

## Acceptance Criteria

- `job_movie_link_bridge` cold `lftj_build_us` drops from `64.1ms` by at least `70%`.
- `job_q01_top_production` cold `lftj_build_us` drops from `4.33ms` by at least `80%`.
- `atom_temp_relation_source_rows` drops substantially for empty/selective LFTJ queries.
- Steady-state performance is not worse.

## Risks

- Lazy iterator/provider integration may complicate LFTJ recursion and lifetimes.
- Canonical cache keys must preserve repeated-variable equality and literal filters exactly.
- Existing relation indexes may not have the same row multiplicity semantics as filtered temp atom relations when repeated variables are involved.

## Rollout Plan

1. Add variable-bearing empty shortcut after current eager build to prove correctness.
2. Add literal prechecks before eager build.
3. Add canonical cache keys.
4. Add lazy provider.
5. Add full-relation index reuse.
6. Add durable segment index views.
