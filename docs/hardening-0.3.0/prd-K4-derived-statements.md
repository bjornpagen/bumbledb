# PRD-K4 — Derived statements: synthesis, dedupe, tail order, closed refs

Wave K · Repo: bumbledb `ts/` · depends on: K3 · blocks K7 · hard break

## Objective

`schema()` learns to derive the containment a `ref` states:
`ref(Service, "id")` on `Outage.service` synthesizes
`contained(on(Outage, "service"), on(Service, "id"))`. The dedupe, ordering,
and opt-out laws are ratified (00-README rulings 2–3) — implement exactly
those. Closed refs get the same derivation, and the
`verifyClosedReferences` error class — which today instructs the user to type
the exact statement the SDK could have written — is deleted.

## Work

1. **Synthesis** (`ts/src/schema.ts`): after collecting the written statement
   array, walk relations in DECLARATION ORDER, fields in declaration order;
   for each field carrying `refTo`, synthesize
   `contained(on(Owner, field), on(Target, targetField))` through the ordinary
   public constructors (never a parallel statement representation — byte-equal
   lowering and rendering fall out for free). `citeTo` derives NOTHING.
2. **Dedupe — derived yields to hand-written**: before appending, render the
   candidate via `renderStatement` and skip it if any WRITTEN statement's
   render equals it string-exactly. The hand copy stays where it stands
   (statement ORDER is fingerprint-hashed; the yield rule is what keeps a
   migrated store's fingerprint stable). Document, in the module doc at the
   synthesis site, WHY this deliberately diverges from the fresh-implied-key
   law ("redundant — rejected"): implied keys are engine-materialized, derived
   containments are real declared statements whose position is identity.
3. **Ordering**: genuinely new derived statements TAIL-APPEND after all
   written statements, in the relation-declaration × field-declaration walk
   order of step 1. This order is pinned forever; write it in the module doc.
4. **Closed refs**: a field whose descriptor is a closed `ref` (K3's
   `ref(Kind, "id")`) derives the containment to the closed target the same
   way. Then DELETE the `verifyClosedReferences` check and its error class
   from `schema.ts` — with derivation, an unlinked closed-id field is
   impossible to construct through `ref`, and a hand-labeled one (via the
   closed vocabulary's own id domain) keeps the existing engine-side judgment
   as the final authority. Sweep the error's exports/tests.
5. **The type surface**: the schema's statement-list TYPE must include derived
   statements (downstream K7 recipes and violation-rendering read them) —
   derive the type alongside the value with the same walk; no `any`, no
   widening.
6. **Probes** (intrinsic):
   - A ref-only schema and its hand-written twin produce IDENTICAL manifests
     (statement-for-statement, spelling-for-spelling) and identical
     fingerprints when the hand twin writes the same statements in the same
     positions the derivation produces.
   - Dedupe: a schema with BOTH the ref and the hand statement yields exactly
     one copy, at the hand statement's position; fingerprint equals the
     hand-only schema's.
   - Tail order: two refs across two relations land in the pinned walk order
     (manifest golden).
   - `cites`: the Calendar shape — `cites(Attendance, "id")` + a selected
     `mirrors` — produces NO plain containment (manifest golden), and the
     selected mirrors still compiles/paires (domain flows from the cite).
   - `verifyClosedReferences` gone: grep zero references; the closed-ref
     schema builds without the old hand statement.
   - Fingerprint-motion honesty: a probe documenting (as an assertion) that
     ADDING a ref to a schema whose hand statements did not already spell the
     containment CHANGES the fingerprint — the PRD's own reminder that
     derivation is real statements, not decoration. (Humans own migrations;
     the assertion is the documentation.)

## Technical direction

- Never inspect or special-case what K7's recipes will write — the laws above
  are total; recipes conform to them, not vice versa.
- `renderStatement` equality is the ONE dedupe key (no structural comparison
  fallback — the renderer is already the schema-level spelling authority,
  byte-pinned against the engine manifest).
- Zero casts; the derived-statement walk must be a plain-data fold anybody can
  read.

## Passing criteria

- All probes green; manifests and fingerprints pinned as described.
- `verifyClosedReferences` absent from the tree (grep).
- The module doc carries the dedupe-divergence rationale and the tail-order
  law verbatim enough to survive review.
- `tsc --noEmit` green for `schema.ts` + probes; zero casts in the diff.
  Push per the wave's commit discipline.
