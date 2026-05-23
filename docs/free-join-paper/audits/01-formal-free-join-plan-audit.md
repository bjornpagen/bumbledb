# Formal Free Join Plan Audit - Investigator 1

## Sources Read

- `docs/ROSETTA_STONE.md`, especially relation semantics, query semantics, query execution, public output, benchmark, and validation contracts.
- `docs/free-join-paper/arXiv-2301.10841v2/main.tex`, especially the paper structure and included Free Join sections.
- `docs/free-join-paper/arXiv-2301.10841v2/tex/02-background.tex`, especially the full conjunctive query model, self-join renaming assumption, pushed-down selections, binary join, and Generic Join/LFTJ background.
- `docs/free-join-paper/arXiv-2301.10841v2/tex/03-free-join.tex`, especially the GHT interface, subatom and partitioning definitions, valid Free Join node definition, cover definition, build phase, and join phase.
- `docs/free-join-paper/arXiv-2301.10841v2/tex/04-optimizations.tex`, especially `binary2fj`, factorization, COLT, vectorized execution, and dynamic cover selection.
- `crates/bumbledb-lmdb/src/free_join.rs`.
- `crates/bumbledb-lmdb/src/query.rs` and query submodules under `crates/bumbledb-lmdb/src/query/`.
- `crates/bumbledb-core/src/query_ir.rs` and `crates/bumbledb-core/src/query_builder.rs`.
- `crates/bumbledb-lmdb/src/query_image.rs` and query image submodules under `crates/bumbledb-lmdb/src/query_image/`.
- `crates/bumbledb-lmdb/src/sorted_trie.rs`, `crates/bumbledb-lmdb/src/storage_schema.rs`, `crates/bumbledb-core/src/schema/descriptors.rs`, `crates/bumbledb-core/src/schema/layout.rs`, and `crates/bumbledb-core/src/schema/validation.rs`.
- Query tests under `crates/bumbledb-lmdb/src/query_tests*` and `crates/bumbledb-lmdb/src/query_test_helpers*`.

## Executive Summary

The repository does not currently implement the formal Free Join plan model from the paper. It implements a set-semantic, LFTJ/GJ-style variable-order executor and exposes it through names like `FreeJoinPlan`, `free_join_node`, and `execute_free_join`.

The implementation can return correct duplicate-free projection sets for the subset of positive typed queries that its durable access paths can serve, which aligns with the Rosetta Stone output contract. It does not represent paper Free Join semantics: no subatoms, no atom partitioning, no relation-bearing Free Join nodes, no cover abstraction, no GHT/COLT interface, no build phase that derives per-atom GHT schemas from a Free Join plan, and no join phase that iterates a cover tuple and probes sibling subatoms.

This is not a cosmetic naming issue. The current `FreeJoinPlan` deliberately rejects multi-variable nodes, while the paper's central contribution is that a node may bind any number of variables and involve any number of relations as subatoms. The current plan is a strict singleton-variable special case resembling Generic Join/LFTJ, and even that special case is implicit rather than modeled as Free Join subatoms.

The required overhaul is breaking. Either rename the current implementation honestly as LFTJ and stop claiming formal Free Join, or replace the plan/runtime boundary with a real Free Join IR and executor where LFTJ is only one special-case execution strategy.

## Paper Requirements

The paper's query model is a full conjunctive query `Q(x) :- R_1(x_1), ..., R_m(x_m)` where every atom has a relation name and a tuple of variables. The paper starts from bag semantics, but Bumbledb must adapt this to the Rosetta Stone set semantics: base relations are sets, full solution bindings are sets, and projected output is duplicate-free.

The paper assumes self-joins can be handled by renaming relation occurrences, and assumes selections are pushed down to base tables so every atom variable is distinct. In Bumbledb terms, that means repeated variables, field equalities, literals, inputs, omitted fields, and wildcard fields need an explicit normalization story before formal Free Join planning.

A subatom of `R_i(x_i)` is `R_i(y)` where `y` is a subset of the atom variables. A partitioning of an atom is a set of subatoms whose variable sets partition the atom variables. A Free Join plan is a list of nodes, and each node is a list of subatoms.

The nodes must partition the query: for every atom, all subatoms for that atom across all nodes must form a partition of that atom's variables. This is the core formal invariant. It is not optional metadata.

For a node `phi_k`, `vs(phi_k)` is the set of variables in its subatoms and `avs(phi_k)` is the set of variables bound by preceding nodes. A valid Free Join node must satisfy two conditions: no two subatoms in the same node share the same relation occurrence, and at least one subatom contains all newly introduced variables `vs(phi_k) - avs(phi_k)`. Such a subatom is a cover.

The paper initially assumes one designated cover listed first, then later relaxes this to multiple covers and dynamic cover choice. A formal implementation therefore needs a cover set or an equivalent validated representation.

The GHT interface is relation-aware and subatom-aware. A GHT has a `relation`, `vars`, `iter() -> Iterator<Tuple>`, and `get(key: Tuple) -> Option<GHT>`. Internal map keys are tuples, not just scalar values. Leaves are vectors of tuples.

The build phase constructs one GHT per relation occurrence or atom. If the plan partitions atom `R_i` into subatoms `R_i(y_0), ..., R_i(y_l-1)`, the GHT schema is `[y_0, ..., y_l-1, []]`, with the final empty vector level dropped when the last subatom is a cover.

The join phase recursively executes nodes. For each node, it iterates the chosen cover, uses the current tuple plus prior bindings to build tuple keys for other subatoms, probes their GHTs, replaces successful tries with subtries, and recurses.

The optimization section requires that a conventional binary plan can be translated to Free Join with `binary2fj`, then factored by moving eligible subatoms earlier while preserving validity. COLT is a lazy implementation of the GHT interface with map or vector nodes and offset-vector leaves. Dynamic cover selection chooses among all covers at join time using key-count estimates.

The Rosetta Stone adds product requirements: relation membership and projection output have set semantics, existential variables must not multiply projected output, query images are snapshot-local internals, `QueryResultSet` is duplicate-free and canonicalized, malformed IR must be rejected at execution boundaries, and the retained execution backbone is described as Free Join/LFTJ plus fact-native projection/storage paths.

## Current Implementation

`crates/bumbledb-lmdb/src/free_join.rs:3-10` defines `FreeJoinPlan` as only `nodes: Vec<PlanNode>` plus `output: OutputPlan`. A `PlanNode` contains only `id` and `bind_vars` at `free_join.rs:31-38`. There is no atom list, subatom list, partition map, cover, relation occurrence, or GHT schema.

`FreeJoinPlan::validate` checks only dense ordered node IDs and exactly one bound variable per node at `free_join.rs:13-28`. The singleton-variable constraint at `free_join.rs:21-24` is the opposite of the paper's node generality.

`crates/bumbledb-lmdb/src/query/planner.rs:197-211` builds a `FreeJoinPlan` by turning a chosen variable order into one `PlanNode` per variable. It ignores relation atoms except through the earlier variable-order selection and output projection.

`crates/bumbledb-lmdb/src/query/lftj_runtime.rs:3-18` exposes `execute_free_join`, validates the summary plan, and immediately calls `execute_lftj`. `lftj_runtime.rs:97-107` converts the alleged Free Join plan back into `Vec<usize>` variable IDs.

The real runtime is `LftjExecutor` in `lftj_runtime.rs:122-220`. At each depth, it picks one variable, opens every atom iterator that contains that variable, runs leapfrog intersection, binds one encoded value, and recurses. That is a Generic Join/LFTJ execution shape, not the paper Free Join node execution shape.

`crates/bumbledb-lmdb/src/query/lftj_access.rs:3-48` builds one `LftjAtomPlan` per normalized atom from a durable query-image relation. It chooses a `LazyAccessSlice` over an existing `RelationIndexImage`; it does not build a GHT schema from Free Join subatom partitions.

`crates/bumbledb-lmdb/src/query/lftj_iter.rs:67-292` implements `LazyAccessSlice` and `LazyAccessIter`. The iterator has `open`, `up`, `key`, `next`, and `seek` over scalar encoded components in a sorted durable index image. It has no `get(key: Tuple)`, no tuple-key node, no relation/vars metadata, and no GHT child object.

`crates/bumbledb-lmdb/src/sorted_trie.rs:43-61` defines `LinearIter` and `TrieIter`. These are LFTJ iterator traits, not the paper GHT interface.

`crates/bumbledb-lmdb/src/query_image/builder.rs:103-248` builds relation images by scanning the fact-set access path, loading encoded columns, and materializing selected durable index bytes. This is a query-image/access-slice system, not a Free Join build phase.

`crates/bumbledb-lmdb/src/query/hash.rs:3-51` chooses a query-image scope from required atom fields and access paths. It does not derive scope from a formal Free Join plan, GHT schemas, covers, or subatoms.

`crates/bumbledb-lmdb/src/query/sinks.rs:82-161` implements encoded projection as a `BTreeSet`, and `crates/bumbledb-lmdb/src/query/model.rs:166-172` sorts and deduplicates final facts. This is aligned with Rosetta Stone set output semantics.

The tests primarily assert that LFTJ is used and that a node binds exactly one variable, for example `query_tests/basic.rs:217-224` and `query_tests/basic.rs:635-643`. They do not assert formal Free Join plan invariants.

## Violations

### P0-01: `FreeJoinPlan` is not a Free Join plan

The paper's plan is a list of nodes, each node is a list of subatoms, and all subatoms partition the query atoms. The repository's `FreeJoinPlan` stores only singleton variable-binding nodes plus output at `free_join.rs:3-10` and `free_join.rs:31-38`.

This means the current plan cannot express `[[R(x, a), S(x)], [S(b), T(x)], [T(c)]]`, `[[R(x), S(x), T(x)], [R(a)], [S(b)], [T(c)]]`, binary Free Join, factorized Free Join, or any mixed tuple-cover plan from the paper.

### P0-02: Validation forbids paper-valid multi-variable nodes

The paper explicitly allows a node to bind any number of variables as long as it has a cover. The implementation rejects every node whose `bind_vars.len() != 1` at `free_join.rs:21-24`, and the unit test `free_join.rs:97-107` asserts that multi-variable nodes are invalid.

This is a formal contradiction. A paper-valid node such as `[R(x, a), S(x)]` is impossible in the current IR because it binds `x` and `a` together through a cover tuple.

### P0-03: Atom partitioning is absent

The paper requires each atom's variables to be partitioned across subatoms. The current planner builds nodes only from `variable_order_ids` at `planner.rs:197-211`. It never records which atoms participate in which node, which variables belong to each atom subatom, or whether every atom variable is covered exactly once.

The current LFTJ runtime implicitly derives per-atom variable order with `atom_variables_in_plan_order` at `lftj_access.rs:342-348`, but that is not a Free Join partition. It is an access-order projection of an atom onto the global variable order.

### P0-04: Valid Free Join node invariants are not represented or checked

The paper's validity rules require no duplicate relation occurrence inside a node and at least one cover per node. The current `PlanNode` has no relation occurrence list and no subatoms, so it cannot check either rule. `FreeJoinPlan::validate` at `free_join.rs:13-28` checks only node IDs and singleton variable count.

This allows the system to label an arbitrary variable order as a Free Join plan without proving it computes the same query under the paper definition.

### P0-05: Covers do not exist

The paper's join phase depends on covers: iterate the cover, probe the other subatoms, and recurse. The implementation has no `Cover`, no `cover(phi_k)`, no chosen cover, and no cover validation. `lftj_runtime.rs:164-183` instead gathers all atom participants for one variable and leapfrog-intersects them.

This can emulate Generic Join over singleton subatoms, but it cannot emulate cover tuple iteration such as iterating `(x, a)` from `R(x, a)` and probing `S(x)`.

### P0-06: GHT is missing

The paper's GHT has tuple-key `iter` and `get` methods, relation metadata, and vars metadata. The repository has `LinearIter` and `TrieIter` in `sorted_trie.rs:43-61`, which expose only scalar `key`, `next`, `seek`, `at_end`, `open`, and `up`.

`LazyAccessIter` reads one encoded field component at the current depth in `lftj_iter.rs:206-292`. It cannot return tuple keys, cannot accept tuple probes, cannot represent a vector-of-tuples leaf, and cannot carry per-subatom vars metadata.

### P0-07: Build phase does not follow the paper

The paper build phase computes each atom's GHT schema from the sequence of subatoms in the plan. The repository builds `QueryImage` from storage access paths at `query_image/builder.rs:103-248`, then builds `LftjAtomPlan`s from existing durable index images at `lftj_access.rs:3-48`.

The build phase is therefore access-path dependent rather than plan-partition dependent. There is no implementation of schema generation like `[y_0, y_1, ..., []]` and no optimization that drops the final `[]` when the last subatom is a cover.

### P0-08: Join phase does not follow the paper

The paper join phase selects tries in the current node, iterates the cover tuple, constructs tuple keys for other tries from prior and current bindings, gets subtries, and recurses. The current executor opens all trie iterators containing the next variable and performs leapfrog intersection at `lftj_runtime.rs:173-203`.

This is not a Free Join join phase with nodes and covers. It is LFTJ over a global variable order.

### P0-09: `execute_free_join` is a misleading dispatcher name

`execute_free_join` at `lftj_runtime.rs:3-18` validates the plan and calls `execute_lftj`. The span name `bumbledb.query.free_join.dispatch` and the plan name make a formal claim that is not true.

The accurate current name would be `execute_lftj_variable_order` or `execute_lftj_plan`. If the product wants to retain the Free Join name, the implementation must make LFTJ an explicitly modeled special case of formal Free Join.

### P0-10: The planner is a variable-order planner, not a Free Join planner

`choose_variable_order` at `planner.rs:60-117` chooses a total variable order. It uses access-path predecessors from `planner.rs:119-195` and scoring from `planner_scoring.rs:28-129`. It never creates binary-derived Free Join nodes, never invokes `binary2fj`, never factors subatoms, and never computes covers.

The output `QueryPlan.variable_order` at `model.rs:203-213` accurately describes the current plan. The adjacent `free_join` field does not.

### P0-11: Binary Free Join and factorized Free Join are impossible

The optimization section's `binary2fj` creates nodes like `[[R(x, y), S(y)], [S(z), T(z)], ...]`. The current plan IR cannot store relation subatoms, cannot store `S(y)` and `S(z)` as different subatoms, and cannot move `T(x)` earlier by factorization.

This loses the paper's main unification claim: the system cannot represent both traditional binary hash join and Generic Join in one Free Join plan model.

### P0-12: Dynamic cover selection is absent

The paper later allows multiple covers and chooses the cover whose trie has the fewest keys. Current execution does not have a cover set and does not choose among cover subatoms. `LeapfrogState` at `lftj_leapfrog.rs:35-165` sorts iterators by current key for leapfrog search, not by key-count estimate and not as cover selection.

For singleton-variable Generic Join, leapfrog intersection can be efficient, but it is not the paper's dynamic cover interface.

### P0-13: COLT is not implemented

COLT is a GHT implementation with lazy map materialization and offset-vector leaves. The repository's `LazyAccessSlice` is a view over already materialized durable index bytes in a query image. `query_image/builder.rs:164-231` loads selected access keys into `RelationIndexImage.bytes`, and `lftj_iter.rs:93-292` traverses those bytes.

There is no `force()` operation that replaces an offset vector with a hash map, no `get(key)` that lazily materializes a child map, and no vector leaf of relation offsets as the GHT representation.

### P0-14: Access-path dependency violates the full conjunctive query model

The formal query model is logical. A valid atom projection should be executable by scanning if no better access path exists. Current LFTJ atom planning requires a durable access path whose leading fields can match the atom's variables in the global order. If none exists, `build_lftj_atom_plan` returns an internal error at `lftj_access.rs:44-47`.

The sharp edge is omitted leading fields. `lazy_access_shape` breaks when an index field is not present in the atom before any variable has been seen at `lftj_access.rs:103-113`. Therefore, for a relation with fields `[a, b]`, a query atom that constrains only `b` can fail unless an index beginning with `b` exists, even though the fact-set access path exists and a scan would be semantically valid.

### P0-15: Malformed or unsupported atom shapes are not cleanly rejected

The Rosetta Stone says malformed IR must be rejected at execution boundaries. The query validator checks IDs, names, and types in `normalize.rs:15-99`, but it does not reject repeated variables within one atom or duplicate field bindings.

`lazy_lftj_access_slice` silently refuses repeated variables via `atom_repeats_variable` at `lftj_access.rs:56-58` and `lftj_access.rs:268-278`, after which planning can fail with an internal LFTJ access error. That is neither a formal Free Join selection pushdown nor a user-facing invalid-query rejection.

### P0-16: Self-join handling is not formalized as relation occurrence aliasing

The paper says self-joins are handled by renaming relation names, so formal plan validity is over relation occurrences. The repository can execute duplicate base-relation atoms, as shown by `query_tests/atom_cache.rs:3-23`, because it builds one `LftjAtomPlan` per `NormAtom` over the same `RelationImage`.

However, `FreeJoinPlan` has no `AtomId` references in nodes and cannot distinguish relation occurrence aliases for the paper's `no two subatoms share the same relation` rule. `AtomId` exists at `free_join.rs:68-70` and `NormAtom.id` exists at `model.rs:82-93`, but the plan never uses them.

### P1-17: Query atoms conflate base relation filters, projections, and formal CQ atoms

`TypedRelationAtom` stores only explicitly bound fields at `query_ir.rs:84-106`, and `RelationAtomBuilder` allows fields to be added one by one at `query_builder.rs:275-363`. Omitted fields and `Wildcard` terms are not formalized as a derived atom schema before Free Join planning.

This can be a good product feature, but formal Free Join needs a normalized full CQ model: base relation selection/projection should be transformed into logical atoms with well-defined variables before subatom partitioning.

### P1-18: Comparisons are not pushed down into atom sources

The paper assumes selections are pushed down to base tables before the Free Join query is shown. The repository stores comparisons separately as `NormPredicate` at `model.rs:121-145` and evaluates ready comparisons during LFTJ recursion in `comparison_eval.rs:3-74`.

This is semantically acceptable for many queries, but it is not the paper's build-phase model. It also means range indexes are not used as true range scans by the LFTJ build path; they mostly influence access-path statistics and ordering.

### P1-19: Projection is embedded in the alleged Free Join plan

The paper Free Join plan computes the full join, with projection and aggregation after the full join. The repository stores `OutputPlan` inside `FreeJoinPlan` at `free_join.rs:7-10` and `free_join.rs:40-58`.

This matches the product's result-set sink architecture, but it muddies the formal boundary. A formal Free Join plan should be separable from output materialization.

### P1-20: Explain output makes a false formal claim

`QueryPlan::explain` prints `free_join_plan` and `free_join_node id=... bind_vars=...` at `query/explain.rs:77-84`. It does not show subatoms, atom partitions, covers, GHT schemas, access paths, or LFTJ atom plans.

This makes observability misleading. A reader cannot audit whether the plan is valid Free Join because the plan is not a Free Join plan.

### P1-21: Metrics and counters use trie/Free Join names without matching abstractions

`PlanCounters` includes `trie_intersections`, `trie_open`, `trie_seek`, and LFTJ counters at `metrics.rs:174-241`. `trie_intersections` is printed in explain at `explain.rs:99-100`, but no implementation path found increments it. `lftj_lazy_access_slices` at `metrics.rs:231-232` describes access slices, not COLT/GHT construction.

These names should be split between real LFTJ metrics and formal Free Join metrics after the refactor.

### P1-22: Query image scope is not derived from the physical plan

`query_image_scope_for_query` at `query/hash.rs:3-51` loads fields and access paths based on normalized atom required fields. A formal Free Join executor should derive required relation images, GHT schemas, and access layouts from the chosen Free Join plan.

The current scope can over-load or under-load relative to a future formal plan because it does not know subatom grouping, covers, or tuple-key requirements.

### P1-23: `AccessId` and `AtomId` are misleading in `free_join.rs`

`AtomId` and `AccessId` live next to `FreeJoinPlan` at `free_join.rs:68-74`, but `FreeJoinPlan` and `PlanNode` do not use either. `AccessId` is actually a dense storage access ID used by query images and storage schema, not a Free Join physical access node.

These names create the appearance of a richer Free Join plan interface that does not exist.

### P1-24: Manual plan validation is incomplete even for the current LFTJ special case

`FreeJoinPlan::validate` does not check duplicate variables, omitted variables, out-of-bounds variables, projected variables covered by the plan, or consistency with `NormalizedQuery`. `free_join_variable_order_ids` at `lftj_runtime.rs:97-107` simply extracts one variable per node.

The current planner constructs sane variable orders internally, but the type itself does not encode the invariants implied by its name.

### P2-25: Tests codify the wrong abstraction

Tests assert singleton nodes and Free Join labels rather than formal plan properties. Examples include `query_tests/basic.rs:217-224`, `query_tests/basic.rs:611-643`, and `query_tests/sinks_and_projection.rs:155-164`.

These tests will need to be rewritten. A correct formal implementation should have tests for subatom partitions, covers, invalid plans, GHT schemas, and paper-example plans.

### P2-26: The implementation lacks vectorized Free Join execution

The paper presents vectorized execution as a Free Join optimization. The current runtime is recursive scalar LFTJ. There is no `iter_batch` equivalent over Free Join cover tuples and no batch probing of sibling subatoms.

This is a performance and completeness gap, not the first correctness blocker.

## Required Breaking Changes

Introduce a formal query occurrence layer. `NormAtom` needs a stable atom occurrence identity, relation ID, alias identity for self-joins, ordered atom variables, and a clear representation for literals, inputs, wildcards, omitted fields, and pushed-down selections.

Split the plan types. Either rename the current plan to `LftjPlan` or replace it with a formal `FreeJoinPlan` containing nodes of subatoms. Keeping the current type name while adding side tables would preserve the misleading API and should be avoided.

Define explicit `Subatom` and `FreeJoinNode` types. A subatom should reference an atom occurrence, not just a relation name, and carry an ordered variable tuple. A node should carry its subatoms and validated cover candidates.

Move `OutputPlan` out of the formal Free Join plan. The executor can still receive output handling, but projection should not be part of the Free Join plan definition.

Implement full Free Join validation. Required checks include dense node IDs, atom subatom partition completeness, no duplicate variable within an atom subatom, no atom occurrence repeated in one node, non-empty node unless intentionally supporting zero-variable nodes, cover existence, cover correctness, variable availability, no duplicate bind events, and consistency with normalized query variables.

Make the current LFTJ/GJ plan an explicit special case. For each variable in the variable order, create a Free Join node containing singleton subatoms for every atom occurrence containing that variable. Then validate it under the same Free Join validator.

Implement a GHT interface with tuple keys. It must support relation or atom occurrence metadata, current vars metadata, `iter()` over tuple keys or leaf tuples, `get(tuple)`, and key-count estimates for dynamic cover choice.

Implement a real build phase. Given a formal Free Join plan, compute each atom occurrence's GHT schema from its subatoms and instantiate a GHT/COLT source. Existing durable sorted indexes can be used as an optimization only when they match the required schema.

Implement a real join phase. The executor must operate over Free Join nodes, iterate the selected cover tuple, probe sibling subatoms by tuple keys, replace tries with subtries, extend the binding with all newly bound variables, and recurse.

Add a scan-backed fallback source. Valid logical atoms must execute without requiring an index whose leading fields match the global variable order. A COLT over relation offsets is the natural fallback.

Handle repeated variables and duplicate fields at normalization. Either reject them as invalid query IR with user-facing errors, or lower them into selection predicates before formal Free Join planning.

Add binary and factorized planning. Implement the paper's `binary2fj` translation and `factor` optimization, then preserve the current variable-order/LFTJ planner as a Generic Join planning mode.

Implement cover selection. Store all covers per node and choose a cover during execution using exact or estimated key counts. This can initially be deterministic and simple.

Rework explain and counters. Explain output must show atoms, subatoms, covers, GHT schemas or access sources, and whether a node used the LFTJ special case. Counters must distinguish LFTJ scalar intersections from Free Join cover tuple iterations and probes.

## Suggested Implementation Sequence

1. Rename the current plan and executor internally to `LftjPlan` and `execute_lftj_plan`, or add a temporary compatibility wrapper that makes the misnaming explicit in comments and explain output.

2. Add formal Free Join IR types without changing execution: `AtomOccurrenceId`, `Subatom`, `FreeJoinNode`, `Cover`, and a validator. Write unit tests against only the validator.

3. Lower the existing variable-order plan into a validated singleton-subatom Free Join plan. This proves the current LFTJ shape is a legitimate Generic Join special case when access sources support it.

4. Add a GHT/COLT source abstraction over `RelationImage`. Start with a simple scan/offset-vector implementation that is correct for every atom projection, then add durable-index-backed sources as an optimization.

5. Implement the node-and-cover Free Join recursive executor. Keep the existing LFTJ executor as a fast path for plans whose nodes are all singleton-variable Generic Join nodes.

6. Replace planner output with real Free Join plans. First emit Generic Join singleton plans. Then add `binary2fj`, factorization, and dynamic cover selection.

7. Move query-image scoping after formal planning. Scope must be based on the chosen plan's source needs, not only on normalized atom field sets.

8. Update explain output and counters to report the formal plan. Remove or rename misleading `free_join_node bind_vars` output.

9. Add differential and property tests before removing old assumptions. Treat every current singleton-node assertion as a migration target, not as a permanent invariant.

## New Tests/Proof Obligations

Add validator tests for valid paper examples: clover binary plan, clover Generic Join plan, factorized clover plan, triangle Generic Join plan, and chain binary-derived plan.

Add validator rejection tests for missing atom variable partitions, duplicate partition assignment, duplicate atom occurrence in one node, node without cover, cover missing a newly introduced variable, duplicate variable in a subatom, and out-of-order unavailable probe variables.

Add build-phase tests that assert GHT schemas per atom for the paper examples, including the final `[]` leaf rule and the cover-vector optimization.

Add join-phase tests that execute a multi-variable cover node, such as `[R(x, a), S(x)]`, and prove it does not degenerate into one-variable LFTJ.

Add equivalence tests where binary Free Join, Generic Join singleton Free Join, and factorized Free Join all return the same duplicate-free projection set on the clover, triangle, and chain queries.

Add set-semantics tests with duplicate witnesses: existential variables must not multiply projected facts, and exact duplicate inserts must remain no-ops.

Add access-completeness tests for atoms that bind only non-leading fields, atoms with omitted fields, wildcard fields, and no useful declared index. These must execute by scan/COLT rather than fail with an internal LFTJ access error.

Add repeated-variable and duplicate-field IR tests. They must either lower to explicit equality selections or fail as `InvalidQuery`, never as an internal LFTJ planning error.

Add self-join alias tests where two atom occurrences over the same base relation appear in the same query and where the Free Join validator treats them as distinct occurrences.

Add comparison pushdown tests. Define which comparisons are part of atom source construction and which remain residual predicates, then test both behavior and explain output.

Add explain golden tests that show subatoms, partitions, covers, chosen cover, atom source type, and whether LFTJ fast path was selected.

Add differential property tests against the existing reference evaluator for randomly generated small set-valued conjunctive queries, with projections, self-joins, static atoms, literals, inputs, and comparisons.

Add proof obligations in code comments or design docs: formal plan validation implies each output binding satisfies every atom; every satisfying full binding is reachable by exactly one path through the plan under set semantics; projection sink canonicalization implements Rosetta Stone set output semantics.

## Open Questions

Should Bumbledb v4 implement the full paper Free Join model, or should the product contract be revised to say the retained executor is LFTJ/GJ with lazy durable access slices?

Should relation atoms in the public typed IR denote full base-relation atoms with explicit wildcards, or derived projected/selected atoms after normalization?

Should repeated variables in one relation atom be supported as field-equality selection, or rejected as invalid IR to match the paper's distinct-variable atom assumption?

How should string and bytes tuple keys be represented in a real GHT, given current `EncodedOwned` only supports fixed widths of 1, 8, and 16 bytes?

Should dynamic cover choice be mandatory for the first formal Free Join implementation, or can the first version designate one static cover and add runtime cover selection later?

Should range comparisons be pushed into atom source construction, especially when a range index exists, or remain residual predicates outside formal Free Join?

Should query images be plan-specific and rebuilt per chosen Free Join plan, or should they remain broader snapshot-local caches that expose enough raw relation data for multiple possible plans?

What is the intended public/explain terminology: should users see `FreeJoinPlan`, `LftjPlan`, or both with an explicit statement that LFTJ is a special execution strategy for singleton-subatom Free Join plans?
