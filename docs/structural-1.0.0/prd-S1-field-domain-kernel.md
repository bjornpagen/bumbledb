# PRD-S1 — The field & domain kernel (structural)

Wave 1 · Repo: bumbledb `ts/` · depends on: — · blocks: S2, S3, S4 · the foundation

## Objective

Invert the SDK's type foundation from nominal-brands to **structural values with
schema-level domain labels**. A field's value type becomes its bare structural
type; the domain becomes a string label living in the field's *descriptor type*,
never a brand on the value. Delete every trace of the brand machinery. This is the
root of trust the whole SDK rebuilds on, so its type surface must be exact and
cast-free before S2/S3/S4 touch it.

## Scope (files)

`ts/src/fields.ts`, `ts/src/brand.ts` (DELETE or reduce to the structural
descriptor types), `ts/src/closed.ts`, and the field probes in
`ts/test/{types,type-kernel}.test.ts`. (S2 owns relation/schema; do not edit those
here beyond what a shared type import requires.)

## The target shape (ratified design; build exactly this)

```ts
// bare structural value types — NO brand, NO phantom:
u64  → bigint        i64 → bigint        str → string        bool → boolean
bytes(N) → Uint8Array         interval(E) → { start: bigint; end: bigint }   (half-open)
interval(E, W) → same value shape, width W is a descriptor-type label

// domain label lives in the DESCRIPTOR TYPE, not the value:
const HolderId = u64.as("HolderId")        // a field descriptor whose type carries domain "HolderId"
Account = relation("Account", { id: u64.as("AccountId").fresh, holder: u64.as("HolderId") })
// Infer<F>  → the field's bare VALUE type (bigint), used in Fact<> and results
// the field descriptor's TYPE additionally carries { kind: "u64"; domain: "HolderId"; fresh?: true } structurally
```

## Invariants to achieve (each becomes a probe)

1. **Values are bare and structural.** `Fact<typeof Account>["id"]` is `bigint`
   (not a brand); `.holder` is `bigint`; a `str` field is `string`; a `bool` is
   `boolean`; `bytes(32)` is `Uint8Array`; `interval(i64)` is `{ start: bigint;
   end: bigint }`. No `Brand<>`/phantom appears in any value type. Two `bigint`
   fields of *different* domains ARE mutually assignable at the value level (that
   is the point of structural — the domain wall lives in the builders, S2/S3, not
   on the value).
2. **`.as("Domain")` is a descriptor-type label.** It attaches `domain: "Domain"`
   to the field's descriptor type as a string literal; it performs NO runtime
   branding and requires NO cast to produce a value. `.as` exists on the four
   Rust-`as`-legal constructors (`u64`, `i64`, `bytes(n)`, `interval(e[,w])`) and
   is a type-level absence on `bool`/`str` (they carry no reference domain).
3. **`.fresh`** exists only on `u64` (optionally after `.as`), marking an
   engine-minted key in the descriptor type; a `.fresh` field is omittable in
   `insert` (S4 reads this).
4. **`closed()` both tiers, structural.** `closed("Kind", ["A","B"])` yields a
   vocabulary whose `.id` is a reference field descriptor with domain `"KindId"`
   (or the declared handle-domain), and whose handle constants (`Kind.A`) are bare
   values (the declaration-order id, a `bigint`) — NOT branded. Payload tier
   `closed("Sev", { pages: bool })({ Critical: { pages: true }, … })` types the
   payload columns by their structural kinds; `.where({ pages: true })` (used by
   S2) reads them. No fact struct is emitted for a closed relation (unwritable — a
   type-level absence). The `__proto__`-safe own-property minting stays
   (`Object.defineProperty`, not assignment) — representation, not a name-blocklist.
5. **`bytes<N>` width and `interval<E,W>` width are descriptor-type labels**, so
   S2/S4 can enforce width where the engine does; order operators on
   `bytes`/`interval` values are unavailable in the type (the engine refuses them).
6. **`Infer<F>` is total and precise** over every field kind, yielding the bare
   value type used in `Fact<>`/results/query terms — one definition, no per-site
   divergence.
7. **The kernel is cast-free.** Zero `as`/`any`/`!`/`unknown`-launder in product
   code — deleting the brands is what makes this literally achievable (there is no
   phantom to assert). This is the elegance dividend; protect it.

## Work

1. Rewrite `fields.ts`: the constructors produce structural value types + a
   descriptor type carrying `{ kind, domain?, fresh?, width? }`. `.as`/`.fresh`
   populate the descriptor type only. Delete the value-branding path entirely.
2. `brand.ts`: delete the `Brand<>`/phantom machinery. If a small structural
   descriptor-type helper is worth keeping, reduce the file to that (rename it if
   `brand` is now a misnomer); otherwise remove the file and its exports.
3. Rewrite `closed.ts`: handles are bare values; `.id` is a domain-labeled
   reference descriptor; payload tier types columns structurally; keep the
   own-property minting.
4. Rewrite `Infer<F>` to the bare value type; rewrite the type probes in
   `test/{types,type-kernel}.test.ts` to the structural expectations.
5. Hard-break freely; no back-compat; rename types for clean hovers
   (`U64Field<"AccountId">` etc.). No `@ts-expect-error` in product code — only in
   `test/*`, and each fail-probe must be real (removing the directive breaks
   compilation).

## Technical direction

- The domain label is a plain string literal in the descriptor type; keep it that
  way (structural comparison in S2/S3 is a string-literal equality, not a brand).
- `interval` value shape is `{ start; end }` with `bigint` endpoints for `i64`/`u64`
  element domains; the width label is on the descriptor type, not the value.
- Do NOT weaken any engine law to make a type nicer: order-refused stays
  order-refused; width stays load-bearing.
- Between this PRD and S4 the tree WILL be red (S2/S3/S4 consume the new kernel and
  are not done). That is expected; do not add shims to keep old callers compiling.

## Passing criteria (scoped — whole-SDK green is S4's job)

- **Compile-must-PASS probes** (`test/types.test.ts`): `Fact<typeof R>` fields at
  bare structural types; `.as("X")` present on the four ctors, absent on
  `bool`/`str`; `.fresh` only on `u64`; `Infer<typeof HolderId>` = `bigint`;
  `bytes(32)`/`interval(i64,W)` width labels present in the descriptor type.
- **Compile-must-FAIL probes** (`// @ts-expect-error`, real): `.as` on
  `bool`/`str`; `.fresh` on non-`u64`; `.newtype` anywhere (gone); a brand type
  referenced anywhere (gone); an order comparison typed on a `bytes`/`interval`
  value.
- Runtime: the `__proto__` own-property probe passes; `closed()` both tiers behave;
  handle constants are the right bare values.
- `pnpm exec tsc --noEmit` green FOR THE KERNEL + its probes (other files may be
  transiently red — S2/S3/S4 pending); `pnpm exec biome check .` clean on the
  touched files; the field/type-kernel test suites green.
- Zero casts in `fields.ts`/`closed.ts` (grep `as `/`any`/`!` in product lines).
- Report `breaks` = the exact descriptor/`Infer`/`closed` shape S2/S3/S4 must
  consume. Commit is deferred to the Land phase (no git here).
