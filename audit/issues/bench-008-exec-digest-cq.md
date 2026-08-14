# bench-008: `exec_digest` is a CQ stats consumer

- **Severity:** medium
- **Tree:** bench
- **Status:** OPEN
- **Source:** audit/bench.md F8
- **Depends on:** engine-012 (stats shaped like the pipeline sum)

## The bug

`crates/bumbledb-bench/src/driver/read_family.rs:12-44` — the profile digest:

```rust
for (index, node) in stats.rules.iter().flat_map(|r| &r.nodes).enumerate() {
    // worst estimate factor, covers
}
emitted: stats.emits,
absorbed: stats.rules.iter().map(|rule| rule.absorbed).sum(),
```

It does not read `stats.interiors` or `stats.reach`. Ledger/calendar families are CQ so it is true today. It is the old per-stratum rule table as the counted surface (engine-012). Closure skips profile rather than digest a Reach stats arm (engine-011) — two encodings of "rec is not a stats shape we have."

## Why it's wrong

Insight 2: stats and the digest that reports them are two representations of one execution. A digest that only knows `stats.rules` cannot describe interiors-or-rec without lying or skipping (Insight 1). engine-012 collapses the engine side; this is the bench consumer.

## The fix

Per `audit/CONTRACT.md` §C3: digest matches the pipeline sum. CQ: main-rule covers. Reach: interior emits + reach rounds. No `stats.rules` as the universal table. Once engine-012 lands, `exec_digest` matches on the stats sum (or the prepared pipeline arm). Closure's `exec: None` (engine-011) becomes a real digest, not a skip.

## Acceptance criteria

- [ ] After engine-012: `exec_digest` reads whatever fields the pipeline-shaped stats expose; `rg -n 'stats\.rules' crates/bumbledb-bench/src/driver/read_family.rs` is either gone or matches only a CQ/main arm.
- [ ] A rec family that is profiled (closure, once engine-008/011 land) produces a digest that includes reach rounds or interior emits — not `exec: None` as the only rec encoding.
- [ ] Unchanged: ledger/calendar digest numbers for CQ families (same covers/emits/absorbed on the same queries).
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb-bench`; `./scripts/check.sh`.

## Constraints

- Blocked on engine-012's stats shape. Do not invent a parallel rec digest before that sum exists. Report-class closure rows may stay report-class.
