## The "local twin" copies: facts_per_sec twice, GhzStamp→GhzReport three times, Rotation hand-rolled at three SQLite sites

category: unification | severity: low | verdict: CONFIRMED | finder: bench:honesty
outcome: fixed 714a8222 + c55168e2

### Summary

Three small bench derivations are duplicated across modules with doc comments that acknowledge the copy instead of removing it. None is a live miscount today — each pair (or triple) is currently byte-identical — but nothing (no shared definition, no compiler check, no test) keeps them identical. A future correction to one copy silently forks the published numbers between lanes. Verification found the duplication is worse than originally reported: the `GhzStamp → GhzReport` conversion exists in **three** places, not two.

### Evidence (all verified against the working tree)

**1. `facts_per_sec` — two verbatim copies.**
- `crates/bumbledb-bench/src/driver/write_families.rs:14-17`
- `crates/bumbledb-bench/src/lanes/writes.rs:295-298`, whose comment (lines 288-290) reads: *"The `driver/write_families.rs` `facts_per_sec` derivation, copied locally (the driver keeps its own)"*.

Both bodies are identical:
```rust
let total_secs = (m.stats.mean_ns * u64::from(samples)) as f64 / 1e9;
m.work as f64 / total_secs.max(f64::EPSILON)
```
The driver copy feeds `WriteFamilyReport.facts_per_sec` (write_families.rs:134); the lane copy feeds the writes-lane bulk rates (lanes/writes.rs:829-830). These are the same physical measurement published through two artifacts.

**2. `GhzStamp → GhzReport` — three copies, not two.**
- `crates/bumbledb-bench/src/driver.rs:66-73`
- `crates/bumbledb-bench/src/lanes/writes.rs:269-276` — comment: *"The driver's stamp→report conversion, local twin (the driver keeps its own private copy)"*
- `crates/bumbledb-bench/src/lawful/run.rs:120-127` — comment escalates: *"local twin (**every lane module** keeps its own private copy)"*

All three are the identical four-field construction (`pre`, `post`, `retried`, `contaminated: stamp.contaminated()`). No `From<clockproxy::GhzStamp> for report::GhzReport` exists — the only `From` impl in report.rs is `AllocSnapshot → AllocReport` (report.rs:94), which is the established pattern this conversion should follow.

**3. Rotation hand-rolled at three SQLite call sites.**
`harness::Rotation` (struct at harness.rs:105-108, `next_set` at harness/rotation.rs:15-19) exists precisely to round-robin param sets:
```rust
self.cursor = (self.cursor + 1) % self.sets.len();
```
Yet the SQLite side restates exactly this modulus by hand:
- `crates/bumbledb-bench/src/driver/read_family.rs:213-218` — needs one index into two parallel vectors (`sqlite_families` and `sets`), which `next_set() -> &T` cannot provide;
- `crates/bumbledb-bench/src/lanes/curves.rs:549-553` (`time_lane`);
- `crates/bumbledb-bench/src/lanes/curves.rs:821-825` (`theirs_memoized`).

In each case the engine-side twin of the *same measurement over the same draws* uses `Rotation` (read_family.rs:122 `Rotation::new(sets.clone())`; curves.rs:619 and 783 `Rotation::new(bundle.draws.clone())`), so ours/theirs rotation-phase equality currently rests on three hand-copies of the modulus staying in sync with `next_set`, rather than being identical by construction.

### Doc bearing

This is the bench-lane analogue of `docs/design/representation-first.md`'s doctrine: the invariant "ours and theirs rotate draws in the same order" and "the suite report and the writes lane derive the same rate" should be carried by one shared definition (a representation), not by parallel code kept aligned through comments. The comments themselves ("local twin", "every lane module keeps its own private copy") are the codebase admitting the drift hazard.

### Failure scenario / Bench impact

Not a live miscount. The risk is drift: e.g. a future fix replacing `mean_ns × samples` with a true per-sample sum in the driver's `facts_per_sec` leaves the writes lane publishing the old derivation — the suite report and the writes lane then disagree on the identical bulk-load measurement with no compiler error and no test failing (no test compares the twin derivations). Likewise, adding a field to `GhzReport` requires remembering three conversion sites; changing rotation policy (e.g. a stride) requires touching `next_set` plus three hand cursors, and missing one desynchronizes ours-vs-theirs draw phase in a comparative benchmark.

### Suggested fix

1. `impl From<clockproxy::GhzStamp> for report::GhzReport` in report.rs (mirroring the existing `AllocSnapshot → AllocReport` impl at report.rs:94); delete all three local `ghz_report` fns.
2. One shared `facts_per_sec(m: &Measurement, samples: u32)` in `harness` (it already owns `Measurement` and `Stats`); delete both copies.
3. Add `next_index(&mut self) -> usize` to `Rotation` (or a `Rotation<(PreparedFamily, Draw)>` for the read-family site's parallel vectors); replace the three hand-rolled cursors so engine and SQLite rotation phase is identical by construction.
