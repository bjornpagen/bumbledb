# GHT/COLT Storage Layout Audit - Investigator 2

## Sources Read

- `docs/ROSETTA_STONE.md`, especially product thesis, compatibility policy, relation semantics, storage model, query execution, public output, benchmark, and validation contracts.
- `docs/free-join-paper/arXiv-2301.10841v2/tex/03-free-join.tex`, especially the GHT definition and interface at lines 55-147, Free Join plan validity and covers at lines 177-319, the build phase at lines 383-438, and the join phase at lines 452-493.
- `docs/free-join-paper/arXiv-2301.10841v2/tex/04-optimizations.tex`, especially `binary2fj` and factoring at lines 31-149, COLT at lines 163-283, vectorized execution at lines 371-417, and dynamic cover choice at lines 435-451.
- `crates/bumbledb-lmdb/src/storage/keys.rs`, `storage/write.rs`, `storage/read.rs`, `storage/cursor.rs`, `storage/types.rs`, and `storage/encoding.rs`.
- `crates/bumbledb-lmdb/src/storage_schema.rs` and `crates/bumbledb-core/src/schema/descriptors.rs`, `schema/layout.rs`, `schema/canonical.rs`, and `schema/validation.rs`.
- `crates/bumbledb-lmdb/src/query_image/builder.rs`, `query_image/types.rs`, `query_image/access.rs`, `query_image/columns.rs`, `query_image/scope.rs`, and `query_image/cache.rs`.
- `crates/bumbledb-lmdb/src/sorted_trie.rs`, `query/lftj_access.rs`, `query/lftj_iter.rs`, `query/lftj_runtime.rs`, `query/lftj_prefix.rs`, and `query/lftj_leapfrog.rs`.
- `crates/bumbledb-lmdb/src/free_join.rs`, `query/planner.rs`, `query/planner_scoring.rs`, `query/hash.rs`, `query/model.rs`, `query/api.rs`, `query/sinks.rs`, and `planner_stats.rs`.
- Relevant tests in `crates/bumbledb-lmdb/src/query_image_tests.rs`, `storage_tests/access.rs`, `query_tests/basic.rs`, `query_tests/cache_and_planner.rs`, `query_tests/atom_cache.rs`, and `query_tests/sinks_and_projection.rs`.
- `docs/free-join-paper/audits/01-formal-free-join-plan-audit.md` for prior plan-level findings.

## Executive Summary

Bumbledb currently satisfies important Rosetta constraints: it is embedded LMDB-only storage, base relations have set semantics, query output is duplicate-free, query images are snapshot-local internals, and there is no SQL or bag-semantic public contract.

It does not currently implement the paper's GHT or COLT storage/execution model. The current engine persists canonical row-like fact bytes plus predeclared sorted access-key entries, eagerly builds snapshot `RelationImage` columns and selected sorted index byte arrays, then executes a singleton-variable LFTJ/GJ-style runtime over those sorted bytes.

The gap is structural, not cosmetic. A paper-compliant GHT/COLT engine needs plan-derived tuple-key trie schemas, `iter()`/`get(tuple)` access, lazy `force()` materialization from columnar base offsets, vector leaves of relation offsets, cover-based Free Join execution, dynamic cover choice, and a scan-backed path that does not require predeclared access permutations.

Preparing a real paper-compliant engine requires breaking storage and API changes. The clean path is a new storage format with canonical set membership plus durable columnar base data, an execution-local COLT forest over immutable LMDB snapshot columns, and a formal Free Join plan whose nodes contain subatoms and cover candidates. Rosetta permits this break: storage mismatches are hard failures and ETL into a new database is the migration path.

## Paper Requirements

GHT requirements from `tex/03-free-join.tex`:

- A GHT is a tree whose internal nodes are hash maps from tuple keys to child nodes and whose leaves are vectors of tuples (`tex/03-free-join.tex:78-82`).
- Each GHT carries `relation` and `vars` metadata, and exposes `iter() -> Iterator<Tuple>` plus `get(key: Tuple) -> Option<GHT>` (`tex/03-free-join.tex:57-67`, `tex/03-free-join.tex:103-147`).
- `vars` means the key tuple schema for map nodes or tuple schema for vector leaves (`tex/03-free-join.tex:107-118`).
- A Free Join plan partitions every relation atom into subatoms; each node must have a cover containing all newly introduced variables (`tex/03-free-join.tex:197-220`, `tex/03-free-join.tex:293-305`).
- The GHT schema for each atom is computed from that atom's subatom sequence in the plan, plus a final `[]` vector leaf unless the last subatom is a cover (`tex/03-free-join.tex:426-438`).
- The join phase iterates a cover tuple, builds tuple keys for sibling subatoms from current and prior bindings, calls `get(key)`, replaces tries with returned subtries, and recurses (`tex/03-free-join.tex:452-493`).

COLT requirements from `tex/04-optimizations.tex`:

- Raw relation data is stored column-wise in main memory (`tex/04-optimizations.tex:175-178`).
- A COLT leaf is a vector of offsets into the base relation; an internal node is a hash map from tuple to child COLT (`tex/04-optimizations.tex:180-184`).
- COLT starts as one vector `[0, ..., relation.len - 1]` and implements the GHT interface lazily (`tex/04-optimizations.tex:207-243`, `tex/04-optimizations.tex:256-283`).
- `get(key)` forces the current vector into a hash map if needed, then probes the map (`tex/04-optimizations.tex:227-242`, `tex/04-optimizations.tex:267-276`).
- `iter()` over a vector can stream tuple values from base columns only when the current `vars` form a suffix; otherwise it forces the vector into a map (`tex/04-optimizations.tex:218-226`, `tex/04-optimizations.tex:277-282`).
- COLT can avoid building any auxiliary structure for a relation that is only directly iterated, recovering left-deep hash-join behavior (`tex/04-optimizations.tex:284-292`).
- The tree may be unbalanced and may have maps and vectors at the same level (`tex/04-optimizations.tex:195-198`).
- Dynamic cover choice should choose among cover subatoms at runtime using the smallest exact key count or vector length estimate (`tex/04-optimizations.tex:435-451`).
- Vectorized Free Join replaces scalar cover iteration with `iter_batch(batch_size)` and batched sibling probes (`tex/04-optimizations.tex:371-417`).

Rosetta constraints to preserve:

- LMDB remains the only storage backend (`docs/ROSETTA_STONE.md:116-132`).
- Every relation is a set of full facts; duplicate insert and absent delete are idempotent no-ops (`docs/ROSETTA_STONE.md:36-47`).
- Query images are snapshot-local internals, not public API (`docs/ROSETTA_STONE.md:154-159`).
- Projection output has set semantics and `QueryResultSet` must be duplicate-free and canonicalized (`docs/ROSETTA_STONE.md:146-171`).
- No SQL, no server, no bag semantics, no runtime DDL, and malformed IR must be rejected at execution boundaries (`docs/ROSETTA_STONE.md:13-35`).
- Durable historical relation snapshots are not part of the write path; LMDB MVCC read transactions provide current-state snapshots (`docs/ROSETTA_STONE.md:130-132`).

## Current Storage/Image Reality

Current durable LMDB shape:

- `Environment` opens exactly three LMDB databases: `_meta`, `_index`, and `_dict` (`environment.rs:17-21`, `environment.rs:314-321`).
- Storage format is version `4` (`lib.rs:45-46`).
- `_index` stores canonical facts, fact-id lookups, current access entries, unique guards, and reverse FK guards in separate byte namespaces (`storage/keys.rs:3-14`).
- Canonical membership is `NS_CANONICAL_FACT | relation_id | fact_bytes -> empty` (`storage/keys.rs:88-98`).
- Fact identity is a 16-byte BLAKE3-derived hash of relation ID plus encoded fact bytes (`storage/keys.rs:100-107`).
- Fact-id lookup is `NS_FACT_ID | relation_id | fact_id -> fact_bytes` (`storage/keys.rs:110-120`, `storage/write.rs:86-99`).
- Access entries are `NS_ACCESS_ENTRY | relation_id | index_id | declared component bytes | fact_id -> empty` (`storage/keys.rs:81-86`, `storage/keys.rs:144-160`).
- Unique guards and reverse FK guards use dedicated namespaces and values (`storage/keys.rs:50-79`, `storage/write.rs:259-327`).
- Writes insert/delete canonical facts, guards, access entries, and metadata counts synchronously on every logical change (`storage/write.rs:31-79`, `storage/write.rs:364-394`).

Current schema/access layout:

- `SchemaDescriptor::access_layouts` creates a fact-set access path, unique access paths, FK access paths, range access paths, and explicit indexes (`schema/layout.rs:9-51`, `schema/layout.rs:54-103`).
- The fact-set access path contains all relation fields in declaration order (`schema/layout.rs:58-62`).
- Every access layout contains only declared leading components plus a trailing fact ID; there is no payload region for undeclared fields (`schema/layout.rs:21-27`, `storage_tests/access.rs:117-183`).
- Explicit `IndexDescriptor` values are equality or permutation indexes over declared leading fields (`schema/descriptors.rs:342-399`).

Current query image:

- `QueryImageBuilder` builds from current access state under a read transaction (`query_image/builder.rs:22-62`).
- Each relation image is built by scanning the fact-set access path and appending selected field bytes into fixed-width column builders (`query_image/builder.rs:103-163`).
- It then scans selected access paths and concatenates their full encoded access-key bytes into `RelationIndexImage.bytes` (`query_image/builder.rs:164-231`).
- `RelationImage` stores `fact_count`, field metadata, fixed-width encoded columns, and durable sorted index images (`query_image/types.rs:176-197`).
- `RelationIndexImage` is a sorted byte array with prefix binary search over encoded access entries (`query_image/access.rs:6-22`, `query_image/access.rs:64-134`).
- `ColumnImage` supports only fixed encoded widths 1, 8, and 16; strings and bytes appear as 8-byte intern IDs (`query_image/columns.rs:47-90`, `query_image/columns.rs:161-230`).
- Query image cache keys include schema fingerprint, storage transaction ID, and loaded relation/index/column scope, but not a Free Join plan or GHT schema (`query_image/scope.rs:6-20`, `query_image/cache.rs:47-86`).

Current query access/runtime:

- `query_image_scope_for_query` loads fields and access paths from normalized atom requirements, not from a Free Join plan's subatom schemas (`query/hash.rs:3-51`).
- `sorted_trie.rs` defines scalar LFTJ iterator traits: `key`, `next`, `seek`, `at_end`, `open`, and `up` (`sorted_trie.rs:43-61`).
- `build_lftj_atom_plans` builds one LFTJ atom source per normalized atom (`query/lftj_access.rs:3-48`).
- `lazy_lftj_access_slice` chooses the smallest compatible durable sorted `RelationIndexImage` prefix/range; it does not build a GHT or COLT tree (`query/lftj_access.rs:50-89`).
- `LazyAccessIter` groups contiguous sorted access entries by one field component per depth and supports `open`, `up`, `key`, `next`, and `seek` (`query/lftj_iter.rs:93-292`).
- `execute_free_join` dispatches directly to `execute_lftj`; `LftjExecutor` binds one variable per recursion depth and intersects participant atom iterators with leapfrog search (`query/lftj_runtime.rs:3-18`, `query/lftj_runtime.rs:122-220`).

## Violations

| ID | Violation | Evidence and impact |
| --- | --- | --- |
| V-01 | No GHT interface exists. | The paper requires `iter() -> Iterator<Tuple>` and `get(key: Tuple) -> Option<GHT>`. `sorted_trie.rs:43-61` exposes scalar LFTJ iterator methods only. There is no relation/vars-bearing GHT node API. |
| V-02 | Tuple keys are unsupported. | `LazyAccessIter::key` returns one encoded field component at a time (`query/lftj_iter.rs:206-213`). `EncodedOwned` is a scalar fixed-width value (`sorted_trie.rs:3-41`). Paper GHT map keys are tuples, including multi-variable cover keys. |
| V-03 | `get(key)` probing is absent. | Current access uses `seek` over sorted byte arrays (`query/lftj_iter.rs:226-253`) and leapfrog search (`query/lftj_leapfrog.rs:126-164`). It cannot probe a tuple key and return a child GHT/COLT subtrie. |
| V-04 | Leaf representation is wrong. | Paper GHT leaves are vectors of tuples; COLT leaves are vectors of base-relation offsets. Current `RelationIndexImage.bytes` stores complete encoded access keys plus fact IDs (`query_image/access.rs:6-22`), and `LazyAccessSlice` stores an index range, not offset-vector leaves (`query/lftj_iter.rs:67-73`). |
| V-05 | COLT `force()` is absent. | Paper COLT lazily replaces an offset vector with a hash map. Current `LazyAccessIter` never mutates a vector into a map; it repeatedly scans/group-bounds already materialized sorted index bytes (`query/lftj_iter.rs:135-179`, `query/lftj_iter.rs:261-292`). |
| V-06 | The current "lazy" source is not COLT lazy materialization. | `lftj_lazy_access_slices` count durable access slices built from `RelationIndexImage` (`query/lftj_access.rs:36-42`, `metrics.rs:231-232`). The expensive access-key materialization already happened in storage writes and query-image build (`storage/write.rs:364-394`, `query_image/builder.rs:164-231`). |
| V-07 | Raw base storage is row-like plus duplicated access keys, not column-oriented. | Canonical membership stores concatenated full fact bytes (`storage/keys.rs:88-98`). Query-image columns are reconstructed by scanning the fact-set access path (`query_image/builder.rs:103-163`). Paper COLT assumes column-wise base data is the raw relation data available to lazy trie nodes. |
| V-08 | Query execution does not use the in-memory columns as COLT base data. | `RelationImage.columns` can decode field values (`query_image/types.rs:199-209`), but `lftj_access` and `LazyAccessIter` read `RelationIndexImage` entry bytes instead (`query/lftj_access.rs:60-88`, `query/lftj_iter.rs:118-133`). The column vectors are mainly projection/planner support, not the source of GHT/COLT tuples. |
| V-09 | Access entries are predeclared and cannot represent arbitrary plan-derived GHT schemas. | `AccessLayout` components come from fact set, constraints, range annotations, and explicit indexes (`schema/layout.rs:54-103`). Paper GHT schemas are derived from the Free Join plan's subatom partitions at query build time (`tex/03-free-join.tex:426-438`). |
| V-10 | Valid logical access depends on predeclared leading-field permutations. | `lazy_access_shape` can only use fields encountered in `index.fields` order and breaks on missing pre-variable fields (`query/lftj_access.rs:91-149`). A valid atom over a non-leading field can fail without an explicit index, even though COLT should always be able to scan offsets and lazily group by the requested tuple key. |
| V-11 | Write amplification contradicts COLT's lazy-build goal. | Every insert writes every generated/explicit access entry (`storage/write.rs:371-377`), and every delete removes every entry (`storage/write.rs:388-393`). COLT is specifically introduced to avoid building unused trie levels and hash tables until runtime (`tex/04-optimizations.tex:165-178`, `tex/04-optimizations.tex:284-292`). |
| V-12 | There is no dense base offset model for COLT leaves. | Current durable identity is a 16-byte fact hash (`storage/keys.rs:100-107`). `FactId(pub u32)` is dense only inside an eager `RelationImage` scan order (`query_image/types.rs:18-21`). No durable or execution-local mapping is exposed to COLT nodes as offset vectors. |
| V-13 | Deletes have no offset/liveness strategy compatible with `[0..n-1]`. | Deletes remove canonical, fact-id, guard, and access entries (`storage/write.rs:71-78`, `storage/write.rs:381-394`). A COLT base needs snapshot-local dense offsets or durable row handles plus a live-row projection to construct `[0..n-1]` for the current snapshot. |
| V-14 | Query image cache is incompatible with mutable shared COLT nodes. | `QueryImage` is cached as an immutable `Arc<QueryImage>` (`query_image/cache.rs:47-86`). Paper COLT lazily mutates nodes. A compliant design needs execution-local COLT nodes referencing immutable base columns or a thread-safe per-snapshot materialization cache. |
| V-15 | Relation index images are sorted tries, not hash tries/hash tables. | `RelationIndexImage::prefix_range` uses binary search over sorted encoded entries (`query_image/access.rs:64-134`). Paper GHT/COLT internal nodes are hash maps, and binary hash join is modeled as hash-map key to tuple/vector child. |
| V-16 | Cover iteration over tuple-valued subatoms is impossible. | `FreeJoinPlan::validate` requires exactly one bound variable per node (`free_join.rs:13-28`), and `LftjExecutor` binds one variable at a time (`query/lftj_runtime.rs:164-203`). GHT/COLT storage cannot currently iterate a cover tuple such as `(x, a)` from `R(x, a)`. |
| V-17 | Dynamic cover selection is absent. | The runtime gathers all atom participants for one variable and leapfrog-intersects them (`query/lftj_runtime.rs:164-183`). It does not enumerate node covers, compare key counts, or choose a cover trie as required by `tex/04-optimizations.tex:435-451`. |
| V-18 | Paper build-phase GHT schema generation is absent. | `QueryImageBuilder` builds relation images from fact-set/access scans (`query_image/builder.rs:103-248`), while `build_lftj_atom_plan` projects atoms onto a global variable order (`query/lftj_access.rs:16-48`, `query/lftj_access.rs:342-348`). Neither computes `[y_0, ..., y_l, []]` from subatom partitions. |
| V-19 | The final-vector/cover optimization is absent. | The paper drops the final empty leaf level when the last subatom is a cover (`tex/03-free-join.tex:431-438`). Current access layout always stores complete access key entries plus fact ID, and `LazyAccessIter::open` just descends scalar field depths (`query/lftj_iter.rs:261-287`). |
| V-20 | Left-most relation no-build behavior is not represented as COLT. | Paper COLT can iterate directly over the base table when no `get` is needed (`tex/04-optimizations.tex:284-292`). Current query execution still requires an atom source backed by a compatible durable access image (`query/lftj_access.rs:44-47`). Even fact-set fallback is a prebuilt access path. |
| V-21 | Range and residual predicates are not pushed into GHT/COLT sources. | Comparisons are normalized as separate predicates (`query/model.rs:121-145`) and evaluated during recursion, while LFTJ access filters only equality-style input/literal component matches (`query/lftj_access.rs:217-266`). A paper-style build phase assumes selections are pushed down before trie construction. |
| V-22 | Current planner stats are not key-count stats for GHT/COLT nodes. | `PlannerIndexStats::cheap` estimates distinct/fanout from sampled columns and declared leading fields (`planner_stats.rs:197-263`). Dynamic cover choice needs exact map key counts when forced and vector-length estimates when not forced. |
| V-23 | Query image scope is not plan-specific. | Scope is selected before planning from normalized atoms (`query/api.rs:53-69`, `query/hash.rs:3-51`). A paper-compliant source builder needs the chosen Free Join plan's relation occurrences, subatom schemas, cover candidates, and required columns. |
| V-24 | Current explain/diagnostics cannot audit GHT/COLT. | Explain prints `free_join_node id=... bind_vars=...` (`query/explain.rs:77-84`) and storage diagnostics print access-entry counts (`environment.rs:212-249`). Neither reports GHT schemas, COLT force counts, cover choices, offset-vector sizes, or map key counts. |
| V-25 | Tests codify the current non-COLT abstraction. | Tests assert singleton Free Join nodes and LFTJ counters (`query_tests/basic.rs:217-225`, `query_tests/basic.rs:635-643`), and query-image tests assert durable access-image byte arrays (`query_image_tests.rs:320-347`). These will break under a real COLT-first engine. |

## Required Breaking Changes

- Bump the on-disk storage format and schema fingerprint namespace. Current version `4` and canonical bytes label `bumbledb.schema.v4.set-native-layout` must not be reused (`lib.rs:45-46`, `schema/canonical.rs:12-15`). Rosetta allows hard failure plus ETL into a new database.
- Replace current access-entry-centered query storage with canonical set membership plus columnar relation base data. Access entries may survive only as optional accelerators and constraint support, not as the required query source.
- Introduce an internal immutable fact handle usable as a durable row address. To preserve Rosetta's no DB-side logical ID allocator, the safest handle is the existing 16-byte content-derived fact ID, mapped to dense execution-local `ColtOffset` values inside each snapshot.
- Add durable column namespaces keyed by relation, field, and fact handle. Query images should no longer reconstruct columns by scanning `fact_set` access keys.
- Add a current live-row namespace or equivalent canonical scan that yields fact handles for one relation under an LMDB read snapshot. The execution-local base image can assign dense offsets `[0..n-1]` from this handle list.
- Keep canonical fact membership as the source of set semantics. Duplicate insert remains a canonical membership check; exact delete removes canonical membership, live row, columns, optional accelerators, and guards atomically.
- Keep unique and reverse-FK guard namespaces logically separate from query access structures. They can point to fact handles instead of fact IDs or use fact handles directly if the handle is the fact ID.
- Split `AccessLayout` responsibilities. Constraint guards, optional persisted accelerator layouts, and GHT/COLT runtime schemas are different concepts and should not share one `AccessLayout` type.
- Change `IndexDescriptor` semantics. Equality/permutation indexes should become optional physical accelerators for known hot tuple orders, never a requirement for correctness of a valid conjunctive query.
- Replace `RelationIndexImage` as the primary atom source with immutable `RelationBaseImage` data: loaded field metadata, required encoded columns, fact handles, and snapshot-local dense offset order.
- Add `EncodedTuple` and `TupleSchema` abstractions. Tuple keys must support heterogeneous encoded widths and stable comparison/hash semantics over intern IDs for strings/bytes.
- Add a real `GhtNode` or `ColtNode` interface with `relation`, `vars`, `iter_tuple`, `get_tuple`, and `estimated_key_count` or exact `key_count` when materialized.
- Implement execution-local COLT forests over immutable relation base images. Do not mutate cached `Arc<QueryImage>` relation images directly unless a concurrency-safe materialization cache is deliberately designed.
- Replace or wrap `sorted_trie::{LinearIter, TrieIter}`. These traits can remain for an LFTJ fast path, but they are not the GHT interface.
- Replace `lftj_access::LazyAccessSlice` as the default source builder. It can become an optional sorted-index accelerator under the GHT/COLT source abstraction.
- Make `QueryImageBuilder` plan-aware or split it into `BaseImageBuilder` and `PlanSourceBuilder`. Source requirements must come from the chosen Free Join plan, not only normalized atom field sets.
- Replace `FreeJoinPlan` and `PlanNode` with formal nodes containing subatoms, atom occurrence IDs, ordered tuple variables, cover candidates, and optional chosen-cover policy. The current singleton-variable plan can be retained as `LftjPlan` or as a validated Generic Join special case.
- Move output/projection handling out of the formal Free Join plan. Keep Rosetta's encoded set projection sink, but do not make projection part of GHT/COLT plan validity.
- Update planner stats and cover selection. Runtime cover choice needs map key counts for forced nodes and vector-length estimates for unforced COLT vectors, not only declared-index fanout estimates.
- Update explain, diagnostics, and counters to show storage source kind, GHT schema, COLT node state, force counts, offset-vector lengths, cover candidates, chosen cover, tuple probes, and optional accelerator use.
- Rewrite tests that assert current singleton-node, access-image-byte, and LFTJ-only behavior. New tests must assert formal GHT/COLT behavior and Rosetta set semantics.

## New On-Disk/In-Memory Layout Proposal

This proposal preserves embedded LMDB, set semantics, interned strings/bytes, no SQL, no bag output, and no compatibility readers.

Durable logical namespaces for a new storage format:

| Namespace | Key | Value | Purpose |
| --- | --- | --- | --- |
| `T` canonical | `T | relation_id | fact_bytes` | `fact_handle` | Exact set membership and duplicate insert check. |
| `H` handle lookup | `H | relation_id | fact_handle` | `fact_bytes` | Reverse lookup/collision check for content-derived handles. |
| `L` live rows | `L | relation_id | fact_handle` | empty or compact row metadata | Snapshot-visible current row handle set for relation scans. |
| `C` columns | `C | relation_id | field_id | fact_handle` | fixed encoded field bytes | Durable columnar base data for snapshot image construction. |
| `U` unique guard | `U | relation_id | constraint_name | unique_key` | `fact_handle` | Named unique constraints. |
| `R` reverse FK guard | `R | target_relation_id | target_constraint | target_key | source_relation_id | source_constraint | source_fact_handle` | empty | Restrict delete checks. |
| `A` optional accelerator | `A | relation_id | accelerator_id | tuple_key | fact_handle` | empty | Optional persisted prefix/permutation indexes, never required for correctness. |
| `S` stats | `S | relation_id | stat_name` or `S | relation_id | accelerator_id | stat_name` | fixed counters/sketches | Fact counts, live counts, and cheap planner estimates. |

Fact handle policy:

- Use the existing BLAKE3-derived 16-byte fact ID as `fact_handle` to avoid introducing a DB-visible generated ID allocator.
- Keep the collision check currently performed by `storage/write.rs:90-94`.
- Build dense `ColtOffset(u32 or u64)` values per relation image by scanning `L | relation_id | *` under the read transaction and assigning offsets in deterministic handle order.
- Store `row_handles: Vec<FactHandle>` in the immutable base image only when reverse lookup to durable rows is needed; most query execution can use offsets directly.

Write path under the proposed layout:

- `insert` encodes a full fact, checks `T`, checks FKs and unique constraints, then writes `T`, `H`, one `C` entry per field, one `L` entry, guard entries, optional accelerator entries, and stats in one LMDB write transaction.
- Duplicate insert returns `AlreadyPresent` before writing columns or accelerators.
- `delete` encodes the exact full fact, checks `T`, checks reverse FK restrictions, then removes optional accelerators, guards, `L`, `C` entries, `H`, `T`, and stats in one LMDB write transaction.
- Absent delete remains an idempotent no-op.

Immutable snapshot base image:

```rust
struct RelationBaseImage {
    relation: RelationId,
    name: String,
    fields: Vec<FieldImage>,
    row_handles: Vec<FactHandle>,
    columns: BTreeMap<FieldId, ColumnImage>,
    offset_by_handle: Option<HashMap<FactHandle, ColtOffset>>,
    stats: RelationStats,
}
```

Execution-local COLT over the base image:

```rust
struct Colt<'base> {
    relation: AtomOccurrenceId,
    base: &'base RelationBaseImage,
    schema: Vec<TupleSchema>,
    vars: TupleSchema,
    data: ColtData,
}

enum ColtData {
    Offsets(Vec<ColtOffset>),
    Map(HashMap<EncodedTuple, ColtNodeId>),
}
```

COLT behavior:

- `new(relation, schema)` starts with an offset vector covering all live offsets in the base image.
- `iter()` over a map returns map keys.
- `iter()` over an offset vector streams tuple values from base columns if `vars` is a suffix of the relation/schema path or if the node is a leaf vector.
- `iter()` over a non-suffix offset vector calls `force()` and then iterates map keys.
- `get(tuple)` calls `force()` if needed and returns the child node for the tuple key.
- `force()` groups the current offset vector by the tuple values of `vars`, creates child COLT nodes with the remaining schema, and replaces the vector with a hash map.
- Optional durable accelerators can seed a root map or a filtered offset vector only when they exactly match the requested tuple schema; correctness must not depend on their existence.

Free Join source model:

- A formal `FreeJoinPlan` owns atom occurrence IDs and ordered subatoms.
- For each atom occurrence, the source builder computes the GHT schema from that atom's subatoms in plan order.
- Each source is a `Colt` implementing the GHT interface over one immutable `RelationBaseImage`.
- LFTJ remains a possible fast path only for singleton-variable Generic Join plans where sorted accelerator scans are beneficial.

## Suggested Implementation Sequence

1. Rename the current runtime internally as LFTJ or mark it as a Generic Join special case; stop treating current `PlanNode { bind_vars }` as the final Free Join plan model.
2. Add formal Free Join IR from the paper: atom occurrence ID, subatom, node, cover candidates, atom partition validation, and output-independent plan validation.
3. Add a storage-format-v5 experiment behind a feature or new module with `T/H/L/C/U/R/A/S` namespaces and ETL-only creation.
4. Implement write/read tests for v5 canonical set membership, columns, live rows, guards, duplicate insert no-op, absent delete no-op, delete restrictions, snapshot visibility, and reopen.
5. Build immutable `RelationBaseImage` from the column/live namespaces for selected relations and fields; keep current `QueryResultSet` and projection sink semantics unchanged.
6. Implement execution-local `EncodedTuple`, `TupleSchema`, `Colt`, `ColtNode`, `iter`, `get`, `force`, and key-count estimates over `RelationBaseImage`.
7. Implement a scan-only Free Join executor over GHT/COLT nodes. Make correctness independent of any persisted accelerator.
8. Lower the current variable-order/GJ plan into validated singleton-subatom Free Join plans and compare results against the old LFTJ runtime.
9. Add `binary2fj`, factorization, and dynamic cover choice from the paper. Keep the current variable-order planner as one planning mode, not the whole model.
10. Reintroduce optional durable sorted/permutation accelerators under the GHT source abstraction, with explain output showing when an accelerator is used.
11. Update query image scope after planning so loaded columns and optional accelerators match the chosen plan's source requirements.
12. Replace explain, diagnostics, counters, and tests with GHT/COLT/Free Join terminology and keep LFTJ-specific counters only for the fast path.

## New Tests/Benchmarks

- Storage v5 exact set tests: duplicate insert no-op, absent delete no-op, exact delete, delete/reinsert, stable fact-handle collision checks, and no partial writes under failpoints.
- Column layout tests: every live handle has exactly one value per field, per-field column scans align to live handle order, string/bytes values store intern IDs, and dictionary reverse lookup still decodes output.
- Snapshot tests: a read transaction sees stable `L` and `C` namespaces while a writer inserts/deletes; query images built inside that read transaction keep the old view.
- COLT unit tests: initial all-offset vector, suffix `iter()` without force, non-suffix `iter()` with force, `get()` force and lookup, repeated `get()` no refold, unbalanced tree shape, maps and vectors at same level, and empty relation behavior.
- GHT schema tests for paper examples: clover binary plan, clover Generic Join plan, factorized clover plan, triangle plan, and chain `binary2fj` plan, including final `[]` drop when the last subatom is a cover.
- Free Join executor tests: multi-variable cover node `[R(x, a), S(x)]`, tuple-key sibling probe, static zero-variable atom, self-join occurrence aliases, and invalid plans rejected before execution.
- Access-completeness tests: relation atom over a non-leading field with no explicit index must execute by scan/COLT, not fail with an internal LFTJ access error.
- Predicate tests: repeated variables either lower to equality predicates or fail as `InvalidQuery`; range comparisons have a defined pushdown/residual behavior; literal/input filters work without declared indexes.
- Set-semantics tests: duplicate witnesses do not multiply projected output, exact duplicate base facts remain no-ops, and existential variables do not affect `QueryResultSet` cardinality.
- Differential tests against the reference evaluator for random small positive conjunctive queries with projections, inputs, literals, self-joins, comparisons, omitted fields, and no useful indexes.
- Benchmarks with correctness gates: clover skew showing factorized Free Join avoids the `n^2` intermediate, left-most relation no-build behavior, COLT avoiding unused second-level materialization, triangle Generic Join fast path, and exact result comparison against SQLite `SELECT DISTINCT` before timing.

## Open Questions

- Should the durable fact handle be the existing content-derived 16-byte fact ID, or is an internal monotonic row ID acceptable under Rosetta's no DB-side generated ID allocator?
- Does Bumbledb need durable columnar base storage, or is an eager snapshot-local column image sufficient for the first paper-compliant COLT implementation?
- Should optional persisted accelerators stay in the schema as `IndexDescriptor`, or should they move to a lower-level physical tuning descriptor separate from logical schema?
- Should lazy COLT materialization be execution-local only, or should a query-image-level concurrent cache share forced nodes across repeated reads of the same snapshot?
- How should deletes and high churn be handled: immediate removal from `C/L`, tombstones with live filters, or periodic ETL/compaction into a new database?
- How aggressively should comparisons and range predicates be pushed into source construction before formal Free Join planning?
- Should string/bytes tuple keys compare by intern ID only, or do any planner/explain paths need raw lexical semantics?
- Should query image scope be keyed by the full Free Join plan, by base relation/field needs, or by a reusable relation-base scope plus execution-local source schemas?
- Is dynamic cover choice mandatory in the first compliant version, or can the first version use a validated static cover and add runtime cover selection later?
- Should the current LFTJ runtime remain as an optimized fast path, and if so what exact plan shapes are allowed to select it?
