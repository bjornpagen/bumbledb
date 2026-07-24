## Trace spans pay two runtime u128 divisions per stamp; TraceEvent should carry raw ticks and convert once at drain

category: perf | severity: medium | verdict: CONFIRMED | finder: engine:interval-allen

### Summary

Every recorded trace span converts ticks to nanoseconds eagerly, twice: `now_ns()` at span open and `now_ns_ss()` at span close each call `fastclock::ticks_to_ns`, which performs a u128 multiply and a u128 division by a **runtime** divisor (`cntfrq_el0`, re-read via `mrs` on every call). On aarch64 a u128/u128 division with a non-constant divisor lowers to a `__udivti3` libcall — there is no hardware 128-bit divide, and the dividend (`ticks * 1_000_000_000`) genuinely overflows u64 after ~13 minutes of process uptime at the 24 MHz tick rate, so the compiler cannot narrow it. The module's own cost model (obs.rs:305-317, backed by `docs/reference/apple-silicon-performance.md`: `m2max.timer.cntvct-read-cost` 0.30 ns, `m2max.timer.cntvctss-cost` ≤4.6 ns, `m2max.timer.isb-fence-cost` +164% at 10 ns phases) prices stamp policy to fractions of a nanosecond and explicitly rejects `isb` fencing as too expensive — yet each span spends two division libcalls whose cost is on the order of the rejected fence, inside the measured windows of the enclosing spans. The codebase already knows the right representation: `PhaseTimers` stamps raw ticks at both ends and converts once per (node, phase) at flush. The span path violates the discipline its sibling follows.

### Evidence (verified)

- `crates/bumbledb/src/obs.rs:343-346` — `now_ns()`: `fastclock::ticks_to_ns(fastclock::ticks().wrapping_sub(anchor))`; `obs.rs:351-354` — `now_ns_ss()` likewise.
- `crates/bumbledb/src/obs.rs:464` — span open stamps via `now_ns()` inside `span_args` (gated on `capturing()`, so trace-lane only).
- `crates/bumbledb/src/obs.rs:415` — span close in `Drop for SpanGuard`: `dur_ns: now_ns_ss().saturating_sub(live.start_ns)`.
- `crates/bumbledb/src/obs/fastclock.rs:151-154` — `ticks_to_ns`: `u128::from(ticks) * 1_000_000_000 / u128::from(frequency())`; `fastclock.rs:111-118` — `frequency()` is an uncached `mrs cntfrq_el0` per call.
- Contrast, same repo: `crates/bumbledb/src/exec/run/counters.rs:89` (`open[...] = fastclock::ticks()`), `:95` (`cell.0 += ticks().wrapping_sub(...)`) accumulate raw ticks; `:46` converts once per (node, phase) at `flush()`.
- Cost-model spec: `crates/bumbledb/src/obs.rs:305-317` module doc; corroborated by `docs/reference/apple-silicon-performance.md` (`m2max.timer.*` facts: raw stamp 0.30 ns, CNTVCTSS ≤4.6 ns, `isb; mrs` ≈9.4 ns / +164% at 10 ns phases — "raw stamps for accumulation, self-synchronized reads for single-shot, isb for neither").
- Hot short spans affected: `crates/bumbledb/src/exec/dispatch/key_probe_fact.rs:259` (KEY_PROBE), `crates/bumbledb/src/api/prepared/run_join.rs:101,148` (VIEW_BUILD), `:179` (SELECT_PROBE), `crates/bumbledb/src/api/prepared/resolve_memo.rs:45` (DICT_RESOLVE, per distinct intern in finalize).
- Drain layer off the measured path exists: `crates/bumbledb/src/obs.rs:368-370` (`finish_capture`) and `crates/bumbledb-bench/src/trace_out.rs` / `trace_out/flame_summary.rs` / `trace_out/write_chrome.rs` consume `start_ns`/`dur_ns` after capture ends.

### Bench impact

Trace-lane only (capture is gated; release benches never stamp), so headline numbers are unharmed — the damage is to the honesty of the diagnostic lane itself. The stamps are read *before* their conversions, so each span's own `dur_ns` is not inflated by its conversions; instead the open conversion lands inside the child span's window and the close conversion (plus the TLS/RefCell/Vec push of `record`) lands inside every enclosing parent's window. A parent that opens many tens-of-ns children (KEY_PROBE, SELECT_PROBE, VIEW_BUILD, DICT_RESOLVE in a finalize loop) absorbs two division libcalls per child into its measured time, so the flame summary's containment math (`flame_summary.rs:31,41`: `self_ns = total - children`) over-attributes self-time to span-opening parents — distorting exactly the sub-500 ns regions the CNTVCTSS closing-stamp policy was adopted to time honestly at a documented cost of 4.6 ns.

### Suggested fix

Adopt the `PhaseTimers` representation for spans: store raw anchor-relative ticks in `TraceEvent` (rename `start_ns`/`dur_ns` to `start_ticks`/`dur_ticks`, or convert in place), keep the anchor subtraction at stamp time, and perform `ticks_to_ns` once per event at drain — in `finish_capture` (keeping the public ns contract and all existing consumers/tests unchanged) or in the bench `trace_out` exporter. This removes both the division libcall and the redundant per-call `cntfrq` read from every measured window; the per-stamp cost returns to the module doc's priced 0.30 ns / 4.6 ns. Caching `frequency()` in a `OnceLock` (as `has_ecv` already does at `fastclock.rs:71-72`) is a lesser, compatible hardening but does not remove the division; the representation change does.
