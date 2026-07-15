# The ephemeral roster characterization

The in-memory measurement pass over the FULL extended roster — the 15
ledger read families, the 9 calendar read families (the two slot-grid
families included), the 2 closure families, and the 8 write/cold
families (the three window-judgment rows included) — run against
`Db::ephemeral` stores through the real bench surface, alloc windows
on. This session CHARACTERIZES the ephemeral estate; it pins no
margins and re-earns no claim (the disk ALL-WIN record stands on its
own prior runs; the pinned-margin re-earn is a dedicated quiet-machine
session). Produced by the `--ephemeral` bench mode this record lands
with:

```sh
cargo run -p bumbledb-bench --release --features obs -- verify --scale S --seed 1 --dir bench-data
cargo run -p bumbledb-bench --release --features obs -- bench --alloc --ephemeral --dir bench-data
cargo run -p bumbledb-bench --release --features obs -- bench --alloc            --dir bench-data   # the durable twin
```

## Stamp

| | |
|---|---|
| Machine | Mac14,5 — Apple M2 Max (the pinned fact-ledger machine) |
| macOS | 15.7.7 (build 24G720) |
| Toolchain | rustc 1.99.0-nightly (be8e82435 2026-07-11), release profile, `obs` feature (the alloc window's counting allocator is compiled in for EVERY run recorded here — both kinds pay it equally) |
| Date | 2026-07-15 |
| Corpus | scale S, seed 1, digest `06e9620d…` |
| Verify | green BEFORE any timed lane: 2862 cases (the stamped family registry + 500 randomized queries + the naive differential slices, ledger and calendar), stamp `7eb950ca…`; the closure lane re-verifies inline per draw (row-identical across engines before its first timed sample); the windowed lane's naive parity is a standing suite test |
| Runs | three back-to-back ephemeral runs + one durable twin, separate processes; tables record per-run p50s and min-of-runs |
| Load | `uptime` 1-min load 1.4–2.5 across every run boundary (sibling agent sessions were live on the machine; the session's PROVISIONAL bar was load ~4 — no run crossed it). Characterization-grade by charter regardless: single-day, S scale, no margin pinned |
| Scratch | every timed store on the internal SSD (APFS) — the device-honesty rule stands unchanged in ephemeral mode (RAM-backed targets refuse), and R6 pinned ephemeral device-independence at 1.0–1.1x, so ephemeral-on-SSD is the legitimate zero-setup cell. `BUMBLEDB_SCRATCH_DIR` (the 5 GiB ram disk) stayed unused by the timed lanes |

## The lane (what `bench --ephemeral` is)

`Db::ephemeral` on the stamped durable corpus is the typed
`StoreKindMismatch` refusal — the kind is on-disk identity — so the
ephemeral mode loads the SAME generated corpus (the stamp's digest
identity) fresh into ephemeral twins and compacts them into place (the
compacted `_meta` carries the kind, so the twin reopens ephemeral with
the stamped corpus's live-sized geometry). Scratch write stores
(ledger commits, the windowed twin worlds, the closure world, bulk's
throwaway stores) build through the ephemeral constructor directly.
Everything else — protocols, draws, the SQLite mirror, the clock
proxy, gate semantics — is byte-for-byte the durable lane. One
reporting artifact of the kind: `db_bytes` prints the 4 GiB `WRITEMAP`
ftruncate, not live bytes (`du` stays small on APFS — sparse).

The mirror caveat, named: the SQLite side of every paired write block
stays durable-on-SSD (`synchronous=FULL` — the protocol's honest
opponent is the durable one), so paired write blocks brand
CONTAMINATED by the clock proxy — the mirror's fsync loop drags the
measured effective clock from ~3.3 GHz to ~0.9 GHz inside the block.
The brand is recorded below where it applies; the engine-side numbers
reproduce across all three runs inside those blocks. Write-family
ours-vs-sqlite ratios in ephemeral mode are kind-vs-kind
(ephemeral-vs-durable) and are NOT comparison claims.

## Read families — per-family warm p50 (ns), three ephemeral runs + the durable twin

256 samples/family, warm protocol, alloc window over the measured
samples. The alloc columns are IDENTICAL in all four runs —
byte-identical windows per family, store-kind-independent.

| family | eph1 p50 | eph2 p50 | eph3 p50 | durable p50 | dur/eph(min) | allocs | alloc bytes |
|---|---|---|---|---|---|---|---|
| point | 500 | 500 | 500 | 541 | 1.08 | 257 | 8192 |
| containment_walk | 2,708 | 2,333 | 2,459 | 3,000 | 1.29 | 257 | 8192 |
| chain | 64,042 | 64,916 | 62,708 | 67,250 | 1.07 | 257 | 8192 |
| range | 22,292 | 22,333 | 22,333 | 22,333 | 1.00 | 257 | 14336 |
| balance | 959 | 959 | 1,083 | 1,166 | 1.22 | 257 | 8192 |
| stats | 1,233,834 | 1,239,541 | 1,242,875 | 1,239,166 | 1.00 | 1 | 2048 |
| string | 2,625 | 2,708 | 2,625 | 2,625 | 1.00 | 257 | 8192 |
| skew | 1,625,083 | 1,626,041 | 1,634,625 | 1,625,875 | 1.00 | 257 | 8192 |
| spread | 10,830,833 | 10,904,500 | 10,875,292 | 10,855,666 | 1.00 | 1 | 2048 |
| triangle | 10,856,375 | 10,795,584 | 10,916,458 | 11,460,375 | 1.06 | 257 | 14336 |
| entries_for_account_set | 1,375 | 1,167 | 1,292 | 1,250 | 1.07 | 257 | 8192 |
| postings_without_tag | 3,042 | 3,541 | 2,833 | 2,916 | 1.03 | 257 | 8192 |
| latest_posting_per_account | 2,120,416 | 2,162,542 | 2,123,834 | 2,138,416 | 1.01 | 1 | 2048 |
| mandate_at_instant | 309 | 309 | 307 | 309 | 1.01 | 4097 | 198656 |
| mandate_overlap | 8,750 | 8,584 | 9,166 | 8,625 | 1.00 | 257 | 8192 |
| busy_scan | 8,334 | 8,375 | 8,292 | 8,333 | 1.00 | 257 | 8192 |
| meets_chain | 3,542 | 3,542 | 3,500 | 3,542 | 1.01 | 257 | 14336 |
| rsvp_union | 950,458 | 947,375 | 946,834 | 949,250 | 1.00 | 1 | 2048 |
| conflict_pairs | 22,291 | 25,834 | 22,333 | 30,000 | 1.35 | 257 | 8192 |
| conflict_free | 625 | 625 | 625 | 625 | 1.00 | 257 | 14336 |
| free_busy | 2,667 | 2,667 | 2,750 | 2,667 | 1.00 | 257 | 14336 |
| claim_hours | 507,791 | 507,834 | 508,250 | 508,042 | 1.00 | 1 | 2048 |
| slot_scan | 31,000 | 31,875 | 31,167 | 31,042 | 1.00 | 257 | 8192 |
| slot_booking_overlap | 23,958 | 23,500 | 20,000 | 21,125 | 1.06 | 257 | 8192 |
| closure_depth | 5,750 | 4,666 | 6,250 | 6,583 | 1.41 | 257 | 8192 |
| closure_fanout | 1,041 | 1,042 | 1,125 | 1,042 | 1.00 | 257 | 8192 |

**The read finding: the read path is store-kind-independent.** Median
dur/eph ratio 1.00 across 26 families; every divergence above ~1.1x
(conflict_pairs 1.35x, closure_depth 1.41x, containment_walk 1.29x)
sits inside the family's own cross-run spread or its draw-rotation
bimodality (closure_depth's rotation includes the 4096-round chain
head: p50 ~5 µs, p95 ~13.8 ms — by design), on a SINGLE durable twin
run. No family shows a `WRITEMAP` read tax.

## Write families — per-family p50 (ns), three ephemeral runs + the durable twin

| family | eph1 p50 | eph2 p50 | eph3 p50 | durable p50 | dur/eph(min) | proxy brand |
|---|---|---|---|---|---|---|
| commit_single | 24,583 | 27,417 | 26,209 | 5,050,417 | **205x** | block CONTAMINATED (the durable mirror's fsync), all runs |
| commit_witnessed | 26,625 | 27,959 | 57,792 | 5,166,542 | **194x** | eph3 clean but 2.2x its siblings — recorded, not averaged in |
| commit_batch (512 facts) | 3,546,458 | 3,712,917 | 3,540,541 | 24,029,333 | **6.8x** | block CONTAMINATED (mirror fsync), all runs |
| commit_window_baseline | 10,458 | 10,458 | 11,375 | 4,481,000 | 428x | clean (engine-only rows) |
| commit_window_admission | 16,833 | 16,500 | 17,750 | 5,069,375 | 307x | clean |
| commit_window_exclusion | 16,458 | 15,417 | 16,417 | 5,124,958 | 332x | clean |
| bulk | 748,822,959 | 753,631,166 | 750,426,375 | 1,220,428,500 | **1.63x** | block CONTAMINATED (mirror bulk fsync), all runs |
| cold_containment_walk | 4,104,042 | 4,232,125 | 4,108,292 | 4,456,875 | 1.09x | block CONTAMINATED (per-sample touch-commit mirror), all runs |

Bulk throughput: 266.7k / 264.5k / 266.5k facts/sec ephemeral vs
163.3k durable (the same 1.63x). Alloc windows on the write lanes are
not captured (the write runners measure through the plain protocol —
a standing suite shape, not a gap this session opened).

**The write findings:**

- **The flags dividend is a function of commit shape, and the roster
  now prices the whole curve on the real surface: ~205x on one-fact
  commits → ~194x witnessed → 6.8x on 512-fact batches → 1.63x on
  bulk chunks.** The one-fact figure sits ABOVE R6's banked 83–158x
  because R6's small cell was 16 facts/commit — the smaller the
  commit, the more of it is the fullfsync floor. The durable one-fact
  p50 (5.05 ms) re-confirms R2's ~4–5 ms fullfsync baseline.
- **The ephemeral lane is the microscope for CPU-priced admission.**
  The window judge prices at **+6.0–6.4 µs/commit** over its
  window-free twin (baseline 10.5 µs → admission 16.5 µs, 1.6x) and
  the `{0}` exclusion at **+5.0–6.0 µs** — deltas that are REAL on
  the durable lane too but invisible there (5.07 vs 4.48 ms reads as
  1.13x fsync jitter). Same story one size down: the write witness
  prices at ~2 µs (26.6 vs 24.6 µs), unmeasurable under a 5 ms floor.
- **The cold rebuild is kind-independent (1.09x):** the image-rebuild
  spike is CPU + page-cache work, not sync work.

## What this bears on the µarch reread (confirmations and falsifiers)

- **CONFIRMS the fsync DVFS floor** (`m2max.clock.fsync-floor`,
  `m2max.clock.p-core-deep-floor`, judgment-layer Insight 14): the
  clock proxy measured ~3.3 GHz → **0.91 GHz** inside every
  fsync-paired block, in-band, on all four runs — while the
  ephemeral engine side, fsync-free, held 3.3–3.5 GHz through the
  same wall-clock window.
- **CONFIRMS the layer law in the storage domain**
  (`m2max.method.layer-law`, Insight 12): below the sync floor the
  surviving cost class is instructions — the window judge's ~6 µs
  and the witness's ~2 µs surface exactly when the floor is removed,
  and nowhere else.
- **CONFIRMS R6's bulk arithmetic:** durable/ephemeral 1.63x on bulk
  chunks against R6's 1.56x (insert work, not sync, dominates the
  chunk shape).
- **EXTENDS (and would falsify a misreading of) R6's dividend band:**
  any reread line that pins "~90x (83–158x)" as THE ephemeral
  dividend is shape-truncated — the band is parameterized by commit
  size, and the roster's one-fact cell measures 194–205x. Flag if the
  merged reread states a scalar.
- **CONFIRMS read-path kind-independence** (the `70-api.md` claim
  that the kind changes ONLY the sync boundary): 26 families,
  no `WRITEMAP`/`NOSYNC` read tax at any percentile worth naming.
  Any reread prediction of an mmap write-mapping read penalty is
  contradicted at this corpus scale.
- **CONFIRMS allocation is engine-structural:** alloc windows
  byte-identical per family across both kinds and all runs — device
  and store kind never reach the allocator. (The alloc-counter build
  itself is a constant of this record, paid equally by every cell.)

## What this record does NOT claim

No ALL-WIN re-earn (the gate printed ALL-WIN on all four S-scale runs;
that is informational here — the standing claim rests on its own
record). No pinned margins: single day, S scale, sibling sessions
live. The ~6 µs window-judge price and the 194–205x one-fact dividend
are characterization numbers awaiting the quiet-machine re-earn
session before anything gates on them.
