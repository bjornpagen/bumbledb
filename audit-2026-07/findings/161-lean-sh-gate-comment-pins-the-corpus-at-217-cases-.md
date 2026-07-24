## lean.sh gate comment pins the corpus at 217 cases; 272 are on disk

category: bench-honesty | severity: low | verdict: CONFIRMED | finder: r2:lean-unswept-modules

### Summary
The Battery 4 note in the lean.sh gate script records a per-push cost measurement against a "217-case corpus" (dated 2026-07-14), but the checked-in conformance corpus has since grown to 272 case files. Nothing functional depends on the number — the Lean driver enumerates the directory, so every case on disk is replayed — but the recorded evidence line no longer describes the corpus the battery actually runs, and a reader trusting it under-counts coverage by ~20%.

### Evidence
- `scripts/lean.sh:56-61` — "Battery 4 (PRD 13): the conformance corpus run … Measured 2026-07-14 on the pinned M2 Max: ~1.0 s for the 217-case corpus — comfortably per-push."
- `ls lean/conformance/cases | wc -l` → **272** (200 `seeded-*`, 24 `program-seeded-*`, 3 `program-hand-*`, 26 `judgment-*`, 19 `hand-*`).
- `lean/Main.lean:432-436` — the driver defaults to `conformance/cases`, calls `System.FilePath.readDir`, and filters `·.extension == some "json"`; there is no roster file, so no case can go silent-green.
- `lean/Main.lean:487` — the driver's summary line (`conformance: {files.size} cases …`) already reports the true count at runtime.
- Grep for `217` across `scripts/` and `lean/` (excluding case files): the stale figure appears only at `scripts/lean.sh:60`, so a one-line fix closes it.

### Failure scenario
Not a correctness failure. A maintainer reading lean.sh to assess the per-push gate's cost and coverage reasons from a corpus 55 cases smaller than what CI replays; the ~1.0 s timing figure also predates the 24 program-seeded cases (the RECURSIVE arm) and the judgment/hand additions, so the pinned cost estimate is unverified against the current corpus.

### Suggested fix
Either re-measure on the pinned M2 Max and restate the comment with the current count and date, or (better, per the repo's representation-first doctrine of not duplicating facts the system already emits) drop the hard-coded count from the comment entirely and let the driver's own `conformance: N cases …` summary line at lean/Main.lean:487 be the single source of the number — a comment like "timing measured on the pinned M2 Max; the driver prints the live case count" cannot go stale again.
