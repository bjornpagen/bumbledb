# PRD-K6 — Closed ergonomics: `Kind.match` + the 3-arg `closed`

Wave K · Repo: bumbledb `ts/` · depends on: — · blocks K7 · hard break (the
curried tier-2 spelling dies)

## Objective

Two proven-sound closed-vocabulary improvements, and one proven-unsound idea
explicitly rejected:

1. **`Kind.match(value, arms)`** — exhaustive matching over handles WITHOUT
   literal types or brands: the handle-name union already lives on
   `Closed<Name, Handles, Cols>`; arms typed `{ [H in Handles]: … }` give
   missing-arm and extra-arm compile errors, and the payload tier's arm
   receives the typed axiom row.
2. **The 3-arg `closed(name, cols, axioms)`** — the curry was never an
   inference limitation (proven: `Cols` infers from arg 2, `Handles` from arg
   3's keys via reverse mapped-type inference, rows contextually checked,
   per-property error locality). The curried tier-2 spelling
   `closed(name, cols)(axioms)` is DELETED — canonical utterance.
3. **Rejected — do not build**: literal-id handle types (`Kind.A: 0n`). The
   bare tier admits them but the payload tier cannot (axiom-record keys carry
   no order), and literal-typed readback fights bare structural values.
   Handles stay plain `bigint` constants.

## Work

1. **`ts/src/closed.ts`**:
   - Mint `match` on the closed value (both tiers), reserved-name checked
     (add `"match"` to `reservedHandleNames`, like K1 does `"where"`).
     Signature: bare tier `match<T>(id: bigint, arms: { [H in Handles]: () => T }): T`;
     payload tier `match<T>(id: bigint, arms: { [H in Handles]: (row: AxiomRow<Cols>) => T }): T`.
     Exhaustiveness = the mapped type (missing arm: missing-property error;
     extra arm: excess-property error). Runtime: resolve via the existing
     `fromId` + `axioms` (≈4 lines); an id outside the roster THROWS with a
     pointed message (the type admits any bigint — the runtime twin must
     refuse dishonest input, not misdispatch).
   - Add the 3-arg overload; DELETE the curried tier-2 form (its overload and
     implementation arm). Bare tier (`closed(name, [handles])`) unchanged.
2. **Probes** (intrinsic):
   - `match` exhaustive both directions (compile-FAIL probes, real): missing
     arm; extra arm; payload arm receives the typed row (`Equal`-probe on the
     row type); bare arm takes no row.
   - runtime: dispatch correctness per handle; out-of-roster id throws.
   - 3-arg inference: `Cols` from arg 2, handle set from arg 3, per-property
     failure locality (wrong value type errors on the property — pin one).
   - The curried call `closed("Kind", cols)(axioms)` is a compile error
     (real directive).
   - Reserved names: a vocabulary with a handle named `match` (or `where`) is
     a construction-time error.

## Technical direction

- `match` and the K1 `where` mints follow the same own-property
  `Object.defineProperty` discipline as the handle constants (the `__proto__`
  law).
- Zero casts — the 0.2.0 review already forced honestly-typed builders in
  closed.ts; extend them, do not reintroduce `unknown` returns behind
  overloads.
- The deleted curried form will break K7's not-yet-rewritten recipes and any
  test spelling it — expected mid-wave red; do not shim.

## Passing criteria

- All probes green; compile-fail directives real; the curried spelling is
  uncompilable and absent from `ts/src`.
- Zero casts, zero `unknown`-laundering in `closed.ts` (grep + the existing
  lie-sweep probes stay green).
- `tsc --noEmit` green for `closed.ts` + probes. Push per the wave's commit
  discipline.
