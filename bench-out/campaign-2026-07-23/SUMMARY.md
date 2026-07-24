# Campaign 2026-07-23 vs night 2026-07-20 — full-suite geomeans

Protocol identical to the night run (seed 1, registered lane protocols, oracle-gated everywhere the night gated, shared-machine boost, measurement mutex per lane). Corpora regenerated under the fixed RNG (R20): identity digest `6518394f080c2273…`, verify stamp and per-world store hashes pinned in each lane's `digests.txt`. Every geomean below is computed over the cells common to both runs, same pairing both sides; capped/DNF cells and twinless engine-only families are excluded-and-counted identically. ratio = ours/sqlite (lower is better); vs-night = campaign geomean / night geomean (<1 = we improved relative to SQLite).

| suite | cells | campaign geomean | night geomean | vs-night | note |
|---|---:|---:|---:|---:|---|
| bench-durable-r1 | 37 | 0.0770 | 0.0844 | 0.91 | reads + twinned write families; ALL-WIN holds, budget gates unarmed in both runs (4 p99-over-budget reads vs the night's 5 — triangle rejoins budget) |
| scenarios | 34 | 0.0563 | 0.0835 | 0.67 | the targeted-query fixes cash (o3 21×, o5 12×, r1 6×, t2 2.9×); r4/t2 SQLite DNF both runs, excluded-and-counted; detail in scenarios/delta.md |
| curves | 4 | 0.0484 | 0.0507 | 0.95 | clock-proxy bracketed per finding 072 this run (ghz on every point, 0 contaminated); per-family movement below |
| crud | 22 | 1.6715 | 1.9433 | 0.86 | SQLite's home turf, benched to lose honestly; the loss narrows on every insert ladder rung (insert_1k durable 4.53→3.91, nosync 8.59→6.28) |
| lawful | 12 | 3.0573 | 3.1414 | 0.97 | judged-law admission vs constraint enforcement; law_commit_attempt stays a win (0.84 durable), law_reject_key durable prices the full-judgment refusal vs SQLite's instant UNIQUE abort (843× vs the night's 573×, both p50s within noise of the night's absolutes) |
| writes | 18 | 1.1207 | 1.0965 | 1.02 | absolute p50s at 0.87–1.04× of the night on both ladders; the SQLite twin sped up slightly more, so the relative geomean gives back 2% |
| churn | 480 | 0.0365 | 0.0447 | 0.81 | per-sample re-gated clean, all three profiles; churn_point p50 down 2.2× (583→270 ns), ours-lane disk down 17–19% |
| storage (bytes) | 2 | 3.547 | 4.212 | 0.84 | engine-compacted / sqlite-indexed bytes; the graded chunk geometry (094) lands −17.5% ledger, −14.1% calendar on identical fact counts |

Pending at this writing: bench-durable-r2/r3, bench-ephemeral-r1–r3, sweep-commit (obs build). adversarial SKIP-UNAVAILABLE, probe-parity with the night.

## Curves per-family (SQLite p50 / ours p50, S scale)

| family | campaign | night | mover |
|---|---:|---:|---|
| triangle | 13.9× | 3.7× | ours 9.85 ms → 2.56 ms (the R6 ray-verdict + fold-split work lands on the cyclic self-join) |
| point | 4.7× | 2.8× | ours 500 → 292 ns |
| busy_scan | 416× | 477× | ours 7.29 → 7.92 µs on the R20-regenerated stream (answers 415 → 410); hand-tuned twin 149× (night 168×) |
| closure_fanout | 6.7× | 30× | ours flat (1083 → 1209 ns); the SQLite twin sped up 4× (32.5 → 8.1 µs) on the R20-regenerated fanout shape — same facts (17554) and answers (1316), different distribution. The night's 30× (and the README's) does not survive the regenerated corpus; reported as measured. |

Warmth panel: cold/warm/memoized shape holds; closure_fanout ours-cold 378.6 → 297.5 µs, busy_scan ours-cold 730.7 → 747.3 µs, triangle ours-cold 16.7 → 7.7 ms.

## Oracle honesty

Zero mismatches campaign-wide. Every timed point in every gated lane re-earned value-identical multiset agreement before its timed window; crud and lawful post-state folds certify engine agreement after the op streams; churn re-gates every sample. The two scenario DNFs are the night's same two, excluded and counted. The curves lane now carries the contamination discriminator it lacked on the night (finding 072): every block stamped 3.2–3.5 GHz, zero retries escalated, zero contaminated blocks.
