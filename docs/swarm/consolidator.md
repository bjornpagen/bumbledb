# consolidator checklist

Resume from the first step that is not `done`.

| # | Step | Status |
|---|---|---|
| 1a | Integration: leftover working tree, Cargo.lock | done |
| 1b | Integration: lane leftovers (ValidationError variants, AggOp, store alias, Direction, float_aggregates Miri, schema diagnostic names, bench_night/bench_viz, image cache option, probe buffer) | done |
| 1c | Integration gate: fmt, clippy x2, rustdoc, nextest, doctests | done (1918 tests, cargo-deny/shear not installed locally) |
| 1d | Integration gate: TS (addon, biome, tsc, node --test), notes | pending |
| 2 | Comment purge (Rust, TS, TOML, YAML, shell, Python) | pending |
| 3 | Legacy cull grep gate | pending |
| 4 | D20 bdb grep gate | pending |
| 5a | README.md + docs/cookbook.md rewrite, cookbook doctests | pending |
| 5b | Delete docs/release-1.*, stale docs/perf/runs; write docs/release-2.0.md | pending |
| 5c | Versions to 2.0.0 (Cargo, npm) | pending |
| 5d | ts/README.md, ts/COOKBOOK.md | pending |
| 6a | Delete scripts/swarm/ and docs/swarm/ | pending |
| 6b | Merge origin/main | pending |
| 6c | cutover-plan status done + outcome | pending |
| 6d | Final full gate | pending |

## Notes
