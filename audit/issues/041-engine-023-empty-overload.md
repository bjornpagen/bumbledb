# engine-023: `PreparedBody::Empty` conflates dead-main, empty-query, and interiors-preamble

- **Severity:** medium
- **Tree:** engine
- **Status:** FIXED
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

Two adjacent comments: "always the whole program" and "interiors-only with a dead main still runs the preamble". `run_rules` special-cases Empty twice (`execute.rs:164, 170`). `introspect` spells emptiness as a sentinel plan unit (`PreparedBody::Empty => (vec![RulePlan::Empty], …)` at `:42`) and `empty_stats` mints a phantom one-element `RuleStats` (`:397-406`). `empty_stats` is only reached when `interiors.is_empty() && Empty` (`:214`) — dead-main-with-live-interiors already reports `interior_stats()`. The overload is still the product; the stats hole is a regression waiting for a naive `Empty → empty_stats()` remap after engine-001, not a current observable.

## Why it's wrong

One tag, three meanings, disambiguated by a sibling field (`interiors.is_empty()`) — Minsky's product again (Insight 4), and the honest confusion is already written down as two contradictory comments in one block (Insight 1). Emptiness-as-sentinel (`RulePlan::Empty`, phantom `RuleStats`) is the same overload on the stats/introspection surface.

## The fix

Per `audit/CONTRACT.md §C3`: **`Empty` is not a variant.** Dead main is `Pipeline::Cq { interiors, rules: vec![] }`; the empty fast path is the zero-iteration rule loop:

- `run_bound`'s short-circuit becomes structural: `Cq { interiors: [], rules: [] }` has nothing to do after bind — and that's just what the zero-iteration loop does; keep an explicit early-return ONLY if the profiler shows the sink-reset/finalize skip matters, and then it tests the one parsed shape, not a tag+flag pair.
- Dead-main-with-interiors runs the preamble (unchanged) and keeps reporting interior emits (already true today). After Empty dies, do not route that shape through `empty_stats`.
- `RulePlan::Empty` is display-only: either drop it (zero plan units + `stats.dead`) or keep it as an introspection sentinel that is **not** a `PreparedPipeline` variant. Do not mint a phantom `RuleStats` row for a query with no surviving main rules.
- The death record (`stats.dead`) remains the story for refuted rules.

## Acceptance criteria

- [x] Gone: `rg -n 'PreparedBody::Empty|Pipeline::Empty' crates/bumbledb/src` → no matches; the contradictory comment block at `execute.rs:84-93` rewritten to the one-shape story.
- [x] Unchanged tests: `statically_empty.rs`, `folded.rs` suites pass UNCHANGED (same answers, same death records).
- [x] New lock: shared with engine-012 — dead-main + live-interiors profiling reports interior emits.
- [x] Green: `cargo test -p bumbledb --lib` pass (`statically_empty`, `folded`, new dead-main interior lock).

## Constraints

- Statically-empty refutation at prepare (fold.rs) untouched — only its *representation* in the prepared object changes.
- Co-lands with engine-001.
