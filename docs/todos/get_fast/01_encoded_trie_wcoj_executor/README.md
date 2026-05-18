# 01: Encoded Trie WCOJ Executor

**Goal**
- Replace the recursive atom executor with a variable-at-a-time, encoded trie executor based on Leapfrog Triejoin and Generic Join.

This is the central refactor. All later get-fast stages are subordinate to it.

**Thesis**
- Datalog queries are joins over variables, not loops over relation rows.
- LMDB sorted covering indexes can be viewed as tries over encoded key components.
- Joining should happen by intersecting candidate encoded values for the next variable across all relevant atoms.
- Full logical rows should not be produced during join search.

**Hard Cut**
- Delete the recursive `execute_atoms` execution model.
- Delete atom-order-as-plan as the physical execution primitive.
- Delete full-row `match_atom` as the normal query path.
- Replace `QueryPlan.atoms` as the primary explain story with variable-order trie operators.
- Keep only one executor after the cutover.

**Current Design To Remove**
- `ReadTxn::execute_query` plans relation atoms, then calls `execute_atoms` recursively.
- `execute_atoms` opens a scan for each atom invocation.
- `open_scan` returns `IndexScan`, which decodes each candidate into a full `Row`.
- `match_atom` binds variables by field-name lookup in a decoded `BTreeMap`.
- Comparison and projection run after many values are already decoded and cloned.

This shape caused:
- `triangle_count`: `EdgeBC` completed about `18000` nested invocations per run.
- `red_boat_sailors`: `Sailor` completed about `3320` nested primary lookups per run.
- `tag_lookup_join`: a full `PostingTag` primary scan because `tag` is not leading in any current index.

**Replacement Shape**
- Parse/typecheck still produces typed Datalog IR.
- Compile typed atoms into a join hypergraph.
- Compile relation fields into encoded index components.
- Choose a variable order.
- For each variable level, collect all relation/index constraints that can constrain that variable given prior bound variables.
- Intersect candidate encoded values from those constraints.
- Recurse by variable, not by relation atom.
- Emit complete encoded bindings directly into projection or aggregation.

**Core New Internal Types**
- `JoinHypergraph`: query variables, relation atoms, comparisons, projected variables, aggregate terms.
- `AtomFieldConstraint`: maps one atom field to a variable, input, literal, or wildcard.
- `VariableOrder`: ordered variable IDs plus rationale and estimated cost.
- `TrieAccessPlan`: one relation atom bound to one physical index layout and a field-to-index-component map.
- `TrieCursor`: LMDB-backed cursor over one encoded index viewed as a trie.
- `TrieLevel`: cursor state for one leading component depth.
- `EncodedBinding`: fixed-width vector of optional encoded values keyed by typed variable ID.
- `WcojExecutor`: variable-at-a-time search engine.
- `WcojCounters`: variable-level seeks, advances, intersections, candidates, failures, decode counts, output rows.

Names can change during implementation, but these responsibilities should not blur.

**Trie Cursor Contract**
- `open(prefix)`: position on the first tuple matching all already-bound leading components.
- `seek_at_level(depth, value)`: seek to the smallest tuple whose component at `depth` is at least `value` while preserving prior prefix components.
- `current_at_level(depth)`: return the encoded component at `depth` without decoding.
- `advance_at_level(depth)`: move to the next distinct value at `depth` under the current prior prefix.
- `has_prefix(prefix)`: efficiently prove whether a partial assignment exists.
- `save/restore` or scoped cursor state: support variable recursion without re-opening an LMDB cursor for every candidate.

The cursor API should expose encoded slices or compact owned encoded values. It should not expose decoded `Row`.

**Physical Index Assumption**
- An index is a sorted key over all relation fields.
- A trie view is valid when earlier components in the chosen index are bound by earlier variables, literals, or inputs.
- If the desired variable is not reachable as the next trie component for any useful index, the planner must request a new index permutation in stage 04.

**Executor Algorithm**
```text
execute(level, binding):
  if level == variable_order.len:
    if all deferred comparisons pass:
      emit binding
    return

  variable = variable_order[level]
  constraints = constraints_ready_for(variable, binding)

  if constraints is empty:
    fail planning; no unconstrained domain scans in the WCOJ executor

  candidates = leapfrog_intersection(constraints)
  for encoded_value in candidates:
    if immediate comparisons fail:
      continue
    binding.bind(variable, encoded_value)
    execute(level + 1, binding)
    binding.unbind(variable)
```

Unconstrained scans are allowed only when the whole query logically requires enumerating a base domain, such as the first variable of a full relation scan. That must be explicit in the plan and counters.

**Leapfrog Intersection**
- Every ready constraint yields a sorted stream of distinct encoded values for the current variable.
- Maintain all streams at the same candidate value.
- Seek lagging streams to the current maximum candidate.
- When all streams agree, bind that encoded value.
- Advance one stream and continue.

This is the foundational fix for cyclic joins and many-way joins.

**Planning Pipeline**
- `TypedQuery` to `JoinHypergraph`.
- Determine variables required by each atom field.
- Determine which inputs/literals can be encoded before execution.
- Select an initial variable order using simple structural heuristics in this stage.
- Select one `TrieAccessPlan` per atom that can support the chosen order.
- Reject unsupported queries loudly rather than falling back to the old executor.

Stage 03 will make variable ordering cost-based. Stage 04 will add required index permutations. This stage may use a conservative order, but it must use the new executor.

**Comparisons**
- Equality comparisons between same-type encoded values should run encoded.
- Range comparisons should run encoded only if the encoding preserves logical order for the type.
- Comparisons involving type normalization between `Id` and `Ref` must normalize at encoding time.
- Comparisons that cannot safely run encoded must decode only the operands involved.
- Comparison readiness is per variable level: evaluate as soon as all operands are bound.

**Projection**
- Projection receives encoded bindings.
- Decode only final output variables.
- Preserve set semantics using encoded row keys where possible before decoding.

**Aggregation**
- Initial cut can aggregate after complete encoded bindings.
- Do not build aggregation around decoded row materialization.
- Stage 05 moves aggregation into the variable-order pipeline.

**Explain Output**
- Show variable order first.
- Show each variable's participating constraints and index layouts.
- Show per-variable candidate counts, seek counts, intersection advances, comparison failures, and output counts.
- Keep relation/index lines as supporting detail, not the primary plan shape.

**Implementation Steps**
- Introduce encoded query-time value representation that can compare and hash without decoding.
- Split index-key decode into component access without constructing `Row`.
- Add `TrieCursor` over `CurrentIndexLayout` and LMDB key prefixes.
- Build `JoinHypergraph` from typed Datalog atoms.
- Build a conservative `VariableOrder` from query structure.
- Build `TrieAccessPlan` for every atom.
- Implement variable-recursive WCOJ executor.
- Wire `ReadTxn::execute_query` to the WCOJ executor only.
- Remove old recursive atom executor functions.
- Rewrite query tests to assert variable-order plans and outputs.

**Passing Criteria**
- `cargo test --workspace` passes.
- `cargo clippy --workspace --all-targets -- -D warnings` passes.
- All existing benchmark queries execute through the WCOJ executor.
- There is no nested-loop executor fallback.
- `triangle_count` no longer performs tens of thousands of nested `EdgeBC` cursor reopen-equivalents.
- Explain output makes it obvious where candidate generation and intersection time went.

**Risk To Watch**
- LMDB cursors may not support cheap save/restore exactly as desired. If so, build explicit cursor stacks around prefix iterators, but do not reintroduce relation-atom recursion.
- Missing index permutations may block some ideal variable orders. Use conservative variable orders temporarily, then solve physically in stage 04.
- The first WCOJ version can be simple, but its data model must be encoded and variable-centric from day one.
