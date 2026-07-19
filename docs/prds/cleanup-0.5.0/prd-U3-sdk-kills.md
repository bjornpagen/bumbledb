# PRD-U3 — The SDK kills + the two ratified adds

Wave 1 · Repo: bumbledb `ts/` (src, crate, scripts, npm, runbook) ·
depends on: — · executes rulings 2 (kill class), 5 (`covers` dies),
9 (`SameArity` twin)

## Objective

Execute every SDK-census KILL (census `cleanup-sdk.md`; the ts/ tree is
unchanged since the census — still re-verify sites), delete the `covers`
spelling, and give `SameArity` its runtime twin. SDK law throughout: zero
casts in `ts/src`; `@ts-expect-error` only in tests, each real; every type
claim keeps (or here, gains) its runtime twin.

## Work — the kills, grouped by representation move

**Dead surface (delete outright):**
1. The `R.fields.x` addressing mode — `FieldRef`/`FieldRefs`, `refsComplete`,
   the refs minting loop, `Relation.fields`, both index exports, the false
   doc sentence (`relation.ts`, `index.ts`). Nothing consumes it — verified
   across src/test/README/COOKBOOK. One addressing mode remains.
2. `covers()` (**ruling 5**): delete the constructor and its exports;
   `pointIn(t, w)` is the one spelling; the name belongs to `ALLEN.covers`
   alone. Sweep recipes/docs that spelled interval-first prose through it.

**One meaning, one home (dedupe):**
3. The verbatim helper clones: `literalShapeError` ×2, the interval-shape
   predicate ×3 (`fields.ts`/`marshal.ts`/`query/lower.ts`), `bridged()` ×2,
   `ALLEN_ALL_BITS` ×2 — each parked in its one obvious owner and imported.
4. `Db.get`'s duplicated selector dispatch: forward raw args to `snap.get`
   under the overloads; the one dispatch site judges; one error voice.
5. The closed-member discriminant, 14 sites × 3 spellings → ONE exported
   `isClosedMember(member): member is AnyClosed`.
6. The sealed-shape readers ×4 (+2 inline re-derivations) → one
   `sealedFieldsOf(owner)` (ordered, id-first for closed) + one derived
   `sealedFieldOf(owner, name)`; kills `faceField`'s silent missing-`id`
   divergence by construction.
7. The roster reader ×5 → one exported `rosterOf(field)`.
8. The stale witnessed-write doc: rewrite `Db.writeWitnessed`'s interface doc
   to state the cap + livelock refusal (the deeper doc already argues it);
   the "unbounded / any cap would be an invented limit" sentence dies.
9. `groundRow` + the `axioms: undefined` arm inside `mintClosed`: bare tier
   passes `columns:{} / axioms:{}`, the impossible state stops being spelled.

**The crate bridge (`ts/crate`):**
10. **`wire_tags!`** — one declarative macro per mirrored enum emitting
    (a) exhaustive `tag(&E)` (a new core variant breaks compile),
    (b) `parse(str)` for the `*_in` fns, (c) a golden `tags.json` that a TS
    test asserts `native.ts`'s unions against. Kills the ~90 unprotected
    arms, the AggOp/HeadOp duplicate table, and the in/out ValueType twin.
    Payload marshal stays by hand (the u64-as-bigint law).
11. The synthetic-id law ×3 → one `sealed_fields()` accessor on the engine's
    `SchemaDescriptor` (engine-side change, small and sanctioned by the
    census sketch); manifest renderer, bridge, and (via manifest) the SDK
    consume it. This is §6's Rust half — with kill 6 it is the unified
    descriptor interface.
12. lib.rs boilerplate: the reply-unwrap `reply!` macro (~10 triplets), the
    Outcome `ToNapiValue` macro (×5 near-clones), the tx_insert/delete/
    contains constructor-parameterized helper, the `close<T>` take-or-throw
    helper, the `throw_engine` chain, the `"and" | "or"` arm split.

**Packaging / runbook paper cuts:**
13. Dev-twin manifest derived from the committed publish manifest (rewrite
    only name/os/cpu/description) — the hand-written literal has already
    drifted (`scripts/build.ts`).
14. PUBLISHING.md: delete the one-liner (stale absolute worktree path), the
    `--access public` flag duplication; `package.json`: drop the legacy
    `module`/`main`/`types` fields (exports-only); unify the test-glob
    spelling (one `test` script; runbook and CI invoke `pnpm test`).

## Work — the SameArity twin (ruling 9)

`SameArity` (`face.ts:195`) is the one type wall with no construction-time
runtime seat: `assertRosterAgreement` and `computeClasses` both skip positions
where the other side's projection is `undefined`, so an untyped caller's
arity-mismatched containment silently truncates until `Db.create`. Add the
runtime arity check at statement construction (the `assertRosterAgreement`
site is the natural home), with the house error voice (facts, never row ids)
and a runtime test through the untyped path. The type tier is untouched.

## Explicitly NOT in scope

The 18 OWNER-RULING-NEEDED census items not named by a ruling (the bijection
write half, the kind-switch triple, `ParamUse`/`ParamEntry`, the src/dist
imports fork, the error-lane catch-alls, the shared PIN constant, …) stand
as-is; the census remains their register. The limit clause governs every
kill here as in U2.

## Passing criteria

- All kills landed or aborted-with-reason; `covers` absent from `ts/src`
  exports; `grep -rn "covers" ts/src` hits only `ALLEN`.
- The fork-pinning tests green UNCHANGED except where a killed surface's own
  pins die with it (only 1.1's dead mode qualifies): native-loader,
  build-platform, render-golden, notation-corpus, law-scale,
  marshal-bijection, the closed-arm suite, types/type-kernel/law-typing.
- New pins exist and are green: the `tags.json` golden asserted from TS; the
  SameArity runtime-twin test; a dev-twin-manifest field-inheritance check.
- Zero casts in `ts/src` (unchanged); the two near-vacuous seams question
  (census 2.8) NOT acted on — no sanctioned cast introduced.
- Full SDK gate green: `pnpm run build`, `tsc --noEmit`, `biome check .`,
  `node --test`, `cargo test` in `ts/crate`; engine `scripts/check.sh` green
  (kill 11 touches the engine crate).
