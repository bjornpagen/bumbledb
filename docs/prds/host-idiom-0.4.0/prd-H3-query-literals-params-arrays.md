# PRD-H3 — Query literals, params & membership arrays: `oneOf` dies

Wave H · Repo: bumbledb `ts/` · depends on: H1 · runs concurrent with H2, H5
(this PRD owns `ts/src/query/atom.ts`, `query/lower.ts`'s literal paths,
`query/scope.ts` params, `relation.ts` selections; do not touch
`marshal.ts`, `closed.ts`)

## Objective

Handle names become the query surface's literal vocabulary, and set
membership is a plain array. `r.match(Certificate, { kind: "DirectPass" })`;
`r.match(member, { kind: ["Practice", "Review"] })`;
`on(program.where({ kind: "hierarchy_program" }), "id")`; params anchored at
closed fields take the union. `oneOf()` is deleted — arrays are the ONE
membership spelling (drizzle law + canonical utterance).

## Work

1. **Match/not literals** (`query/atom.ts::BindingInput`, ~lines 189–194):
   for a closed field, the literal position takes `Infer<F>` — which is now
   the handle union via H1 — and ADDITIONALLY `readonly Infer<F>[]` for
   membership. Arrays are membership for EVERY literal-capable field kind
   or closed-only? RULING: closed-only in this packet (ordinary u64/str
   membership already has its spelling through `in ?param` sets; widening
   arrays to all kinds is a separate future taste call — note it in the
   module doc, do not do it).
2. **Selections** (`relation.ts::SelectionInput` ~line 139 and the closed
   `.where` input from K1): replace the `OneOf` wrapper arm with
   `readonly Infer<F>[]`; DELETE `oneOf` — the constructor, its type, its
   export, every probe spelling it (grep `oneOf` → zero in `ts/src`; test
   spellings rewrite to arrays).
3. **Lowering** (`query/lower.ts::taggedHandleId`, ~lines 1313–1327): input
   flips from verified-bigint to verified-NAME (translate through the
   descriptor's roster; unknown name throws the H2-style pointed error);
   output UNCHANGED — still `{ kind: "u64", value: id }`; "queries cross
   ids, never handle names" stays true on the wire. Array membership lowers
   to the existing set/word-set form the engine already accepts for folded
   literal sets — read how `oneOf` lowered and produce the identical IR
   (the wire program for an array must be byte-identical to the old
   `oneOf` program: pin one lowering golden proving it).
4. **Params** (`query/scope.ts::ParamValueAt` ~line 295 + the execute-side
   `wireValue` translation): a param anchored at a closed field types as
   the union and translates name→id at execute; the anchoring descriptor is
   already threaded for domain typing — reuse it.
5. **`eq`/`ne` literal rhs** (`query/atom.ts` ~lines 548–561): closed-bound
   var against a handle literal takes the union.
6. **Probes** (intrinsic):
   - compile-PASS: literal, array, param, eq-rhs — each at the union;
     cross-vocabulary literal on the wrong relation compile-FAILS (real);
     `0n` in any of these positions compile-FAILS (bigint is gone from the
     closed surface);
   - lowering goldens: name literal → same IR as the old bigint spelling;
     array → same IR as the old `oneOf` spelling (byte-compared);
   - runtime: a prepared query with a name param returns the same rows as
     its 0.3.0 bigint twin over the same store.

## Technical direction

- The engine IR and the napi contract are UNTOUCHED — every translation is
  SDK-side, pre-wire. Any `ts/crate` diff is a scope violation.
- `taggedHandleId`'s roster verification is the single verification point —
  do not duplicate the check per call site.
- Zero casts; runtime twins for the new type claims (the array-membership
  arm's runtime is the folded set lowering — the golden is the twin).

## Passing criteria

- All probes + both lowering goldens green; `grep -rn "oneOf" ts/src` →
  zero; `ts/test` spellings rewritten (grep shows only historical mentions
  in comments if any, preferably none).
- No diff outside the four named files + probes.
- `tsc --noEmit` green for the touched modules + probes in isolation. Push
  per the wave's commit discipline.
