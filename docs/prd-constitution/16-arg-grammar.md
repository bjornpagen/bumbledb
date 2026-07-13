# PRD 16 — Arg aggregates enter the grammar: the render/parse asymmetry dies

**Depends on:** 09 (the macro's agg table is quiet).
**Modules:** `crates/bumbledb-query/src/lib.rs` (the agg grammar :15,
the macro `AggOp` enum :241-245, the name table :479-483, the error
message :557-559), `ir/render.rs` (:194-200, already renders the Arg
forms), round-trip tests, `docs/architecture/20-query-ir.md` +
`docs/cookbook.md` if a recipe wants them.
**Authority:** audit #13, verified: `AggOp::ArgMax { key }` /
`ArgMin { key }` exist in the IR, execute, and render — but the
`query!` macro cannot write them, so the "round-trip golden" pins only
a subset of the IR and hosts must hand-build raw IR for a first-class
feature. The spec offered expose-or-formally-reject; the campaign
chooses EXPOSE (it is executable, tested machinery — hiding it is the
asymmetry, not the feature).
**Representation move:** none in the engine. The macro grammar grows
two forms; the notation becomes total over the executable IR's
aggregate surface.

## Context (decided shape)

- Surface grammar: `ArgMax(value, key)` / `ArgMin(value, key)` in find
  position — the projected term and the extremized key, matching the
  IR's `{ op-over + key }` decomposition. Exact spelling follows the
  existing macro convention (read the grammar block and mirror
  `CountDistinct`'s two-token style).
- The macro `AggOp` enum, name table, and error message ("takes
  Sum/Min/Max/Count/CountDistinct/Pack") all gain the two forms.
- `ir/render.rs`'s existing Arg rendering is the round-trip anchor:
  parse(render(q)) == q for Arg queries — the new round-trip goldens
  assert both directions over singleton and composite cases, including
  the self-carry form (`ArgMax(x, x)`) the signature table already
  types.
- `ArgAcrossRules` refusal (post-DNF) is untouched and re-pinned from
  the macro side: a multi-rule query with an Arg form rejects with the
  existing typed error — now writable in notation, so the rejection
  gets a notation-level test.
- The PRD-04-era signature table already carries Arg rows — unchanged;
  if the macro exposure reveals a typing hole the table missed, policy
  5.

## Technical direction

Grammar first (parse → macro AggOp → lowering to IR), then the name
table and error string, then round-trips. The macro crate's tests
follow its existing golden style. Bench querygen may NOW gain Arg
shapes as a follow-up — explicitly OUT of scope here (generator
coverage expansion is registered future work; this PRD only closes the
notation asymmetry).

## Passing criteria

- `[shape]` The macro grammar block, enum, name table, and error
  message all list the Arg forms (grep each).
- `[test]` Round-trip goldens green both directions (singleton,
  composite, self-carry); the ArgAcrossRules notation-level rejection
  green; full bumbledb-query suite green.
- `[shape]` Zero engine-crate source changes (`git diff --stat` shows
  bumbledb-query + docs + tests only).
- `[gate]` Fingerprint pin untouched; clippy; fmt.

## Doc amendments (rule 6)

`20-query-ir.md`'s aggregate table marks the Arg forms
notation-writable; the grammar block in the macro's module docs
follows; cookbook mention optional (no new recipe required).
