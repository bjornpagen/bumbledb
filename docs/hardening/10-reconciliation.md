# PRD 10 — Reconciliation: docs true, ledger closed

Findings fixed (docs/audit/, every remaining doc/comment/micro item):
**executor MEDIUM ×2** (the stale label-first cover rule at
30-execution.md:74; the NEON-compaction claim at :176); **plan NOTEs** (the
stale "plans on the base row count" statistics sentence; the DP
O(2ⁿ·n²)/table-size notes); **image NOTEs** (the peek/get_or_build doc
fusion; the `Const::PendingIntern` Eq-only miss wording; the cfg(test)-only
dual-output builder note); **ir NOTEs** (the 2-variant fixture comment; the
resolve_filter unreachable-Eq-arms comment; the duplicate/contradictory
predicate acceptance note); **api-schema NOTEs** (bulk_load committed
semantics; compact-concurrent-with-writer; constraint-id materialized order;
`Fact::encode_read` surface decision; the buffer-on-error contract from
sink-pipeline); **storage NOTEs** (R-delete asymmetry; generation/row-id
overflow asymmetry; the cache-eviction leak note's comment); **concurrency
NOTE** (the alloc-gate one-test fragility) — plus the audit resolution
ledger itself.

## Purpose

Everything the audit found that is words, not machinery — plus three tiny
code items too small for their own PRD — landed in one sweep, ending with
the audit ledger closed: every finding in docs/audit/ marked with its
resolution and the PRD that carried it. After this PRD the audit is a
historical record with zero open items.

## Technical direction

Code (small, real, and in scope here because each is a two-line fix the
audit priced):

- **Planner DP micro-fixes** (plan NOTE): carry `prefix_vars` in the DP
  `State` (or a per-mask memo) killing the O(2ⁿ·n²) refold; fix the module
  comment's table-size figure (32 MB at the cap, or shrink `State` to earn
  the documented 24 MB — either, but code and comment must agree).
- **Alloc-gate one-test guard** (concurrency NOTE): enforce the
  single-test-binary invariant — simplest honest form: a `const _: () = {}`
  count-of-tests assertion is not expressible; instead have `check.sh`
  invoke the gate with `--test-threads=1` (belt) and add the loud comment
  at the top of alloc_gate.rs naming the invariant (suspenders).
- **Cache-mutex comment** (concurrency NOTE): the one-sentence comment on
  the cache lock's panic-free critical section, so it survives growth.

Docs and comments, by file (each item cites its audit line — work the list,
check them off in the commit message):

- `30-execution.md`: §4.4 cover bullet → magnitude-first (kill line 74's
  label-first rule); D4's line 176 → "fixed-width predicate scans" only;
  the Statistics paragraph's "plans on the base row count" → the ladder
  reality; add the buffer-on-error sentence's cross-reference if it lands
  in 60-api (below).
- `60-api.md`: prepared-query-belongs-to-its-Db rule (PRD 00 landed the
  mechanism; the doc sentence lands here if not already); "ignore `out` on
  `Err`" result-buffer contract; `bulk_load` `committed` = changed-not-
  consumed semantics sentence; `compact` is safe concurrent with a writer
  (consistent snapshot via LMDB's copy txn); `Db::write` is non-reentrant
  (panics with a named message, per PRD 00); `Fact::encode_read`'s reader
  is host code — a stated surface decision.
- `10-data-model.md`: "constraint ids by declaration order" → materialized
  order (auto-uniques first), with the note that materialization is a
  deterministic function of declaration so the fingerprint claim survives;
  the serial-escape rule as amended by PRD 01.
- `40-storage.md`: the R-delete verification asymmetry recorded next to the
  M/F/U desync rule (deliberate, offline-sweeper-deferred); the
  generation/row-id overflow non-guard recorded as a scale-axiom decision
  (the audit: unreachable by ~12 orders; write that down so the asymmetry
  with the guarded serial ceiling reads as chosen).
- `image/cache.rs`: split the fused doc comment — `peek` gets its
  never-builds contract, `get_or_build` gets its own block (image NOTE's
  exact fix).
- `image/view.rs`: `Const::PendingIntern` doc → "an Eq miss empties the
  query; any other operator resolves to the sentinel id"; a one-line note
  on the cfg(test) dual-output builder recording that the production cold
  path builds unfiltered-then-filter (the 40-storage sentence it echoes).
- `api/prepared.rs`: `resolve_filter` doc gains the
  "Eq arms are unreachable post-split; belt-and-braces" sentence.
- `ir/validate.rs:709`: "3 variants" → "2 variants"; a short module-doc
  note that duplicate/contradictory predicates are accepted deliberately
  (semantics exact, "write the query you mean" not extended to statically
  false conjunctions — the audit's trace recorded).
- `plan/fj.rs`: slot-order comment fix (if PRD 03 did not already land it).
- **The resolution ledger:** `docs/audit/README.md` gains a Resolution
  column (or per-finding annotations in each report — choose the README
  table; do not rewrite the reports, they are evidence): every finding maps
  to `fixed (PRD NN)` / `documented (PRD 10)` / `closed by audit (no
  action, reason)`. The three explicitly-no-action audit closures (chunk
  token wrap, pre-probe growth, generation overflow) are marked as such.

## Non-goals

Deleting docs/audit (it is the evidence record — the perf-suite retirement
precedent applies only once findings age into history; the owner decides
when); the offline consistency sweeper; any behavioral change not listed
above.

## Passing criteria

- Every doc item above landed; a reviewer can diff 30-execution/40-storage/
  10-data-model/60-api against the audit citations and find each
  contradiction gone (list them checked off in the commit message).
- The DP refold fix: planner tests green; a unit assertion that
  20-occurrence planning completes without the quadratic refold is not
  wall-clock-expressible — instead pin the structural change (State carries
  the var set / the memo exists) via the planner test module compiling
  against it; comment and code agree on the table size.
- `check.sh` runs the alloc gate with `--test-threads=1`.
- The resolution ledger: zero findings without a resolution entry; grep
  `docs/audit/README.md` for the three closure classes finds all 65
  findings accounted for.
- Full `scripts/check.sh` green end to end. The suite is complete; the
  re-run (`scripts/bench.sh`) and everything after it belongs to the human
  owner.
