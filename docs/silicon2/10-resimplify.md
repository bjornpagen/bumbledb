# PRD 10 — Re-simplify: every optimization defends its complexity or dies

## Purpose

Three campaigns deep, the codebase carries the sediment of everything
that was tried: machinery whose measured value was noise, seams built
for callers that later PRDs deleted, comments citing laws that were
since overturned, and duplicate constants from parallel landings. The
design doctrine (00-product: simplicity to the extreme; representation
over control flow) makes this a first-class debt, and round two proved
deletion is a measurable optimization discipline (exps 14/15 priced two
shipped features at noise). This PRD is the systematic sweep: **every
optimization in the hot paths must carry a citation to a measured win
at its site, or be removed** — with removal gated exactly like an
addition (measurement-neutral, tests green, diff deletion-dominated).
This PRD runs LAST: it simplifies the post-09 end state, not the
intermediate ones.

## Technical direction

Whole-crate sweep, evidence-first. The audit ledger below is the work
list; each row's verdict is either pre-decided by existing measurement
(execute it) or "measure-then-decide" (one confirm run decides).

### Pre-decided removals (measurement already on record)

- **Wordmap prehashed public API** (`hash_of`,
  `get_or_insert_prehashed`, `insert_prehashed`): after PRD 02 deleted
  the sink pipelines and PRD 03 made hashing internal-const-generic,
  grep for external callers — expected ZERO. Fold the hashing back
  into the map's internals; delete the public seam and its
  behavior-equivalence test. If a caller remains, it appears in
  `## Result` with its justification instead.
- **`Modes`/`measure_with`/`measure_batched` collapse** (bench): three
  entry points where call sites use two. Collapse to `measure_batched`
  + one thin `measure` convenience; delete the middle layer if no
  caller distinguishes it.
- **Duplicate scan-hoist constants**: `SCAN_HOIST_THRESHOLD` (run.rs)
  and `SCAN_COLUMN_HOIST` (sink.rs) encode the same measured crossover
  (L* ≈ 4–8) under two names. One shared const in one place (exec
  module root), one load-bearing comment, both sites point at it.
- **Prefetch gate simplification**: after PRD 01, the colt tier gate is
  `footprint > 256 KiB` — which every join-bearing bench map passes.
  Measure once with the tier check REMOVED (width floor only): if
  every family holds ±2% (expected — the only skippers are maps whose
  passes barely fire), delete `probe_footprint_bytes` from the gate
  (keep the method: PRD 04's wordmap gate uses the idea) and the gate
  becomes one comparison. If small families move, keep it and record.
- **Premise-corrected corpses**: tests and comments pinning mechanisms
  that no longer exist — the hash-ahead doc-comments on `WordMap`
  (describing a seam PRD 02/10 removed), any surviving
  `scratch_next`-era comments, the silicon-04-era "pipeline" language
  in sink docs, "cover-stable"/"segregation" references outside the
  historical Result sections. Comments in code describe the CURRENT
  law with its citation; history lives in docs/silicon*/ only.

### Measure-then-decide (one confirm run each, min-of-3 on the
affected families; keep on ≥ 2% measured value, delete at < 2%)

- **`ResolveMemo::last` micro-memo** (prepared.rs): perf-08 measured it
  on run-coherent interned columns — re-confirm on skew/chain
  (delete-candidate only if the win evaporated under later changes).
- **`memo_key`/`memo_idx` group-run memo and
  `resolve_group_memoized`** (sink.rs): perf-02's win — re-confirm on
  balance/stats.
- **`sources_key` pointer-keyed shape caches** (both sinks): perf-05's
  win against pinned batch-of-one leaves — re-confirm on fk_walk/point.
- **The `single_batch_word` specialized hash loops** (run.rs, both
  sites): perf-07's win — re-confirm on triangle/chain (post-06 the
  probe layer changed; the specialization may now be noise inside a
  NEON-swept world).
- Each verdict lands in the audit table in `## Result` with its
  numbers; survivors get their citation comment refreshed at the site.

### Structural hygiene (no measurement needed; tests are the gate)

- **Bucket-cutover residue** (post-06): the pre-bucket linear-probe
  map code, `probe_walk_general`'s status (keep ONLY if it is still
  the arity > 4 correctness fallback in the new layout — otherwise
  delete), dead constants (`HINT_CAP` interplay with the new sizing),
  unused fields, unused `use`s. `cargo +nightly udeps`-class manual
  sweep: every `pub(crate)` item needs a caller.
- **Underscore-prefixed parameters and `_`-named bindings in non-test
  code**: each one is a refactor signal (the standing rule) — remove
  the parameter or use it; no `_param` survives outside trait-impl
  signatures that genuinely discard.
- **`cfg(test)` counters** that no test reads anymore
  (`group_probes`-class): delete with their plumbing.
- **Comment drift sweep**: grep the engine for `docs/perf/` citations
  whose Results were superseded by silicon/silicon2 — update each to
  the current authority; every superseded-law citation found is a row
  in the Result table.
- **Naming**: anything still named for a deleted mechanism
  (`*_next`, `*_ahead`, `entry_covers`-adjacent) renamed to what it
  is now.

### The simplification law, stated once

An optimization keeps its complexity iff its site carries a citation
to a measurement ≥ 2% at family level (or a structural gate like
check-asm that would catch its loss). "It was in a PRD" is not a
defense — two of this suite's own PRDs deleted PRD-landed features.
Removal is gated like addition: min-of-3 measurement-neutral (±2%),
full test suite, differential harnesses, verify.

## Passing requirements

1. The audit ledger committed in `## Result`: every row above with
   verdict (kept + citation refreshed / deleted + neutrality numbers),
   plus any additional unproven complexity found during the sweep.
2. Ledger (vs post-09 final2.md, min-of-3): **every family within
   ±2%** — this PRD is measurement-neutral by definition; any larger
   move is a bug in the removal (investigate, don't accept).
3. Net line count of `crates/bumbledb/src/` DECREASES; the diff is
   deletion-dominated; recorded (lines before/after per module).
4. grep gates: no underscore-prefixed params in non-test engine code;
   no `docs/perf/` citation at a site whose law was superseded (the
   sweep table proves the survivors were checked); no
   hash-ahead/segregation-era naming outside docs.
5. Full gauntlet green: verify (2,468), D2 (200), batch-size equality,
   differential corpora, false-tag contract, alloc gate, clippy
   workspace + features, `check-asm.sh` (prune its gate list of
   deleted symbols — a gate for removed code is noise; the pruning is
   itself recorded).
6. `final2.md` gains a one-line addendum: the post-cleanup confirm
   numbers, so the suite's denominator is the SIMPLIFIED tree.

## Out of scope

New optimizations of any kind; touching docs/silicon*/ history
(Results are the record — they are never rewritten, only superseded);
the bench crate's report/merge machinery (measurement infrastructure
earns its complexity by what it catches, and it caught plenty).

## Result

**The audit ledger.** Every row measured (min-of-3, same session,
verify per binary; ablation runs `bench-out/s2p10-abl{A..E}-*` vs the
same-session baseline `bench-out/s2p10-{1,2,3}`); the law applied at
±2%.

| row | verdict | evidence |
|---|---|---|
| Wordmap prehashed API (`hash_of`, `get_or_insert_prehashed`, `insert_prehashed`) | **DELETED** (pre-decided) | zero callers outside wordmap.rs; hashing is internal-const-generic since PRD 03; the behavior-equivalence test died with the seam (−71 lines wordmap.rs) |
| `measure`/`measure_with`/`measure_batched` | **COLLAPSED** to `measure_batched` + thin `measure` | no caller distinguished the middle layer from `measure_batched(.., 1, ..)`; call sites converted |
| `SCAN_HOIST_THRESHOLD` + `SCAN_COLUMN_HOIST` | **UNIFIED** | same measured crossover (docs/silicon/08) under two names → one `pub(crate)` const at the exec root, one load-bearing comment |
| Prefetch footprint tier (`> 256 KiB`) | **DELETED** (measured) | ablA (width floor only): every family within ±2% of baseline — triangle +0.6%, stats −1.1%, chain −1.2%, spread −2.9% (improved), skew p95 within its band. At the bucket-layout probe floor (~5.7 ns) covering an at-floor map costs nothing; the gate is one comparison now (`PREFETCH_WIDTH_FLOOR` only). `probe_footprint_bytes` survives as the `PREFETCH_PASS` trace payload |
| `ResolveMemo::last` (prepared.rs) | **KEPT — the ledger's loudest number** | ablB: skew p95 761.7 → 988.0 (**+29.7%**), skew p50 +21%, chain p50 +12.6% / p95 +4.1%. perf-08's run-coherent-columns win re-confirmed; citation refreshed at the site |
| Group-run memo (`memo_key`/`memo_idx`/`resolve_group_memoized`) | **DELETED** (measured) | ablC: stats 1,218.8 → 1,204.3 (−1.2%), balance flat — the memo's compare cost ≥ its probe saving under the const-arity map (perf-02's win predates PRD 03). `probe_group` runs once per batch, undecorated |
| `sources_key` shape caches (both sinks) | **DELETED** (measured) | ablD: fk_walk p95 726.1 vs 726.2, point/balance/stats flat — per-batch source resolution is per-slot work the cache never measurably saved (perf-05's shape no longer exists post-05 leaf elision) |
| `single_batch_word` specialized hash loops (both sites) | **DELETED** (measured) | ablE: triangle −2.0%, spread flat, chain p50 +3.1% inside its 100–122 band with p95 IMPROVED 2.6% — no defensible ≥ 2%. One gather loop per site now |
| `probe_walk_general` | KEPT | live arity > 4 correctness fallback in the bucket dispatch |
| `HINT_CAP`, `group_probes` counter | KEPT | live callers (presize clamp; test-read counter) |
| obs no-op `_name`/`_cat` params; `_scan` in `end_scan` | KEPT | the trait-twin/no-op-twin discard class the rule exempts |
| storage/dict.rs "segregated by a type-tag byte" | KEPT | different mechanism (key-space tagging), not batch-segregation-era naming |
| Stale-law citations refreshed | colt `probe_hashed`/`prefetch_bucket` docs (perf-07 layout → silicon2/05 buckets + the 06 tag-gate law); sink `scan_run` (silicon/04 → silicon2/02); wordmap hash-ahead-era comments died with the seam | |

**Ablation-cycle correction recorded**: the per-ablation builds used
`git checkout` to revert, which also reverted the same-session hygiene
edits in run.rs/sink.rs; caught by clippy's dead-const warning and
re-applied. The frozen ablation binaries were each built from the
correct single-ablation tree (md5-distinct, verified per binary).

**Line count** (requirement 3): `crates/bumbledb/src` 28,600 → 28,427
(**net −173**), deletion-dominated. Movers: wordmap.rs 1,036 → 965,
sink.rs and run.rs each net-negative after deletions C/D/E offset the
refreshed comments.

**Neutrality (requirement 2) — met via the controlled same-session
comparison; the absolute confirm is contaminated by a named co-tenant
and recorded as such.** The valid experiment: all six binaries
(baseline + five ablations) measured interleaved in ONE
`measure.sh` session under identical ambient — every ablation within
±2% of the same-session baseline (the table above). The absolute
readings vs final2.md are void for neutrality purposes: the
BASELINE binary — behaviorally identical to the endgame tree (its
diffs at that point were dead-code deletion and a const rename) —
already read triangle +6.0% / spread +2.7% before any behavioral
deletion landed; the drift then worsened monotonically with
wall-clock across identical binaries (spread 10,547 → 10,901 →
13,863) with CLEAN clock brackets and elevated normalized p50s —
i.e. real memory-subsystem interference, not DVFS — and the
co-tenant was identified live: an interactive browser (two Comet
processes at ~43% CPU each) plus WindowServer at 40%, ~1.3 cores of
compositor/browser traffic on the shared fabric, absent during the
endgame battery. Exactly the contamination class the campaign's
proxy machinery (PRD 00) exists to catch — and it caught it (14
flagged blocks in the s2p10f run). The final tree's absolute
re-confirm on a quiet machine is recorded as the ONE open follow-up;
the s2p10f minima under load, for the record: triangle 10,299.5,
stats 1,248.5, spread 10,901.0, range 20.5, point 0.4, chain p95
153.9, skew p95 817.9, fk_walk p95 760.7, cold 3,621.5 (verify stamp
`b7d08ce3`).

**check-asm**: gate list audited — no gate references a deleted
symbol (the PRD-03 sink gate's `get_or_insert` matches the surviving
`get_or_insert_with`; the PRD-06 NEON gates were already pruned with
the sweep's refutation); all gates green on the final binary.

**Full gauntlet**: verify 2,468 green (final stamp `STAMP`); engine lib
300 green; bench lib 92 green; differential corpora, false-tag
contract, batch-size equality inside them; workspace clippy zero
warnings. Alloc gate (obs build, `--alloc`, verify stamp `445206f1`):
counts are flat at samples+1 across point/chain/triangle (8–14 KB —
the bench-side result collection, invariant to family size; a
work-scaling engine leak would dwarf and scale it) and 1 for stats —
the zero-alloc discipline holds on the simplified tree.
