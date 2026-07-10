# PRD 19 — Derived relations: the view story, canonized (doc unit)

**Depends on:** 18 (the maintenance idiom cites the witness); PRD 12 usefully
first (Pack output is the canonical derived-relation feedstock) but not
required.
**Modules:** documentation only — `10-data-model.md`, `70-api.md`,
`00-product.md`. No engine code; this PRD exists because the absence of a
written view story is how `CREATE VIEW` gets requested, and the set's policy
is that every refusal names its replacement.
**Authority:** `30-dependencies.md` (statements as the coherence calculus),
`20-query-ir.md` (queries as plain data).

## Context (decided shape)

SQL's "view" is one word for two different things, and bumbledb answers each
with machinery it already has:

1. **Virtual views = host-level IR composition.** Queries are plain data; a
   view is a function returning IR fragments (atoms, predicates, rule
   bodies) that the host splices into queries. The host language is the
   composition layer — this is the recorded doctrine doing its job, and it
   needs zero engine surface. The docs name the idiom (*"a view is a
   function returning atoms"*), show one worked example (the calendar's
   busy-claims fragment reused across three queries), and record the
   refusal: no named-view registry in the engine, ever — a registry is a
   second schema with none of the theory's guarantees.
2. **Materialized views = a relation plus statements — strictly stronger
   than SQL's.** Materialize derived data into an ordinary relation; state
   its relationship to the sources (`<=` for soundness — everything derived
   is justified; `==` where the derivation is exact and the acceptance gate
   admits it). The commit judgment then makes a stale or unsound
   materialization **uncommittable**: SQL matviews go stale silently and
   `REFRESH` is a prayer; here coherence is a theorem checked on every
   commit that touches either side. The host maintains; the engine judges.
   Maintenance is exactly PRD 18's idiom: query the sources on a snapshot →
   recompute → `write_from` the delta with the snapshot as witness — the
   derived relation cannot commit against sources it didn't actually read.
3. **The honest limit, stated** (the value-agreement boundary from the
   design record): statements prove *presence and topology* (every derived
   row justified, every source row represented, keys and arms right); they
   cannot prove *arithmetic agreement* across relations (a copied interval
   or a summed balance matching its inputs is a computation, outside the
   ∀∃ vocabulary by the acceptance gate). Cross-field agreement is host
   discipline plus, where wanted, an offline `verify_store`-grade re-derive
   — recorded with its trigger, not papered over.
4. **Deleted vocabulary, extended**: *view* → a function returning atoms;
   *materialized view / refresh* → a relation under statements, maintained
   by witnessed writes. Both rows enter the `00-product.md` table.

## Technical direction

Write the chapter; it has three sections mirroring 1–3 above, one worked
example each (calendar fragment; a Pack-fed coalesced-claims relation under
`<=`; the interval-copy limit from the claim↔attendance `==` design). Cross
reference PRD 18's conditional-write chapter rather than duplicating the
idiom.

## Passing criteria

- `[shape]` The derived-relations section exists in `10-data-model.md`'s
  modeling discipline with all three sections and both refusals (no view
  registry; no arithmetic-agreement statements).
- `[shape]` The deleted-vocabulary table carries *view* and *materialized
  view / refresh* with their replacements.
- `[shape]` `70-api.md`'s composition idiom names the pattern and points at
  the worked example.
- `[gate]` n/a — no code.

## Doc amendments (rule 5)

This PRD *is* its doc amendments; listed above.
