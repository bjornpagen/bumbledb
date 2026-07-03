# PRD 03 — Read-path instrumentation

Authority: PRD 02's seam; `30-execution.md` (the phases being named).

## Purpose

Every phase of prepare and execute becomes a span; every cache and memo decision
becomes an event — so a trace of one query answers "where did the time go" without a
profiler.

## Technical direction

All instrumentation via `obs::span`/`obs::event` with names from `obs::names` —
zero `#[cfg]` at call sites (PRD 02 guarantees feature-off no-ops).

- `api/prepared.rs::prepare`: outer span `prepare` (Category::Prepare); child spans
  `validate`, `normalize`, `classify`, `stats` (with `a0` = occurrences measured
  concretely), `plan_dp`, `lower` (binary2fj+factor+plan-validate), `build_colts`.
- `PreparedQuery::execute`: outer span `execute` (Category::Execute, `a0` = result
  rows on completion via `set_args`); child spans `bind_params`, `resolve_filters`,
  `views` (the run_join view loop), `join` (the executor call), `finalize`.
- Inside the view loop (run_join), per occurrence: event `view_memo_hit` (`a0` =
  occurrence index) on the memo fast path; else span `view_build` (`a0` = occ index,
  `a1` = survivor count via set_args) wrapping cache get + apply + colt reset.
- `image/cache.rs::get_or_build`: event `cache_hit` (`a0` = relation id) on the
  first-lock hit; span `image_build` (Category::Image, `a0` = relation id, `a1` =
  row count) around the build; event `cache_adopt` when losing the insert race;
  event `cache_query_local` for the old-generation no-insert path.
- `exec/colt.rs::force`: event `colt_force` (Category::Execute, `a0` = position
  count, `a1` = distinct keys after) — an event, not a span: forcing is frequent and
  cheap; the *count and shape* are the signal. Emitted at force completion.
- Guard path: span `guard_probe` (a0 = 1 hit / 0 miss) inside `execute_guard`.
- Every name lands in `obs::names` with a doc comment naming its args' meanings.

## Non-goals

Per-tuple or per-batch events (cardinality data already flows through the Counters
seam and PRD 05's stats; traces are phase-grained by design).

## Passing criteria

- Unit tests (feature `trace`, small in-crate fixture): capturing one `prepare` +
  two `execute`s of a join asserts — exact expected span-name multiset for prepare;
  first execute contains `view_build` per occurrence and NO `view_memo_hit`; second
  execute contains only `view_memo_hit` events and no `image_build`; `execute`'s
  `a0` equals the result row count; span containment (all children within their
  parents' windows). A guard-shaped query capture contains `guard_probe` and no
  `join`.
- Zero events recorded when not capturing (asserted in the same tests).
- Default-feature build byte-identical behavior: the release allocation gate green.
