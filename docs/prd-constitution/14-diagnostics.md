# PRD 14 — Diagnostics with teeth: the acceptance gate explains itself

**Depends on:** 08 (error.rs quiet), 11 (exact-partition idiom exists —
the diagnostics point at it).
**Modules:** `crates/bumbledb/src/error.rs` + `error/display.rs`
(`NoMatchingTargetKey`, `NoPointwiseTargetKey` if present as its own
variant), `schema/validate.rs` (`resolve_target_key` :645 — the
rejection construction site; the duplicate-functionality check
:280-293), `api/prepared/bind.rs` (`unresolved_literals` :305-358) +
`api/prepared/introspect.rs` (the EXPLAIN surface),
`ir/validate/tests/reject.rs` (the negated-binder locks).
**Authority:** spec P1-5 + audit test list #6/#12, verified: the
exact-field-set rule is the single most user-facing acceptance rule and
its rejection names nothing; redundant superkeys seal silently (guard
write-amplification the engine cannot see); a dictionary-miss empty is
invisible in EXPLAIN despite the counter existing; the param-only/
aggregate-only negated-binder rejections are structurally foreclosed
but unpinned.
**Representation move:** rejection payloads gain the facts the user
needs; no acceptance change anywhere (the one hard rule of this PRD).

## Context (decided shape) — four cuts

1. **`NoMatchingTargetKey` carries the evidence.** The variant gains
   `{ target: RelationId, projection: <the house projection carrier>,
   available: Box<[KeyId]> }` (or the display-oriented equivalent the
   error taxonomy prefers — match the violations-refactor style).
   Display renders the target relation, the requested field set, and
   every declared key's field set, ending with the pointed hint when
   the projection contains an interval position: "declare the exact
   pointwise key `R(prefix…, interval) -> R`" (the PRD-11 idiom's
   lesson as a diagnostic). If the pointwise case is a separate
   variant today, both gain the same payload.
2. **Redundant-superkey warning, diagnostics-only.** At seal time,
   when a declared key's field set is a strict superset of another
   declared key's, record a warning (the schema validation report's
   non-fatal channel; if no such channel exists, add one —
   `Schema::validate` returning warnings alongside the sealed witness
   is the decided shape, `Vec<SchemaWarning>` with one variant
   `RedundantSuperkey { relation, key, implied_by }`). ACCEPTANCE IS
   UNCHANGED — the audit is explicit that superkeys are write
   amplification, not unsoundness.
3. **Unresolved-literal visibility.** The prepared introspect/EXPLAIN
   output gains a line when `unresolved_literals > 0`: which literals
   are pending interning and that an unresolved Eq literal empties its
   rule at execution until latched (the latch semantics, stated where
   the user will look when a query "returns nothing"). Counter already
   exists (bind.rs); this is surfacing, not new state.
4. **The negated-binder locks.** Two rejection tests pin what the
   structure forecloses: a negated-atom variable whose only other
   occurrence is (a) a `Param` position, (b) an aggregate output —
   both `NegatedVariableUnbound`, both written with the negated atom
   textually FIRST (order-independence pinned again from the hostile
   side).

## Technical direction

Cut 1's payload must not capture `Schema` internals by reference —
owned data only (the error may outlive the descriptor). Cut 2's warning
channel: survey how `SchemaDescriptor::validate`'s callers handle the
return; the decided shape adds warnings WITHOUT changing the happy-path
type if a non-breaking carrier exists (e.g., warnings method on the
sealed Schema); otherwise the signature changes and every caller
re-anchors — no shims. Cut 3 follows introspect's existing record
style. All display strings golden-pinned.

## Passing criteria

- `[test]` A rejected containment's message lists available keys and
  (interval case) the declare-the-pointwise-key hint — golden.
- `[test]` `{id}` + `{id, span}` schema seals WITH the warning
  recorded; acceptance unchanged (both keys enforce — existing
  behavior pinned).
- `[test]` The unresolved-literal EXPLAIN line appears for an
  uninterned literal and disappears after latching — extends the latch
  test.
- `[test]` Both negated-binder locks green.
- `[gate]` No acceptance-behavior change (the full reject suite green
  with values unchanged except the enriched payloads, enumerated in
  the commit body); fingerprint pin untouched; clippy; fmt.

## Doc amendments (rule 6)

`30-dependencies.md` § the acceptance gate: the exact-set rule now
demonstrated with its diagnostic; `70-api.md`: the warning channel and
the unresolved-literal note.
