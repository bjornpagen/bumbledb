# 11: Cutover And Code Deletion

**Goal**
- Delete old query hot paths and leave one coherent v2 architecture.

**Code To Delete**
- Current candidate-set WCOJ executor.
- LMDB encoded prefix scan usage from production query execution.
- `BTreeSet<EncodedValue>` candidate-domain intersections.
- Old `PlannedAtom`-first explain if superseded by Free Join plan explain.
- Public/internal functions that exist only for old query execution.
- Any old benchmark interpretation that describes the deleted architecture as current.

**Search Gates**
```sh
rg "candidate_values_for_variable|collect_atom_candidates|BTreeSet<EncodedValue>" crates/bumbledb-lmdb/src
rg "scan_encoded_index_prefix" crates/bumbledb-lmdb/src/query.rs
rg "execute_atoms|execute_atom|ChosenAccess" crates/bumbledb-lmdb/src
```

Expected result after cutover: no production hot-path matches.

**Replacement Public Explain**
```rust
pub struct QueryPlan {
    pub free_join: FreeJoinPlanSummary,
    pub counters: ExecutionCounters,
    pub recommendations: Vec<IndexRecommendation>,
}

pub struct FreeJoinPlanSummary {
    pub nodes: Vec<PlanNodeSummary>,
}
```

**Migration Policy**
- No persisted compatibility layer for the experimental query image is required.
- Existing LMDB databases may require ETL if durable segment layout changes.
- Docs must state that schema/storage layout changes require rebuild/ETL.

**Hardening Tasks**
- Remove dead exports.
- Remove dead tests tied to old explain shape.
- Add replacement tests for new explain shape.
- Run full property/differential suite.
- Run crash tests if segment storage changed.
- Rebaseline benchmark docs.

**Passing Criteria**
- One query executor architecture remains.
- Query execution uses QueryImage, Free Join plan IR, and sorted/hash indexes.
- No LMDB cursor creation occurs in query inner loops.
- No full-row decode occurs in query inner loops.
- Scale-10000 generated benchmark meets the current v2 gates or has documented blockers.
- `docs/ROSETTA_STONE.md`, `docs/BENCHMARKS.md`, and tracing docs reflect the new architecture.

**Non-Goals**
- Do not preserve old internal APIs for external compatibility.
- Do not keep old code for comparison; git history is the comparison path.
