# PRD-07 — Harden: the `Db` runtime, results & rejection wire

Repo: bumbledb · depends on: 05, 06 · blocks: 08

## Objective

Make the `Db` runtime, the marshaling boundary, the results surface, the
rejection-as-data wire, and the exhume surface end-to-end typesafe, and
re-establish whole-SDK green — this is the integration PRD that closes the loop
from typed schema (04/05) and typed queries (06) to typed writes, typed reads,
typed results, and typed rejections. After this PRD the SDK's public API is frozen
for 1.0.0: everything the host touches is typed, cast-free at the surface, and
hover-clean.

## Scope (files)

`ts/src/db.ts`, `ts/src/marshal.ts`, `ts/src/native.ts` (typed FFI surface only —
the loader mechanics are PRD-03), `ts/src/face.ts`, `ts/src/exhume.ts`,
`ts/src/index.ts`, and the runtime/rejection/exhume test suites
(`db`, `ffi`, `consumer-patterns`, `read-scope-leak`, `selected-source-containment`,
`open-ledger-multikey-get`, `exhume`).

## Invariants to achieve (each becomes a probe)

1. **Transactions are typed by the schema.** `db.write(tx => …)` gives a `tx`
   whose `insert`/`delete`/`get`/`contains` are typed to the schema's relations and
   branded facts; a schema-A fact into a schema-B `Db` is a type error; a fresh-id
   omit returns the branded minted id usable in the same delta. An async build
   callback is refused (the landed thenable probe stays — the sanctioned boundary
   exception where TS cannot express it). The reader-scope-leak fix (paired
   open/close on every exit) stays.
2. **`get` is typed through any declared key.** The primary-key `get(R, key)` and
   the declared-key `get(R, keyStatement, key)` overloads (the multi-key OPEN-ledger
   row that FIRED) are both typed by the key statement's own projection;
   `undefined` on a miss is in the type.
3. **Results are typed by the query head.** The results surface (`Answers` and its
   accessors) yields rows typed to the prepared query's head projection with the
   right brands — no `unknown`, no per-field cast by the host. If a `FromAnswers`-
   shaped typed decode is the right elegance move, land it; if the census showed the
   real consumer never reached for it, it stays declined — but the raw results MUST
   still be typed (branded cells, typed columns), not `unknown`.
4. **Rejection is fully typed data.** `db.write`'s failure yields typed
   `Violation`s: the statement identity (`===`-matchable to the statement constant),
   the kind, the canonical spelling (`=== renderStatement(statement)` — the pin),
   the `direction`, and for `mirrors` the `orientation` slot identity (the bug-hunt
   fix — the 4 wire states, not the collapsed 1). Offending facts are typed branded
   facts. The lone-surrogate refusal at the marshal seam stays.
5. **The marshal boundary is schema-directed and typed both ways**: `bigint`
   u64/i64 (incl. `i64::MIN`/`u64::MAX`), `bytes<N>` width, `interval` incl. ray
   sentinel, `str` intern round-trip, empty strings, unicode — each crosses with its
   brand applied by assertion on the way out (the store is the proof carrier) and
   its shape refused typed on the way in. `native.ts` stays the SOLE FFI boundary;
   its typed surface is exhaustive over the bridge exports.
6. **Exhume is typed.** `exhume(path)` (self-describing-stores, opened theory-less)
   exposes scans by relation name with typed rows; the `DescriptorMissing` /
   legacy-store path surfaces as a typed outcome. The exhume surface is part of the
   frozen 1.0.0 API.

## Work

1. Audit `db.ts`/`marshal.ts`/`face.ts`/`exhume.ts` against invariants 1–6.
   Eliminate surface-level casts and `unknown` leaks; move runtime type-guards into
   the types where expressible (keep the sanctioned boundary parses: async-callback
   thenable probe, lone-surrogate refusal — TS cannot express those).
2. Integrate 05 (typed schema/statements) and 06 (typed queries) so the runtime
   threads their types end to end: `Db<typeof Ledger>` → typed `tx` → typed
   `prepare(query)` → typed `execute(params, results)` → typed rows and typed
   rejections. Hard-break `db.ts` signatures as needed.
3. Keep `native.ts` typing exhaustive and the single FFI boundary; PRD-03 owns HOW
   the `.node` loads, this PRD owns WHAT its typed surface is.
4. **Re-establish whole-SDK green** as this PRD's own closing criterion: after 04,
   05, 06, and this PRD, `tsc --noEmit`, `biome check ts/`, and the full
   `node --test` suite are green together. This is the "restore green" PRD for the
   SDK subsystem (the packet's rulings allow red BETWEEN PRDs; it ends here).

## Technical direction

- Doctrine: the store is the proof carrier — brands on the way out are applied by
  assertion because the engine already judged the facts (the one honest asymmetry:
  the SDK's types are the ergonomic shadow, the engine holds soundness). That
  assertion is the SANCTIONED cast at the marshal boundary and ONLY there; the rest
  of the surface is cast-free.
- Rejection-as-data is load-bearing for the downstream repair loop — its typing
  must be complete (every `Violation` variant, every citation field, the canonical
  spelling pin) so a consumer can `===`-match statements and read offending facts
  typed.
- `// @ts-expect-error` only in `test/*`.

## Passing criteria

- **Compile-must-PASS**: typed `db.write`/`read` closures; both `get` overloads;
  a prepared query executed with typed params yielding typed rows; a `Violation`
  whose `statement` `===`-matches its constant and whose facts are branded; `exhume`
  yielding typed scans.
- **Compile-must-FAIL** (`// @ts-expect-error`): schema-A fact into schema-B `Db`;
  an async `db.write` callback (must be a type error OR the runtime thenable refusal
  fires — keep whichever the design lands, and pin it); a result row consumed at the
  wrong brand; a marshaled value of the wrong shape.
- Runtime: `db`, `ffi`, `consumer-patterns`, `read-scope-leak`,
  `selected-source-containment`, `open-ledger-multikey-get`, `exhume`, and the
  render-golden + fingerprint pins ALL green. The pre-existing "fresh mint across a
  rejected commit" consumer test passes (engine fresh law + PRD-01).
- **Whole SDK green together**: `tsc --noEmit` + `biome check ts/` + full
  `node --test` all pass on one tree. Surface is cast-free except the documented
  marshal-boundary brand assertions.
- Commit(s) in the repo's voice; push.
