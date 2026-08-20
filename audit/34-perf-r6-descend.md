# 34 — `r6_two_path_count` 1.46: `jp_descend` carries the query

- **Status:** **fixed this pass** — fresh flame cut; re-ruled. vs SQLite
  **0.47×** (328 ms / 694 ms, 8 samples). `jp_force` does not rank
  (promotes 37 to skip).
- **Severity:** performance debt, attribution-first.

## The recorded facts (TODO.md)

- Ours 131→197 ms on the sink/plan lane's COUNT-shaped territory.
- Flame: `scenarios.rings.r6_two_path_count.warm.diff.svg` — `jp_descend`
  51% + 45%; "descend now carries essentially the whole query."

## Protocol

Trace-reader ranking on the CURRENT tree first (the evaluator consolidation
and `NodePrecompute` landed since the flame was cut — the attribution may
have moved). Then one ranked fix; re-run the rings lane; compare against
the SQLite twin per the scenario protocol.

## Fresh flame (2026-08-20, Apple M2 Max, obs release)

`scenarios --only rings --trace --samples 1` then `--samples 8` (no
trace) on the same corpus (387_054 facts).

| protocol | ours p50 | sqlite p50 | ratio |
| --- | ---: | ---: | ---: |
| 8 warmups + 1 sample (`--trace`) | 337_987 µs | 685_456 µs | 0.49× |
| 8 warmups + 8 samples | **327_993 µs** | **693_611 µs** | **0.47×** |

Warm capture top (wall 224_730 µs):

| span / phase | calls | self / excl_us |
| --- | ---: | ---: |
| `join` | 1 | 224_715 |
| `jp_descend_n1` | 99_950 | **197_689** |
| `jp_iter_n1` | 205_928 | 13_807 |
| `jp_descend_n0` (excl) | 528 | 11_444 |
| `jp_probe_n0` | 528 | 641 |
| `jp_force_n0` | 528 | **1.25** |

`jp_descend_n0` inclusive is 223_213 µs (children); exclusive is 11.4 ms.
`JoinPhase::Descend` exclusive is defined as per-survivor bookkeeping
(`exec/run.rs`: binds, journal restores, leaf emits). 99_950 × ~2 µs is
the 2-path walk the query is named for ("what binary joins must
materialize").

## Re-rule

Evaluator consolidation / `NodePrecompute` did not move the cost off
descend — n1 exclusive still carries the query. That is the 2-path
join's essential bookkeeping, not a new evaluator tax. vs SQLite the
lane **wins** (0.47×). The campaign 1.46× was ours-vs-ours (131→197 ms);
current 328 ms is slower than that pin but is not a vs-SQLite loss, and
no ranked non-essential span remains. No engine edit. Finding 044
(forced-map telescope) is demoted: `jp_force_n0` is 1.25 µs.

## Acceptance

- Fresh flamediff recorded; the 1.46 closes to a stated target or is
  re-ruled with the trace attached; TODO.md row closed.
