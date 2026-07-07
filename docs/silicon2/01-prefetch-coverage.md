# PRD 01 — Full phase-1.5 prefetch coverage: buy back the displacement tax

## Purpose

Exp 19 dissolved the campaign's "irreducible" probe residual: the
engine's ~37 ns/probe over the 17 ns shaped floor is NOT component
instruction weight (all six attributed components sum to +7.3 ns) — it
is inter-phase cache displacement multiplied by issue-queue stranding.
Between two probe passes over the same node's map, the executor's other
phases (n0 probes, leaf scans, seen-set inserts, pending traffic) walk
enough memory that the map's ctrl and bucket lines are gone again —
reuse distance dwarfs the L2, so "L2-resident" is false in situ even
though it is true in isolation. The engine already owns the cure: its
own phase-1.5 prefetch pass, measured by exp 19 to buy the entire
residual back (34.7–40.9 → 11.4–12.1 ns/probe at every pressure tier)
— but the pass is gated `survivors ≥ 16 && footprint > 2 MiB`, so most
probes go uncovered. Exp 19's predicted engine translation:
`jp_probe_n1` 3,667 → **~1,300–1,600 µs** — the twice-missed inherited
gate, reachable by widening a gate condition.

## Technical direction

`crates/bumbledb/src/exec/run.rs` (both phase-1.5 sites: `run_node`'s
sibling pass and `probe_pass`), `crates/bumbledb/src/exec/colt.rs`
(`prefetch_bucket`), docs.

- **Drop the width floor 16 → 4.** Exp 19 measured the prefetch pass
  itself at 11.6–12.1 ns per pass with per-probe cost ~0.3 ns — a
  4-survivor pass amortizes it. Below 4, skip (a 1–3-probe pass is
  pure overhead). Named const `PREFETCH_WIDTH_FLOOR: usize = 4` with
  the exp-19 citation.
- **Lower the residency budget 2 MiB → 256 KiB.** The silicon-10 tier
  gate was built on the isolation law ("resident ⇒ prefetch is pure
  loss, +7–12%"); exp 19 shows the precondition fails in situ — a map
  whose lines are displaced between passes benefits from prefetch
  regardless of its nominal footprint, and the measured unpressured
  cost of a useless prefetch is only +0.2–2.6 ns/probe on covered
  passes. 256 KiB keeps truly tiny maps (guard-scale, always L1-hot
  even in situ) exempt. Named const, comment rewritten to the
  interleaving law. Do NOT delete `probe_footprint_bytes` — the gate
  stays, re-tuned.
- **Cover the pinned-sibling case?** No — `prefetch_bucket` is a no-op
  for `Cursor::Row` and unforced nodes; leave that shape.
- **Verify coverage end to end.** The `PREFETCH_PASS` trace event
  already records (survivors, footprint) per fired pass. After the
  change, a traced triangle run must show pass counts ≈ the probe-pass
  counts on n0 AND n1 (every pass with ≥ 4 survivors fires) — this is
  the coverage evidence exp 19 said the engine lacked (782 of 2,555
  passes covered before ≈ 30%).
- **Doc**: update `docs/silicon/10-prefetch-tiering.md`'s Result with a
  superseded-by note pointing here, and rewrite the run.rs constant
  comments: the law is now "prefetch pays whenever another phase runs
  between passes over the same structure — which in a pipelined
  multi-node executor is ALWAYS — because residency is a property of
  phase interleaving, not structure footprint (exp 19)".

## Passing requirements

1. Traced coverage evidence: `PREFETCH_PASS` count ≥ 90% of
   (probe-pass count with ≥ 4 survivors) on triangle n0 and n1,
   recorded in `## Result` with the counts.
2. Measured (vs final.md, min-of-3): **`jp_probe_n1` ≤ 1,500 µs** (the
   inherited hard gate; exp 19 predicts 1,300–1,600; documented-miss
   protocol applies with a high bar — if it misses, the Result must
   show the traced per-pass coverage AND a pressure-tier argument for
   the residual); `jp_probe_n0` ≤ 900 µs (from 1,168 — n0's passes
   were also uncovered); triangle p50 ≤ 9,800 µs (from 11,742).
3. No family regresses > 5% (confirm-run protocol) — in particular
   range/balance/point (their maps sit under the 256 KiB floor and
   must be untouched: traced zero `PREFETCH_PASS` events on them).
4. Verify green (2,468); emits digests unchanged; zero-alloc holds;
   `check-asm.sh` green.

## Out of scope

The bucket-of-8 layout (05/06 — stacks on top of this); wordmap
prefetch (04); leaf-scan prefetch (no evidence yet — record as a
candidate if the traced residual after this PRD points at the leaf).
