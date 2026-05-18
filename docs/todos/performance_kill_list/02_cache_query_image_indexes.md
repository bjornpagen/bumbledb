# 02: Cache QueryImage Indexes

**Goal**
- Stop rebuilding temporary atom relation images and `SortedTrieIndex` instances per query.
- Make sorted/hash indexes reusable per `QueryImage` and access-path field order.

**Trace Evidence**
Tiny queries prove useful work is small but execution still costs milliseconds:

| Query | Plan→Exec | Actual Work |
|---|---:|---|
| `joinstress/chain4_from_a` | `5.4ms` | `3` candidates, `3` trie_next |
| `ledger/postings_for_holder_range` | `13.5ms` | `10` candidates |
| `sailors/sailor_range_reserves` | `0.3ms` in one traced run, but `20ms` total | `10` candidates |

The source explains the cost: `build_lftj_atom_plan` scans the source relation image, copies encoded bytes into temporary columns, constructs a temporary `RelationImage`, and calls `SortedTrieIndex::build` for each atom.

**Current Code Path**
- `execute_lftj` calls `build_lftj_atom_plans`.
- `build_lftj_atom_plan` builds a per-atom temporary relation image.
- It then builds a new `SortedTrieIndex` from that temporary image.
- This repeats for every atom, every query execution, every benchmark repeat.

**Required Design**
- Add a `RelationIndexCatalog` to `RelationImage` or adjacent `QueryImage` runtime state.
- Cache declared sorted indexes by `AccessId` and lazy field permutations by `(RelationId, Vec<FieldId>)`.
- Cache hash indexes by `AccessId`/field order for future hash runtime.
- Replace temporary atom relation images with prefix-bound/index-view iterators over cached indexes.

Conceptual structure:

```rust
pub struct RelationIndexCatalog {
    sorted_by_access: BTreeMap<AccessId, Arc<SortedTrieIndex>>,
    sorted_by_fields: RwLock<BTreeMap<Vec<FieldId>, Arc<SortedTrieIndex>>>,
    hash_by_access: RwLock<BTreeMap<AccessId, Arc<HashTrieIndex>>>,
}
```

**Prefix-Bound Views**
- Atom `PostingTag(posting: ?posting, tag: $tag)` with index `[tag, posting]` should expose a unary trie over `posting` after applying hidden prefix `tag=$tag`.
- No filtered temporary relation should be constructed.
- Repeated variables and residual predicates must still be checked.

**Implementation Steps**
1. Add cached index catalog to `QueryImage`/`RelationImage`.
2. Build declared access-path sorted indexes once per image.
3. Add lazy sorted index lookup by field vector.
4. Add `SortedTrieIndex::byte_len` for memory accounting.
5. Refactor `build_lftj_atom_plan` into atom access planning over cached indexes.
6. Add prefix-bound trie view support.
7. Delete production construction of temporary `RelationImage` and `ColumnImage` in query execution.
8. Add benchmark counters: sorted trie cache hits/builds, lazy index count, atom temp relation builds.

**Tests**
- Cached index lookup returns the same `Arc` on repeated requests.
- Prefix-bound trie over `[tag, posting]` with `tag=1` returns only matching postings.
- Missing prefix returns empty iterator.
- Repeated variable atom filters unequal encoded fields.
- Existing LFTJ results match before/after for all query tests.

**Acceptance Criteria**
- Production query execution does not call `SortedTrieIndex::build` directly.
- Production query execution does not construct temporary `RelationImage` or `ColumnImage` atom plans.
- Warm repeated query reports `atom_temp_relation_builds == 0` and `sorted_trie_builds == 0`.
- `QueryImageStats.sorted_trie_bytes > 0` on generated datasets.
- Scale-10000 focused query latency improves at least 35% for `tag_lookup_join`, `red_boat_sailors`, and `supplier_nation_orders` compared to current trace baselines.

**Risks**
- Prefix-bound trie views are semantically subtle.
- Lazy arbitrary permutations can grow memory; restrict to planner-approved vectors or add caps.
- Residual checks are mandatory for correctness.
