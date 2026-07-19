# The host-idiom-0.4.0 packet — DB idioms become modern TS idioms; the match operator dies

The nitpick era's first packet, ratified by the owner 2026-07-19. One
philosophy executed to the end, then the consumer cut over. Ships as
**0.4.0** — a deliberate backwards-incompatible hard break over the published
law-typed `0.3.0`. Per house convention this directory is DELETED once
shipped (it lives at the release tag).

## The drizzle law (the packet's philosophy — record it, then obey it)

**The SDK's job at the host surface is translation, not abstraction: every
database idiom arrives as the modern TypeScript idiom for that concept, and
the SDK never invents an operator where the language already has one.**
Enums arrive as string-literal unions and dispatch is native `switch`
narrowing with `satisfies never` exhaustiveness; set membership is an array;
rows are records of meaningful values. A combinator that replaces a native
control-flow form is a defect (the "Effect monstrosity" failure mode).
`Kind.match` was exactly that — built as an imitation of Rust's `match`
because handles were opaque bigints; the imitation dies with its cause.

## Ratified rulings (no PRD re-litigates these)

1. **Closed handles are string-literal unions at the TS surface.** A
   closed-referencing column types and holds `"DirectPass" | "JudgedPass" |
   "Failed"` — in `Fact`, inserts, query match records, select rows, params,
   selections, and violation offending facts. The ENGINE keeps u64 row ids;
   the marshal owns the total, static bijection (sealed roster, declaration
   order, ≤256 rows). The wire, manifest, and fingerprint are untouched —
   schema selection literals already cross as `{kind:"handle", handle}` and
   query literals still cross as tagged u64 ids after SDK-side translation.
   Zero fingerprint pins move in this packet; a moved pin is a defect.
2. **`Kind.match` is hard-removed. `fromId` dies. The handle constants die
   entirely** (owner ruling: no `Kind.DirectPass`, not even as a string
   constant — the literal `"DirectPass"` is the ONE spelling, canonical
   utterance). What survives on a closed value: `.id`, `.where()`,
   `.axioms`, `.name`, `.columns`. With no handle-named properties minted,
   `reservedHandleNames` dies too — handles are data, not properties.
3. **`oneOf()` dies for plain arrays.** Set membership in selections and
   match records is spelled `["A", "B"]`. Same law, next instance.
4. **Closed fields exit the orderable/foldable set.** `lt`/`sum` over a
   vocabulary id was always a declaration-order accident (the data-model doc
   says so); with strings it would be visibly absurd — it becomes
   unspellable, type-tier and lowering-tier both.
5. **Insert gets STRICTER, deliberately**: a wrong string is a compile error
   and a marshal refusal — strictly better than today's any-bigint-compiles
   / commit-time-violation. A decoded id outside the roster (possible only
   in a store whose closed-typed column was never pinned by its containment
   law) is a pointed runtime error, never a silent latin-1-style fallback.
6. **What is deliberately NOT changed** (the places we already beat the
   drizzle texture — protect them): violations-as-data (`WriteResult`
   unions), the typed `axioms` readback, synchronous `execute` with params
   typed by use, bigint for genuine u64/i64 values, the class map and the
   whole law-typing engine, every statement form, the Rust macro (its host
   enum IS this idiom, natively — ruling 9 of hardening-0.3.0 extended to
   values).
7. **No shims, hard break, rip to the end state.** The tree may be red
   between PRDs; H7 restores whole-SDK green; PRDs are work organization,
   not atomic checkpoints. No smoke/e2e PRDs (humans test); no migration
   PRDs (humans migrate; primer's dev stores are unaffected anyway — this
   packet has zero store surface). No 1.0.0 anywhere; publishes and tags
   are owner ceremony.

## The dependency graph (concurrency-explicit)

```
H1 the precise roster type                 (first — everything reads Infer)
   → { H2 marshal bijection  ∥  H3 query literals & params  ∥  H5 closed surface cleanup }
        (disjoint files: marshal.ts+db.ts ∥ query/atom+lower+scope+relation ∥ closed.ts)
   → H4 answer rows arrive named + the orderable ban        (after H3 — same query files)
   → H6 cookbook + READMEs + docs                           (after H1–H5 — documents the end state)
   → H7 whole-SDK green                                     (after all H)
   → P1 primer cutover                                      (new worktree + PR, after H7)
   → V1 0.4.0 staging                                       (last; owner publishes)
```

| PRD | Title | Depends on |
| --- | --- | --- |
| H1 | The precise roster type: `ClosedIdField<Handles>` and the `Infer` arm | — |
| H2 | The marshal bijection: writes, fact decode, violations | H1 |
| H3 | Query literals, params & membership arrays (`oneOf` dies) | H1 |
| H4 | Answer rows arrive named + the orderable ban | H3 |
| H5 | The closed surface cleanup: `match`, `fromId`, constants die | H1 |
| H6 | Cookbook, READMEs, docs — the drizzle law recorded | H1–H5 |
| H7 | Whole-SDK green restored | all H |
| P1 | Primer: the 0.4.0 cutover (new worktree + PR) | H7 |
| V1 | 0.4.0 release staging (owner publishes) | everything |

## The gates (run in full only at H7 and after)

- SDK (`ts/`): `pnpm run build`; `pnpm exec tsc --noEmit`;
  `pnpm exec biome check .`; `node --test $(find test -name '*.test.ts')` —
  100% green. `test/fixtures/*.ts` are spawned-child helpers, never tests.
- Engine: `scripts/check.sh` + `scripts/lean.sh` must remain green but are
  expected UNTOUCHED — an engine diff in this packet is a scope violation.
- The zero-store-surface proof: the T5 cookbook-fingerprint fixture and the
  CrossHost lock constants are byte-identical before and after the packet.
- Primer: `pnpm typecheck`, `pnpm knip`, the graph-builder store/driver test
  files green.
- Mandates: zero casts in `ts/src`; `@ts-expect-error` only in tests, each
  real; every type claim carries its runtime twin; no underscore-prefixed
  functions or dead underscore params.
