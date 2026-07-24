## Lean lane runs the three-way comparator twice, and lean.sh's in-script run defeats the cargo cache

category: unification | severity: high | verdict: CONFIRMED | finder: r2:scripts-ci-packaging
outcome: fixed 41ec17ca

### Summary

The three-way conformance comparator — one test, `three_way_conformance_over_the_checked_in_corpus` at `crates/bumbledb-bench/src/conformance.rs:2138` — has two spellings in the lean lane. `scripts/lean.sh` battery 5 runs it (with a guard that refuses the `--exact`-with-stale-name vacuous pass), and `.github/workflows/ci.yml`'s lean job runs it AGAIN as a standalone step. Worse, the yaml's step order places the `rustup show` and cargo-cache-restore steps AFTER `scripts/lean.sh`, so battery 5's cargo invocation builds `bumbledb-bench` debug (including the bundled-rusqlite C compile) from a cold registry and empty `target/` on every single run — and because `actions/cache` saves only on a key miss, that build is never captured into the cache on hit runs either. The yaml comment's own claim that "the cost is the bench-crate build, which the cargo cache absorbs" (ci.yml:153-154) is defeated by its own step order.

This is a classic two-spellings drift, verifiable in history: the yaml comparator step landed in `072d3a7b` (2026-07-14, "the conformance lane"); battery 5 was added to lean.sh one day later in `5b45b87b` (2026-07-15, "the reconciliation") without deleting the yaml step. At `072d3a7b` the yaml comment was true — the cache restore preceded the only comparator run. It has been false since `5b45b87b`.

### Evidence (all verified against the working tree)

- `scripts/lean.sh:73-74` — battery 5: `cargo test -p bumbledb-bench --lib -- --ignored --exact conformance::tests::three_way_conformance_over_the_checked_in_corpus`; guard against the zero-tests vacuous pass at `lean.sh:82-85`.
- `.github/workflows/ci.yml:164-165` — the duplicate: `cargo test -p bumbledb-bench three_way_conformance -- --ignored --nocapture`. The test exists exactly once (`crates/bumbledb-bench/src/conformance.rs:2138`; grep over the repo finds no other definition), so both commands run the same test.
- Step order in the lean job: `- run: scripts/lean.sh` at `ci.yml:144` precedes `install the pinned Rust toolchain (for the comparator)` at `ci.yml:155-156` and the cargo cache step keyed `lean-conformance-...` at `ci.yml:157-163`. The lean job's only earlier cache (`ci.yml:128-133`) covers `~/.elan` and `lean/.lake` — nothing cargo.
- `ci.yml:153-154` — "the cost is the bench-crate build, which the cargo cache absorbs" — false under this ordering: lean.sh's build sees no restored cache, and on cache-hit runs the post-job save is skipped (actions/cache saves only on exact-key miss), so lean.sh's cold build is structural, every run.
- `crates/bumbledb-bench/Cargo.toml:21` — `rusqlite = { version = "0.32", features = ["bundled", "hooks"] }` — the cold build includes the bundled SQLite C compile.
- `ci.yml:110-121` — the lane-3 comment describes lean.sh as: `lake build`, the placeholder battery, `scripts/spec-census.sh`, and the conformance corpus run. Battery 5 is omitted — the comment is stale against `5b45b87b`.
- `scripts/lean.sh:5-6` — "One entry point: CI's lean job runs exactly this script." — no longer true; the job runs three more steps after it.
- History: `git log -S "three_way_conformance" -- .github/workflows/ci.yml` → `072d3a7b` (2026-07-14); `git log -S "Battery 5" -- scripts/lean.sh` → `5b45b87b` (2026-07-15), which kept the yaml step (verified via `git show 5b45b87b:.github/workflows/ci.yml`).

### Bench impact

Every push and every PR run of the lean lane pays: (a) a cold `bumbledb-bench` debug build inside lean.sh — crate downloads plus the rusqlite C compile, minutes on a macOS runner (billed at the macOS multiplier) — before the cache restore step ever executes; then (b) a second full corpus replay (~12.4 s measured per ci.yml's own comment) in the yaml step. The cache key `lean-conformance-...` protects only the duplicate run, not the one lean.sh owns. One nuance vs. the original finding: the second run does NOT pay a second build (it reuses the `target/` lean.sh just produced and/or the restored cache) — the duplicate cost is the replay; the structural cost is the never-cached build in lean.sh.

### Suggested fix

One owner, per the doctrine in docs/design/representation-first.md (one spelling of one thing):

1. Delete `ci.yml:145-165` — the comparator comment block, the trailing `rustup show`, the `lean-conformance` cache step, and the duplicate comparator step. lean.sh's battery 5 is the comparator, and its vacuous-pass guard is strictly stronger than the yaml spelling (a substring filter with no passed-count check).
2. Move `rustup show` and the cargo cache step (re-keyed as before) ABOVE `- run: scripts/lean.sh` so battery 5's build is the one the cache absorbs — restoring the truth of the "cargo cache absorbs" claim in whatever comment survives.
3. Fix the lane-3 comment (ci.yml:110-121) to name battery 5: lean.sh ends at the three-way comparator, not at the corpus run.
