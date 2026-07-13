# PRD 08 — The classified comparison: place_comparisons made total

**Depends on:** 05 (written against the `ConditionTree` vocabulary).
**Modules:** `crates/bumbledb/src/ir/validate/context.rs` (the
comparison-shape checks — the ~27-assert cluster),
`ir/normalize/place_comparisons.rs` (the re-derivation site),
`ir/validate.rs` (the witness pipeline), tests covering comparison
placement and rejection.
**Authority:** parse-don't-validate (King), the `RuleWitness` precedent
(validation's findings sealed as a typed object the pipeline reads
instead of re-deriving), and the recorded trigger from the witness
campaign: the comptime audit counted the asserts, ruled "fires when a
third module needs comparison classification," and the census now shows
`place_comparisons` re-deriving what `context.rs` already proved — the
trigger has fired.

**Representation move:** validation currently PROVES each comparison's
shape (var-vs-var, var-vs-const, side typing, interval-endpoint
legality…) and then throws the proof away; `place_comparisons` re-derives
it defensively with `unreachable!`/asserts. Seal the proof as a
`ClassifiedComparison` sum at validation time; normalization consumes the
sum with total matches. The asserts don't get "cleaned up" — they become
unrepresentable.

## Context (decided shape)

- `ClassifiedComparison` — a closed sum whose variants are exactly the
  legal comparison shapes validation accepts today (read them off
  `context.rs`'s accept paths, not invented: the var/var join-order
  form, the var/const forms per type class, the interval-endpoint
  forms, the ∈-set membership forms if they flow through the same
  check). Each variant carries the RESOLVED facts the placer needs
  (which rule-var indices, the sealed constant/handle, the operator) —
  no re-lookup downstream.
- It lives with the other pipeline witnesses (the `RuleWitness`
  neighborhood in `ir/validate/`), is produced only by validation, and
  is **pipeline-internal**: it never appears in `ir.rs`, never in the
  public API, never serialized. Exactly the `RuleWitness` placement
  discipline.
- `RuleWitness` carries the classified comparisons for its rule (the
  natural home — the witness already carries per-rule sealed findings);
  `place_comparisons` signature changes from re-walking `ConditionTree`
  comparisons to consuming `&[ClassifiedComparison]`, and becomes a
  TOTAL function: every match arm constructs placement; zero
  `unreachable!`, zero asserts, zero error returns from shape.
- The ~27 defensive checks in `context.rs`/`place_comparisons.rs` die
  by construction. Count before and after; the census (PRD 09) records
  the delta.

## Technical direction

1. Enumerate the accept paths in `context.rs` FIRST and write the
   variant list from them — the sum must be exactly the accepted
   language, no aspirational variants. If an accept path turns out to
   be dead (accepted by validation but unconstructible from the public
   IR), that is a policy-5 finding: record it, then delete the path
   rather than reifying it.
2. Pin second (policy 8): the existing placement tests plus a sweep
   test asserting placement output over every variant × a
   representative rule shape, green against current code.
3. Construct `ClassifiedComparison` at the exact point `context.rs`
   finishes proving a comparison legal (the proof and the seal are the
   same lines); thread through `RuleWitness`.
4. Rewrite `place_comparisons` as the total consumer; delete the
   re-derivation walk and every defensive check it carried.
5. Sweep: any other module found re-classifying comparisons (grep for
   the operator-match idiom) moves to the witness or is recorded as a
   refusal with trigger.

## Passing criteria

- `[shape]` Zero `assert!`/`debug_assert!`/`unreachable!` in
  `place_comparisons.rs` (grep); the assert census in the two files
  drops by the enumerated count, recorded in this file.
- `[shape]` `grep -n "ClassifiedComparison" crates/bumbledb/src/ir.rs
  crates/bumbledb/src/api` → zero hits (pipeline-internal).
- `[test]` The pinned placement sweep green before and after with
  unchanged assertion values; every rejection test still rejects with
  the same typed error.
- `[gate]` Workspace gates green at campaign close.

## Doc amendments (rule 5)

`20-query-ir.md` § validation: the witness roster gains the classified
comparison (one paragraph: proved once, sealed, consumed totally —
alongside `RuleWitness`, `ResolvableFilter`, `SinkSpec`, `ParamSpec` as
the fifth sealed finding).
