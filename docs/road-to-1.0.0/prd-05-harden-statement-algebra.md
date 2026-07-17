# PRD-05 — Harden: the statement algebra & `schema()`

Repo: bumbledb · depends on: 04 · blocks: 07 · parallel with 06

## Objective

Make the statement builders (`key`, `contained`, `mirrors`, `window`, `on`, the
`count` vocabulary) and `schema()` end-to-end typesafe under the doctrine: the
canonical-utterance ban table is UNWRITABLE (no constructor yields a banned
spelling), every statement's field references are type-checked against the
relations they name (existence AND brand/type compatibility), and `schema()`
lowers a set of statements to the descriptor with types carried throughout. This
is where "the ban table is unwritable, not checked" lives.

## Scope (files)

`ts/src/statements.ts`, `ts/src/relation.ts`, `ts/src/schema.ts`,
`ts/src/count.ts`, `ts/src/spec.ts`, and `ts/test/{statements,types}.test.ts`.

## Invariants to achieve (each becomes a probe)

1. **The window ban table has no constructor for a banned spelling.** The five
   count constructors `exactly(n)` / `none()` / `between(lo,hi)` / `atLeast(n)` /
   `atMost(n)` PARTITION the legal windows; `{n..n}` (write `exactly`), `{0..0}`
   (write `none`), `{0..*}` (vacuous — provably says nothing) and other banned
   forms are UNWRITABLE — there is no argument shape that produces them. A
   degenerate/vacuous window is a type error or a nonexistent call, never a
   runtime rejection.
2. **`on(Relation, "field")` is field-checked in the type.** The field name must be
   a key of that relation's fields (`"field"` autocompletes; an unknown field is a
   type error). `on(R.where({ closedField: Handle.Variant }), "field")` types the
   selection: the selected field must be a closed reference and the handle a member
   of its vocabulary (`Kind.Sav` where `Sav` is not a handle → type error).
3. **`contained(on(A,x), on(B,y))` type-checks the pair**: `x` and `y` must be
   brand/type-compatible (the containment relates comparable projections), and `B`'s
   `y` must be (or resolve to) a key — the acceptance gate the engine also enforces,
   surfaced at the type level where expressible; where it is a semantic property the
   engine judges at open, it stays a typed `Db.create` error (the two-boundary
   split), documented as such.
4. **`mirrors(...)` is the selected `==` bijection**, lowered to two containments
   source-first (matching the engine); its type reflects the keyed one-to-one
   correspondence. `key(R, [...])` types its projection field list against `R`.
5. **`schema("Name", { relations }, [statements])` is fully typed**: the relations
   map is typed, the statements array elements are the typed statement values, the
   result carries the schema's relation set as typestate (so `Db<typeof Ledger>`
   only accepts that schema's relations and facts), and it lowers to the same
   `SchemaSpec`/descriptor the Rust macro emits (the fingerprint-parity law).
6. **Handle selections require their companion containment where the paste-back law
   demands it** (the bug-hunt finding): a closed-handle `where`-selection whose
   canonical rendering would diverge from `renderStatement` is refused — this stays
   enforced (representationally in the schema build where possible, else a typed
   `schema()`/open error), never silently rendered wrong.

## Work

1. Audit each builder against invariants 1–6. Delete every runtime guard a type can
   carry (the ban table especially — banned windows must be UNCONSTRUCTIBLE, not
   thrown). Eliminate casts from the builders.
2. Hard-break the builder signatures as needed for full field-name and brand
   checking (generic over the relation's field record). Keep the surface
   value-shaped and hover-first (no type-level string parsing — ratified). Read
   through the IDE, not the page.
3. Keep the two-tier ban enforcement: the SDK forbids representationally at
   construction; the engine lowering remains the law for a hostile FFI spec. Where
   a property is only semantic (target-is-a-key resolution, etc.), route it to the
   typed `Db.create`/`schema()` error and doc-comment WHY the type cannot state it.
4. Confirm the lowering output is byte-identical to the macro's for every construct
   (the standing fingerprint-parity pin, PRD-02's `render-golden`/`fingerprint`
   tests) — the hardening changes the SURFACE, never the emitted descriptor.

## Technical direction

- Doctrine: unwritable-not-checked is the headline here. If a reviewer can write a
  banned window and get a runtime error, the PRD failed — it must not compile.
- `on`/`where` generics must autocomplete field names in the editor (a hover/IDE
  quality bar, not just a compile bar).
- Do not change the engine's canonical-utterance law or the descriptor bytes; the
  fingerprint parity test is the tripwire.
- `// @ts-expect-error` only in `test/*`.

## Passing criteria

- **Compile-must-PASS**: every legal construct — plain key FD, composite/pointwise
  FD, containment plain + σ + ψ, `mirrors`, and each window spelling
  (`exactly`/`none`/`between`/`atLeast`/`atMost`) — builds and lowers; `schema()`
  types the store and `Db<typeof Ledger>` accepts only its relations.
- **Compile-must-FAIL** (`// @ts-expect-error`): a banned window spelling (must be
  unconstructible — assert the constructor/shape does not exist); `on(R, "nope")`
  unknown field; a cross-brand containment pair; `R.where({ f: NotAHandle })`; a
  `key` projection naming a non-field; assigning a schema-A fact to a `Db<schemaB>`.
- The fingerprint-parity + render-golden tests from PRD-02 stay green (surface
  changed, descriptor unchanged).
- `tsc --noEmit` green; `biome check ts/` clean; `node --test` green for the
  statement suites; zero casts in the audited builders.
- Commit in the repo's voice; push.
