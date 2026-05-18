# 04: Index Permutations And Access Layouts

**Goal**
- Add the physical index permutations required by the variable-order trie executor.

This is not blind indexing. It is physical design for the new execution model.

**Thesis**
- WCOJ needs trie streams whose component order follows useful variable orders.
- Current automatic indexes are too narrow: primary, ref, range, unique.
- Scalar and symbol equality fields are invisible unless they are primary, ref, range, or unique.
- Composite edge relations need multiple permutations when the optimizer wants to approach them from different variables.

**Hard Cut**
- Do not add ad hoc special-case indexes for one benchmark query.
- Do not let the optimizer silently choose bad orders because a useful index permutation is missing.
- Do not keep automatic index generation as an opaque policy that the optimizer cannot inspect.

**Physical Design Direction**
- Make index declarations explicit in schema descriptors where useful.
- Keep generated indexes only when they are obviously universal, such as primary keys and foreign-key refs.
- Add equality index support for fixed-width scalar and symbol fields.
- Add composite permutation indexes for high-value relation atoms.
- Preserve covering index behavior so the WCOJ executor can read all needed components from keys.

**Immediate Missing Permutations**
- `PostingTag(tag, posting)` for `tag_lookup_join`.
- `Boat(color, id)` for sailors color predicates.
- `Customer(nation, id)` for TPC-H customer filtering.
- `Supplier(nation, id)` for TPC-H supplier filtering.
- Additional cyclic edge permutations if `triangle_count` variable ordering needs them beyond current primary/ref permutations.

**Schema API Direction**
- Add an explicit `IndexDescriptor` or equivalent relation-level physical index declaration.
- Support index kind tags such as primary, ref, range, equality, and permutation.
- Keep index key layout deterministic and fingerprinted.
- Fail schema construction if declared indexes exceed LMDB key size limits.
- Keep ETL-only schema changes acceptable; no migration compatibility layer.

**Optimizer Integration**
- Planner should ask: which trie stream can constrain this variable at this level?
- If no good stream exists, the planner should identify the missing permutation.
- Explain output should distinguish chosen physical indexes from rejected/missing indexes.
- Bench docs should list required index permutations for each workload class.

**Implementation Steps**
- Extend schema descriptors with explicit index declarations.
- Update canonical schema fingerprinting for explicit indexes.
- Generate `CurrentIndexLayout` from declared and generated indexes deterministically.
- Update storage writes to maintain all declared current indexes.
- Update bulk-load to build all declared current indexes.
- Update WCOJ planning to choose declared permutations by variable order.
- Add tests for key layout, duplicate component prevention, and max-key-size failure.
- Add benchmark schemas with required equality/permutation indexes.

**Passing Criteria**
- `tag_lookup_join` starts from `PostingTag(tag, posting)`.
- `red_boat_sailors` and `high_rating_red_boats` start from `Boat(color, id)` or an optimizer-justified better domain.
- `supplier_nation_orders` starts from `Supplier(nation, id)`.
- `revenue_by_customer_range` can choose between `Customer(nation, id)` and `LineItem(ship_date, order, ...)` based on stats.
- No benchmark query is forced into a primary scan due to missing scalar equality support.

**Design Trap To Avoid**
- Do not mistake more indexes for architecture. Indexes are only useful when the variable-order executor can exploit them as trie streams.
