# PRD 18 — Sinks: CountDistinct and Arg-restriction

**Depends on:** 15 (slot layout in witness), 11.
**Modules:** `crates/bumbledb/src/exec/sink/aggregate/`, `crates/bumbledb/src/api/prepared/result_buffer.rs`.
**Authority:** `docs/architecture/20-query-ir.md` (§ aggregation — normative semantics), `40-execution.md` (§ set semantics in the executor — sink bullets).

## Goal

The aggregate sink grows the two new fold behaviors with the exact documented
semantics: CountDistinct as a per-group distinct-value fold, Arg-restriction as
restrict-then-project group state with set-honest ties. Interval values flow
through binding slots and the result buffer.

## Technical direction

1. **Slot plumbing first:** interval-typed variables occupy two slots (PRD 13
   layout). The recursion's slot writes, the binding-dedup seen-set (which hashes
   full binding tuples — must hash both words), the group-key extraction, and the
   result-buffer emit all consume the slot-layout map. Do this before the new
   ops; it is the change most likely to be half-done — grep every slot-array
   consumer and route each through the layout map.
2. **CountDistinct:** per-group state = a word-set (interval values: the two
   words hashed as one key — reuse the seen-set's tuple hashing over a 1–2 word
   span) in sink arena storage, exactly the projection-dedup mechanism scoped per
   group. Fold: insert; finalize: `len() as u64`. The binding-level dedup
   (first-occurrence fold) still applies upstream — comment why that is correct
   (distinct bindings ⊇ distinct values; the value set dedups further).
3. **Arg-restriction:** per-group state
   `{ key: u64 /*encoded orderable word*/, rows: <pooled row storage> }`.
   Fold per binding: compare the key var's slot word against the group's extreme
   under the op's direction (encoded words compare correctly for U64 and
   sign-flipped I64 — one comment); strictly better ⇒ clear rows, push this
   binding's projected row; equal ⇒ push (**ties are set-honest**: dedup pushed
   rows against the group's row set — two distinct bindings may project equal
   rows); worse ⇒ nothing. Finalize: emit every stored row for every group.
   Multi-carry coherence is automatic — rows are projected whole from surviving
   bindings, never per-term (this is the restrict-then-project semantics; cite
   it).
4. **Elision interaction:** the provably-distinct-bindings flag still elides the
   binding seen-set; it must NOT elide CountDistinct's value set or Arg row-dedup
   (different sets). Assert with a test on a serial-keyed fixture.
5. **Result buffer:** interval find values materialize as `Value::IntervalX`
   rows (two words re-encoded through the checked type — a stored invariant makes
   `Interval::new(...).unwrap()` legitimate here; comment it); `column_types()`
   reports the interval type.

## Out of scope

`Pack` (OPEN). Mixing Arg with folds (rejected in PRD 12).

## Passing criteria

- `[shape]` Every slot-array consumer routes through the layout map (no bare
  `slots[var]` arithmetic assuming width 1 survives — grep).
- `[test]` CountDistinct: multiplicities collapse (3 postings, 2 distinct amounts
  ⇒ 2); per-group scoping; over strings (intern-id words); over intervals (value
  identity, not overlap).
- `[test]` ArgMax: latest-posting-per-account fixture — single winner per group;
  constructed tie (two bindings, equal key, different carries) ⇒ both rows;
  constructed tie projecting equal rows ⇒ one row; ArgMin mirror; key-also-
  projected case; global group (no group keys) works.
- `[test]` Elision fixture: serial-keyed query with CountDistinct still dedups
  values while skipping the binding seen-set (assert both via the stats/EXPLAIN
  counters).
- `[test]` Interval find round-trip: a query projecting an interval var yields
  `Value::IntervalI64` rows equal to the stored facts'.
