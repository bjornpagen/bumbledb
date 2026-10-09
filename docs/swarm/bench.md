# bench lane board

Owns: `crates/bumbledb-bench/**`, `docs/swarm/bench.md`.

Items: G8 (cuts, restructure, seeded conformance from digests), H4 (rusqlite 0.40), E2 (`micro`
and `float_stats`), toolchain provenance, adapting to engine API changes.

## Status

| Item | State |
|---|---|
| G8 cuts: hashprobe (+aegis), largefix, corpus-float, storemode, duralane (`--lanes`), devhonesty, clockproxy (`--proxy-per-rep`, every `ghz*` report field), `merge`, history_model, correspondence, tripwires, stress, appperf scorecard/hosted | in progress |
| G8 layout census (`space/`, `storage --profile home-costs`) | next |
| G8 restructure into `oracle/`, `worlds/`, `harness/` | todo |
| G8 seeded conformance generated in-test from seeds, digests checked in | todo |
| H4 rusqlite 0.40 | todo |
| Provenance from `rustc -vV` at build time | todo |
| E2 `micro --levels all`, `float_stats` read family | todo (needs numeric's kernel seam) |
| Adapt: `testing` feature, C7, C8, C9, C3/C4 | as they land |

## CLI changes (bench binary)

- Deleted commands: `merge`, `corpus-float`, `hash-probe`.
- Deleted flags: `bench --proxy-per-rep`, `writes --lanes`, `app-perf --plan`, `app-perf --dir`.
- `app-perf --regimes` accepts only `warm,cold-open,post-write,large-result,tenant-churn`.
- Report JSON: every `ghz`, `ghz_ours`, `ghz_theirs`, `p50_norm` and `clock_proxy` field is gone;
  `config.store` is gone. `writes-report.json` is `{provenance, scale, seed, samples,
  sqlite_sync, rows}` (no `lanes` array). `crud.json` and `lawful.json` carry `rows` at the top
  level (no `lanes` array). `app-perf.json` is `{provenance, seed, store: {file_bytes,
  allocated_bytes}, rows}`.

## Requests to other lanes

### engine-storage

- **Blocking the bench test build:** since `23e4ef997` `plan::ground::with_grounding_disabled`
  exists only under `feature = "testing"`, but `crates/bumbledb/src/lib.rs` still re-exports it
  under `#[cfg(feature = "ground-off")]`, so `bumbledb` with `ground-off` no longer compiles.
  Please switch that re-export to `#[cfg(feature = "testing")]`. The bench dev-dependency moves
  to `features = ["testing"]` in the same breath; I switch as soon as your commit lands.

### ci

- `scripts/bench_night.py`: drop the `hash-probe`, `correspondence-oracles` and `scorecard-plan`
  (`app-perf --plan`) jobs; the conformance replay test keeps its name
  (`the_corpus_replays_byte_identical_from_its_provenance`).
- `scripts/bench_viz.py`: drop `hash-probe/hash-probe.json` and every `ghz`/`p50_norm` read.
- `scripts/check.sh` (if it survives): `bumbledb-bench` keeps the `alloc-counter` feature until
  engine-storage's G2 lands.

### log-core

- `bumbledb_bench::closure::history_model` is deleted. Nothing else in the bench serves the log.
- `crates/bumbledb-log/tests/structural_conformance.rs` may keep reading
  `crates/bumbledb-bench/fixtures/conformance/structural-algebra.json`; that file stays.
