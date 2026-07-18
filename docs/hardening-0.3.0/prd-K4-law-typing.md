# PRD-K4 — The law-typing engine: `schema()` computes domains from statements

Wave K · Repo: bumbledb `ts/` · depends on: K3, K1 (selected-closed faces must
exist to be paired) · blocks K7 · the heavy PRD — the owner explicitly wants
the type machinery here ("ts is the best place for type heavy machinery")

## Objective

Implement rulings 2/3: the statements ARE the typing. `schema()` computes, at
BOTH the type level and runtime, an equivalence-class map over field slots
from the statement list, names the classes, enforces the one-generator wall,
and exposes the classes as the domains every downstream surface reads
(queries, the wire lowering). Nothing is ever synthesized — the statement
list in the manifest is exactly the statement list in the source.

## The three class laws (ratified; implement exactly)

Over the statement tuple, every paired face (containment `<=` including
ψ-selected targets, `mirrors`, pointwise `==`, window source/target pairs)
unions its positionwise field slots:

1. **Generators**: a `fresh` field is a generator; its class is named by its
   declaration coordinate `` `${Rel}.${field}` ``. A closed relation's id is a
   generator named `` `${Name}.id` ``.
2. **Generator-less classes** are named by their least member coordinate in
   relation-declaration × field-declaration order (both orders are readable
   off the schema's relations object — K3 guarantees it). Deterministic,
   pinned forever.
3. **Bare**: a field in no law has NO class and pairs only with bare in
   queries (the sum-domain-pointer case stays legal).

**The wall**: a class containing MORE THAN ONE generator is a contradiction —
two mints cannot share a carrier. Schema-level compile error (a named,
self-locating type: which two coordinates collided, through which statement)
AND the construction-time runtime twin (`schema()` throws with the same
content).

## Work

1. **Type level** (`ts/src/schema.ts` + a support module if the machinery
   wants its own file): from the statements tuple type, extract every paired
   face's (relation, field) slot pairs; compute connected components as a
   bounded fixpoint (iterate to the statement-list length — the diameter
   cannot exceed it; on TS recursion-limit failure, fail LOUDLY with a named
   type error, never silently widen). Name components per the laws. Output:
   the schema type carries a class map
   `{ [Rel]: { [field]: ClassName | undefined } }` alongside the
   relations/statements it already carries.
   Scale bar: primer's real schema (audited: ~123 statements, ~200 field
   slots across ~40 relations) MUST compile within default `tsc` limits — a
   probe fixture of that shape (generated, checked in) is part of this PRD.
2. **Runtime twin**: the same computation as a plain union-find in `schema()`
   (~50 lines); the value-level class map exposed on the schema value (own,
   frozen). One probe diff-checks the runtime map against the type-level map
   for a fixture schema (generated assertion, not hand-duplication).
3. **The wire lowering** (`ts/src/lower.ts`): emit the computed class name as
   the spec `newtype` for classed fields, omit for bare — preserving the
   existing bridge contract (the engine drops newtypes at lowering). Handle
   resolution must be re-verified with the closed-id class name `"Kind.id"` —
   if the bridge's handle resolution keyed on the OLD `` `${Name}Id` ``
   spelling anywhere, fix the SDK side to the new name and prove it with the
   existing handle-resolution tests. M5's Rust-side check is unaffected; the
   TS path is coherent BY CONSTRUCTION (paired faces share a class by
   definition).
4. **Queries** (`ts/src/query/*`): var domain-typing reads the CLASS off the
   schema type instead of the deleted descriptor label — `JoinOk`/sibling
   checks compare class names; bare-pairs-bare implemented exactly (a bare
   slot joins only bare slots; a bare↔classed pairing refuses); the runtime
   `domain ===` twin in `scope.ts` reads the runtime class map.
   `ClosedIdField`'s `` `${Name}Id` `` mentions move to the class name.
5. **Results/`Fact`**: value types untouched (bare). Nothing else reads
   domains.
6. **Probes** (intrinsic):
   - class-computation goldens: a fixture schema exercising every law —
     generator naming; a 3-hop chain (A.x <= B.y, B.y <= C.id) landing the
     whole chain in `"C.id"`; a generator-less mirrors pair named by least
     coordinate; a bare field staying bare; a ψ-selected face pairing; a
     selected-mirrors pair (the Calendar shape — the mirrors law types the
     source column with the target's class: pin exactly this);
   - the wall: two generators unified → the named compile error (real
     `@ts-expect-error`, self-locating type pinned) AND the runtime throw;
   - queries: same-class join compiles + lowers; cross-class pairing fails at
     the use site; bare↔bare joins; bare↔classed fails — all four re-pinned
     through `vars()` too;
   - the primer-scale fixture compiles (wall-clock/tsc memory noted in the
     commit body);
   - runtime/type agreement (step 2).

## Technical direction

- The class map is THE domain authority — no module may keep a parallel
  label table (grep for leftovers of the descriptor-domain era).
- Determinism everywhere: iteration orders come from declaration order.
  Integer-like relation/field names would break JS insertion-order
  enumeration — REFUSE them at construction with a pointed error (this also
  closes the enumeration-order fingerprint hazard the parity audit flagged).
- Zero casts; the union-find must be readable plain data-flow; every type
  claim (the class map, the frozen exposure) has its runtime twin.

## Passing criteria

- All probes green, including the wall (both tiers), the Calendar mirrors
  typing, bare-pairs-bare, and the primer-scale fixture under default tsc.
- The wire: spec `newtype` = class name / omitted; handle resolution proven
  against the new closed-id class name; fingerprints UNAFFECTED (newtypes
  are dropped — re-pin the neutrality law with class names).
- No `SameDomains`-era machinery anywhere (grep); no synthesized statements
  anywhere (manifest golden: statements in == statements out, order
  preserved, count equal).
- `tsc --noEmit` green for schema/query modules + probes; zero casts.
  Push per the wave's commit discipline.
