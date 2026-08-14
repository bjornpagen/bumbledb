# engine-008: `execute` and `profile` are two copies of one protocol — and their fast-lane predicates disagree

- **Severity:** high
- **Tree:** engine
- **Status:** FIXED
- **Source:** audit/engine.md F8
- **Depends on:** engine-001 (the pipeline sum is the shared shape). engine-031 is the key-probe rematch that this protocol unification makes deletable — land 031 after, not as a cycle.

## The bug

`run_bound` (`execute.rs:78-144`) and `profile` (`introspect.rs:202-377`) each hand-roll the dispatch: empty short-circuit copied (`execute.rs:94` vs `introspect.rs:214`), reach copied (`execute.rs:162` vs `introspect.rs:263`), interiors-preamble copied (`introspect.rs:300-302` re-implements what `run_rules` does), and the key-probe fast lane copied WRONG:

```rust
// execute.rs:100-107 — requires plain-var finds
matches!(self.body.rules(),
    [PreparedRule::KeyProbe(KeyProbeRule { key_probe_finds: Some(_), .. })])
// introspect.rs:221 — any single key probe, aggregate/measure probes included
if self.interiors.is_empty() && matches!(self.body.rules(), [PreparedRule::KeyProbe(_)]) {
```

Profile's key-probe arm then calls `execute_args` (so the **query body** runs execute's path) and **fabricates** key-probe-shaped stats (`nodes: []`, `key_probe: Some(hit)`) for every single `KeyProbe`. A single-rule aggregate/measure key probe therefore executes via the sink path (`key_probe_finds` is `None`, so `run_bound` does not take the direct lane) while ANALYZE reports a key-probe access path. The counted surface lies about the path that ran.

## Why it's wrong

Two representations of "which access path is this?" — the body enum, and a forest of `if`s that disagree (Insight 2: duplicated logic WILL drift, and here it already has). The counted path exists to explain the real path; a widened predicate on the counted copy makes introspection lie about execution (Insight 1).

## The fix

Per `audit/CONTRACT.md §C3`: ONE execution protocol parameterized by `Counters`; profile IS `execute` with `CountingCounters`/`ReachCounters` plugged in.

- The access-path decision is made ONCE, at build, as pipeline data (engine-001's arms + engine-031's parsed direct lane). `run_bound` dispatches by matching the pipeline; `profile` calls the same dispatch with counting counters and assembles `ExecutionStats` from what the counters saw (engine-012's stats sum).
- The direct key-probe lane is one predicate, tested nowhere at run time: `Pipeline::Cq { interiors: [], rules: [KeyProbe { key_probe_finds: Some(_) }] }` parsed at build into its own lane (or a build-computed property consumed by both callers).
- Behavior note: today's profile **stats** predicate is wider than execute's direct lane. Converge the counted access path on execute's predicate (plain-var finds only) — aggregate/measure single key probes keep the sink path and report sink-shaped stats. Capture that in the new lock below.

## Acceptance criteria

- [x] One protocol: `interiors.is_empty()` gone from introspect.rs; profile dispatches on the build-parsed `key_probe_direct` flag and calls `run_rules` / `run_rules_cq_profile` (shared `run_derived` + `run_rule`), not a parallel loop.
- [x] New lock: `execute_and_profile_agree_on_an_aggregate_key_probe`.
- [x] Unchanged tests: existing execute/profile tests green.
- [x] Green: `cargo test -p bumbledb --lib api::prepared` 85 passed.

## Constraints

- ANALYZE semantics stay: the query really executes under counting. `INTROSPECTION_VERSION` increments iff rendered/structured output changes shape.
- Lands after engine-001; engine-031 may fold into this change.
