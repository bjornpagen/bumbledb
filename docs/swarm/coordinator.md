# coordinator

## D20 (new owner decision): canonical extension `bdb`

Apply this in YOUR owned paths now. The consolidator finishes it repo-wide with a grep gate.

- Database directories and checkpoint images are `<name>.bdb`; the notes data dir is `.bdb/`.
- The lock file is `bdb.lock` (was `bumbledb.lock`).
- Format family tags are `bdb.<kind>.v1`: `bdb.result.v1`, `bdb.evidence.v1`, and every new log frame/entry family.
- The native addon file is `bdb.<platform>.node` and `bdb.node` in the platform packages.
- TS brands and symbols are `bdb.*` (e.g. `bdb.db`, `bdb.query.term`).

Unchanged: crate names, npm package names (`@bjornpagen/bumbledb`), the `bumbledb` CLI, env var names.

## Owner instruction (2026-10-09, evening): keep local CPU work light

- Do NOT run benchmarks (bumbledb-bench runs, micro reports, appperf, bench-night), Miri, `scripts/ci.sh miri`, `scripts/ci.sh deep`, `udeps`, or musl/static builds locally today.
- The consolidator's local gate is ONLY: `cargo fmt --all --check`, `scripts/ci.sh lint`, `scripts/ci.sh test` (or the equivalent clippy and nextest workspace runs), `scripts/ci.sh addon` (TS and notes), and rustdoc. Run each once per step, not in loops.
- The coordinator pushes main to origin after the consolidator finishes. Agents never push.
