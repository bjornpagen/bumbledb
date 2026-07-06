# The performance push — microarchitecture PRD suite

This suite documents, once and for all, the remaining engineering to push
bumbledb's warm execution constants as close to the machine's limits as the
representation allows. It is the successor to the hardening suite
(docs/hardening/): same protocol, different axis. Hardening closed audit
findings; this closes the gap between where the profiler says the time goes
and where the M2's execution resources say it *could* go.

## Evidence base

Every PRD in this suite is grounded in per-(node, phase) executor time
attribution (`Category::Phase` events; `bench --trace` / `trace --family`,
obs build), captured at S/seed 1 on the reference host (Apple M2 Max). The
baseline capture lives in [baseline.md](baseline.md) (made normative by
[PRD 00](00-doctrine-and-baseline.md)) and is the denominator for every
measured passing gate in the suite. The headline attribution:

- Fold/scan families are **descend-bound, not memory-bound**: `stats` spends
  3,635 µs of its 4,357 µs join in `jp_descend_n1` exclusive time — per-row
  recursion bookkeeping plus per-row sink emit — against a ~20 µs
  streamed-column floor. `balance` is 774 µs descend + 272 µs iter.
- Suffix iteration costs ~6.5 ns/position (`jp_iter_*`, stats) against a
  ~1 ns gather floor: per-position `ColumnView` matches and bounds checks —
  and at pinned-row leaves it degenerates to ~16 ns *per call* at batch
  size 1 (spread: 3,342 µs across 200,000 iter calls).
- Deep nodes run at fanout-sized batches, starving the M2's ~28
  misses-in-flight budget the two-phase probe was designed to fill:
  triangle's jp_probe_n1 is 6,005 µs across 100,000 single-probe passes —
  60 ns of serialized latency each — while spread's root-level batch-128
  probes run the same map machinery at 21 ns/probe.
- `finalize` is 36% of `skew` (617 µs of 1,730): per-cell typed pushes.
- The cold path is 98% `image_build` (4,619 µs of the cold fk_walk sample).

## Doctrine for this suite

- **Machine model**: aarch64 is assumed; Apple Silicon (M1 and above) is the
  performance target. NEON (128-bit) is always present; SVE never is. Raw
  system-register reads, `prfm`, and inline asm are available tools. Portable
  (non-aarch64) builds must still compile and pass every test through scalar
  reference paths — with no performance promises, exactly as
  docs/architecture/30-execution.md already words it for kernel.rs.
- **Unsafe is sanctioned, tested, and caged.** `unsafe` (including inline
  asm and intrinsics) is permitted in named kernel/hot modules listed in
  00-product's amended policy — nowhere else. The law, extended from
  kernel.rs: **every unsafe path has a safe portable reference
  implementation, and a property test asserts bit-identical results across
  randomized inputs including boundary shapes**. The differential oracle
  (verify, 2,468 cases) remains the outer gate; the property tests are the
  inner one.
- **No auxiliary structures unless they pay immediately.** The branching
  principle: we push representation and microarchitecture first. An extra
  persistent structure (indexes, caches, precomputed layouts) is admissible
  only when the PRD demonstrates an easy, large win and names the maintenance
  cost it adds. Scratch buffers sized once per prepared query are not
  auxiliary structures; they are the existing zero-alloc discipline.
- **Phase-table-driven acceptance.** Each PRD's measured gates name specific
  `jp_*` rows and family p50s with numeric targets against the 00 baseline.
  A PRD is not done because its code merged; it is done when the phase table
  moved the way the PRD predicted. If a target is missed after honest
  implementation, the PRD documents the measured wall and why — a recorded
  miss with analysis is an acceptable completion; silent target-shaving is
  not.
- **Invariant gates on every PRD** (in addition to its own gates):
  `scripts/check.sh` green; `bumbledb-bench verify` (2,468 cases) green;
  batch-size equality tests green (results identical across batch sizes
  {1, 2, 64, 128, 1024}); the ALL-WIN read gate holds; no read family's p50
  regresses more than 5% from the previous PRD's recorded numbers; the
  zero-alloc warm gate stays green (`--alloc` window).

## Protocol (unchanged from the hardening suite)

Work the PRDs in order. Each PRD is a work-organizational unit, **not** an
atomic passing-code-state checkpoint — the tree may fail to typecheck
between PRDs; never build transitional shims to avoid that. Rip directly to
the end state. Commit per PRD with `--no-verify`. No smoke-test or
end-to-end-test PRDs exist in this suite by design: e2e validation is
human-owned. No migrations: stores are regenerated, never migrated, and any
store-format-affecting change is out of scope here anyway. Where a PRD
changes doctrine, it amends docs/architecture in the same commit —
the architecture docs remain the record.

## The suite

| PRD | Title | Attacks |
|---|---|---|
| [00](00-doctrine-and-baseline.md) | Doctrine + baseline | The unsafe policy, kernel law, and the committed baseline capture |
| [01](01-leaf-batch-emit.md) | Leaf batch emit | Per-row recursion + emit dispatch at the last plan node |
| [02](02-aggregate-batch-fold.md) | Aggregate batch fold | Per-row group probes + accumulator dispatch under folds |
| [03](03-fold-kernels.md) | Fold kernels | Scalar-ILP/NEON accumulation kernels behind the batch fold |
| [04](04-suffix-iteration.md) | Suffix iteration | Per-position match/bounds cost in `iter_batch` gathers |
| [05](05-leaf-scan-pushdown.md) | Leaf scan pushdown | The remaining copy between suffix iteration and the sink |
| [06](06-sink-map.md) | Sink map | WordMap probe layout + rehash-in-the-loop |
| [07](07-colt-probe-prefetch.md) | COLT probe prefetch + layout | Unprefetched bucket loads; two-line probes |
| [08](08-finalize-batching.md) | Finalize batching | Per-cell typed pushes in result materialization |
| [09](09-batched-executor-aggregates.md) | Cross-node batching (folds) | Fanout-starved MLP on aggregate plans; the recursion itself |
| [10](10-batched-executor-projections.md) | Cross-node batching (D2) | The same, under projection sinks with suffix skips |
| [11](11-point-path.md) | Point path | Fixed per-execution overhead on point lookups |
| [12](12-cold-decode-kernels.md) | Cold decode kernels | `image_build`'s per-fact decode loop |

Out of scope for the whole suite: representational cold-write redesign
(pay-at-commit vs incremental images — a design conversation, not a PRD),
the L-scale run and the performance claim (human-owned), scenario-suite
tracing, PMU-counter capture, and anything that changes query semantics.
