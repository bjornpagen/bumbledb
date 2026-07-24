## Nested start_capture is only debug-guarded: a latent silent-event-loss hazard, not an active bug

category: design-robustness | severity: low | verdict: PLAUSIBLE | finder: engine:interval-allen
outcome: fixed f15a40c9

### Summary

`obs::start_capture` detects nesting only via `debug_assert!` and then unconditionally overwrites the thread-local buffer. In a release build a nested call would silently drop every event recorded so far. The mechanism is real, but the finder's claimed trigger is not: an audit of every call site shows all capture pairs are flat and self-contained, and the specific composition cited (tripwires.rs driving capture around harness/traced.rs) cannot occur — tripwires.rs is `#[cfg(test)]`-only. This downgrades from "bug" to a low-severity design finding: the illegal state (two owners of one buffer) is representable and its only detection compiles out of exactly the builds (release bench/trace runs) where the module's measurement-honesty purpose matters.

### Evidence

- `crates/bumbledb/src/obs.rs:360-366` — `start_capture`: `debug_assert!(slot.is_none(), "nested start_capture"); *slot = Some(Vec::with_capacity(4096));`. With `debug_assertions` off, the assignment drops an existing `Some(events)` silently. `finish_capture` (obs.rs:368-370) is `take().unwrap_or_default()` — an unmatched finish is safe, so the asymmetry is only on the start side.
- Call-site audit (every `start_capture` in the repo):
  - `crates/bumbledb-bench/src/tripwires.rs:10` — the whole module is `#[cfg(test)]` ("test-only enforcement; it compiles no production code", lines 7-8). Its three captures (55/59, 161/164, 201/206) are flat pairs around single executions; it never calls `traced_sample` and cannot run in the same process as the bench CLI drivers.
  - `crates/bumbledb-bench/src/harness/traced.rs:15-19, 36-46` — `traced_sample`/`traced_cold_sample` each open one capture and always drain it before propagating closure errors (`finish_capture` runs before the `?`).
  - Closures fed to `traced_sample` are engine executions (`driver/trace.rs:45-50`, `driver/read_family.rs:167`, `harness/measure.rs:112`); the engine itself never calls `start_capture` — only tests and harness drivers do.
  - `crates/bumbledb-bench/src/harness/measure.rs:111-115` — the trace sample runs after the timing loop, outside any enclosing capture; lines 77-78 refuse `alloc_window && trace` combined, matching the obs.rs module doc's mutual-exclusivity contract (obs.rs:11-14).
  - `crates/bumbledb-bench/src/sweep.rs:341/352` and `403/405` — flat pairs (the latter a deliberate probe-then-drain feature check).
  - All remaining sites are unit tests with flat pairs; `obs/tests.rs:96-101` exercises sequential captures, and no test covers nesting.
- Doctrine check: the audit lens (docs/design/representation-first.md lineage — make illegal states unrepresentable) treats a representable illegal state whose only detection is a debug-only branch as a finding; obs.rs's own header (lines 1-14, citing docs/architecture/60-validation.md and 00-product.md) frames the module as the honesty seam for measurement, which strengthens the case for structural rather than assert-based ownership.

### Failure scenario

None reachable today. The hazard is latent: any future capture driver, retry wrapper, or helper that composes with an existing one (or a panic that unwinds out of a closure between start and finish, leaving a stale live buffer for a later same-thread driver) would, in a release build, silently reset the timeline mid-run — the exported Chrome trace and flame-containment math would compute over a truncated population with no error and no truncation marker.

### Suggested fix

Make ownership structural: have `start_capture` return a `CaptureGuard` whose `Drop` performs the finish, so a second concurrent owner is unrepresentable at the type level (this also fixes the panic-unwind leak for free). Minimally, replace the overwrite with `Option::get_or_insert_with` (nesting can extend but never destroy) or promote the debug_assert to a typed refusal. The guard form matches the module's existing `SpanGuard` idiom (obs.rs:380-421), so the fix stays inside the file's own vocabulary.
