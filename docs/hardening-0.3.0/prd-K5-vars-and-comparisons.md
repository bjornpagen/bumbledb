# PRD-K5 — `vars()` + free comparison exports

Wave K · Repo: bumbledb `ts/` · depends on: — · blocks K7

## Objective

Kill the `r.var("x")` string repetition and the `r.lt(...)` method verbosity —
with the two shapes the prototypes proved sound, and WITHOUT the two shapes
proven unsound. Proven unsound (do NOT build): the destructuring-proxy
`(r, { service, w }) => …` (type parameters have no inference site in a
parameter pattern; index-signature bags collapse to `Var<string> | undefined`,
which is silently legal as a wildcard — a type/runtime divergence); any
`Record<string, Var>` proxy bag. Proven sound (build exactly these):

1. `const { service, w } = r.vars("service", "w")` — tuple-to-object mapping,
   inference identical to the string baseline (`Var<"service">` exact).
2. Free comparison functions — `eq/ne/lt/le/gt/ge/pointIn/covers/allen/and/or/not`
   are ALREADY environment-independent value builders in `ts/src/query/atom.ts`
   (`TermOps` merely aliases them; all judgment happens at the `.where` seam
   via `CheckCond` against the rule env). They are simply not exported.

## Work

1. **`vars`** on `TermOps` (`ts/src/query/scope.ts` or wherever `TermOps`
   lives — follow the module structure at HEAD):
   `vars<const N extends readonly string[]>(...names: N): { [K in N[number]]: Var<K> }`.
   Runtime: mint each var through the existing `r.var` path and assemble the
   record with `Object.defineProperty` own-property definition (the `__proto__`
   discipline — a name like `"__proto__"` must produce an own property, not a
   prototype write; pin it with a probe, mirroring the closed-handle probe).
   Duplicate names in one call: construction-time error, pointed message.
2. **Exports**: add the comparison/connective builders to `ts/src/index.ts`.
   Do not rename them; do not wrap them. JS shorthand punning
   (`r.match(Outage, { service })` over a pre-minted var) now works by
   construction — it is item 1's dividend, not separate machinery.
3. **Docs note**: one line in the module doc (and K7 will teach it): the free
   names `eq/not/and/or` collide with common host identifiers — import
   aliasing is the answer; the SDK does not rename for collision-avoidance.
4. **Probes** (intrinsic):
   - `vars` inference: `Var<"service">` exact (Equal-probe); domain flows from
     first binding; cross-domain reuse of a `vars`-minted var errors at the
     use site exactly like `r.var` (reuse the existing wall probes with the
     new mint).
   - `select` rows exact through `vars`-minted names.
   - `__proto__` own-property probe for the record.
   - Free `lt` through `.where`: params inference identical
     (`{ cap: bigint }`-style probe), interval-var-under-lt refused, unbound
     var refused — the three existing refusals re-pinned through the free
     spelling.
   - `lowerQuery` golden: a rule written entirely with `vars` + free
     comparisons lowers to IR identical to its `r.var`/`r.lt` twin.

## Passing criteria

- All probes green; the IR-identity golden holds.
- `index.ts` exports the enumerated names (grep-pinned in a probe that
  imports each from the package root).
- No proxy bag, no `Record<string, Var>`, no destructuring-callback overload
  anywhere in the diff.
- `tsc --noEmit` green for touched modules + probes; zero casts. Push per the
  wave's commit discipline.
