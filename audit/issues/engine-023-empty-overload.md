# engine-023: `PreparedBody::Empty` conflates dead-main, empty-query, and interiors-preamble

- **Severity:** medium
- **Tree:** engine
- **Status:** OPEN
- **Source:** audit/engine.md F23
- **Depends on:** engine-001 (co-lands — "`Empty` is not a variant" is a clause of the pipeline sum)

## The bug

`PreparedBody::Empty` means "every MAIN rule was statically refuted", but the comments claim more and then contradict themselves — `execute.rs:84-93`:

```rust
// The statically-empty program ... Always the whole program: this variant is
// built only when every rule died.
// Statically-empty **main** with no interiors: bind errors
// already surfaced; nothing to run. Interiors-only with a dead
// main still runs the preamble (an interior can be the only
// measure site).
```

Two adjacent comments: "always the whole program" and "interiors-only with a dead main still runs the preamble". `run_rules` special-cases Empty twice (`execute.rs:164, 170`); `empty_stats` (`introspect.rs:394-414`) hardcodes `interiors: Vec::new()` — wrong for dead-main-with-live-interiors, whose interior emits vanish from stats.

## Why it's wrong

One tag, three meanings, disambiguated by a sibling field (`interiors.is_empty()`) — Minsky's product again (Insight 4), and the honest confusion is already written down as two contradictory comments in one block (Insight 1). The stats hole (`empty_stats` assuming no interiors) is a real observable defect the overload caused.

## The fix

Per `audit/CONTRACT.md §C3`: **`Empty` is not a variant.** Dead main is `Pipeline::Cq { interiors, rules: vec![] }`; the empty fast path is the zero-iteration rule loop:

- `run_bound`'s short-circuit becomes structural: `Cq { interiors: [], rules: [] }` has nothing to do after bind — and that's just what the zero-iteration loop does; keep an explicit early-return ONLY if the profiler shows the sink-reset/finalize skip matters, and then it tests the one parsed shape, not a tag+flag pair.
- Dead-main-with-interiors runs the preamble (unchanged behavior) and REPORTS interior emits in stats (engine-012's `empty_stats` fix — behavior change in stats only, captured by the new lock there).
- The death record (`stats.dead`) remains the story for refuted rules.

## Acceptance criteria

- [ ] Gone: `rg -n 'PreparedBody::Empty|Pipeline::Empty' crates/bumbledb/src` → no matches; the contradictory comment block at `execute.rs:84-93` rewritten to the one-shape story.
- [ ] Unchanged tests: `statically_empty.rs`, `folded.rs` suites pass UNCHANGED (same answers, same death records).
- [ ] New lock: shared with engine-012 — dead-main + live-interiors profiling reports interior emits.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- Statically-empty refutation at prepare (fold.rs) untouched — only its *representation* in the prepared object changes.
- Co-lands with engine-001.
