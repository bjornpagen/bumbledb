# PRD 16 — The calendar family

**Depends on:** 15 (no timing without the stamp), and every representation it
times (03–12).
**Modules:** `crates/bumbledb-bench/src/` (corpus generator, family
definitions, SQLite mirror), `docs/architecture/60-validation.md`.
**Authority:** `00-product.md` (success criteria: per-family median, every
family must win), `60-validation.md` (protocol).
**Representation move:** the shipping law, honored. "An optimization that
cannot cite its number does not ship" — and a *representation* that cannot
cite its number is a museum piece. The algebra earns its existence here or
the set is rolled back; that sentence is the point of writing it down.

## Context (decided shape)

A calendar-shaped benchmark family — the workload this phase's vocabulary
exists for, drawn from the census shape (ledger-adjacent scheduling), scaled
to the standard corpus points:

- **Corpus:** accounts, calendars, events (bounded + ray recurrence horizons),
  attendance with RSVP arms (a DU), per-person claims (busy/OOO arms over
  intervals), rooms with pointwise-keyed bookings, working-hour segments —
  the schema from the design discussion, fresh-keyed, statement-complete
  (room exclusion, claim↔attendance `==`, working-hours coverage).
- **Timed queries, one per new representation:**
  1. *Busy scan* — `Allen(INTERSECTS)` against a param window (03/04).
  2. *Named-relation probes* — `MEETS` chains and `DURING` filters (the mask's
     singleton cost = composite cost claim, measured).
  3. *DU whole-read* — the attendance union across RSVP arms (05/07/08: the
     elision family; measured with the proof on and forced off, the delta is
     the elision's number).
  4. *Conflict pairs* — self-join on claims with `INTERSECTS`, anti-probe
     variant for "conflict-free" (04 + negation).
  5. *Free-busy* — `Pack` per person per window (11/12).
  6. *Time insights* — `Sum(Duration(...))` grouped by claim arm (10).
- **SQLite mirror:** fully indexed, prepared, `ANALYZE`d, `synchronous=FULL`,
  `SELECT DISTINCT`, endpoint-comparison SQL for masks, window/CTE SQL for
  free-busy (SQLite *can* coalesce with a recursive CTE — it gets its honest
  best shot; where the translator cannot express a family it is reported
  unpaired, never silently dropped — the no-silent-caps rule).
- **Gate:** the family joins the ALL-WIN ratchet (warm medians, cold reported,
  rebuild spikes exempt). The latency budget applies: if the O(n) busy scan
  violates p99 at the top scale point, the range-accelerator OPEN item fires
  with this family as its evidence — the trigger this family exists to arm.

## Technical direction

1. Corpus generator: seeded, stratified (persons × density × ray fraction);
   claim distributions with realistic overlap (Zipfian meeting density —
   hand-rolled per the bench-crate dependency quarantine).
2. Family definitions + goldens for the fixed queries; the randomized verify
   pass (the oracle lanes, landed — `60-validation.md`) runs the same shapes
   first — no timing without the stamp.
3. Report: per-family medians into the standard report; the elision on/off
   delta as a named sub-measurement.

## Passing criteria

- `[test]` Verify green over the calendar corpus before any timing runs
  (stamp discipline).
- `[shape]` Every family names the representation it times (the table above,
  in the family definitions' doc comments); unpaired SQLite families are
  reported as unpaired.
- `[gate]` The bench runs; the report generates; ALL-WIN status is *reported*.
  (Whether it wins is a measurement, not a passing criterion — the criterion
  is that the number now exists. Acting on a loss is the owner's ruling, per
  the ratchet protocol.)

## Doc amendments (rule 5)

`60-validation.md`: the calendar family joins the protocol. `00-product.md`:
the workload census sentence gains scheduling, with this family as its
measured form.
