# The performance PRD suite

The benchmark exists, verify passes 2,029/2,029 cases, and the first honest report
says **FAIL: 5 of 8 read families win, 3 lose** (fk_walk 2.37×, balance 1.77×,
string 6.35×), with the p99 budget line red and the write numbers distorted by a
durability asymmetry. Every loss was traced to a specific engine defect with
span-level evidence. This suite fixes them **for good** — ambitious,
backwards-incompatible cutovers, no compatibility shims that outlive their PRD.

`docs/architecture/` stays the design authority; PRD 10 reconciles it. The
evidence below is from the 2026-07-03 S-scale run on the pinned corpus
(`12d08c93…`), report `bench-out/2026-07-03T20-05-03Z/`, plus targeted obs traces.

## The five findings (measured)

1. **Param-equality has no access path.** The only plan for `field = ?param` is a
   filter inside the occurrence's view, and the view memo
   (`api/prepared.rs`, `built_generation`/`built_filters`) is one slot deep, keyed
   on exact filter equality — so every param change rebuilds the view by a
   **full-relation scan**. string: `view_build` 51.7 µs of a 54.4 µs execution to
   produce 21 survivors from 100k rows. fk_walk: 46.6 µs of 118.9 µs for 109
   survivors. SQLite probes an index.
2. **Finalize resolves interned strings per row.** `ResultBuffer::push_word`
   (`api/prepared.rs`) calls `dict::resolve` — one LMDB B-tree lookup — for
   *every String/Bytes cell of every emitted row*. fk_walk: 60.9 µs of 118.9 µs
   spent resolving **one distinct holder name 109 times** (~560 ns each). The
   same defect inflates the 9.6–11 ms hot-set p99s failing the budget line.
3. **Cover choice prefers label over magnitude, and map iteration is
   O(capacity).** `run.rs::choose_cover`: "an Exact always displaces an
   Estimate" — so `Exact(500)` (Posting's forced account level) beats
   `Estimate(7)` (the param-filtered Account view), and the executor iterates
   the wrong side. Worse, `colt.rs::force` sizes maps at `2 × ingested
   positions` (200,000 slots for 500 distinct account keys — 0.25 % occupancy)
   and `iter_map` walks **every slot**. Measured: a light-holder balance
   execution spends 220 µs of 223 µs walking that map.
4. **The write comparison is unfair to us.** commit_single: 5,086 µs vs 124 µs.
   LMDB pays `F_FULLFSYNC` on macOS (true flush-to-media, 3–8 ms on Apple
   SSDs); SQLite's default `fullfsync=OFF` issues a plain `fsync` that macOS
   does not propagate to the disk cache. Both engines claim
   `synchronous=FULL`; only ours pays for it.
5. **The store file is 101 MB vs SQLite's 13.6 MB** — three multipliers:
   39 % of pages are freelist churn from 46 bulk-load commits (LMDB never
   shrinks its file), `_data` carries ~5–6 B-tree entries per fact by design
   (fact + membership hash + unique guard + one back-reference per FK), and
   pages are 16 KB on Apple Silicon. Live data ≈ 64 MB; the freelist half is
   pure recoverable waste.

Planner estimate dishonesty threads through 1 and 3: worst per-node
estimate-vs-actual factors of 114,679× (fk_walk), 4,762× (string), 102× (balance).

## The fixes, in order

| PRD | Title | Kills | Status |
|---|---|---|---|
| 00 | Selection lowering (plan-level Eq extraction) | groundwork for 1 | landed |
| 01 | COLT selection levels | groundwork for 1 | landed |
| 02 | Execution cutover: probe, don't scan | finding 1 | landed |
| 03 | View memo LRU for residual filters | finding 1 (range/chain tail) | landed |
| 04 | Finalize intern memo + buffer dedup | finding 2 | landed |
| 05 | COLT dense iteration + honest map sizing | finding 3 (iteration) | landed |
| 06 | Magnitude-first cover cost model | finding 3 (choice) | landed |
| 07 | Distinct-count statistics and planner honesty | estimate dishonesty | landed |
| 08 | Fsync fairness | finding 4 | landed |
| 09 | Store compaction and size honesty | finding 5 | landed |
| 10 | Perf tripwires and doc reconciliation | regressions, forever | landed |

**Status: the suite is fully landed.** Enforcement lives in
`crates/bumbledb-bench/src/tripwires.rs` (plus the per-PRD unit tests named in
each file); the measured outcomes: every read family's profiled work is
bounded by its logical selectivity, no family rebuilds a view after one param
rotation, finalize resolves each distinct string once, and worst est/actual
fell from 114,679× to ≤ 3.3×. The re-run and the claim belong to the human
owner (`scripts/bench.sh`, then `BENCH_SCALE=L`).

## Rules

1. **No smoke-test or end-to-end PRDs.** Humans own e2e. Every passing criterion
   here is a unit/integration test inside the workspace, `scripts/check.sh`, or
   a structural assertion.
2. **No migrations, ever.** Storage-format or corpus-affecting changes
   re-baseline pinned digests inside the same PRD; existing stores are
   regenerated, never migrated. Humans own anything beyond that.
3. **No wall-clock assertions.** Machine-independent criteria only: trace-event
   counts, `CountingCounters` work bounds, allocation counts, byte sizes with
   relative bounds. The timing claim is re-earned by a human re-running
   `scripts/bench.sh`; nothing in this suite asserts microseconds.
4. **Cut over hard.** Transitional adapters may exist *within* the suite (a PRD
   may land representation before behavior) but must be deleted by the PRD that
   consumes them — named explicitly. No dead old paths survive PRD 10.
5. **The zero-warm-allocation contract is non-negotiable.** Every PRD that
   touches the execute path states its allocation story and extends the alloc
   gate where the protocol grows (02, 04 do).
6. **Verify stays green after every PRD.** These are performance changes;
   results must be bit-identical. `verify`'s full S run is already a `#[test]`;
   it gates every commit via `scripts/check.sh`.
7. **Docs stay true.** PRD 10 amends `30-execution.md`, `40-storage.md`, and
   `50-validation.md`; interim PRDs note deviations in code comments pointing at
   PRD 10.

Humans own after 10: re-running `scripts/bench.sh` at S, then the L-scale
ALL-WIN attempt on the canonical machine, and the claim itself.
