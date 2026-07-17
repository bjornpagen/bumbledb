# PRD-S3 — The query surface

Wave 1 · Repo: bumbledb `ts/` · depends on: S1 · parallel with S2 · blocks: S4

## Objective

Rebuild the Datalog-as-values query surface on the structural kernel, kysely-shaped:
from the schema's domain-labeled fields, through atom binding, joins, comparisons,
param sets, aggregates, membership, Allen, negation, and stratified recursion, to a
prepared query whose parameter object type and result row type are both inferred —
cast-free, illegal queries unwritable, and the param/result types exactly what the
engine accepts and returns.

## Scope (files)

`ts/src/query/{atom,lower,predicate,run,scope,select}.ts`, `ts/src/lower.ts`, and
`ts/test/{query,sdk-kernel-probes,c-sdk-2-probe}.test.ts`. Do NOT edit S1's or S2's
files.

## The target shape (ratified; build exactly this)

```ts
query(S)
  .rule((r) =>
    r.match(Outage, { service: r.var("s"), window: r.var("w") })   // EDB atom; vars bind fields
     .where(r.pointIn(r.param("t"), r.var("w")))                    // residual predicate
     .select("s"))                                                  // head projection
  .rule((r) => …)                                                   // more rules = set union

// vars: string-named, DOMAIN-TYPED from the field they first bind; reuse joins them.
// predicates: r.eq/ne/lt/le/gt/ge, r.pointIn(point, interval), r.allen(a, mask, b),
//             r.not(Rel, { … })  (negation / anti-join), r.inSet("frontier") (∈-set param)
// aggregates in select: r.sum("v") r.min r.max r.count r.countDistinct r.pack("v")
//                       r.duration("v") r.argMax("v", "keyvar") r.argMin
// recursion (stratified program):
program(S, (p) => {
  const reach = p.rec("reach")
  reach.rule((r) => r.match(Node, { id: r.var("c") }).where(r.eq(r.var("c"), r.param("root"))).select("c"))
  reach.rule((r) => r.match(Parent, { child: r.var("c"), parent: r.var("m") }).idb(reach, r.var("m")).select("c"))
  return p.output((r) => r.match(Posting, { account: r.var("a"), minor: r.var("m") }).idb(reach, r.var("a")).select(r.sum("m")))
})
```

## Invariants to achieve (each becomes a probe)

1. **Vars are domain-typed by the field they bind** (structurally, off the S1/S2
   schema types). Binding a var to a `HolderId`-domain field then joining it to an
   `AccountId`-domain field is a compile error (the structural analog of the old
   brand-equal join — now domain-equal, no value brands). A comparison
   `field == literal` type-checks the literal against the field's element type; a
   cross-type comparison (interval-vs-scalar, except the legal `pointIn`/`covers`
   interval-left form) is a compile error — the bug-hunt fix, now a type-level
   guarantee.
2. **Params are typed by domain AND usage.** A param's type is its binding field's
   element type; the prepared query's parameter-object type is inferred to be
   EXACTLY the params the rules use; a declared-but-unused param must not make the
   query unexecutable under its own inferred type (the bug-hunt finding);
   supplying the inferred params object always executes. Inferred `Params` and the
   runtime wire agree.
3. **Aggregates & the measure are typed**; the head's result type reflects the
   aggregate; the creation quarantine holds (no minting term in a head —
   unrepresentable in the type, permanent law); the measure's legal positions are
   boundary-only (one side of an order comparison, never a head/binding).
4. **Recursion is typed within the fence**: `idb`/`p.rec` predicates and
   self-recursion expressible; the safety roster (measure-in-recursive-head and
   kin) unrepresentable; mutual recursion's self-recursion-only cut is a type-level
   boundary. A fold over a *finished higher stratum* (recipe 25) is expressible;
   aggregation *through* a cycle is not.
5. **The lowered IR is accepted by the engine `prepare` for every construct** — the
   surface lowers to the same plain-data IR the Rust surfaces emit (IR bijection).
   Surface + types change; IR bytes do NOT.

## Work

1. Rewrite the query builders on S1/S2's domain-labeled schema types. Every runtime
   type-check a static type can carry moves into the type; every cast in the query
   lowering is eliminated. Keep the `pointIn`/`covers` op-aware literal tagging (the
   bug-hunt fix) AND make it a type-level guarantee.
2. Make the prepared-query type carry its inferred `Params` and result-row types so
   `db.execute(prepared, params, results)` (or the SDK's shape) is fully typed:
   wrong params → type error; results typed by the head. Vars are string-named and
   join by reuse; domain from S1/S2. Value-shaped, hover-first — no type-level
   string parsing.
3. Preserve the IR bijection: the lowering must still be accepted by engine
   `prepare` for every construct (the standing pin). A change that would alter
   emitted IR is a semantics change, out of scope — stop and flag.

## Passing criteria (scoped — whole-SDK green is S4's job)

- **Compile-must-PASS**: a multi-atom domain-equal join; a recursive closure and a
  finished-stratum aggregate fold; an aggregate head typed by its op;
  `covers(span(...), t)` (legal interval-left `pointIn`); `db.execute(prepared,
  <inferred params>, results)` with no cast; the result row typed by the head.
- **Compile-must-FAIL** (`// @ts-expect-error`, real): joining a `HolderId`-domain
  var to an `AccountId`-domain field; an interval-vs-scalar comparison under a
  non-`pointIn` op; a minting/arithmetic term in a head; a param at the wrong type;
  a results shape mismatched to the head.
- The IR-bijection/prepare-acceptance pins stay green; the unused-param query
  executes under its inferred type. `test/query.test.ts` green.
- `pnpm exec tsc --noEmit` green FOR THIS SCOPE (statement files may be transiently
  red — S2 concurrent; note it); biome clean on the touched files; zero casts in
  the query lowering.
- Report `breaks` = the query/program API shape S4 consumes. Commit deferred to the
  Land phase.
