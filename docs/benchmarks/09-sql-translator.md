# PRD 09 — The IR→SQL translator

Authority: `50-validation.md` (the translator is named infrastructure; comparison
via typed API never CLI; the aggregate template; hand-written SQL goldens arbitrate
3-way).

## Purpose

Total, mechanical `Query` → SQLite SQL, faithful to set semantics — the oracle's
other half. Where the translator and the engine disagree, the hand-written goldens
decide who is wrong.

## Technical direction

- `sqlmap::translate(query: &Query, schema: &Schema) -> Result<Translated, String>`
  where `Translated { sql: String, params: Vec<ParamId> }` (`params[i]` = the
  ParamId bound to SQL positional `?i+1`).
- Atoms: alias `t{occ}` per atom occurrence (self-joins natural). Build variable →
  `(occ, column)` first-binding map. Predicates:
  - var repeated across atoms → `t_a.col = t_b.col` join predicates (first binding
    is canonical; each later binding equates to the first).
  - var repeated within one atom → `t.colA = t.colB`.
  - literal/param bindings → `t.col = <lit|?n>`.
  - comparisons: var side resolved to its first binding; ops map 1:1; same-atom and
    cross-atom identically (SQL does not care).
  - zero-binding atom (gate) → `EXISTS (SELECT 1 FROM {table})` in WHERE.
- Projection: `SELECT DISTINCT {find columns}`. Literals: integers as decimal;
  strings as SQL-escaped `'…'` ('' doubling); bytes as `X'hex'`; bools 0/1; enums
  ordinal; i64 as decimal; u64 asserted `< 2^63`, decimal.
- Aggregates (the doc's normative template): project the **distinct full binding
  set** first, then fold:
  `SELECT {group}, SUM(x) FROM (SELECT DISTINCT {all bound vars as columns} FROM …)
  GROUP BY {group}`. Global aggregates (empty group): append `HAVING COUNT(*) > 0`
  so SQL's one-NULL-row-over-empty collapses to the engine's empty set. Count →
  `COUNT(*)`; Min/Max direct. Sum result width: SQLite INTEGER sum can overflow at
  i64 — mirror the engine's finalization check by using `CAST(TOTAL(x) AS …)`? No:
  keep `SUM(x)` and document that the verify corpus keeps per-group sums within
  i64 (generator amounts bound × group sizes — assert the bound in a test);
  overflow-behavior comparison is out of the oracle's scope (the engine's own
  overflow unit tests cover it).
- Never-interned strings/bytes need no special case: SQL compares values, which is
  exactly the sentinel semantics (`Eq` → no row matches, `Ne` → all match).
- **Hand-written goldens**: `sqlmap::goldens` module holds a literal SQL string per
  read family (PRD 14 names them; this PRD creates the module with the point,
  fk_walk and balance goldens as the first three, written by hand, not generated).
  A test asserts `translate(family_ir) == golden` byte-for-byte — when they
  diverge, a human reads both and rules.

## Non-goals

SQL for write paths (INSERT lives in PRD 08/16). Query optimization hints — the
translator emits the plain form; SQLite's planner is its own business.

## Passing criteria

- Unit tests: one per IR construct — multi-atom join predicates, self-join
  aliasing, repeated in-atom var, gate atom EXISTS, every comparison op, literal
  escaping (incl. a memo with a quote), param ordering with several params,
  grouped and global aggregates (the HAVING rule pinned), Ne with an
  out-of-vocabulary literal.
- The three hand-written goldens match `translate` output exactly.
- Every translation error path returns `Err(reason)` — no panics on any valid
  `Query` over the ledger schema (fuzz-lite test: 500 PRD 11 generator queries —
  forward reference; until PRD 11 lands, a hand-built list of 20 queries covering
  the grammar).
- `scripts/check.sh` green.
