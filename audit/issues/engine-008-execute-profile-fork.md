# engine-008: `execute` and `profile` are two copies of one protocol — and their fast-lane predicates disagree

- **Severity:** high
- **Tree:** engine
- **Status:** OPEN
- **Source:** audit/engine.md F8
- **Depends on:** engine-001 (the pipeline sum is the shared shape), engine-031 (fast lane parsed once)

## The bug

`run_bound` (`execute.rs:78-144`) and `profile` (`introspect.rs:202-377`) each hand-roll the dispatch: empty short-circuit copied (`execute.rs:94` vs `introspect.rs:214`), reach copied (`execute.rs:162` vs `introspect.rs:263`), interiors-preamble copied (`introspect.rs:300-302` re-implements what `run_rules` does), and the key-probe fast lane copied WRONG:

```rust
// execute.rs:100-107 — requires plain-var finds
matches!(self.body.rules(),
    [PreparedRule::KeyProbe(KeyProbeRule { key_probe_finds: Some(_), .. })])
// introspect.rs:221 — any single key probe, aggregate/measure probes included
if self.interiors.is_empty() && matches!(self.body.rules(), [PreparedRule::KeyProbe(_)]) {
```

So a single-rule aggregate key probe takes the sink path under `execute` and the direct path under `profile` — ANALYZE observes a different access path than execution uses.

## Why it's wrong

Two representations of "which access path is this?" — the body enum, and a forest of `if`s that disagree (Insight 2: duplicated logic WILL drift, and here it already has). The counted path exists to explain the real path; a widened predicate on the counted copy makes introspection lie about execution (Insight 1).

## The fix

Per `audit/CONTRACT.md §C3`: ONE execution protocol parameterized by `Counters`; profile IS `execute` with `CountingCounters`/`ReachCounters` plugged in.

- The access-path decision is made ONCE, at build, as pipeline data (engine-001's arms + engine-031's parsed direct lane). `run_bound` dispatches by matching the pipeline; `profile` calls the same dispatch with counting counters and assembles `ExecutionStats` from what the counters saw (engine-012's stats sum).
- The direct key-probe lane is one predicate, tested nowhere at run time: `Pipeline::Cq { interiors: [], rules: [KeyProbe { key_probe_finds: Some(_) }] }` parsed at build into its own lane (or a build-computed property consumed by both callers).
- Behavior note: today's `profile` fast lane is WIDER than execute's. The fix must converge both on execute's predicate (plain-var finds only) — this changes profile's counted access path for single aggregate key probes to match execution. That is the bug being fixed; capture it in the new lock below.

## Acceptance criteria

- [ ] One protocol: `rg -n 'interiors\.is_empty\(\)' crates/bumbledb/src/api/prepared/introspect.rs` → no matches; profile contains no duplicate of the run loop (`rg -n 'run_rule\(|run_derived\(' crates/bumbledb/src/api/prepared/introspect.rs` shows calls into the SHARED protocol, not a parallel loop body).
- [ ] New lock: a test executing AND profiling a single-rule aggregate key probe, asserting both report the same access path and identical answers (pins the converged predicate).
- [ ] Unchanged tests: all existing execute/profile tests green with zero assertion edits EXCEPT any test that pinned the divergent profile lane (list such edits in the commit message; they are the bug).
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`. Rec queries profile through the SAME seam (no "rec skips profile" — see engine-011's closure.rs note; the bench profile skip for rec families is removed if it only existed because of this fork; check `crates/bumbledb-bench/src/closure.rs:502`).

## Constraints

- ANALYZE semantics stay: the query really executes under counting. `INTROSPECTION_VERSION` increments iff rendered/structured output changes shape.
- Lands after engine-001; engine-031 may fold into this change.
