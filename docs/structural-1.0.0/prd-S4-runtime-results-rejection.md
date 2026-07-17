# PRD-S4 — The `Db` runtime, results & rejection (+ restore whole-SDK green)

Wave 1 · Repo: bumbledb `ts/` · depends on: S2, S3 · blocks: S5 · the integration + restore-green PRD

## Objective

Rebuild the `Db` runtime, the marshal boundary, results, the rejection-as-data
wire, and exhume on the structural kernel, integrate S1+S2+S3, and re-establish
whole-SDK green. After this PRD the SDK's public API is the frozen structural
surface: bare values in and out, domains flowing from the schema, cast-free
end to end (the marshal brand-assertion is DELETED — the elegance dividend).

## Scope (files)

`ts/src/{db,marshal,native,face,exhume,index}.ts` and the runtime/rejection/exhume
test suites (`db`, `ffi`, `consumer-patterns`, `read-scope-leak`,
`selected-source-containment`, `open-ledger-multikey-get`, `exhume`,
`native-loader`, `bughunt`, `f5-revision-dance`). `native.ts` FFI *typing* only —
the loader mechanics (platform-package resolution) are already correct from the
0.1.0 arch-split; do not regress them.

## Invariants to achieve (each becomes a probe)

1. **Transactions typed by the schema, bare values.** `db.write(tx => …)` gives a
   `tx` whose `insert`/`delete`/`get`/`contains` are typed to the schema's
   relations with **bare structural facts** (`bigint`/`string`/`{start,end}`/…, no
   brands); a schema-A fact into a schema-B `Db` is a type error; a `.fresh` id
   omitted returns the minted `bigint`, usable in the same delta. The
   async-callback thenable refusal stays (the sanctioned boundary parse TS can't
   express). The reader-scope-leak fix (paired open/close on every exit) stays.
   NOTE the conscious non-goal (design ruling 3): a raw `bigint` in the wrong
   `insert` field is NOT a compile error (structural) — the engine's containment
   judgment catches it at commit.
2. **`get` typed through any declared key** — the primary-key `get(R, key)` and the
   declared-key `get(R, keyStatement, key)` overloads both typed by the key
   statement's projection; `undefined` on a miss in the type.
3. **Results typed by the query head, bare.** The results surface yields rows typed
   to the prepared query's head projection at bare structural types (`bigint`/…),
   no `unknown`, no host cast. The domain is available via the schema for a host
   that wants it; results themselves are bare (elegance). If a `FromAnswers`-shaped
   typed decode is warranted, land it; else raw results stay typed-bare.
4. **Rejection is fully typed data.** `db.write`'s failure yields typed
   `Violation`s: statement identity (`===`-matchable to the statement constant),
   kind, `canonical === renderStatement(statement)` (the pin), `direction`, and for
   `mirrors` the `orientation` slot identity (the bug-hunt fix — 4 wire states, not
   1). Offending facts are typed BARE facts. The lone-surrogate refusal at the
   marshal seam stays.
5. **The marshal boundary is pure structural both ways — and CAST-FREE.** `bigint`
   u64/i64 (incl. `i64::MIN`/`u64::MAX`), `bytes<N>` width, `interval` incl. ray
   sentinel, `str` intern round-trip, empty strings, unicode — each crosses as its
   bare structural type; refuse ill-formed shapes typed on the way in. **Delete the
   old brand-by-assertion cast** — values are bare, so nothing is asserted on the
   way out. `native.ts` stays the sole FFI boundary, its typed surface exhaustive.
6. **Exhume typed** — `Db.exhume(path)` (theory-less self-describing-stores read)
   exposes scans by relation name with typed bare rows; `DescriptorMissing` /
   legacy-store surfaces as a typed outcome. Part of the frozen surface.

## Work

1. Rewrite `db.ts`/`marshal.ts`/`face.ts`/`exhume.ts`/`native.ts`(typing) on the
   structural kernel. Eliminate ALL surface casts (the marshal brand-assertion is
   deleted; keep only the sanctioned boundary PARSES — async thenable, lone
   surrogate — which are refusals, not casts). Move runtime type-guards into the
   types where expressible.
2. **Integrate** S1/S2/S3: thread bare values + schema domains end-to-end —
   `Db<typeof S>` → typed `tx` → typed `prepare(query)` → typed
   `execute(params, results)` → typed bare rows and typed rejections. Reconcile any
   transient redness the concurrent S2/S3 edits left.
3. **Restore whole-SDK green** — this is S4's own passing criterion (the packet
   allows red BETWEEN PRDs; it ends here).

## Technical direction

- The store is the proof carrier — but with bare values there is now NO cast on the
  way out (the historical "one sanctioned marshal cast" is gone). Product code is
  cast-free, period; document that in the marshal module header.
- Rejection-as-data must be complete (every `Violation` variant, every citation
  field, the canonical spelling pin) so a consumer can `===`-match statements and
  read offending bare facts.
- `@ts-expect-error` only in `test/*`, each real.

## Passing criteria (WHOLE-SDK GREEN — the restore-green PRD)

- **Compile-must-PASS**: typed `db.write`/`read` closures with bare facts; both
  `get` overloads; a prepared query executed with typed params yielding typed bare
  rows; a `Violation` whose `statement` `===`-matches its constant and whose facts
  are bare-typed; `Db.exhume` yielding typed scans.
- **Compile-must-FAIL** (`// @ts-expect-error`, real): schema-A fact into
  schema-B `Db`; an async `db.write` callback (type error OR the runtime thenable
  refusal — pin whichever the design lands); a result row consumed at the wrong
  structural shape; a marshaled value of the wrong shape.
- Runtime: ALL of `db`, `ffi`, `consumer-patterns`, `read-scope-leak`,
  `selected-source-containment`, `open-ledger-multikey-get`, `exhume`,
  `native-loader`, `render-golden`, `fingerprint` green; the "fresh mint across a
  rejected commit" consumer test passes (engine fresh law + PRD-A).
- **Whole SDK green together**: `pnpm run build` (cargo bridge + tsc + both package
  trees, loadable `.node`) + `pnpm exec tsc --noEmit` + `pnpm exec biome check .` +
  `node --test $(find test -name '*.test.ts')` 100% all pass on ONE tree.
  Product surface is CAST-FREE (grep confirms; only `test/*` `@ts-expect-error`).
- Report the complete public-API break list (the owner's changelog for the Wave-3
  republish). Commit deferred to the Land phase.
