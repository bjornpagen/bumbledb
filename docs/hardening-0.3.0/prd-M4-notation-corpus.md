# PRD-M4 — The notation conformance corpus: one grammar, mechanically refereed

Wave M · Repo: bumbledb (`crates/bumbledb-query` + `ts/`) · depends on: M2
(pins the FINAL query notation; the schema notation is not in scope here)

## Objective

The query notation is normative in `docs/architecture/20-query-ir.md` and
pinned Rust-side only (`crates/bumbledb-query/tests/notation.rs`); the TS
builder is pinned to the IR, never to the notation. Before any second text
surface can ever ship (the tagged-template endgame), the two sides need a
mechanical referee: a checked-in corpus of (notation string ⇄ ProgramIr JSON)
cases that Rust and TS both replay. The repo has two exact precedents for this
pattern: the Lean three-way conformance corpus (`lean/conformance/cases/*.json`)
and the TS-render ⇄ manifest-render golden.

## Design

- **Interchange format**: the napi bridge's `ProgramIr` JSON — it already
  exists as plain data on the TS side (`ts/src/native.ts` `dbPrepare(db,
  program: ProgramIr)`; `program_in` in `ts/crate/src/marshal.rs` mirrors
  `ir::Program` 1:1). The corpus README documents the shape normatively.
- **Corpus location**: `crates/bumbledb-query/tests/notation-corpus/` — one
  JSON file per case: `{ "name": …, "notation": "<the query! source>",
  "program": <ProgramIr JSON> }`, plus a `README.md` stating the law and the
  wire shape.
- **Rust side** cannot reuse `marshal.rs` (out-of-workspace, napi-typed):
  write a small deterministic encoder in the test crate
  (`tests/support/ir_json.rs` or inline), ~100 lines, mapping `ir::Program` →
  exactly the documented JSON shape (field order fixed; integers as JSON
  numbers or strings — match what the TS side produces through
  `JSON.stringify` of its `ProgramIr` value, verified, not assumed).
- **TS side**: a new `ts/test/notation-corpus.test.ts` reads the same corpus
  dir (relative path from ts/test), and for each case expressible in the
  builder constructs it and asserts `JSON.stringify`-normalized equality of
  its `ProgramIr` against the pinned `program`. Cases not expressible in the
  builder (sparse idb selections, if any) carry `"builder": false` and are
  skipped WITH a count assertion (so silent skips are impossible).
- **Bridge acceptance**: the TS test also `dbPrepare`s each case's program
  against a store built from the corpus's shared schema(s) — prepare must
  succeed (or the case documents its expected refusal).

## Work

1. Author ≥20 cases covering EVERY grammar production at least once —
   enumerate in the corpus README and assert the enumeration in the Rust test:
   punning, `field: var`, `== literal`, `== Handle`, `== ?param`, `!=`,
   `< <= > >=`, `in ?param`, `?t in v` membership, `Allen(_, MASK, ?p)` (mask
   unions included), `!atom` negation, every aggregate (`Sum/Count/Pack/…` ×8 +
   `Duration`), named columns, multi-rule union, `program`/rec recursion, idb
   ordered-dense heads, idb sparse binding, idb position selection.
2. Rust test: for each case, `query!`-compile the notation (via a generated
   test fn per case or a macro-expansion table — the notation.rs pattern),
   encode the IR through the encoder, assert equality with the pinned JSON;
   AND assert `ir::render` of the IR reparses (the fixed-point, tying the
   corpus to the renderer).
3. TS test as designed above; shared schema(s) for the corpus defined once in
   each language and fingerprint-asserted equal (one line reusing the T5
   mechanism, so the corpus schemas themselves cannot drift).
4. Wire both tests into the normal suites (they must run in the T6 CI lane by
   construction).

## Passing criteria

- ≥20 cases; the production-coverage enumeration is asserted (a case list per
  production; an uncovered production fails the Rust test).
- Rust: every case compiles, encodes to the pinned JSON byte-normalized, and
  render-round-trips. Editing any case's notation or program fails.
- TS: every `"builder": true` case produces the identical ProgramIr JSON; the
  skipped-count assertion matches the corpus's `"builder": false` count
  exactly; every case's program is accepted by `dbPrepare` (or its documented
  refusal is asserted).
- The corpus README documents the JSON shape and the law ("a disagreement is a
  trophy, not a merge conflict").
- `cargo test -p bumbledb-query` and the TS suite green for these tests.
  Commit in the repo's voice; push.
