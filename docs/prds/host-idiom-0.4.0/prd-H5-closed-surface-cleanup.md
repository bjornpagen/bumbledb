# PRD-H5 — The closed surface cleanup: `match`, `fromId`, the constants die

Wave H · Repo: bumbledb `ts/` · depends on: H1 · runs concurrent with H2, H3
(this PRD owns `ts/src/closed.ts`; do not touch marshal or query files)

## Objective

Delete the compensation machinery whose cause H1–H4 removed. Owner rulings,
verbatim: the match operator is hard-removed; the handle constants die
ENTIRELY (no `Kind.DirectPass`, not even as a string constant — the literal
is the one spelling); `fromId` dies. The closed value keeps exactly:
`.id`, `.where()`, `.axioms`, `.name`, `.columns`.

## Work

1. **Delete from `ts/src/closed.ts`**:
   - the handle-constant mint: the `{ readonly [H in Handles]: bigint }`
     intersection arm of `Closed<…>` (~line 228) and the
     `Object.defineProperty(out, handle, { value: BigInt(index) … })` loop
     (~line 348);
   - `ClosedMatchBare` / `ClosedMatchPayload` (~lines 201–213) and the
     `match` implementation (~lines 548–554);
   - `fromId` (declaration ~line 148, implementation ~lines 519–521);
   - `reservedHandleNames` and its construction-time check — with no
     handle-named properties minted, handles are pure data and NO name is
     reserved (a vocabulary may now legally contain handles named `match`,
     `where`, `id` — the axioms record and the roster are their own
     namespaces; pin one such vocabulary as a probe).
2. **The surviving surface**, re-verified honest: `.id`
   (`ClosedIdField<Handles>`, H1), `.where()` (K1's ψ face — its selection
   input now speaks names/arrays via H3's `SelectionInput`; verify the
   closed `.where` path reads the same input type and needs no local
   change), `.axioms` (string-keyed, already the right texture), `.name`,
   `.columns`. Everything else on the value is a defect.
3. **Type-lie sweep**: the `Closed` type must claim EXACTLY the runtime
   properties (`Object.keys` probe against the type's key union — the
   pattern the 0.2.0 review forced; re-pin it for the slimmed shape).
4. **Probes** (intrinsic):
   - compile-FAIL (real): `Kind.DirectPass` (property gone),
     `Kind.match(…)`, `Kind.fromId(…)`;
   - the handles-named-like-methods vocabulary
     (`closed("Weird", ["match", "where", "id"])`) constructs, its axioms
     record keys correctly, its roster round-trips through a store (this
     probe needs H2's bijection — mark it `// needs H2` and let H7 order
     the suite; do not shim);
   - both tiers (bare + payload) still mint, seal, and `.where()` on the
     payload tier still compiles against H3's array selections.

## Technical direction

- This is a deletion PRD — net-negative lines. If a change here wants to
  ADD machinery, it is wrong.
- The 3-arg `closed` and the sealed `columns` carrier are K6/0.2.0-review
  law — untouched.
- Zero casts; no underscore params introduced by the surgery.

## Passing criteria

- `grep -n "fromId\|ClosedMatch\|reservedHandleNames" ts/src` → zero.
- The `Object.keys`-vs-type probe green; all compile-FAIL directives real.
- `closed.ts` diff is net-negative in lines (state the count in the commit
  body).
- `tsc --noEmit` green for `closed.ts` + its probes in isolation. Push per
  the wave's commit discipline.
