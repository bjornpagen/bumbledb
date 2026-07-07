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
