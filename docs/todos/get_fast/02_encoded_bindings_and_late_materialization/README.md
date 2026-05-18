# 02: Encoded Bindings And Late Materialization

**Goal**
- Make encoded values and field-demand analysis the default query representation so joins do not decode full rows.

This stage makes the new WCOJ executor cheap per candidate. Stage 01 changes join shape; this stage removes decode/materialization waste inside that shape.

**Thesis**
- Most join work needs equality, ordering, and hashing, not Rust `Value` objects.
- LMDB index keys already contain sorted encoded components.
- Decoding every candidate into `Row { BTreeMap<String, Value> }` is structural overhead.
- Dictionary reverse lookups during joins are almost always wrong; strings/bytes should decode only if projected or compared semantically.

**Hard Cut**
- Query execution must not consume `IndexScan` items that contain decoded `Row`.
- Query execution must not use field-name `BTreeMap` lookup in hot paths.
- Query execution must not clone full `Value` objects for intermediate bindings.
- Full-row decode is removed from the query hot path, not hidden behind a helper.

**Field Demand Analysis**
- For each query, compute the minimal field set needed for:
- variable binding,
- input/literal constraints,
- comparisons,
- projection,
- aggregation,
- uniqueness/deduplication.

The executor should know which fields need encoded access and which fields need final logical decode.

**Encoded Value Model**
- `EncodedScalar` should carry bytes plus logical type identity.
- Equality uses bytes after type normalization.
- Ordering uses bytes only for types whose encoding is order-preserving.
- Hashing/deduplication should use encoded bytes plus type identity.
- `Id` and `Ref` normalization must happen before comparison or binding.

**Index Key Views**
- Add a query-facing component view over current index keys.
- It should expose component slices by component ordinal, not by field-name map.
- It should avoid allocation for normal cursor operations.
- It should validate layout widths at boundaries, not per candidate if avoidable.

**Binding Representation**
- `EncodedBinding` is a fixed vector indexed by typed variable ID.
- Each slot stores unset or encoded bytes with type identity.
- Binding and unbinding should be O(1) and should not clone large values unless ownership is required beyond cursor lifetime.
- Projection receives encoded bindings and decodes in projection order.

**Projection And Deduplication**
- Non-aggregate projection should deduplicate on encoded projected tuples first.
- Decode only after deduplication when possible.
- Output row sorting should prefer encoded ordering when it matches logical ordering; otherwise sort decoded output rows at the edge.

**Comparison Execution**
- Equality and order comparisons should run encoded for fixed-width order-preserving types.
- String and bytes equality should compare intern IDs encoded in keys, not reverse dictionary values.
- Semantic string ordering is not supported unless explicitly designed; do not accidentally imply it through intern IDs.
- Decode comparison operands only when encoded semantics are not valid.

**Storage Boundary Changes**
- Split scan APIs into query-oriented encoded cursor APIs and logical row APIs.
- The WCOJ executor uses only encoded cursor APIs.
- Logical row decoding remains for write tests, administrative inspection, and public APIs only if still needed after simplification.
- Do not let query code call logical row scan helpers.

**Implementation Steps**
- Add query-time encoded value and binding types.
- Add index component view utilities over `CurrentIndexLayout`.
- Replace hot-path `Row` matching with component ordinal matching.
- Add field-demand analysis from `TypedQuery`.
- Decode projection results from encoded bindings.
- Convert aggregation inputs from encoded bindings, decoding only aggregate operands as needed.
- Add counters for decoded values, dictionary reverse lookups, encoded comparisons, decoded comparisons, and materialized output values.
- Remove query-layer dependencies on `Row::value`.

**Passing Criteria**
- Existing query tests pass with encoded bindings.
- Benchmarks report decode counts much lower than rows scanned for join-heavy queries.
- String/bytes dictionary reverse lookups are zero for benchmarks unless projected.
- `tag_lookup_join`, `red_boat_sailors`, and `supplier_nation_orders` no longer decode unused string fields during joins.
- There is no full-row decode in the WCOJ executor.

**Design Trap To Avoid**
- Do not build an encoded fast path next to a decoded slow path. The encoded representation is the query representation.
