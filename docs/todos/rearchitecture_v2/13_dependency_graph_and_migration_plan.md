# 13: Dependency Graph And Migration Plan

**Goal**
- Define the strict implementation sequence and dependency graph for the full v2 rearchitecture so the rebuild does not collapse into another partial executor experiment.

**Dependency Graph**
```text
00 Architecture/RCA
  -> 01 QueryImage
    -> 02 Columnar RelationImage
      -> 03 SortedTrieIndex
        -> 04 LeapfrogTriejoinExecutor
      -> 06 HashTrieAndHybridNodes
    -> 08 OptimizerAndStatistics
  -> 12 QueryNormalizationAndRuntimeSpecialization
    -> 05 FreeJoinPlanIR
      -> 04 LeapfrogTriejoinExecutor
      -> 06 HashTrieAndHybridNodes
      -> 07 FactorizedProjectionAndAggregation
  -> 09 DurableSegmentsAndSnapshots
  -> 10 BenchmarkGatesAndTesting
  -> 11 CutoverAndCodeDeletion
```

**Current Completion Snapshot**
| PRD | Milestone | Status | Acceptance Gate |
|---|---:|---|---|
| `00_architecture_and_rca.md` | 0 | Complete | RCA and replacement architecture documented |
| `01_query_image.md` | 1 | Complete | QueryImage build/cache tests pass |
| `02_columnar_relation_image.md` | 1 | Complete | Encoded column APIs and row ranges tested |
| `03_sorted_trie_index.md` | 2 | Complete | Sorted trie iterator/stat tests pass |
| `04_leapfrog_triejoin_executor.md` | 5 | Complete | Query execution routes through LFTJ over QueryImage/sorted tries |
| `05_free_join_plan_ir.md` | 4 | Complete | Free Join plan IR validates and explains node/subatom shapes |
| `06_hash_trie_and_hybrid_nodes.md` | 6 | Complete | Hash trie probe/count/row-retaining primitives tested |
| `07_factorized_projection_and_aggregation.md` | 7 | Complete | Output sinks avoid full binding storage where valid |
| `08_optimizer_and_statistics.md` | 8 | Complete | Stats-backed candidate planning and explain traces tested |
| `09_durable_segments_and_snapshots.md` | 9 | Complete | QueryImage builds from durable visible segment metadata |
| `10_benchmark_gates_and_testing.md` | 10 | Complete | CI benchmark scripts, markdown parser/output, and counter gates exist |
| `11_cutover_and_code_deletion.md` | 10 | Complete | `scripts/check-cutover.sh` passes; legacy hot paths removed |
| `12_query_normalization_and_runtime_specialization.md` | 3 | Complete | Planner/executor consume `NormalizedQuery` plus encoded inputs |
| `13_dependency_graph_and_migration_plan.md` | 13 | Complete | PRD map, stop conditions, and validation script are present |

**Validation Script**
- `scripts/check-prd-map.sh` verifies that every PRD file is linked from the suite README and represented in this dependency/status document.
- This is intentionally lightweight so it can run in CI and as part of local stage-boundary checks.

**Stage Boundary Command Set**
```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo check --manifest-path fuzz/Cargo.toml
scripts/check-cutover.sh
scripts/check-prd-map.sh
cargo run -p bumbledb-bench --release -- --scale 500 --repeats 3 --format markdown
```

For formal benchmark gates, use:

```sh
scripts/bench-quick.sh
scripts/bench-extreme.sh
scripts/bench-focused.sh
```

**Milestone 1: Runtime Image Without Query Cutover**
- Implement `QueryImage`, `RelationImage`, and encoded columns.
- Build images from current durable LMDB state.
- Keep existing query executor untouched during this milestone.
- Add image build/cache diagnostics.

Rust target:
```rust
pub struct RuntimeImages {
    cache: QueryImageCache,
}

impl RuntimeImages {
    pub fn image_for_read(
        &self,
        txn: &ReadTxn<'_>,
        schema: &StorageSchema,
    ) -> Result<Arc<QueryImage>> {
        self.cache.get_or_build(txn, schema)
    }
}
```

Passing criteria:
- QueryImage tests pass.
- Existing query tests remain green during image introduction.
- Image build can be benchmarked independently.
- Image row counts match storage diagnostics.

**Milestone 2: Sorted Trie Primitive**
- Implement sorted trie index and iterator.
- Add unit tests independent of Datalog execution.
- Add microbenchmarks for `seek`, `next`, `open`, and `up`.

Rust target:
```rust
pub fn assert_trie_iter<I: TrieIter>(iter: &mut I) {
    iter.open();
    while !iter.at_end() {
        let _key = iter.key();
        iter.next();
    }
    iter.up();
}
```

Passing criteria:
- No LMDB dependency in trie iterator tests.
- No allocation in normal iterator movement.
- Distinct-key iteration is correct for duplicate-heavy data.

**Milestone 3: Normalized Query IR**
- Introduce `NormalizedQuery` between `TypedQuery` and physical planning.
- Lower all field/relation names to numeric IDs.
- Encode literals and input-compatible constants.
- Attach output and aggregate demand.

Rust target:
```rust
let typed = parse_and_typecheck(schema.descriptor(), source)?;
let normalized = normalize_query(txn, schema, &typed)?;
assert!(normalized.atoms.iter().all(|atom| atom.relation.0 < schema.relation_count()));
```

Passing criteria:
- Existing typechecker tests remain unchanged.
- Normalization tests prove ID mapping, literal encoding, and repeated-variable handling.
- Planner no longer consumes raw `TypedRelationAtom` directly after this milestone.

**Milestone 4: Free Join Plan IR**
- Add `FreeJoinPlan`, `PlanNode`, and `SubAtom`.
- Manually construct plans in tests before optimizer integration.
- Ensure pure LFTJ and binary/probe-like shapes are expressible.

Rust target:
```rust
let plan = FreeJoinPlan::builder()
    .node(NodeImpl::SortedLeapfrog, [var!(a)], [subatom!(EdgeAB(a)), subatom!(EdgeAC(a))])
    .node(NodeImpl::SortedLeapfrog, [var!(b)], [subatom!(EdgeAB(b)), subatom!(EdgeBC(b))])
    .node(NodeImpl::SortedLeapfrog, [var!(c)], [subatom!(EdgeAC(c)), subatom!(EdgeBC(c))])
    .aggregate_count(var!(a))
    .build()?;
```

Passing criteria:
- Plan validation rejects unbound dependencies.
- Plan validation rejects unsupported subatom partitions.
- Explain can render plan nodes before execution exists.

**Milestone 5: LFTJ Query Cutover For Conjunctive Queries**
- Execute normalized conjunctive queries through real LFTJ over sorted tries.
- Keep old executor available only behind tests until parity is proven, then remove it in milestone 9.
- No `BTreeSet` candidate-domain construction in the new executor.

Rust target:
```rust
let mut executor = LeapfrogTrieJoin::compile(&image, &normalized, &plan)?;
let output = executor.execute()?;
```

Passing criteria:
- All current query tests pass through the new executor.
- Differential reference tests pass.
- `triangle_count` no longer reports prefix-scan openings as the main work unit.
- Query counters report trie iterator operations.

**Milestone 6: Hash Trie And Hybrid Plans**
- Add hash trie indexes and hybrid node execution.
- Route exact lookup and static predicate queries through hash/probe nodes where cheaper.
- Use Free Join IR, not a second executor.

Rust target:
```rust
let node = PlanNode {
    implementation: NodeImpl::Hybrid,
    bind_vars: vec![var!(posting)],
    subatoms: vec![subatom!(PostingTag(tag, posting)), subatom!(Posting(id))],
    payload: PayloadDemand::project([var!(posting), var!(account)]),
};
```

Passing criteria:
- `tag_lookup_join` reduces iterator work materially at scale 10000.
- `supplier_nation_orders` reduces scanned rows materially at scale 10000.
- Small selective queries recover toward pre-WCOJ latency.

**Milestone 7: Factorized Aggregation**
- Add aggregate sinks and count multiplicity support.
- Count queries do not emit all full bindings when not needed.
- Group keys remain encoded until final output.

Rust target:
```rust
pub enum SinkKind {
    Project(Vec<VarId>),
    Count { group: Vec<VarId>, counted: VarId },
    Sum { group: Vec<VarId>, value: VarId },
}
```

Passing criteria:
- `triangle_count` materializes no triangle bindings.
- Aggregate overflow semantics remain tested.
- Aggregate outputs match current behavior.

**Milestone 8: Optimizer And Stats**
- Replace manual/heuristic planning with stats-backed Free Join planning.
- Add exact trie stats and fanout stats.
- Emit plan alternatives in explain.

Rust target:
```rust
let candidates = optimizer.enumerate(&normalized)?;
let chosen = candidates.into_iter().min_by_key(|plan| plan.cost_key()).unwrap();
```

Passing criteria:
- Chosen plans are deterministic.
- Plan estimates are shown in explain.
- Bad plans can be explained by stats, not hidden heuristics.

**Milestone 9: Durable Segment Storage**
- Move durable layout toward column/index segments.
- QueryImage builds from segment metadata and bytes.
- Preserve LMDB durability and crash semantics.

Rust target:
```rust
pub trait SegmentStore {
    fn visible_segments(&self, tx_id: u64, relation: RelationId) -> Result<Vec<SegmentDescriptor>>;
    fn read_column(&self, descriptor: &ColumnSegmentDescriptor) -> Result<Bytes>;
}
```

Passing criteria:
- Reopen tests pass.
- Crash tests pass or are updated with equivalent segment crash tests.
- QueryImage no longer requires scanning covering index keys to build relation images.

**Milestone 10: Cutover And Deletion**
- Delete old hot paths.
- Update architecture docs and benchmark docs.
- Rebaseline benchmark gates.

Passing criteria:
- Search gates in `11_cutover_and_code_deletion.md` pass.
- One query execution architecture remains.
- Full benchmark suite runs through v2.

**Milestone 11: Dependency Closure**
- Freeze the dependency graph and actual implementation status after the v2 cutover.
- Keep this file and `README.md` in sync whenever new rearchitecture PRDs are added.
- Validate PRD map coverage in CI.

Passing criteria:
- `scripts/check-prd-map.sh` passes.
- Every PRD file is linked from `docs/todos/rearchitecture_v2/README.md`.
- Every PRD file appears in the status table above.
- Stop conditions and benchmark commands are explicit.

**Migration And ETL Policy**
- No compatibility layer is required for experimental v2 internal layouts.
- Schema changes require full ETL into a new database.
- Storage encoding or segment-layout changes require a storage format bump and full ETL.
- Existing git history is the comparison path for old internals; old code should not remain in-tree for comparison.
- `docs/ETL.md` and `docs/ROSETTA_STONE.md` are the normative migration references.

**Global Stop Conditions**
- If a milestone makes scale-2000 generated benchmarks more than 2x slower without a documented reason, stop and investigate before continuing.
- If a milestone adds a second production executor path, stop and redesign the stage.
- If a milestone introduces logical row decode in an inner loop, stop and redesign the stage.
- If a milestone cannot explain its counters, stop and add observability before optimizing further.

**Passing Criteria**
- This dependency graph is linked from the suite README.
- Every PRD can be mapped onto the milestone sequence.
- The migration avoids a permanent dual-path architecture.
- Each milestone has explicit code-level acceptance gates.
- `scripts/check-prd-map.sh` passes.

**Non-Goals**
- Do not treat this document as a substitute for the detailed PRDs.
- Do not use this sequence to preserve old internals beyond parity validation.
