# exec-008: `LeafPrecompute.single: bool` plus empty vecs for the other arm

- **Severity:** medium
- **Tree:** exec
- **Status:** FIXED(44db7ad6)
- **Source:** audit/plan-exec.md F11
- **Depends on:** none (leaf shape; parallel-safe with exec-004)

## The bug

`crates/bumbledb/src/exec/run.rs:618-621,736-741` and `run/leaf_precompute.rs:24-44`: `single: bool` means the last node is a one-subatom leaf eligible for fast paths. When false, `residual_sources`, `scan_residuals`, `const_residuals`, and `row` are empty and must not be read. `run_node.rs:35` tests `self.leaf_single` then trusts the vecs. A flag plus four payloads.

## Why it's wrong

Insight 4: bool + ghost fields admit `single == true` with empty sources and `single == false` with leftover precompute. The constructor already knew the arm (`if !single { return Self { single, residual_sources: Vec::new(), ... } }`) and stored it as a flag.

## The fix

Per `audit/CONTRACT.md` §C1:

```rust
enum LeafShape {
    Generic,
    Fast {
        residual_sources: Vec<(Source, Source)>,
        scan_residuals: Vec<(CmpOp, Source, Source)>,
        const_residuals: Vec<(CmpOp, usize, usize)>,
        row: Vec<u64>,
    },
}
```

`run_node` matches. Empty-vec-as-None dies. Decline predicates (anti-probes, membership, word/Allen/duration residuals, width > 1) stay — they mint `Generic`.

## Acceptance criteria

- [ ] Gone: `rg -nw 'leaf_single' crates/bumbledb/src/exec` → no matches; `rg -n 'single: bool' crates/bumbledb/src/exec/run.rs` → no matches.
- [ ] Unchanged tests: leaf fast-path, scan-pushdown, and generic-leaf suites green unchanged.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- Fast-path eligibility unchanged (conservative decline is load-bearing). No new fast path fires; no old one is lost.
