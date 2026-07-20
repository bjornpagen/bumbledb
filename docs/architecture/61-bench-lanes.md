# 61 — Bench lanes & the night run

The normative name registry for every Report-class benchmark lane: the lane
table, the SQLite parity config per lane, the
DNF-cap law, the churn protocol, the lane artifact contracts, and the night
runbook. The enforcement of every rule here lives in representations that
already exist — the lane discriminant, the cap⇒null shape, the committed
fixtures, `bench-night.sh`'s lane table, the measurement mutex — and this
chapter is the registry those representations cite. Its readers, named:
`scripts/bench-night.sh` (the lane table and probe list), `scripts/bench_viz.py`
(the artifact contracts and the chart inventory), and every lane emitter under
`crates/bumbledb-bench/src` (the parity configs and the oracle law). No lane
is added without a row in the registry below.

## The class law

Every lane in this chapter is **Report-class**. The Gate-class set — the
ledger and calendar ALL-WIN families and their budget gates
(`60-validation.md` § the primary benchmark, § the calendar benchmark) — is
closed; this chapter never adds to it. The night runner reruns the gate
lanes, but it reruns them *as reports*: a published claim remains owner
ceremony — the owner runs `scripts/bench-night.sh` personally on an idle
machine and publishes by hand-copying artifacts into `bench-out/` and
rendering `assets/` from committed pins. The Report classification is
structural where it can be (`lanes.rs` carries the metric-lanes charter: lane
reports are plain-data types that cannot reach the gated `crate::report`
type; churn is `Kind::Report` by construction) and registry-driven here for
everything else: a lane in this table is Report-class because its row says
so, and no future lane enters without a row.

**Decision.** **Alternative:** gate new lanes in CI — every lane added to
this registry gets a machine-checked budget or win threshold.
**Why it lost:** a Report lane that fails a machine gate would incentivize
flattering lane design — thresholds weak enough to pass, corpora shaped to
win, adversarial cases quietly dropped. The expansion lanes exist to show
regimes honestly, including the ones we lose (`crud`, `lawful`, the
adversarial DNFs, churn degradation); honesty is the product, and evidence is
report data. **Reverses if:** never.

## The oracle law

Every query in every lane is oracle-gated before it may ever be timed:
value-identical multiset agreement against SQLite, wired like the existing
gate/time split in `crates/bumbledb-bench/src/scenarios/run_query.rs` —
`gate` proves agreement for every param set × every SQLite lane
(`compare::multisets`, a disagreement fails the run naming the query, lane,
and set), and only the `Gated` value it returns can enter the timing half, so
"oracle-gated before ever timed" is a call-order fact, not discipline. Where
the harness already carries the naive model, the lane gates against it too
(the lawful world's judgment oracle, t5's three-oracle triangle, churn's
end-state `LiveSet` model — `60-validation.md` owns each). The gate is never
capped: correctness is sacred; the DNF cap bounds timed and pre-flight
samples only. A lane without a passing gate has no timing lane — it does not
exist yet. Non-timed lanes gate on the analogous value identity: storage
cross-checks per-relation row counts across the engine store, the generator
stream, and both SQLite twins before a single byte is reported; writes and
crud verify post-state by full-scan body-multiset comparison.

## The lane registry

The night's lane table (`scripts/bench-night.sh` `lane_table()` is the
executable twin of this table; order is the law — cheap first, adversarial
and churn last). SETUP rows (`gen`, `verify`) precede it and run only when at
least one lane runs. Parity details are owned by `60-validation.md` and cited
here, never restated.

| lane | world | metric | subcommand | artifact | class | SQLite parity config |
|---|---|---|---|---|---|---|
| `bench-durable-r1..r3` (×3) | ledger + calendar | warm p50/p90/p95/p99 per read family; write + cold families | `bench --out` | `report.json` (RunReport, `config.store` durable) | Gate families, rerun as Report by the night | file-backed WAL, `synchronous=FULL`, `fullfsync=ON`, fully indexed per family, prepared statements reused, `ANALYZE` (`60-validation.md` § the primary benchmark) |
| `bench-ephemeral-r1..r3` (×3) | ledger + calendar | the same roster | `bench --ephemeral --out` | `report.json` (RunReport, `config.store` ephemeral) | Report — the NOSYNC characterization lane | the same session minus the sync boundary: `synchronous=OFF`, `fullfsync=OFF` — the honest `MDB_NOSYNC` twin |
| `scenarios` | joins / graph / olap / points / rings / temporal | warm p50 per (query, SQLite lane); 8 warmups / 64 samples | `scenarios --out` | `scenarios.md` + `scenarios.json` | Report | the ledger protocol via `open_for_bench` + `configure_sqlite`: WAL, `synchronous=FULL`, 256 MiB cache, 1 GiB mmap, `wal_autocheckpoint=0` + one pre-run truncating checkpoint, prepared statements reused, per-world indexes, `ANALYZE` (`60-validation.md` § the scenario worlds) |
| `sweep-commit` | ledger commit sizes | judgment spans by touched-parent count | `sweep-commit` (obs build), stdout | `sweep.md` | Report | n/a — engine-only, no twin |
| `storage` | ledger + calendar scales (+ churn checkpoints via `--churn-dir`) | on-disk bytes per fact, absolute store bytes; not timed | `storage --out` | `storage-report.json` / `.md` (auto-ingested by night discovery; the `storage` lane-payload contract is retired — §contracts) | Report | durable config, indexed and table-only twins both reported; byte counts read post-checkpoint; row-count cross-check before any byte is reported (`60-validation.md` § the metric lanes) |
| `curves` | existing families at parameterized scale + the closure world | warm p50 vs corpus size (the fitted exponent is the story); the warmth panel: cold → warm → memoized p50, both engines | `curves --warmth --out` | `curves-report.json` / `.md` (auto-ingested by night discovery; the `curves` lane-payload contract is retired — §contracts) | Report | durable config, prepared statements reused, `ANALYZE`; inline per-draw multiset gate dominates every timer; canonical translation always reported, hand-tuned twin beside it where canonical inflates (`busy_scan`); SQLite points run under the DNF cap |
| `cold-warm-memo` | warmth-carrying families | p50 per phase — ours cold/warm/memo, theirs cold/warm (memo has no SQLite twin, stated) | landed folded into `curves --warmth` (no separate subcommand) | the `warmth` blocks of `curves-report.json` (the separate `cold_warm_memo` payload and chart are retired — §contracts) | Report | as the curves row; cold = process-fresh reopen (OS-page-cache-warm, the honesty bound stated in the report), prepared statements fresh for the cold phase by definition |
| `write-throughput` | commit/delete ladders + bulk | facts/sec per commit batch size, per durability lane | landed as `writes --out` | `writes-report.json` / `.md` (auto-ingested by night discovery); the `write_throughput` chart input derives from its batch ladders per §contracts | Report | matched durability pairs only, by type (`DurabilityLane`): durable = WAL `synchronous=FULL` `fullfsync=ON` vs `Db::create`; nosync = WAL `synchronous=OFF` vs `Db::ephemeral`; hand-authored native SQL write twins; post-state value-verified |
| `adversarial` | worst-case query shapes (the rings/temporal bomb precedent) | p50 both engines under the per-sample cap; capped twins reported as DNF | contract spelling `adversarial --out` — the subcommand has not landed; the night probe reports it SKIP-UNAVAILABLE until it does | `report.json` carrying `"lane": "adversarial"` per §contracts | Report | scenarios parity + the DNF cap below; canonical translation the default lane, hand-tuned twin lanes alongside where canonical inflates — both reported |
| `churn` | steady-state posting working set | per-cycle probe warm p50, store bytes, write facts/sec, engine counters | `churn --out` | `churn-report.json` (`churn_schema: 1`, auto-ingested and charted directly by the viz) + `churn.md`; the one-run condensation is retired — §contracts | Report | per-lane sessions per `60-validation.md` § the churn lanes: `sqlite-bare`/`sqlite-maint` durable, `sqlite-nosync` ephemeral-matched; probes prepared fresh per sample point on both engines; `maint`'s VACUUM/ANALYZE charged into its own throughput window |

Answer ordering is deliberately not a lane: ordering is the host
language's own sort over returned rows — there is no engine work to
measure.

**The canonical-translation law**, registry-wide: the SQL twin is the
canonical IR→SQL rendering (`translate`), always gated and always reported.
Where the canonical rendering inflates SQL — the Allen basics OR-chains are
the named case — a hand-tuned twin lane runs alongside and BOTH are reported
(`Twin::Tuned`, lane `sqlite-tuned`); where the translator refuses the query
outright (`Pack`, the enumerated `Inexpressible` set), the lane is a
hand-written best shot (`Twin::Hand`, lane `sqlite-hand`), legal only where
translation errs, asserted by test. We never flatter ourselves, in either
direction.

## The DNF-cap law (adversarial lanes)

Adversarial SQLite lanes carry a per-sample wall-clock bound, enforced via
the `sqlite3_progress_handler` interrupt (the `CapMs` type;
`Option<CapMs>` per query, so pre-existing lanes are untouched by
construction — `None` never installs the handler). Protocol: one untimed
capped pre-flight sample per param set, then the capped timed window. An
exceeded sample is reported as **exceeded-cap, excluded from every ratio and
geomean, and counted** — never silently dropped, never recorded as a number
(a censored p50 is not a p50, unrepresentable by type: `LaneOutcome` has no
stats in its `ExceededCap` arm). The gate itself is never capped in the
scenario worlds; in the curves lane a capped *gate* region means nothing on
that point is timed at all, on either side — never time what is not
verified, and the typed `CapEvent` names where the cap fired.

The representation carries the law end to end: in the adversarial lane
payload, `theirs_exceeded_cap: true` ⇒ `theirs` is `null`, and the viz
loader (`load_adversarial`) **rejects a payload claiming both** a cap and a
number — a capped SQLite time can never be drawn as a measurement, because
there are no stats to draw, only the cap.

## Lane artifact contracts

**The discriminant law.** Every expansion-lane artifact is a `report.json`
whose top level carries `"lane": "<id>"`; the suite RunReport carries **no
`"lane"` key** and is classified by `config.store` into the durable/ephemeral
pools (`bench_viz.py: ingest_report` is the executable form). A duplicate
lane in one night keeps the first occurrence and prints a note. Each surviving
lane contract below is fixed as a committed synthetic fixture under
`scripts/viz-fixtures/` — `fixture-write-throughput.report.json`,
`fixture-adversarial.report.json`, and `fixture-churn.report.json` (the
last a schema-exact twin of the runner's REAL `churn_schema: 1` artifact) —
each carrying a `_fixture` marker naming itself synthetic, never a measured
claim, validated by the matching loader in `scripts/bench_viz.py`, so
"chart fed a shapeless lane file" is unrepresentable. Unknown extra keys
are ignored everywhere (forward-compatible); readers: the `bench_viz.py`
loaders and the lane emitters.

The live emitters ship *flag-fed* shapes, and night discovery ingests them
from their canonical paths (`bench_viz.py: NIGHT_LANE_REPORTS`; the flags
override): `storage/storage-report.json` (scales → worlds), `writes/
writes-report.json` (lanes → rows), `curves/curves-report.json` (families →
rows + `warmth` blocks), and `churn/churn-report.json` (`churn_schema: 1`,
runs → lanes → samples, validated by `load_churn_report`). Beyond those,
discovery scans `<child>/report.json` through the discriminant (plus the
first `scenarios.json`, the md rendering as fallback). **The contamination
marker:** a run dir carrying `CONTAMINATED.md` (the recorded ruling, prose
in the file) is excluded from every merge and counted in the chart footers —
the exclusion is a file ON the pin, so a contaminated number can never leak
into a chart by someone forgetting a footnote.

Two chart inputs are *derived* from real artifacts (`bench_viz.py:
derive_lanes`), each passed through its lane-contract loader so the
contract stays honest as the adapter's output shape — and a real lane
payload of the same name, once an emitter writes one, wins over the
derivation: `adversarial` derives from `scenarios.json`'s `exceeded_cap`
lanes (the DNF data's one real home while the `adversarial` subcommand
remains unlanded), and `write_throughput` derives from
`writes-report.json`'s commit/delete batch ladders (`bulk_append` is a
single point, not a ladder — it stays fully drawn in
`bench-writes-rates.svg`).

- **`write_throughput`** (the `write-throughput` lane) — `{"lane":
  "write_throughput", "lanes": [{"name": str, "batches": [{"batch": int > 0,
  "ours_facts_per_sec": number, "theirs_facts_per_sec": number}, …]}, …]}`;
  the durability lane's name rides every row, so a rate is never quoted
  without its durability context.
- **`adversarial`** — `{"lane": "adversarial", "cap_ms": number > 0,
  "queries": [{"name": str, "ours": {"p50": ns}, "theirs": {"p50": ns} | null,
  "theirs_exceeded_cap": bool}, …]}`; `ours` stats required;
  `theirs_exceeded_cap: true` ⇒ `theirs` null, loader-enforced (the DNF-cap
  law's shape).
- **`churn`** — the runner's real `churn-report.json` under
  `churn_schema: 1` (runs → lanes → samples, `churn/report.rs`), consumed
  directly. **Retired:** the one-ours-one-theirs `"lane": "churn"`
  condensation (2026-07-20) — it could not carry the steady run's two
  SQLite lanes (`sqlite-bare` + `sqlite-maint`) without hiding one, so the
  fixture-only shape died and the charts draw every lane of every run.
- **Retired contracts** (2026-07-20, with the charts that consumed them):
  the `storage` and `curves` lane payloads and the `cold_warm_memo` payload
  — no emitter ever wrote them, and their data is fully rendered from the
  real flag-shaped reports (see the chart inventory's retirement notes).

## The churn protocol

Cycles of a fixed-size delete+insert wave (`cycle_facts` — the mix's one
`churn` field, so facts-in ≈ facts-out is a construction, not a discipline)
against both stores. After each sampled cycle, three measurements on both
engines: the probe family's warm p50 (oracle-gated per draw — the SQLite
sampler type-requires the engine sampler's reference answers), the store
byte size (post-checkpoint on the SQLite side, size accounting only,
excluded from every throughput window), and the wave's write throughput.
The SQLite twin's VACUUM policy is part of the lane definition
(`60-validation.md` § the churn lanes: `sqlite-bare` never maintains,
`sqlite-maint` runs the operator's periodic VACUUM + ANALYZE with the wall
time charged into its own throughput window as `maintenance_ns`), and every
VACUUM event is recorded on its cycle (`"vacuum": true` in the condensation)
and rendered as a marked event — the marker is data ON the cycle record, so
it can never drift from the measurement it annotates. Degradation is the
story: the lane exists to show what a long-lived, high-churn life does to
both engines — whatever ages, ages on the record. The run ends in a
three-way posting-multiset equality (driver model, engine, every twin) plus
`Db::verify_store` green before the series is worth anything.

## The night runbook

`scripts/bench-night.sh <out-dir>` — the one-command night, run by the owner
on an idle machine (or, under the shared-machine ruling below, with
`--shared` on a loaded one).

- **Mutex.** Refuses with exit 2 if the measurement mutex is already held
  (a night never queues behind another measurement), otherwise re-execs
  itself under `scripts/measure.sh` and takes **one** lock hold for the
  whole night. `BUMBLEDB_MEASURE_LOCK` parameterizes the lock path for
  tests.
- **Build.** `cargo build --release -p bumbledb-bench`, plus the `obs`
  feature build into its own target dir for `sweep-commit`. Building is not
  measurement.
- **The lane table, in order.** The registry above, cheap first,
  adversarial + churn last — their failures cost nothing upstream.
- **Per-lane resume.** An existing artifact is never rerun: a crashed night
  resumes with the same command, and completed lanes report SKIP-EXISTING.
- **Availability probing.** An expansion lane is available iff the binary's
  help lists its subcommand; an unlanded lane reports SKIP-UNAVAILABLE
  (the `adversarial` row today) instead of failing the night.
- **Charts.** Every chart renders via
  `python3 scripts/bench_viz.py --night <out-dir> --out <out-dir>` — into
  the night dir, never into `assets/` (the owner's ceremony path is a
  separate, deliberate invocation from committed pins).
- **Manifest.** `MANIFEST.txt`: date, rev, one status line per lane, the
  chart count, COMPLETE/INCOMPLETE. Exit 0 iff no lane failed (SKIPs are
  not failures).
- **`--plan`** prints the lane table with the statuses the run would have
  (RUN / SKIP-EXISTING / SKIP-UNAVAILABLE) — no lock, no build, no
  execution, no viz.
- **`--shared`** — the shared-machine night (owner ruling, 2026-07-20: the
  bench outranks the owner's own background agents). The idle-machine
  requirement is waived for the run: every lane launches with
  `BUMBLEDB_BENCH_BOOST=1`, the binary claims user-interactive QoS at its
  dispatch seam (`pthread_set_qos_class_self_np`, macOS; no-op elsewhere,
  never sudo — `crate::boost`), and every lane report's provenance stamps
  `shared_machine: true`, `boost: "qos-user-interactive"`, and the 1/5/15
  load averages at lane start and lane end — a boosted number can never
  pass as an idle-machine number. The measurement mutex stays mandatory
  (it is not an idleness check), and the honesty floor under load is
  unchanged: interleaved A/B sampling and the clock-proxy contamination
  exclude-and-count still govern every timed block.

**The chart inventory** (the `CHARTS` registry in `bench_viz.py`, one line
per svg → source lane → what it shows):

| svg | source | shows |
|---|---|---|
| `bench-vs-sqlite.svg` | RunReport pool (reads) | ours vs SQLite p50 per read family — EVERY family in the pin, gate and report class alike — log scale |
| `bench-speedup.svg` | RunReport pool (reads) | the same data as multipliers; below-parity draws red |
| `bench-tails.svg` | RunReport pool (reads) | p50 → p95 → p99 per family, both engines |
| `bench-writes.svg` | RunReport pool (writes) | writes + cold — fsync physics, published anyway |
| `bench-scenarios.svg` | `scenarios.json` preferred, `scenarios.md` fallback | the non-ledger worlds per (query, lane); a DNF lane draws no bar, only the annotation |
| `world-<world>.svg` | `scenarios.json` preferred, `scenarios.md` fallback | one file per scenario world, paired p50 bars per (query, lane); DNF lanes annotated, excluded and counted |
| `ratio-waterfall.svg` | reads (+ `scenarios.json`/`.md`) | every family + (query, lane) as one sorted ratio bar from raw p50s; below-parity draws red; DNF lanes excluded and counted |
| `tails-fan.svg` | reads | the p50 → p90 → p99 fan per family, both engines |
| `bench-storage.svg` | `storage-report.json` (night path, flag override) | bytes per fact per scale/world + churn checkpoints |
| `bench-writes-rates.svg` | `writes-report.json` (night path, flag override) | rows/sec per (family, batch), per durability lane |
| `bench-curves.svg` | `curves-report.json` (night path, flag override) | log-log scale curves, exponents, DNF caps |
| `bench-warmth.svg` | `curves-report.json` (night path, flag override) | cold/warm/memoized, both engines |
| `write-throughput.svg` | `write_throughput` lane, else derived from `writes-report.json` | facts/sec per commit batch, per durability lane × ladder |
| `adversarial-dnf.svg` | `adversarial` lane, else derived from `scenarios.json` `exceeded_cap` lanes | ours vs SQLite p50; capped twins drawn as capped (hatched to the cap), never as numbers |
| `churn-latency-<run>.svg` | `churn-report.json` (night path) | one file per run: every probe's warm p50 over cycles, every lane; VACUUM+ANALYZE samples marked from `maintenance_ns` |
| `churn-size-<run>.svg` | `churn-report.json` (night path) | store size over cycles, every lane, per run |
| `churn-throughput-<run>.svg` | `churn-report.json` (night path) | write commits/sec over cycles, every lane, per run |

**Retired charts** (2026-07-20, the owner's bench-refresh pass — no
phantom-input chart survives): `storage-bytes-per-fact.svg` and
`curves-loglog.svg` consumed lane payloads no emitter writes, and their
data is fully rendered by `bench-storage.svg` / `bench-curves.svg` from the
real reports; `cold-warm-memo.svg`'s contract had no theirs-memo slot, so
it could not represent the real warmth measurement (`curves --warmth` times
`theirs_memoized`) that `bench-warmth.svg` draws whole; the single-file
`churn-*.svg` trio consumed the retired churn condensation and is replaced
by the per-run set above.

**The agent law**, stated plainly: agents never run timing lanes. Correctness
smoke tests (tiny corpora, oracle multiset-agreement gates, the fixture
dry-runs) are not measurement and run anywhere; every number arrives only
from the owner's night — idle-machine, or a declared `--shared` night
stamped as such in provenance — under the one lock, published by hand.
