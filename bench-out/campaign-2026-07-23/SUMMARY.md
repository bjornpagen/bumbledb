# Campaign 2026-07-23 (RUN 2, wall power) vs night 2026-07-20 — full-suite geomeans

Protocol identical to the night run (seed 1, registered lane protocols, oracle-gated everywhere the night gated, shared-machine boost, measurement mutex per lane). The battery-era RUN 1 was retired whole at f474202a; every number below was measured on wall power, AC verified by pmset before launch and at the close of every lane. Corpora regenerated under the fixed RNG (R20): identity digest `6518394f080c2273…`, verify stamp `f1af7aff…`, per-world store hashes pinned in each lane's `digests.txt` — byte-identical across all six report-class reps, the curves/storage loads, and (post-run) the battery-era crud/lawful twins. Every geomean below is computed over the cells common to both runs, same pairing both sides; capped/DNF cells and twinless engine-only families are excluded-and-counted identically. ratio = ours/sqlite (lower is better); vs-night = campaign geomean / night geomean (<1 = we improved relative to SQLite).

| suite | cells | campaign geomean | night geomean | vs-night | note |
|---|---:|---:|---:|---:|---|
| bench-durable-r1 | 37 | 0.0780 | 0.0844 | 0.92 | reads + twinned writes; all_win holds, 33/33 read families WIN |
| bench-durable-r2 | 37 | 0.0810 | 0.0846 | 0.96 | |
| bench-durable-r3 | 37 | 0.0761 | 0.0840 | 0.91 | |
| bench-ephemeral-r1 | 37 | 0.0743 | 0.0689 | 1.08 | NOT like-for-like across runs: the night's ephemeral write cells rode the mismatched twin (finding 020) that flattered commit rows up to ~100×; the campaign side is the honest pairing. Reads-only (32 common): 0.0403 vs 0.0448 → 0.90 |
| bench-ephemeral-r2 | 37 | 0.0761 | 0.0672 | 1.13 | reads-only: 0.0418 vs 0.0436 → 0.96 |
| bench-ephemeral-r3 | 37 | 0.0801 | 0.0663 | 1.21 | reads-only: 0.0440 vs 0.0427 → 1.03 |
| scenarios | 31 | 0.0526 | 0.0773 | 0.68 | the targeted-query fixes cash (o3 22×, o5 12×, r1 5.8×, t2 ours 2.5×, p1 2×); r4/t2 SQLite DNF both runs, excluded-and-counted; detail in scenarios/delta.md |
| curves | 4 | 0.0416 | 0.0507 | 0.82 | clock-proxy bracketed per finding 072 this run (ghz on every point, 0/4 timed blocks contaminated); per-family movement below |
| crud | 22 | 1.6831 | 1.9434 | 0.87 | SQLite's home turf, benched to lose honestly; the nosync ladder narrows on every rung (insert_1k 8.59→7.33, insert_10 4.77→3.25, update 4.73→3.27, delete 2.97→1.79), durable upsert crosses to a win (1.10→0.97); durable insert_1k gives back a little (4.53→4.70) |
| lawful | 12 | 3.1890 | 3.1414 | 1.02 | judged-law admission vs constraint enforcement; law_commit_attempt durable 1.00, cluster 1.03; law_reject_key durable prices the full-judgment refusal vs SQLite's instant UNIQUE abort at 1282× (night 573×) — ours p50 4.22 ms vs the night's 4.40 ms (within noise), the whole ratio move is SQLite's abort dropping 7.67→3.29 µs this run; reported as measured |
| storage (bytes) | 2 | 3.547 | 4.212 | 0.84 | engine-compacted / sqlite-indexed bytes, deterministic on the R20 corpus; the graded chunk geometry (094) lands −17.5% ledger (77.96→64.27 MB), −14.1% calendar (75.33→64.68 MB) on identical fact counts |

Outstanding at this writing: writes, churn (battery-era reruns retired at f474202a, wall-power reruns owed), sweep-commit (obs build). adversarial SKIP-UNAVAILABLE, probe-parity with the night.

## Curves per-family (SQLite p50 / ours p50, S scale)

| family | campaign | night | mover |
|---|---:|---:|---|
| triangle | 13.9× | 3.7× | ours 9.85 → 2.75 ms (the R6 ray-verdict + fold-split work lands on the cyclic self-join) |
| point | 4.3× | 2.8× | ours 500 → 333 ns |
| busy_scan | 424× | 477× | ours 7.29 → 8.17 µs on the R20-regenerated stream (answers 415 → 410) |
| closure_fanout | 13.3× | 30× | ours flat (1083 → 1083 ns); the SQLite twin sped up 2.25× (32.5 → 14.5 µs) on the R20-regenerated fanout shape — same facts (17554) and answers (1316), different distribution. The night's 30× does not survive the regenerated corpus; reported as measured. (The battery-era RUN 1 read the same twin at 8.1 µs — the SQLite side of this family is the volatile one.) |

Warmth panel: cold/warm/memoized shape holds; triangle ours-cold 16.7 → 9.0 ms, busy_scan ours-cold 730.7 → 808.9 µs, closure_fanout ours-cold 378.6 → 315.4 µs. Two warmth blocks stamp contaminated (busy_scan post 3.179 GHz, closure_fanout 3.128/3.129 vs the 3.2 floor) — reported, not hidden; the four timed curve blocks are all clean.

## Oracle honesty

Zero mismatches campaign-wide. Every timed point in every gated lane re-earned value-identical multiset agreement before its timed window; crud and lawful post-state folds certify engine agreement after the op streams (post-run twin hashes byte-identical run over run). The two scenario DNFs are the night's same two, excluded and counted. Contamination discriminators ran everywhere: crud 7/22 and lawful 3/12 blocks stamped (night: 14/22, 8/12 — fsync-DVFS physics, improved on wall power); curves carries the 072 bracket the night lacked, 0/4 timed blocks contaminated.
