# PRD-06 — Harden: the query surface

Repo: bumbledb · depends on: 04 · blocks: 07 · parallel with 05

## Objective

Make the Datalog-as-values query surface end-to-end typesafe: from the schema's
branded fields, through atom binding, joins, comparisons, param sets, aggregates,
and stratified recursion, to a prepared query whose parameter object type and
result row type are both INFERRED — no casts, no `any`, illegal queries
unwritable, and the param/result types the host actually sees are exactly what the
engine will accept and return. This closes the query-side typesafety holes the bug
hunt found (the `covers()` literal-interval lowering, the unused-param
unexecutable query, brand-equal joins).

## Scope (files)

`ts/src/query/{atom,lower,predicate,run,scope,select}.ts`, `ts/src/lower.ts`, and
`ts/test/{query,sdk-kernel-probes,c-sdk-2-probe}.test.ts`.

## Invariants to achieve (each becomes a probe)

1. **Variables are typed by the field domain they bind.** A variable bound to a
   `HolderId` field is a `HolderId`-domain variable; joining it against an
   `AccountId` field is a type error (brand-equal joins). A comparison
   `field == literal` type-checks the literal against the field's element domain;
   a cross-type comparison (interval-vs-scalar, except the legal `pointIn`/`covers`
   interval-left form) is a type error — the exact hole the bug hunt fixed, now a
   type-level guarantee, not just a lowering fix.
2. **Params are typed by their declared domain AND their usage.** `?root`'s type is
   its binding field's domain; the prepared query's parameter object type is
   inferred to be EXACTLY the params the rules use — a declared-but-unused param
   must not make the query unexecutable under its own inferred type (the bug-hunt
   finding), and supplying the inferred params object must always execute. The
   inferred `Params` type and the runtime wire agree.
3. **Aggregates and the measure are typed** (`Sum`/`Min`/`Max`/`Count`/
   `CountDistinct`/`ArgMax`/`ArgMin`/`Pack`, `Duration`): the head term's result
   type reflects the aggregate; the creation-quarantine holds (no minting term in a
   head — unrepresentable in the type, permanent law); the measure's legal
   positions are boundary-only (one side of an order comparison, never a head or a
   binding).
4. **Recursion is typed within the fence**: `Idb` predicates and self-recursion are
   expressible; the safety roster (measure-in-recursive-head and kin) is
   unrepresentable; mutual recursion's current cut (self-recursion only) is a
   type-level boundary, not a runtime surprise.
5. **The lowered IR is accepted by the engine `prepare` for every construct** — the
   query surface lowers to the same plain-data IR the Rust surfaces emit (the IR
   bijection pin). Hardening changes the SURFACE and the TYPES, never the emitted
   IR bytes.

## Work

1. Audit the query builders against invariants 1–5. Every runtime type-check a
   static type could carry moves into the type (delete the guard). Every cast in
   the query lowering is eliminated. The `pointIn`/`covers` op-aware literal tagging
   (bug-hunt fix) becomes a type-level guarantee AND stays correct at lowering.
2. Make the prepared-query type carry its inferred `Params` and result-row types so
   `db.execute(prepared, params, results)` (or the SDK's current shape) is fully
   typed: wrong params shape → type error; the results object typed by the head.
3. Hard-break the query surface signatures as needed for full inference. Keep it
   value-shaped and hover-first (no type-level string parsing). Brand-equal joins,
   typed params, typed aggregates — all through generics over the schema.
4. Preserve the IR bijection: the lowering output must still be accepted by the
   engine `prepare` for every construct (the standing pin). If a hardening change
   would alter emitted IR, it is a semantics change, not a surface change — stop
   and reconsider (out of scope; this PRD is types + surface only).

## Technical direction

- Doctrine: the query is where end-to-end typesafety is most visible — a mistake in
  a join, a param, or a comparison should be a red squiggle at authoring, with the
  fix obvious from the hover. Optimize for the cursor.
- Do not regress the bug-hunt fixes; turn each into a stronger type-level
  guarantee where it was a lowering-only fix.
- `// @ts-expect-error` only in `test/*`.

## Passing criteria

- **Compile-must-PASS**: a multi-atom query with a brand-equal join; a recursive
  closure (`reach(c) | Parent(child: c, parent: m), reach(0: m)`); an aggregate head
  typed by its op; `covers(span(...), t)` (the legal interval-left `pointIn`);
  `db.execute(prepared, <exactly the inferred params>, results)` type-checks with no
  cast; the result row typed by the head projection.
- **Compile-must-FAIL** (`// @ts-expect-error`): joining `HolderId` var to an
  `AccountId` field; an interval-vs-scalar comparison under a non-`pointIn` op; a
  minting/arithmetic term in a rule head; a param supplied at the wrong type; a
  results shape mismatched to the head.
- The IR-bijection / prepare-acceptance tests stay green (surface + types changed,
  IR unchanged). The unused-param query executes under its inferred `{}`/params
  type (bug-hunt regression stays fixed, now type-guaranteed).
- `tsc --noEmit` green; `biome check ts/` clean; `node --test` green for the query
  suites; zero casts in the query lowering.
- Commit in the repo's voice; push.
