# exec-004: `Executor.pipe: Option` — take/put the pipeline tables back

- **Severity:** medium
- **Tree:** exec
- **Status:** FIXED(bab484e3)
- **Source:** audit/plan-exec.md F7
- **Depends on:** none (executor layout; textual overlap with exec-005 in `PipeTables`)

## The bug

`crates/bumbledb/src/exec/run.rs:637-639` — "The one executor," then `pipe: Option<PipeTables>`. Construction (`run/execute.rs:306`): `pipe: (plan.nodes().len() >= 2).then(|| PipeTables::of(plan))`. `execute` rematches (`:380-384`); `run_pipeline` does `self.pipe.take().expect("dispatched on Some")` and stuffs it back at `:441`. Single-node with `Some(pipe)` and multi-node with `None` are representable. Same split-borrow tax as engine-009 (`run_reach` re-matching Reach because interiors stole the other borrow).

## Why it's wrong

A flag that is only meaningful in one arm, stored on the product, then stolen so the other fields of `self` can be borrowed (Insight 4; engine F9). The expect is the typechecker. One-node vs multi-node is a construction fact, not an Option that execute re-discovers.

## The fix

Per `audit/CONTRACT.md` §C1 (trusted layer is a sum):

```rust
enum Drive {
    Leaf,                 // one node: run_node at 0
    Pipeline(PipeTables), // ≥2 nodes
}
```

`execute` matches once and stays matched. No take/put. `PipeTables::of` is the Pipeline constructor's body.

## Acceptance criteria

- [ ] Gone: `rg -n 'pipe: Option' crates/bumbledb/src/exec/run.rs` → no matches; `rg -n 'pipe.take\(\)' crates/bumbledb/src/exec` → no matches.
- [ ] Unchanged tests: single-node and multi-node join suites green unchanged (`cargo test -p bumbledb --lib exec::run`).
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- Behavior identical: one-node still `run_node` at 0; ≥2 nodes still pipeline with the same `PipeTables` contents (exec-005 may reshape `carried_col` in the same change or after). Batch size still a number, not a mode.
