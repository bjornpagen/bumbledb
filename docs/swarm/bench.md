# bench lane board

Owns: `crates/bumbledb-bench/**`, `docs/swarm/bench.md`.

Items: G8 (cuts, restructure, seeded conformance from digests), H4 (rusqlite 0.40), E2 (`micro`
and `float_stats`), toolchain provenance, adapting to engine API changes.

## Status

| Item | State |
|---|---|
| G8 cuts: hashprobe (+aegis), largefix, corpus-float, storemode, duralane (`--lanes`), devhonesty, clockproxy (`--proxy-per-rep`, every `ghz*` report field), `merge`, history_model, correspondence, tripwires, stress, appperf scorecard/hosted | landed |
| G8 layout census (`space/`, `storage --profile home-costs`); `--alloc` pass and the bench `alloc-counter` feature (G2); grounding-off dual runs and the unlawful-store pin (no engine test features needed) | landed |
| G8 restructure into `oracle/`, `worlds/`, `harness/` | landed |
| G8 seeded conformance generated in-test from seeds (deterministic `StepBudget` in the naive evaluator), `seeded.digests`/`reach-seeded.digests` checked in, 220 JSON files gone (22 MB → 1.6 MB); `BUMBLEDB_BLESS=1` replaces the four ignored regenerators; the structural-algebra engine check (deleted with the old log tests) lives in `oracle::conformance::structural` | landed |
| H4 rusqlite 0.32 → 0.40.2 (u64 counts read as `i64`, `progress_handler` results propagated) | landed |
| Provenance from `rustc -vV` at build time (`build.rs`; `provenance.toolchain` in every report) | landed |
| E2 `micro --levels all`, `float_stats` read family | todo (needs numeric's kernel seam) |
| Adapt: `testing` feature, C7, C8, C9, C3/C4 | as they land |

## Layout (crate `bumbledb_bench`)

- `oracle::{naive (+ admission, staged), differential, querygen, conformance, compare, poststate,
  binary64, binary64_interval, sqlite::{sqlmap, translate, verify}}`. `binary64` is the
  independent IEEE binary64 bit/rational model (was `verify::f64_oracle`).
- `worlds::{ledger (was `schema`), corpus, corpus_gen, families, calendar, closure, displaced,
  capacity, windowed, crud, lawful, scenarios, writebench}`.
- `harness::{appperf, boost, driver, lanes, report, sqlite_run}` beside the timing primitives.
- Root: `cli`, `json`.

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
  level (no `lanes` array). Every `provenance` object gains `toolchain` (`rustc -vV` release
  line; host; LLVM version). `app-perf.json` is `{provenance, seed, store: {file_bytes,
  allocated_bytes}, rows}`.

## Dependency changes (consolidator: regenerate `Cargo.lock`)

- `bumbledb-bench`: `rusqlite` 0.32 → 0.40.2 (pulls `libsqlite3-sys` 0.38.2, `hashlink` 0.12);
  `aegis` removed; no features on `bumbledb` in `[dependencies]` or `[dev-dependencies]`.

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
- The engine check against `structural-algebra.json` now runs in the bench
  (`oracle::conformance::structural`).
