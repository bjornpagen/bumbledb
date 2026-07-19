# The Measure phase — the ephemeral re-earn (cleanup-0.5.0 prd-M Part 2)

U1 made the ephemeral kind `NOSYNC`-only over one lazy 32 GiB map
(ruling 1); every WRITEMAP-era ephemeral number was stale by
construction. This session re-earned them. Machine: idle Apple M2 Max
(foreign heavy processes waited out before each timed block —
duetexpertd and an XProtect scan were each observed and allowed to
finish first), all timed runs through `scripts/measure.sh`, release
builds, fresh verify stamp
`e597c9f7288b432630ea7c580dab308ef7b4059336ab1b5604aff07e0894c38b`
(2,862 cases green on the post-twin-merge tree — the stamp also
re-referees prd-M Part 1's finalize merge through both oracles).
Date: 2026-07-19. Tier of every number: S-scale corpus, mmap-warm reads
(DRAM-resident working set), real constructors for every write cell.

## The bench trio (read + write families)

Three `bumbledb-bench bench --ephemeral` runs, 256 samples/family:
`bench-out/eph-nosync-1..3` (each with its own report.json/md).
ALL-WIN held in all three. Per the standing protocol, clock-proxy
contaminated blocks are excluded and counted (run 1: 7 dirty blocks;
run 2: 6; run 3: 7 — listed in each run's report.md).

**Ephemeral read geomean, all 22 gated families, clean-sample min-of-3
(the same derivation as the committed 18.4× WRITEMAP-era number and the
README's durable 18.7×): 21.23× over SQLite p50 → README says 21.2×.**
Every family had ≥1 clean sample; none excluded.

Write families (ephemeral side, clean-sample min-of-3 p50):
`commit_single`, `commit_batch`, and `bulk` were clock-contaminated in
ALL THREE runs — excluded and counted, no number banked (the committed
WRITEMAP-era ephemeral runs were dirty on the same blocks; the
small-commit truth comes from the R6 lane below, whose quiet-machine
guard passed). Clean cells: commit_witnessed 56.2 µs (3 clean),
commit_window_baseline 29.0 µs (1), commit_window_admission 36.8 µs
(2), commit_window_exclusion 35.0 µs (2), cold_containment_walk
4,146.9 µs vs SQLite 104.0 µs (1 clean; the honest cold-loss family,
unchanged in character).

## The R6 lane (the device-tax and staging-win bands)

Three interleaved sessions of `ramdisk_phase_r_ephemeral` (4 cells
interleaved per repetition: create@ssd, ephemeral@ssd, create@hfs+
ramdisk, ephemeral@hfs+ ramdisk; 64 small commits of 16 facts + 8 bulk
of 4096; medians; macOS build 24G720; 2 GiB HFS+ ramdisk, the post-U1
sizing). The quiet-machine guard (bulk spread ≤ 2×) passed in ALL
sessions — every ratio below is decision-grade. Per-session values:

| ratio | s1 | s2 | s3 | band |
|---|---|---|---|---|
| staging win, create@ssd / ephemeral@ramdisk | 42.7x | 70.2x | 43.3x | **43–70x** |
| flags dividend @ ssd, create/ephemeral | 27.1x | 52.3x | 39.5x | **27–52x** |
| flags dividend @ ramdisk, create/ephemeral | 3.5x | 3.1x | 3.1x | **3.1–3.5x** |
| device tax on ephemeral, ssd/ramdisk | 1.6x | 1.3x | 1.1x | **1.1–1.6x** |

Raw cell medians per session are in `sessions.log` beside this report.

## The verdict sentence (prd-M item 6)

`NOSYNC`-only shows a MATERIAL win over durable everywhere the kind's
rationale claims one: 27–52x on the same SSD through the real
constructor, 43–70x for the two-store staging pattern. The kind's
recorded rationale survives its own re-argument; the WRITEMAP-era band
(~75–90x / ~4.2–4.4x / 1.0–1.1x) narrowed but nothing reverses. No
finding for the owner beyond the numbers themselves.

## Still owed (not this session)

The `NOSYNC`-only ≥2,000-round statistical kill session (the recorded
kill-lane sessions are 2026-07-16, WRITEMAP-era; the deterministic
crashpoint sweep and the kill smoke re-ran green at the flip). Tracked
in TODO.md.
