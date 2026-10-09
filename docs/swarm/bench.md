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
| E2 `micro --levels all|a,b` (every kernel at every level vs its scalar twin via `bumbledb::kernels`, outputs checked bit-identical before timing) and the `float_stats` world (8-field `Reading`, 1% NaN; global and grouped SUM/AVG/MIN/MAX at 10k groups and one group per row, `v > c`, `qty * price`, `(a - b) / c`), gated against the naive evaluator | landed |
| Adapt: E5 (naive and SQLite order F64 like IEEE: NaN unordered; six seeded digests re-blessed), E8 (naive orders an integer against an F64 exactly), ValidationError fold, C17 `disk_size()`, C3/C4 | landed |
| `BUMBLEDB_DEEP=1` widens the randomized differential sweeps sixteenfold (same seeds, extended) | landed |
| Adapt: C7 (`bumbledb::host`, private `digest`/`store`: the bench hashes with `blake3` directly, `host::LAYOUT` in the corpus digest), C9 (`WriteOutcome` writes, `write_from`, `witness()` without `Result`; one `harness::committed` helper; the differential maps one write outcome for plain and witnessed writes), `verify_store(&WorkContext) -> StoreReport`, C15/C16 (re-pinned corpus digest and ledger fingerprint; the pre-closed-relation fingerprint test is gone) | landed `7f6096205` |
| E5 MIN/MAX: the naive evaluator and the SQLite translation propagate NaN through F64 MIN and MAX (SQLite: `CASE WHEN MAX(x) = <NaN blob> THEN <NaN blob> ELSE MIN(x) END` unless the column is known non-F64); one naive fold function | landed `49636778b` |
| D20: every bench store directory is `<name>.bdb` (`db.bdb`, `cal.bdb`, `load.bdb`, `target.bdb`, `naive.bdb`, `tenant-N.bdb`, ...) | landed `1edeb380f` |
| L: comment pass (216 empty `# Errors`/`# Panics` sections, truncated fragments, module docs at most five lines, ticket references) | landed `18d2ab31f` |
| `BUMBLEDB_DEEP=1` release run of the whole bench suite green | landed `b27f24add` |
| E2 check at HEAD: `micro --levels all` (neon here), `micro --levels <name>` refusing an unavailable level by name, and `micro --compare a.json b.json` all run against `bumbledb::kernels` | verified |
| C8 stage B (`Error::Store` deleted) | nothing to do: the bench names no `StoreError`/`Error::Store` |
| Tests: the SQLite cap test is decided by operation counts (zero cap), not the clock; every scratch directory is one `fixture::TempDir` (removed on drop, panics included) | landed `623f37f07`, `4a8b2bc2d` |
| Gate at `4a8b2bc2d`: `clippy -p bumbledb-bench --all-targets -- -D warnings` clean, `nextest -p bumbledb-bench` 375/375 | green |

## Layout (crate `bumbledb_bench`)

- `oracle::{naive (+ admission, staged), differential, querygen, conformance, compare, poststate,
  binary64, binary64_interval, sqlite::{sqlmap, translate, verify}}`. `binary64` is the
  independent IEEE binary64 bit/rational model (was `verify::f64_oracle`).
- `worlds::{ledger (was `schema`), corpus, corpus_gen, families, calendar, closure, displaced,
  capacity, windowed, crud, lawful, scenarios, writebench}`.
- `harness::{appperf, boost, driver, lanes, report, sqlite_run}` beside the timing primitives.
- Root: `cli`, `json`.

## CLI changes (bench binary)

- New: `micro [--levels all|a,b] [--elements N] [--float-rows N] [--samples N] [--seed N]
  [--dir PATH] [--out PATH]`. `--out` is the JSON path (e.g.
  `docs/perf/runs/2.0.0/micro.json`); `micro.md` lands beside it. JSON:
  `{provenance, seed, elements, float_rows, samples, kernels: [{kernel, level, level_p50_ns,
  twin_p50_ns, speedup}], float_stats: [{family, about, answers, ours}]}`. Non-asserting.
- `micro --levels all` runs every level `bumbledb::kernels::SimdLevel::available()` reports
  (the detected level and the lower levels it implies).
- New: `micro --compare OLD NEW` prints both reports side by side as Markdown (per kernel × level:
  old/new p50, change, old/new speedup; per float family: old/new p50, change). For ci's
  `bump-toolchain.sh`: `micro --levels all --out <file.json>` on each nightly, then
  `micro --compare <old.json> <new.json>`.
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

- `bumbledb-bench` names no engine feature any more (`alloc-counter`, `ground-off`,
  `collision-probe` and `testing` are all unused by the bench): delete them whenever you like.

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
