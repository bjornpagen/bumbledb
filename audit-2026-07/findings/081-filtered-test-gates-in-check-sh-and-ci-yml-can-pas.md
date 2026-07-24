## Filtered test gates can pass vacuously on rename — the guard lean.sh invented is not unified

category: incoherence | severity: medium | verdict: CONFIRMED | finder: r2:scripts-ci-packaging

### Summary

`scripts/check.sh`'s bench-obs lane narrows its test run to four libtest name filters, and libtest exits 0 when a filter matches nothing — so a module or test rename silently guts the gate while every runner stays green. The repo already recognizes this exact hazard and guards it in exactly one place (`scripts/lean.sh:80-85`, with a comment naming the failure mode), but the guard was never extracted and applied to the other filtered invocations. The stakes are concrete: the obs lane is the ONLY lane in the entire gate suite that executes the `#[cfg(feature = "obs")]` tests, so a stale filter deletes that coverage with no red anywhere.

### Evidence (all verified against the working tree)

- `scripts/check.sh:72` — `cargo test -p bumbledb-bench --features obs -- harness trace_out tripwires the_engine_trace_pins`, under plain `set -eu` (check.sh:7), no result-count check of any kind.
- Vacuous pass reproduced in this repo: `cargo test -p bumbledb-bench --lib this_filter_matches_nothing_zzz` prints `test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 386 filtered out` and exits **0**.
- `scripts/lean.sh:80-85` — the existing guard, with its own comment stating the hazard: "`--exact` with a stale name runs zero tests and still exits 0 — refuse the vacuous pass so a rename can never silently drop the third oracle", followed by `grep -q 'test result: ok. 1 passed'`. The pattern exists; it is applied once.
- The unique coverage at risk: `crates/bumbledb-bench/src/displaced/tests.rs:87-89` (`the_engine_trace_pins_the_forced_map_and_its_memoization` is `#[cfg(feature = "obs")]`) and `crates/bumbledb-bench/src/tripwires.rs:37,141,184` (three obs-gated tests). `cargo test --workspace` (check.sh:18) compiles WITHOUT `--features obs`, so these tests run nowhere but the filtered line 72. check.sh's own comment (lines 68-69) states this: `the_engine_trace_pins` is "obs-gated, so it only runs here."
- Filters currently match (`crates/bumbledb-bench/src/lib.rs:27,43,45` declare `harness`, `trace_out`, `tripwires`), so the hole is latent, not live.
- Consistent with docs/architecture/60-validation.md's framing of check.sh as the sanctioned per-push gate suite: a gate that can report green while executing zero of its tests contradicts the gate contract, and violates the design doctrine (docs/design/representation-first.md) the lean.sh guard itself embodies — the expected pass-count is data the gate should hold, not a hope the filter encodes.

### Correction to the original finding

The finding also cites `.github/workflows/ci.yml:165` (`cargo test -p bumbledb-bench three_way_conformance -- --ignored --nocapture`, unguarded) as a second live instance. Verified: the step is unguarded, but it is **shadowed**, not live — the same `lean` job runs `scripts/lean.sh` first (ci.yml:144), and lean.sh's battery 5 (lean.sh:73-85) runs the same comparator test with `--exact` plus the count guard. A rename of `three_way_conformance_over_the_checked_in_corpus` reddens the job at the lean.sh step before the ci.yml step can pass vacuously. The ci.yml:165 step is therefore redundant duplicate coverage whose own vacuity is masked by its guarded twin — an incoherence worth cleaning (delete it or guard it), but not an unprotected gate.

### Failure scenario

During a refactor, rename `mod tripwires` (or move/rename `the_engine_trace_pins_the_forced_map_and_its_memoization`). check.sh:72 — locally and in both CI check runners (macos + ubuntu, ci.yml:65-86) — executes fewer tests, down to zero if all four filters go stale, and exits 0. The only execution of the obs-gated referee pins quietly stops existing; every gate stays green; nothing ever reports the loss.

### Suggested fix

Extract the lean.sh:82 pattern into a tiny run-and-assert helper that reifies the expectation as data — each filtered gate states its expected passed-count (or a minimum) and refuses anything else:

- check.sh:72 captures the run and requires `test result: ok. N passed` with the pinned N (or at minimum `grep -v 'ok. 0 passed'` plus a floor).
- ci.yml:165: either delete the step as redundant with lean.sh battery 5 (which the same job already runs, guarded), or route it through the same helper.

This is the representation-first move the codebase already made once: the pass-count is the gate's invariant, so the gate should hold it as data instead of trusting the filter string to keep matching.
