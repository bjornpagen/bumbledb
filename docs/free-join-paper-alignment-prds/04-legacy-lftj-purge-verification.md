# PRD 04: Legacy LFTJ Purge Verification

## Purpose

Verify that the old singleton-variable LFTJ/GJ-style implementation remains deleted and cannot masquerade as paper Free Join. This PRD is intentionally a negative gate: future work must rebuild Generic Join-like behavior only through the formal Free Join IR from PRD 03.

## Dependencies

- PRD 03.

## Scope

- Public and crate-private exports from `bumbledb-lmdb`.
- Query/explain/planner module tree.
- Tests and benchmark scaffolding.
- Stale names in code and docs.

## Required Changes

- Keep old `free_join.rs`, old LFTJ runtime, old query planner, old query image code, and old singleton `bind_vars` plan deleted.
- Do not reintroduce `execute_free_join` as a wrapper over LFTJ.
- Do not reintroduce a plan type named `FreeJoinPlan` unless it has subatoms, atom partitions, nodes, and covers from PRD 03.
- If a Generic Join baseline is rebuilt later for benchmarks, build it as a formal singleton-subatom Free Join plan, not as the old LFTJ implementation.
- Keep explain output absent until a formal plan/executor exists, or label any placeholder as unavailable.

## Technical Direction

- This PRD should mostly be grep and compile validation.
- Prefer no compatibility shims.
- If PRD 03 needs helper types, place them in new formal plan modules with paper vocabulary.
- Any future scalar intersection fast path must sit behind the formal GHT/COLT source abstraction.

## Non-Goals

- Do not implement Free Join execution here.
- Do not implement a Generic Join benchmark baseline here.
- Do not restore the old query-image or sorted-trie modules.

## Acceptance Criteria

- No production Rust file contains the old singleton `FreeJoinPlan`/`PlanNode { bind_vars }` model.
- No production Rust file contains `execute_free_join` as an LFTJ dispatch wrapper.
- No production explain output contains `free_join_node` or singleton `bind_vars` plan output.
- No old query-image, LFTJ runtime, sorted-trie, or v4 query tests remain.
- Formal Free Join validator tests from PRD 03 remain the only source of Free Join plan-shape authority.

## Required Tests

- A grep-backed test or script check for stale old LFTJ symbols.
- Formal Free Join validator tests still pass.
- Workspace check still passes.

## Validation Commands

```text
cargo fmt --all --check
cargo check --workspace --all-targets --all-features
rg "FreeJoinPlan|free_join_node|execute_free_join|bind_vars|lftj_runtime|lftj_access|sorted_trie|query_image" crates
```

The final `rg` must return no stale production implementation. It may return deliberate future PRD prose only when run over `docs/`.
