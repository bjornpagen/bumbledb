# Measuring bumbledb

The benchmark crate is `crates/bumbledb-bench/`. Measurements describe a
specific source revision, machine, workload, and durability contract—not the
database in every deployment.

## Run the full local suite

From the repository root, preview the complete ordinary lane roster:

```sh
scripts/bench-night.sh bench-out/new-round --full --plan
```

Then use a fresh output directory:

```sh
scripts/bench-night.sh bench-out/new-round --full
```

The runner builds one ordinary release binary, acquires the measurement
mutex, runs verification before timing, and executes lanes serially.
Verification failure stops timing. Later lane failures are recorded while the
remaining lanes continue. Existing report files are skipped, so a reused
directory is not a fresh run.

The default without `--full` is the compact storage/lifecycle/hash subset.
It does not include the main Free Join versus SQLite comparison.

`BUMBLEDB_BENCH_BIN=/absolute/path/to/frozen/bumbledb-bench` skips the build
and runs that exact executable. `BUMBLEDB_BENCH_DATA=/absolute/corpus/root`
isolates generated corpora and scratch databases from prior layouts. The
manifest records the binary digest. The main read panel fixes read batching
to one, so its percentiles describe individual calls, not batch averages.

## Native stack profiling

Use **Samply** for native stack sampling, separately from clean timing. Its
flamegraph, call tree, source view, and timeline cover the actual optimized
program: Rust inline frames, LMDB, allocation, and operating-system calls.
This is the sole profiling system. The engine has no named-span recorder,
phase timers, begin/end hooks, trace feature, or instrumented execution path. Allocation
counting remains a separate diagnostic build, not a second profiler.
Executor tests retain direct work/batch counts and cache-state assertions;
they do not retain a test-only phase profiler.

One-time setup, before any measurement is running:

```sh
cargo install --locked samply --version 0.13.1
cargo build --profile profiling -p bumbledb-bench
target/profiling/bumbledb-bench verify --dir bench-data/profile-corpus
```

The `profiling` Cargo profile inherits release's O3, fat LTO, and one codegen
unit, adding full, packed debug information without changing frame-pointer
policy. On macOS this produces the dSYM before temporary native objects are
removed; an unpacked debug map can otherwise lose LMDB/SQLite C line records.
There is no production profiler dependency. Native sampling does not require
nightly function-entry instrumentation: `-Z instrument-xray` and
`-Z instrument-mcount` change the instrumented program and are not baseline
flags; `-Z self-profile` profiles the compiler, not the database.

Capture one real, warm engine workload for long enough to collect stacks:

```sh
bash scripts/profile.sh bench-out/profile-balance \
  target/profiling/bumbledb-bench profile \
  --family balance --seconds 15 --dir bench-data/profile-corpus \
  --out bench-out/profile-balance/workload
python3 scripts/flame.py native bench-out/profile-balance/profile.json.gz \
  bench-out/profile-balance/cpu \
  --workload bench-out/profile-balance/workload/workload.json
samply load bench-out/profile-balance/profile.json.gz
```

The wrapper acquires the same measurement mutex as benchmarks and freezes
the executable, packed dSYM, source patch, command, and provenance. On macOS
it refuses missing packed symbols. When freezing a Cargo executable, copy its
matching dSYM with `cp -RL`: a relative symlink can become dangling, and
rerunning `dsymutil` after temporary compiler objects disappear cannot recover
complete symbols. Check matching binary/dSYM UUIDs and actual source/inline
resolution. The profile stays local; nothing is uploaded and no browser opens.
`BUMBLEDB_SAMPLY` can name an installed Samply binary outside `PATH` for both
scripts. Build/test jobs must already have stopped.

The source patch includes staged and unstaged changes plus untracked source.
It records the capture-time worktree, not proof of what built an arbitrary
input binary. Build and freeze together before editing again; preserve that
build's source snapshot when profiling an older executable.

`flame.py native` resolves each sampled address through a temporary localhost
Samply symbol server, then stops that server. It writes a CPU-weighted SVG,
folded stacks, self/inclusive rankings, accounting summary, and the raw
address-specific symbol response (including inline source locations). Outputs
refuse overwrites. The default root is the read workload boundary; an optional
last argument selects another function substring. Export **after** sampling,
and never concurrently with another benchmark.

`scripts/flamediff.sh before.folded after.folded NAME` compares native exports
without overwriting existing output. Use matching roots, draw mixes, and
sampling durations. Colors compare raw sampled CPU weight, not per-operation
latency; confirm any proposed improvement with unprofiled timings.

Do **not** use Samply 0.13.1's `--unstable-presymbolicate`: its cache collapses
debug information by machine-function start address, assigning the same
inline stack to different instructions. The exporter bypasses adjacent
`.syms.json` files using a temporary profile alias. For interactive viewing,
also keep that lossy sidecar away from the original profile name. Preserve
the executable and dSYM; source-level debug information is not optional.

The export attributes each thread CPU delta once to its sampled stack. It
does not multiply CPU deltas by coalesced sample weights or mistake sleeping
records for CPU consumption. Summaries expose selected, unrooted, empty-stack,
zero-CPU, and unresolved attribution. Inclusive rows overlap; never sum them.
This is statistical endpoint attribution, not an exact timer for each function.
The interactive native timeline is still necessary for blocking and wall time.
Full LTO can merge identical functions: a displayed generic specialization can
name another caller's identical machine code. Use callers, addresses, source
locations, and the selected workload to interpret these aliases, not a leaf
type name alone.

The attribution report also records positive sample-record support per
function and exact self-CPU source sites, using each instruction's innermost
inline location. A missing line remains unknown; it is never borrowed from
the enclosing function. Counts distinguish an unresolved leaf from an
unresolved ancestor, and separately expose missing source lines. Invalid
table indices, nonfinite CPU deltas, and mismatched symbol addresses refuse
instead of producing plausible-looking attribution. Sample records can be
correlated; these counts are not confidence intervals.

Pass `--workload` to bind attribution to the completed read-window report.
The exporter checks the executable module, process, sampling root and complete
draw denominator. It records sampled CPU per completed draw, not latency.
For a new candidate, compare these bound reports:

```sh
python3 scripts/flame.py compare bench-out/before/cpu.summary.json \
  bench-out/after/cpu.summary.json bench-out/after/vs-before
```

This comparison requires matching family, scale, seed, draw count per cycle,
output count, schema/query/parameter fingerprint and full-answer fingerprint.
Different miss streams must not compare just because both return nothing.
It divides each function's CPU by
completed draws, **not** by profile duration or total CPU: a faster engine
usually completes more draws in the same sampling window. JSON and text
retain sample support, unresolved accounting and disappeared functions.
Raw folded diffs remain useful views, but their colors are not per-operation
cost. Neither comparison can prove a latency improvement or replace matched
unprofiled controls. Hardware and clock comparability remain separate checks.

### Caller context and whole-roster analysis

Do not optimize a generic leaf such as `checked_sub` or `mach_absolute_time`
without reading its callers. The same library helper can serve trie iteration,
result deduplication, deadline enforcement, or the benchmark driver. A resolved
symbol is not automatically a diagnosis.

Reanalyze an export using its preserved instruction-address symbol response:

```sh
python3 scripts/flame.py analyze bench-out/profile-balance/cpu.summary.json \
  bench-out/profile-balance/analyzed
```

This reads the original raw capture and `.symbols.json`; it neither samples
again nor starts another symbol server. It checks that the complete request
(module identities **and every instruction address**) still matches the
capture, and that CPU and sample accounting are unchanged. Outputs must use a
fresh prefix. The original capture, symbol response, and exports are retained.
Run analysis serially, outside timing/profiling/build/test windows.

The analyzed JSON adds complete self call paths and their positive sample
support. It also assigns each self sample **once** to the nearest `bumbledb`
caller, including library helpers below that caller. This is an exclusive view
of actual stacks, not manually named phases; costs outside the engine remain
explicit. Original caller paths remain authoritative when LTO aliases names.

Sampling-quality fields distinguish missing leaf symbols, missing source
lines, and **unrooted CPU between the first and last rooted samples on each
thread**. The latter can reveal broken unwinding that a low unresolved-symbol
rate would miss. It is an observed sample bracket, not an exact operation
boundary. CPU/elapsed is exposed separately: a low ratio can indicate
descheduling, blocking, or lost attribution, and cannot by itself diagnose I/O.
No invented statistical confidence or automatic quality pass is attached.

Build a survey with an explicit expected family roster and all its analyzed
reports (the example deliberately covers only two reads):

```sh
python3 scripts/flame.py survey bench-out/read-survey \
  --expect balance,stats \
  bench-out/profile-balance/analyzed.summary.json \
  bench-out/profile-stats/analyzed.summary.json
```

The survey emits JSON and Markdown with every supplied family's cost, sampling
quality, and caller leads. Missing expected families are listed, and the command
exits nonzero after writing the partial report. Duplicates, unexpected families,
mixed executable/module identities, scales, or seeds refuse. The cross-family
ordering gives each captured family equal weight using its **within-family CPU
share**; it does not pool raw CPU from different-duration workloads. This is a
way to find shared mechanisms, not a suite speedup or an application's traffic
model. Per-family CPU/draw and sample support remain visible. Full coverage
means only the supplied roster: reads do not qualify writes, durability,
lifecycle, storage, residency, hosted behavior, or a release.

### Native workload coverage

The `profile` command supports all registered read families and individual
scenario queries such as `r4_bomb_t2`, `t2_overlap_join`, or `p5_keyed_get`.
Ledger/calendar reads require a fresh oracle stamp and never mutate their
corpus. Closure/displaced reads and scenario queries use their ordinary SQLite
oracle gate on a fresh corpus under the diagnostic output directory.
Only scenarios have one fixed scale; closure/displaced retain their registered
scale behavior. Displaced captures perform their real 0/24/96 MiB foreign
stream before every draw, **inside the diagnostic window**, and record those
bytes explicitly. The `ForeignStream::stream` stack is conditioning, not query
execution. Unprofiled displaced timing continues to exclude that conditioning.
Existing output directories and `alloc-counter` builds are refused. Every
cycle executes every registered draw through fresh
`Work`/read/execute calls. It records an untimed full-answer fingerprint and
checks each draw's row count during the read-only window. Scenario timing,
allocation windows, and native sampling share the same read operation,
including the direct keyed-get surface. Filter to
**`profile_read_window`** in the call tree to exclude open/prepare/warmup,
fingerprinting, and reporting. The window may exceed `--seconds` to finish
its final draw cycle. `workload.json` is diagnostic metadata, not a benchmark
score or latency distribution.

Profile individual scenario queries separately so a one-second cyclic join
does not hide a microsecond point read in an aggregate flamegraph:

```sh
scripts/profile.sh bench-out/profile-rings target/profiling/bumbledb-bench \
  profile --family r4_bomb_t2 --seconds 10 --out bench-out/profile-rings/workload
```

Setup, full answer checks and warmup are outside the selected sampling root.
The native window repeats complete registered draw cycles, never just a
favorite hit or an artificially amplified inner kernel.

Require that completed workload report before treating a read capture as a
successful run. A profile file alone can also come from an aborted process;
Samply 0.13.1 can even return zero when its child terminates by signal. The
wrapper checks the native `profile` command's completed report against the
capture's process and executable; absent `--out`, it supplies a fresh
`<capture>/workload` directory automatically. For
other benchmark commands, inspect their own completion reports and capture
log. Never convert a partial capture into a successful workload measurement.

The same wrapper accepts other real benchmark commands—for example `writes`,
`crud`, `lawful`, or a selected `scenarios` run. Those profiles include setup,
SQLite controls, and reporting; select the appropriate engine stacks/time
range explicitly. Keep their reports outside the ordinary timing results.
For application activation or commit latency, inspect blocking time as well
as CPU samples; a CPU-hot function cannot explain time spent waiting for I/O.

The optimization loop is:

1. Freeze source and binaries; run the **full unprofiled local suite**.
2. Rank absolute costs and historical regressions by workload, not by one
   aggregate speedup. Preserve flags, caps, refusals, and incomplete lanes.
3. Collect native stacks for the expensive workloads. Select a real workload
   and its execution boundary when the operation is ambiguous. Check symbol resolution,
   inline frames, sample count, setup boundaries, and parameter coverage.
4. Record a specific bottleneck and a falsifiable explanation before editing.
5. Change that bottleneck; verify correctness; alternate clean baseline and
   candidate runs. Reprofile to check the mechanism, then rerun the full suite.

Sampling percentages are shares of the selected samples, not direct latency
improvements. Profiles are not performance verdicts. Keep the production
release binary as the timing authority: adding debug info can change code
placement even with identical optimization flags. Stop rather than invent
attribution when symbols or stacks are missing. Cross-machine qualification
still requires real Graviton and Linux x64 runs.

## What each lane measures

| Lane | Scope |
|---|---|
| `verify` | Registered and randomized engine/oracle comparisons before timing. SQLite-inexpressible cases are explicitly routed to the independent naive evaluator. |
| `storage` | S/M ledger and calendar file sizes: raw and compacted LMDB, indexed and table-only SQLite, checked row counts and checkpointed WAL sizes. |
| `app-perf-warm` | Reused prepared full-account projection, independently checked against canonical-row scanning. |
| `app-perf-cold` | Open/prepare/execute/dispose, and first prepared PostingTag projection after an actual delete or insert. Cold-open is not a flushed OS page cache. |
| `app-perf-large-result` | Prepared full-Posting query through CompleteResult, then real cursor-page construction and typed-value visitation through the terminal frame. Includes native result disposal. Not TypeScript serialization, network delivery, or Effect streaming. |
| `app-perf-tenants` | Open/prepare/project/dispose across small tenant stores with before/after descriptor counts; not the hosted `TenantCache` or S3 activation path. |
| `hash-probe` | Candidate hash timings and one-shot/streaming equivalence. Known-answer vectors are a separate input. |
| `reads` | Verified registered read families compared with SQLite, plus the runner's write/cold panels. |
| `scenarios` | Non-ledger join, graph, analytic, point, ring, and temporal workloads with their own comparisons. |
| `crud` | Application read/write families under the currently supported durability contract. |
| `lawful` | Constraint/admission workloads and their SQL enforcement counterparts. |
| `writes` | Durable commit/delete batch ladder and insertion stream. There is no supported no-sync engine lane. |
| `curves` | Its four registered families (`triangle`, `point`, `busy_scan`, `closure_fanout`) at S/M/L; the warmth panel uses the first requested scale. Reopen-cold times the first execution after fresh open/prepare, not process startup or cold OS pages. Capped SQLite samples remain caps, not timings. |
| `churn` | Default steady/delete-heavy long-lived workloads with SQLite maintenance variants. |
| `heap` | Frozen-instance versus LMDB access and admission-prefix comparisons. |
| `primerlane` | Synthetic Primer-shaped builder/change/write/scan attribution. |

The last nine lanes require `--full`. Native profiles and allocation
experiments require separate diagnostic runs; they are not part of the
ordinary release timing suite. Generators and report
merges are not extra measurements. See `bumbledb-bench help` for exact flags.

Application reports with `protocol: 2` measure native prepared queries and
paged results. Earlier reports without this marker measured metadata-count
and vector-length scaffolds; their numbers are not comparable speedups or
regressions. `work` counts delivered output rows, not source visits. Tenant
rows do not inherit the unrelated main corpus's storage-size metadata.

The historical `cold_containment_walk` row inserts an unrelated Org before
timing the read; it does not invalidate the query's relation images.
`cold_containment_walk_delete` swaps a Posting instead, so the first read
must observe a new version of a queried relation. Both exclude the write
from the read timer. Neither represents process startup or cold OS pages.

## Provenance and ambient load

Record the full source revision, dirty changes, binary digest, toolchain, CPU,
OS, memory, page size, power conditions, command, and workload parameters.
Keep the raw reports, log, and final manifest—including losses and failures.

The mutex serializes participating benchmark processes; it does not establish
an idle machine. For an isolated baseline, stop unrelated builds and indexing
and hold power conditions stable. An explicitly authorized in-use-machine
run is useful, but label it as such rather than claiming isolation.
`--shared` additionally enables scheduler boosting; do not silently treat a
boosted run as an ordinary run.

A historical regression comparison needs repeated baseline/candidate trials
on the same host, with order rotated and baseline/baseline controls. One
current run cannot establish that the new implementation beat yesterday's.

## Storage interpretation

The ordinary storage report measures file lengths, not RSS or exact physical
allocation. LMDB's compacted file removes free-page history; its remaining
size still includes indexes, page/node overhead, and fill slack. SQLite is
checkpointed, with WAL bytes reported separately.

Live namespace bytes, page statistics, allocated blocks, and virtual map size
are distinct quantities. The census helpers under the benchmark crate's
`space/` module expose finer accounting, but the presence of those helpers
does not mean the ordinary storage JSON contains a full attribution.

Compare indexed SQLite with the compacted engine for the main ratio; the
table-only SQLite column is an additional reference, not the same indexing
contract. Do not attribute all overhead to UUIDs or hash width without an
actual namespace census and controlled layout variants.

## Coverage that local success does not supply

- Real S3/IAM publication, contention, and recovery.
- Graviton or Linux x64 application performance measured on those targets.
- A genuinely populated database larger than an enforced memory budget.
- TypeScript/native-boundary, Effect-runtime, and deployed application costs.
- Hash known-answer evidence when no `--kat` vector file was supplied.
- Repeated controls or per-operation raw samples absent from a lane's report.

The runner explicitly marks external prerequisites as not run.
`.config/obligation-inventory.json` and `scripts/release-results.mjs`
retain the independent release requirements. A local manifest must never be
used to mark those missing cells passed.

## Charts and retention

Render charts from reviewed reports with `scripts/bench_viz.py --night`.
It expects the runner's output layout. A chart is presentation, not additional
evidence; inspect capped, missing, and failed inputs before publishing it.

Keep one current checked-in result summary and current charts. Archive the
raw reports with the GitHub Release. Purge superseded charts and generated
database directories only after the replacement evidence is preserved.
Historical comparisons remain recoverable in Git history; do not retain
old charts in the README as if they measured the current engine.
