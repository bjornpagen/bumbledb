# PRD-S2 — The statement algebra & `schema()`

Wave 1 · Repo: bumbledb `ts/` · depends on: S1 · parallel with S3 · blocks: S4

## Objective

Rebuild the statement builders and `schema()` on the structural kernel: the
canonical-utterance ban table is UNWRITABLE, every statement's field references
are checked against the relations they name (existence AND **domain**
compatibility, read structurally off the schema type — no value brands), and
`schema()` lowers to the descriptor with domains carried throughout and
fingerprint-parity to the Rust macro preserved.

## Scope (files)

`ts/src/statements.ts`, `ts/src/relation.ts`, `ts/src/schema.ts`,
`ts/src/count.ts`, `ts/src/spec.ts`, and `ts/test/{statements,types}.test.ts`. Do
NOT edit `fields.ts`/`closed.ts` (S1) or `query/*` (S3).

## The target shape (ratified; build exactly this)

```ts
relation("Name", { field: <S1 descriptor>, … })          // record of S1 field descriptors
closed(…)                                                 // from S1
schema("Name", { Rel1, Rel2, … }, [ …statements… ])       // relations map + statement array; Db<typeof S> typestate
// statements (free functions):
key(R, ["a", "b"])                                        // R(a,b) -> R
contained(on(A, "x"), on(B, "y"))                         // A(x) <= B(y)
contained(on(A.where({ f: Kind.V }), "x"), on(B, "y"))    // A(x | f==V) <= B(y)      σ
contained(on(A, "x"), on(B.where({ g: Kind.W }), "y"))    // A(x) <= B(y | g==W)      ψ
mirrors(on(A.where({…}), "x"), on(B.where({…}), "y"))     // A(..) == B(..), lowered source-first
window(on(T, "id"), atMost(3n), on(S, "f"))               // target-left cardinality
on(R, ["p","q"])                                          // composite/pointwise position (interval-pointwise ==, coverage)
exactly(n) | none() | between(lo,hi) | atLeast(n) | atMost(n)   // the window vocabulary — banned spellings unconstructible
```

## Invariants to achieve (each becomes a probe)

1. **The window ban table has no constructor for a banned spelling.** The five
   count constructors PARTITION the legal windows; `{n..n}` (write `exactly`),
   `{0..0}` (`none`), `{0..*}` (vacuous), and other banned forms are UNWRITABLE —
   there is no argument shape that produces them. A degenerate/vacuous window is a
   type error or a nonexistent call, never a runtime rejection.
2. **`on(R, "field")` is field-checked in the type** — `"field"` must be a key of
   `R`'s fields (autocompletes; unknown field = type error). **`on(R, ["a","b"])`**
   projects a composite position, each name field-checked. `on(R.where({ f:
   Kind.V }), …)` types the selection: the selected field must be a closed
   reference and the handle a member of its vocabulary (`Kind.Nope` where `Nope`
   is not a handle → type error).
3. **Domain compatibility is checked structurally.** `contained(on(A,"x"),
   on(B,"y"))` reads `A.x`'s domain and `B.y`'s domain off the schema types
   (S1 labels) and constrains them equal — a cross-domain pair is a compile
   error, achieved by string-literal comparison of descriptor shapes, NOT by any
   value brand. `mirrors` and `key` project/relate domains the same way.
   Field-list positions compare positionwise. What is only a semantic property
   (target-resolves-a-key) stays a typed `Db.create` error — doc-comment WHY the
   type cannot state it.
4. **`mirrors` is the selected `==` bijection**, lowered to two containments
   source-first (matching the engine); its type reflects the keyed one-to-one
   correspondence.
5. **`schema()` is fully typed**: the relations map typed, the statement array
   elements the typed statement values, the result carrying the relation set as
   typestate so `Db<typeof Ledger>` accepts only that schema's relations/facts;
   it lowers to the SAME `SchemaSpec`/descriptor the Rust macro emits — the
   fingerprint-parity law (surface changes, descriptor bytes do NOT).
6. **Handle-selection paste-back** (the bug-hunt finding): a closed-handle
   `where`-selection whose canonical rendering would diverge from
   `renderStatement` is refused — representationally in the schema build where
   possible, else a typed `schema()`/open error.

## Work

1. Rewrite the builders on S1's descriptors. Every runtime guard a type can carry
   moves into the type (the ban table especially — banned windows must be
   UNCONSTRUCTIBLE). Zero casts.
2. Make `on`/`where`/`key`/`contained`/`mirrors`/`window` generic over the
   relation's field record and its domain labels so field names autocomplete and
   domain mismatches error. Value-shaped, hover-first — no type-level string
   parsing.
3. Preserve two-tier ban enforcement: the SDK forbids representationally; the
   engine lowering remains the law for a hostile FFI spec. Route semantic-only
   properties to typed `Db.create` with a doc-comment reason.
4. Confirm the lowering output is byte-identical to the macro's for every
   construct — the standing fingerprint-parity pin (`test/render-golden.test.ts`,
   `test/fingerprint.test.ts`). Surface changes; descriptor bytes do not.

## Passing criteria (scoped — whole-SDK green is S4's job)

- **Compile-must-PASS**: every legal construct — plain/composite/pointwise `key`,
  containment plain + σ + ψ, `mirrors`, each window spelling — builds and lowers;
  `schema()` types the store; `Db<typeof S>` accepts only its relations.
- **Compile-must-FAIL** (`// @ts-expect-error`, real): a banned window spelling
  (assert the constructor/shape does not exist); `on(R,"nope")`; a cross-domain
  `contained` pair; `R.where({ f: NotAHandle })`; a `key` naming a non-field; a
  schema-A fact assigned to `Db<schemaB>`.
- The fingerprint-parity + render-golden pins stay green (surface changed,
  descriptor unchanged). `test/statements.test.ts` green.
- `pnpm exec tsc --noEmit` green FOR THIS SCOPE (`query/*` may be transiently red —
  S3 concurrent; note it); biome clean on the touched files; zero casts in the
  audited builders.
- Report `breaks` = the statement/schema API shape S4 consumes. Commit deferred to
  the Land phase.
