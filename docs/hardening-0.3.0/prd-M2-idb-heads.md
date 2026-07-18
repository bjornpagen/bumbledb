# PRD-M2 — Ordered-dense idb heads: `reach(m)` replaces `reach(0: m)`

Wave M · Repo: bumbledb (`crates/bumbledb-query` + notation docs) · depends
on: — · flag-day for the dense case; `i: v` survives ONLY for sparse/selection

## Objective

Predicate (idb) atoms currently require indexed bindings (`reach(0: m)`)
because the IR is nameless by law (predicate names never survive expansion;
`FieldId(i)` on an `Idb` atom addresses head position i) and field-name punning
has nothing to pun against. But an ORDERED DENSE form is grammatically free: in
a predicate atom, a bare ident today is only ever a refused pun — so `reach(m)`
/ `reach(m, a)` can mean ordered dense bindings `[(0,m),(1,a)]` with no
ambiguity. Adopt it as THE canonical spelling for dense in-order bindings;
keep `i: v` exclusively for sparse bindings and position selections
(`0 == Kind::Focus`, `0 in ?set`), which remain necessary.

## Context (verified)

- Namelessness: `crates/bumbledb-query/src/lib.rs` ~lines 1147–1152 (names
  never enter the IR/fingerprint); pun refusal on predicate atoms ~line 1243.
- Renderer: `crates/bumbledb/src/ir/render.rs` ~lines 526–540 renders idb
  bindings as `i: v`.
- The TS builder is ALREADY ordered-dense-only (`advanceIdb`,
  `ts/src/query/lower.ts` — "positions take variables"); it maps 1:1 to the
  new spelling and needs zero changes.
- Queries are never persisted or fingerprinted — no store surface anywhere
  near this change; only render goldens move.

## Work

1. **Macro parse** (`bumbledb-query/src/lib.rs`): in a predicate atom, a bare
   ident list is ordered dense binding — positions assigned left to right from
   0. Mixing bare idents and indexed bindings in one atom is a compile error
   (pointed message). An explicitly indexed DENSE prefix (`0: m, 1: a` where
   the indices are exactly 0..n in order) is REJECTED with a message naming the
   ordered form — canonical utterance, one spelling per meaning. Sparse
   (`2: x`) and selection (`0 == …`, `0 in ?p`) forms unchanged.
2. **Renderer** (`ir/render.rs`): detect the dense in-order variable-only case
   and emit bare idents; everything else keeps `i:`/selection spellings. The
   render fixed-point law (render → reparse → identical IR) must hold across
   both forms.
3. Re-pin `crates/bumbledb-query/tests/notation.rs` goldens ("one grammar,
   three consumers: `ir::render` emits it, `query!` parses it, the cookbook
   writes in it").
4. Update the normative grammar block in
   `docs/architecture/20-query-ir.md` (§ the query notation, ~lines 964–997)
   and the macro's module-doc grammar (`bumbledb-query/src/lib.rs` ~lines
   10–46). Sweep every `N: var`-style dense idb spelling in docs and tests to
   the ordered form (`grep -rn '([0-9]\+: ' docs crates` and filter to
   predicate atoms).

## Technical direction

- Positions remain the semantics — this is a SPELLING for dense bindings, not
  named head arguments; do not carry any name into the IR.
- The case split that disambiguates atom sources (lowercase predicate names vs
  UpperCamel relations) is load-bearing — do not weaken it.
- Do not touch the TS builder; its IR output is already the ordered form's
  lowering.

## Passing criteria

- `query!` with `reach(m, a)` lowers to bindings `[(0,m),(1,a)]` (unit-pinned).
- `reach(0: m)` (dense, in-order, explicit indices) fails to compile with the
  pointed error; `reach(2: x)` (sparse) and `reach(0 == Kind::Focus)`
  (selection) still compile — all three pinned.
- Mixed bare + indexed in one atom fails with its own pointed error (pinned).
- Render fixed-point over the notation test corpus: every rendered program
  reparses to identical IR; dense atoms render as bare idents, sparse as `i:`.
- Zero fingerprint pins change anywhere (queries are unfingerprinted — the
  diff must not touch any schema pin).
- `grep` sweep: no dense explicit-index idb spelling remains in docs/tests.
- `cargo test -p bumbledb-query` green. Commit in the repo's voice; push.
