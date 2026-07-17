# PRD-04 — Harden: the field & brand kernel

Repo: bumbledb · depends on: 02 · blocks: 07 · parallel with 05, 06

## Objective

Make the nominal layer of the SDK — the field constructors and the branded
newtypes they mint — sound and end-to-end typesafe under the
representation-over-control-flow doctrine: every illegal spelling is UNWRITABLE
(no constructor produces it), not runtime-checked; every brand a field mints
flows unbroken into `Fact<>`, into query terms, and into results with zero casts.
This is the foundation the statement algebra (05) and query surface (06) build on,
so its type surface must be airtight first.

## Scope (files)

`ts/src/fields.ts`, `ts/src/brand.ts`, `ts/src/closed.ts`, and the field-facing
probes in `ts/test/types.test.ts` / `ts/test/type-kernel.test.ts`.

## Invariants to achieve (each becomes a probe)

1. **`.newtype` exists on exactly the four Rust-`as`-legal constructors**
   (`u64`, `i64`, `bytes(n)`, `interval(e)`) and NOWHERE else — `bool` and `str`
   have no `.newtype`; a declared newtype has no `.newtype` (no re-branding); a
   closed relation's `id` field has no `.newtype`. Each absence is a *type-level*
   absence (the method is not on the type), not a runtime throw.
2. **Brands are name-keyed and declaration-first** (the ratified spelling):
   `const AccountId = u64.newtype("AccountId")` + `type AccountId = Infer<typeof
   AccountId>`; `.as` does not exist anywhere. Two fields referencing the same
   declared newtype are mutually assignable; two DIFFERENT newtypes are NOT
   assignable (the cross-brand wall), and neither is a raw `bigint`.
3. **`__proto__`-class handle/brand names cannot corrupt the object protocol.**
   The already-landed `Object.defineProperty` minting (own-property, never
   assignment) stays; a probe constructs a `closed()`/newtype with reserved-name
   handles and asserts the minted value reads back as the branded primitive, never
   `Object.prototype`. This is representation (own-property definition), not a
   name-blocklist guard — keep it that way.
4. **`bytes<N>` width is in the type.** `bytes(32).newtype("Digest")` carries `N`
   at the type level so a wrong-width literal/value is a type error at the field
   boundary (aligns with the engine's width-in-the-type law and the macro's
   token→Value width enforcement). Order operators on `bytes`/`interval` brands
   remain unavailable in the type (the engine refuses them).
5. **`Infer<F>` is total and precise** over every field kind (raw and branded,
   including a closed `id`), yielding exactly the branded value type used in
   `Fact<>` and query terms — one definition, no per-call-site divergence.

## Work

1. Audit `fields.ts`/`brand.ts`/`closed.ts` against invariants 1–5. Every place a
   runtime check enforces something the types could forbid, MOVE it into the types
   (delete the guard). Every place a cast (`as`, `!`, `any`, `unknown`-launder)
   appears in the kernel, eliminate it — the kernel is the root of trust and must
   be cast-free.
2. Hard-break freely: rename types for clean hovers (`U64Newtype<"AccountId">`
   etc.), reshape constructors, delete dead spellings. No back-compat.
3. Make `closed()` fully typed both tiers (handles-only and payload-columns): the
   handle constants are branded, `fromId` is the typed weld, payload columns are
   typed by their declared field kinds, and no fact struct is emitted for a closed
   relation (it is unwritable — a type-level absence).
4. Ensure the brand a field mints is the SAME brand observed on the way OUT
   (results), so PRD-07's result typing needs no re-derivation — document the one
   place the brand is defined and that both write-in and read-out reference it.

## Technical direction

- Doctrine: illegal states unrepresentable; parse-don't-validate at the boundary;
  zero casts in the kernel; hover-clean names. A `// @ts-expect-error` in PRODUCT
  code is forbidden; it appears ONLY in `test/*` probes to pin an unwritable
  spelling.
- Brand mechanism stays the string-tag under the hood (ratified) — the discipline
  is the single declaration site, not a language-level `unique symbol`.
- Do not weaken any engine law to make a type nicer: order-refused types stay
  order-refused; width stays load-bearing.

## Passing criteria

- **Compile-must-PASS probes** (in `types.test.ts`, positive pins): declaration-first
  newtype + `Infer`; same-newtype mutual assignability across two fields; a branded
  field flowing into `Fact<typeof R>` at the right branded type; `bytes(32)` width
  carried.
- **Compile-must-FAIL probes** (`// @ts-expect-error`): `.as` anywhere;
  `.newtype` on `bool`/`str`/a declared newtype/a closed `id`; assigning one
  newtype to another; assigning a raw `bigint` where a newtype is demanded; a
  wrong-width `bytes<N>` value; an order comparison typed on a `bytes`/`interval`
  brand. Each MUST error, and removing the `@ts-expect-error` MUST make the probe
  fail — i.e. the errors are real.
- Runtime: the `__proto__` own-property probe passes; `closed()` both tiers behave.
- `tsc --noEmit` green; `biome check ts/` clean; `node --test` green for the field
  and type-kernel suites. Zero casts in `fields.ts`/`brand.ts`/`closed.ts` (grep
  for `as `/`any`/`!` in product lines and justify or remove each).
- Commit in the repo's voice; push.
