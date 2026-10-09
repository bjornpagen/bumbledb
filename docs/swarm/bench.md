# bench lane board

Owns: `crates/bumbledb-bench/**`, `docs/swarm/bench.md`.

Items: G8 (cuts, restructure, seeded conformance from digests), H4 (rusqlite 0.40), E2 (`micro`
and `float_stats`), toolchain provenance, adapting to engine API changes.

## Status

| Item | State |
|---|---|
| G8 cuts: hashprobe (+aegis), largefix, corpus-float, storemode, duralane (`--lanes`), devhonesty, clockproxy (`--proxy-per-rep`, every `ghz*` report field), `merge`, history_model, correspondence, tripwires, stress, appperf scorecard/hosted | landed |
| G8 layout census (`space/`, `storage --profile home-costs`); `--alloc` pass and the bench `alloc-counter` feature (G2); grounding-off dual runs and the unlawful-store pin (no engine test features needed) | landed |
| G8 restructure into `oracle/`, `worlds/`, `harness/` | todo |
| G8 seeded conformance generated in-test from seeds, digests checked in | todo |
| H4 rusqlite 0.40 | todo |
| Provenance from `rustc -vV` at build time | todo |
| E2 `micro --levels all`, `float_stats` read family | todo (needs numeric's kernel seam) |
| Adapt: `testing` feature, C7, C8, C9, C3/C4 | as they land |

## CLI changes (bench binary)

- Deleted commands: `merge`, `corpus-float`, `hash-probe`.
- Deleted flags: `bench --proxy-per-rep`, `bench --alloc`, `scenarios --alloc`, `writes --lanes`,
  `app-perf --plan`, `app-perf --dir`, `storage --profile/--rows/--samples` (home-costs).
- Deleted feature: `bumbledb-bench/alloc-counter`. The bench has no `[dev-dependencies]` engine
  features at all; it does not need `bumbledb/testing`.
- `app-perf --regimes` accepts only `warm,cold-open,post-write,large-result,tenant-churn`.
- Report JSON: every `ghz`, `ghz_ours`, `ghz_theirs`, `p50_norm` and `clock_proxy` field is gone;
  `config.store` and every `alloc` object are gone; `report.md` has no Allocations section. `writes-report.json` is `{provenance, scale, seed, samples,
  sqlite_sync, rows}` (no `lanes` array). `crud.json` and `lawful.json` carry `rows` at the top
  level (no `lanes` array). `app-perf.json` is `{provenance, seed, store: {file_bytes,
  allocated_bytes}, rows}`.

## Requests to other lanes

### engine-storage

- `crates/bumbledb/src/lib.rs` re-exports `with_grounding_disabled` under `ground-off`, but since
  `23e4ef997` it exists only under `testing`, so a `ground-off` build fails. The bench no longer
  uses it or `create_store_without_admission`; nothing outside the engine names `ground-off` now.
- E2 needs numeric's requested `#[doc(hidden)] pub mod kernels` re-export in `lib.rs`.

### ci

- `scripts/bench_night.py`: drop the `hash-probe`, `correspondence-oracles` and `scorecard-plan`
  (`app-perf --plan`) jobs; the conformance replay test keeps its name
  (`the_corpus_replays_byte_identical_from_its_provenance`).
- `scripts/bench_viz.py`: drop `hash-probe/hash-probe.json` and every `ghz`/`p50_norm` read.
- `bumbledb-bench` has no features any more (no `alloc-counter`).

### log-core

- `bumbledb_bench::closure::history_model` is deleted. Nothing else in the bench serves the log.
- `crates/bumbledb-log/tests/structural_conformance.rs` may keep reading
  `crates/bumbledb-bench/fixtures/conformance/structural-algebra.json`; that file stays.
