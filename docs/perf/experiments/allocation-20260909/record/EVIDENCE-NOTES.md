# Evidence audit before candidate selection

Source remains `b02a641e087364ec09c161a97c67c87cf626e6b2`. No engine edits.
The previous research turn made progress by freezing both binaries, passing
2,879 verification cases and starting the ordinary full suite. The migration
answer was an intervening user question, not an optimization experiment.

## Running work

The original baseline session is 18763, wrapper PID 45579, runner PID 53567.
The curves child PID 56690 was independently observed alive during this audit.
Do not restart it based on an old manifest or an observation timeout.

The published run's manifest records about 32 minutes for curves, six minutes
for reads and seven minutes for scenarios. These are historical durations, not
a deadline or prediction. New ordinary timing must finish before any profiling,
allocation-build, census, export, test or other competing measurement starts.

## Allocation-window boundaries checked against source

- `harness/measure.rs` resets event counters after warmup but before allocating
  its `Vec<u64>` of timing samples. With proxy-per-rep off, raw window totals
  include one sample-vector allocation requesting `samples * 8` bytes.
  The vector is still alive at snapshot time. Preserve raw counts and explicitly
  distinguish this stable harness component; do not advertise it as an engine
  allocation or treat the window byte difference as retained engine memory.
- `driver/read_family.rs` constructs parameter arguments and `WorkContext`
  inside each measured read. Those are part of this public-operation workload;
  prepared-query and reusable `Answers` construction happen before the window.
- Scenarios run their separate allocation window through the same read
  operation. Their report's allocation object is a window total, despite the
  human report's phrase "per query". Divide by its actual sample count, not
  by answer count or by the profiling window's draw denominator.
- `CountingAllocator` measures Rust allocation requests, including requested
  realloc sizes. Its lifetime live/peak counters are not reset with the event
  window and are not RSS, LMDB mapping residency or C-allocator traffic.
- The ignored deep census's backtrace recorder itself allocates. Attributed
  window totals therefore cannot be read as engine allocation totals. `SITE`
  events identify outer allocation call sites; unattributed windows provide
  counts for their own exact operations, not interchangeable cold/warm controls.
- Several insert census windows include caller fixture construction or row
  cloning. Separate those call paths from engine costs. Preserve event-cap
  losses and unresolved source attribution.

The diagnostic runner now builds the census with release-equivalent profiling
flags and packed symbols, freezes its executable/dSYM, checks their UUIDs, and
requires actual engine source-line attribution after a completed census test.
Previously it selected release, whose Cargo profile has no source debug info.
Both allocation family rosters must now be complete and have counter records.
These changes are syntax-checked only until the live baseline releases the
measurement lane. They modify ignored research orchestration, not engine code.

The next continuation is a verified wait plus further source/caller auditing:
the original process remained live, and the large ledger's `oracle.sqlite`
grew from 798,736,384 to 814,190,592 bytes between observations. Its only open
database handles were SQLite's main/WAL/shared-memory files. The loader source
confirms this comes after Bumbledb ledger loading, compaction and closure,
before calendar setup. Do not attribute this setup delay to the query engine.
The full-suite diagnostic now retains its whole-main view and a second
`bumbledb::`-rooted export from the same raw capture for database lanes. The
second view can include engine work on helper threads, but still includes
engine setup and is not isolated operation timing. The export uses a function
substring, so a generic type argument can also match. Its entire selected CPU
must not be labeled engine-only. The runner separately retains CPU assigned by
the analyzer's anchored nearest-engine-owner test versus the outside bucket;
those caller records distinguish ordinary SQLite-only work from engine work.
The mixed hash-comparison lane keeps its whole-main view only.

Read-profile source was checked against ordinary `read_family`: both create
parameter arguments and a fresh read context inside each operation. Scenario
profiling and allocation use the same `Engine::sample` as ordinary timing.
Profiler warmup uses eight complete draw cycles; do not equate its steady-state
window with the ordinary report's cold/rebuild-tail distribution.

The detailed newer historical `r4_bomb_t2/whale-analyzed.summary.json` has
15,733 positive selected samples, about 0.98 selected CPU/elapsed, no observed
interior-unrooted CPU and 0.0064% unresolved leaf CPU. Its largest self-owner
paths run through pipeline sibling probing (both caller and probe walk),
gather/hash construction, and map key copying. Reading full caller paths
confirms that these are middle-node batched probes, not output serialization.
Generic symbols show LTO aliases (`CrudWorld` on a rings query); caller/workload
identity is authoritative. This is still old-source evidence, not permission
to bypass the new broad trace/allocation round.

## Broad orientation, not a candidate verdict

Read the complete historical 66-family survey
`bench-out/autoresearch-native81-analysis82b.survey.md`: point/keyed, projections,
deduplication, aggregate, graph, ring, closure, temporal and displaced families.
Deadline and older result-carrier costs in it are obsolete. A source diff from
the newer old trace source `df737a6f` to 1.1.0 also shows substantial constructor,
binding, sink, result and execution changes. Neither historical profile ranks
the present engine authoritatively.

The published 1.1.0 ordinary reports are from the byte-identical timing binary.
They provide leads pending the fresh baseline and current stack evidence:

| Family | Published p50 | What to distinguish in current evidence |
|---|---:|---|
| `r4_bomb_t2` | 1.291 s | Failed closing-probe enumeration versus allocation or result work |
| `disp_probe` variants | 169–186 ms | Shared gather/probe/aggregate routing; foreign conditioning is not query execution |
| `t2_overlap_join` | 39.950 ms | Interval traversal, materialization and caller overhead |
| `o4_segment_category` | 27.661 ms | Shared probe routing, grouped folding and repeated shape lookup |
| `range` and full-result delivery | Different contracts | Column gather, word/cell representation and actual ownership transfers |

These are historical single-run absolute costs, not regression or speedup
claims. Read p99 and parameter mixes too: some small medians coexist with much
larger tails. Do not equate an aggregate's one output row with one intermediate
binding. The ring query is a normal three-way Count over a bipartite relation
with a planted triangle; optimizations must generalize beyond that fixture and
preserve set semantics, exact count/overflow behavior and cancellation.

## Next actions

1. Revalidate the existing baseline session and manifest; wait for terminal
   completion without restarting a live child.
2. Review every ordinary lane, including caps, losses and flags.
3. Latest user direction supersedes the originally planned trace passes:
   **no new full trace until the saved traces' actionable improvements are
   exhausted.** After ordinary timing completes, run `diagnostics.py allocations`
   and `census` under `scripts/measure.sh`, serially with distinct outer logs.
   Keep original failures if repairs are needed; phases refuse existing paths.
4. Mine the saved broad family survey and full caller paths for shared leaders,
   auditing source changes and matching current allocation counts/sites to
   exact operations. Select the highest-impact supported mechanism and record
   a falsifiable prediction and controls; implement and test the supported
   improvements before another full trace. Both prepared trace phases now
   explicitly refuse to run until that evidence-backed exhaustion audit.

No release, no removed long churn workloads, no quota/fallback framework.
The open-ended research goal remains active.

## Continuation after the migration answer

Revalidated session 18763 and the original live process tree; no restart.
The migration answer did not advance the research experiment. This
continuation is a verified wait plus new source evidence, recorded in
`PROBE-LEAD.md`: duplicate-triggered table growth and retained dead construction
slots copied by same-shape occurrence reuse. These remain hypotheses to rank,
not permission to bypass the full current trace/allocation survey.

An ambient process snapshot during curves showed unrelated Node processes
using about one core each, plus substantial desktop/indexing activity. The
manifest already declares a shared host. Do not stop those unrelated tasks
or interpret this run against published numbers as an isolated regression
experiment. Matched alternating controls remain necessary for any candidate.

## Current baseline review and denominator correction

`BASELINE-REVIEW.md` now contains the complete curves and 32-family read
review, including both SQLite caps, per-engine frequency flags, the failed
non-gating p99 target, write losses and mixed-draw tails. Scenarios and the
other local lanes are still running; there is no whole-suite success yet.

The six displaced families use 12 samples by default, while the read report's
top-level config says 256. Source confirms the separate protocol and absence
of a CLI override. `diagnostics.py` now records **per-family** denominators
and sample-vector bytes; dividing displaced totals by 256 would understate
per-operation allocations by 256/12. No raw report was rewritten. Syntax was
checked; runtime diagnostics remain pending the timing lane.

Additional current workload/source context for later caller interpretation:

- `spread` is a Posting self-join on entry with a cross-atom amount inequality,
  emitting distinct amount pairs; there are no parameters. It is not a grouped
  aggregate. Check residual evaluation and answer deduplication as well as
  sibling probing; do not attribute its 13.622 ms median to map growth blindly.
- `disp_probe` joins 1,048,576 Spokes with 524,288 Hubs and groups a Sum by
  1,024 tags. `disp_stream` is the corresponding one-relation Sum control.
  ForeignStream owns its byte buffer before the measured window; stream()
  writes one byte per cache line in place. Its conditioning is inside the
  allocation window but outside ordinary latency timers, and inside the
  native profiling window. Keep its CPU separate from engine CPU.
- `forced_spoke_map_bytes` is a source-derived final control/bucket-table
  estimate only. It excludes dense indices, retired construction tables,
  chunks, nodes, vector slack and shared images. It is not an observed
  retained-memory value and cannot substitute for a pool/allocation census.
- `o4_segment_category` is a three-way, two-dimension rollup, 64 groups; it
  supplies a different aggregate consumer of the common join machinery.
