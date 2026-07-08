# PRD 22 — IR→SQL translator extensions

**Depends on:** 11; 24 (schema notation) may land after — the translator works from IR + schema descriptors.
**Modules:** `crates/bumbledb-bench/src/translate/`, `crates/bumbledb-bench/src/sqlmap.rs`, `crates/bumbledb-bench/src/sqlite_run/`.
**Authority:** `docs/architecture/60-validation.md` (§ value mapping, § translation rules — normative).

## Goal

The SQLite lane expresses everything it can: intervals as two INTEGER columns,
negation, IN, CountDistinct, Arg-restriction — with the inexpressible set (the
judgments) explicitly enumerated, never silently skipped.

## Technical direction

1. **Value/DDL mapping** (`sqlmap.rs`): an `Interval(E)` field maps to two columns
   `<name>_start`, `<name>_end` (INTEGER); generated DDL and insert paths split the
   halves (decode through the typed API — the raw i64 values, not the sign-flipped
   words). The comparison decode path reassembles `Value::IntervalX` from the pair.
2. **Predicate translation:** membership binding ⇒
   `f_start <= t AND t < f_end`; `Overlaps(a,b)` ⇒
   `a_start < b_end AND b_start < a_end`; `Contains(a,b)` interval ⇒
   `a_start <= b_start AND b_end <= a_end`; point ⇒ the membership form. Interval
   value equality ⇒ pairwise equality on the halves.
3. **Negation:** each negated atom ⇒ one `NOT EXISTS (SELECT 1 FROM rel WHERE
   <correlated bindings + its own filters>)` appended to the WHERE of the
   `SELECT DISTINCT` core. Correlation uses the positive join's column aliases —
   extend the alias bookkeeping; self-negation (negated atom over a relation also
   joined positively) must alias fresh.
4. **Param sets:** rendered as literal `IN (v1, ..., vk)` lists **re-rendered per
   execution** (document in a comment: prepared-statement parity is not claimed
   for set-bound families — `60-validation.md` says so). Empty set ⇒ `IN (NULL)`
   is a NULL trap — render `1 = 0` instead; comment it.
5. **Aggregate templates** (over the `SELECT DISTINCT <all bound vars>` subquery,
   as today): `CountDistinct(x)` ⇒ `COUNT(DISTINCT x)`;
   Arg-restriction ⇒ the join-back template:
   `WITH d AS (<distinct subquery>) SELECT <group, carries> FROM d JOIN (SELECT
   <group>, MAX(key) mk FROM d GROUP BY <group>) m ON <group-eq> AND d.key = m.mk`
   — with `SELECT DISTINCT` on the outer (ties project set-honestly both sides).
   Global-group variant omits the GROUP BY/join keys.
6. **The inexpressible list:** a single `fn sqlite_expressible(query|write) ->
   Result<(), Inexpressible>` enumerating what the SQLite lane cannot judge
   (dependency verdicts — all of them; nothing on the query side should remain
   inexpressible after this PRD). The verify harness (human-run) consumes it;
   goldens pin it.
7. Golden discipline carries over: every new translation form gets a hand-written
   SQL golden pinned byte-for-byte (the existing translator-test style).

## Out of scope

Trigger emulation (refused, `60-validation.md`). Harness runs.

## Passing criteria

- `[shape]` `sqlite_expressible` exists and lists exactly the dependency
  judgments; no query construct returns `Inexpressible`.
- `[test]` Byte-pinned SQL goldens: negated atom (incl. self-negation aliasing),
  IN with 3 elements, empty-set rendering (`1 = 0`), membership, Overlaps,
  Contains both forms, interval equality, CountDistinct, ArgMax grouped and
  global.
- `[test]` Round-trip: interval facts inserted through the DDL split re-read as
  equal `Value::IntervalX` pairs (boundary values, negative starts).
