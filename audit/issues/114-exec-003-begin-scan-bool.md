# exec-003: Sink scan/skip protocol is bools plus `unreachable!`

- **Severity:** medium
- **Tree:** exec
- **Status:** FIXED(87f44004)
- **Source:** audit/plan-exec.md F6
- **Depends on:** none (trait shape; parallel-safe with exec-001/002)

## The bug

`crates/bumbledb/src/exec/run.rs:105-146`:

```rust
fn emit_batch(&mut self, batch: &LeafBatch<'_>, stop_on_skip: bool) -> Flow;
fn begin_scan(&mut self, scan: &LeafScan<'_>) -> bool { false }
fn scan_run(...) { unreachable!("scan_run without begin_scan == true"); }
fn end_scan(...) { unreachable!("end_scan without begin_scan == true"); }
```

`run/scan_table.rs:57`: `if !sink.begin_scan(&scan) { return None; }`. Calling `scan_run` after a decline compiles and panics.

`stop_on_skip` is a second protocol stuffed into the same trait. The executor already has the two real facts (`run_node.rs:616-619`):

```rust
let stop_on_skip = plan.nodes()[node_idx].suffix_skip
    == SuffixSkip::Licensed
    && sink.skip_capability() == SkipCapability::Licensed;
let batch_flow = sink.emit_batch(&batch, stop_on_skip);
```

When `false` (node `Forbidden` — the leaf binds sink-relevant variables), projection **consumes the entire batch**. When `true`, it stops at the first SkipSuffix. Four states of two bools; `scan_run` after decline is representable.

## Why it's wrong

Insight 6: the executor asks, the sink answers with a discarded bit, later calls re-validate. Insight 4: two bools admit declined+scan_run. The plan already has `SuffixSkip` per node and the sink already has `SkipCapability` — `stop_on_skip` is those two sums AND-ed into a bool.

**Unsafe non-fix (rejected):** dropping the parameter and having projection always return `SkipSuffix` after the first row. That would skip remaining batch rows on `Forbidden` nodes — those rows *are* sink-relevant. Silent answer-set change. The sink must keep iterating when the node is `Forbidden`.

## The fix

Per `audit/CONTRACT.md` §C1 (trusted protocol is a sum):

```rust
enum ScanOffer { Declined, Open }
fn begin_scan(...) -> ScanOffer;
fn scan_run(...);  // only reached on Open (match, or typestate)
fn end_scan(...);

fn emit_batch(&mut self, batch: &LeafBatch<'_>) -> Flow;           // consume every row
fn emit_batch_until_skip(&mut self, batch: &LeafBatch<'_>) -> Flow; // Licensed projection only
```

Executor matches the two existing sums, never a bool:

```rust
match (node.suffix_skip, sink.skip_capability()) {
    (SuffixSkip::Licensed, SkipCapability::Licensed) => sink.emit_batch_until_skip(&batch),
    _ => sink.emit_batch(&batch),
}
```

Default `emit_batch_until_skip` = `emit_batch` (aggregates inherit; they already `debug_assert!(!stop_on_skip)`). Projection overrides. `stop_on_skip: bool` dies.

Equivalently pass `SuffixSkip` as the parameter instead of `bool` — still a sum, still the Forbidden-must-consume-full-batch law. Do **not** pass a bool. Do **not** have the sink always stop at first emit.

## Acceptance criteria

- [ ] Gone: `rg -n 'begin_scan == true' crates/bumbledb/src/exec` → no matches; `rg -n 'stop_on_skip' crates/bumbledb/src/exec` → no matches; `rg -n 'fn begin_scan.*-> bool' crates/bumbledb/src/exec` → no matches.
- [ ] Unchanged tests: scan-pushdown and D2 skip suites green (`cargo test -p bumbledb --lib exec::`); answers identical — Forbidden-node batches still emit every surviving row; Licensed projection still SkipSuffix on first emit.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- Scan-fold still declines under seen-set / measures / Pack / group-varying leaves (same predicates, not a bool return). D2 still projection-only. **No change to when a skip is legal** and **no change to which batch rows a Forbidden node delivers**.
