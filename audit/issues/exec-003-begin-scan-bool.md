# exec-003: Sink scan/skip protocol is bools plus `unreachable!`

- **Severity:** medium
- **Tree:** exec
- **Status:** OPEN
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

`run/scan_table.rs:57`: `if !sink.begin_scan(&scan) { return None; }`. Projection `emit_batch` (`sink/projection/sink.rs:26-67`) takes `stop_on_skip` as a second protocol: when true, return SkipSuffix after the first row; when false, consume the batch. Four states of two bools; calling `scan_run` after a decline compiles and panics.

## Why it's wrong

Insight 6: the executor asks, the sink answers with a discarded bit, later calls re-validate. Insight 4: two bools admit declined+scan_run and skip-on-a-sink-that-cannot-skip. The plan already has `SuffixSkip` per node and the sink already has `SkipCapability` — `stop_on_skip` is a third encoding of D2 license.

## The fix

Per `audit/CONTRACT.md` §C1 (trusted protocol is a sum):

```rust
enum ScanOffer { Declined, Open }
fn begin_scan(...) -> ScanOffer;
fn scan_run(...);  // only reached on Open (match, or typestate)
fn emit_batch(&mut self, batch: &LeafBatch<'_>) -> Flow;
```

D2: the executor already knows the leaf node's `SuffixSkip` and the sink's `SkipCapability`. Do not re-pass that as `stop_on_skip`. Projection's first-emit SkipSuffix stays the sink's reply; the executor unwinds when the *node* is Licensed.

## Acceptance criteria

- [ ] Gone: `rg -n 'begin_scan == true' crates/bumbledb/src/exec` → no matches; `rg -n 'stop_on_skip' crates/bumbledb/src/exec` → no matches; `rg -n 'fn begin_scan.*-> bool' crates/bumbledb/src/exec` → no matches.
- [ ] Unchanged tests: scan-pushdown and D2 skip suites green (`cargo test -p bumbledb --lib exec::`); answers identical.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- Scan-fold still declines under seen-set / measures / Pack / group-varying leaves (same predicates, not a bool return). D2 still projection-only. No change to when a skip is legal.
