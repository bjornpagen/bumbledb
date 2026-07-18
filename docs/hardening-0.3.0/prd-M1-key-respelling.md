# PRD-M1 — `key R(a, b);` — the ceremony RHS dies

Wave M · Repo: bumbledb (macro + engine render + `ts/` render + docs) ·
depends on: — · flag-day (no dual-accept)

## Objective

The key statement's RHS is pure ceremony: `parse_statement` in
`crates/bumbledb-macros/src/lib.rs` (~lines 777–799) parses `-> R` and then
`assert_eq!(right, left.relation)` — it carries zero information. Respell the
statement as `key R(a, b);` everywhere: grammar, BOTH renderers (Rust and TS —
they are mirrored and pinned byte-equal), goldens, and every written site.
Canonical utterance: the old spelling is deleted, not deprecated.

## Store safety (the load-bearing fact — verify, then rely on it)

The fingerprint (`crates/bumbledb/src/schema/fingerprint.rs::canonical_bytes`,
label `bumbledb-schema-v4`) hashes descriptor data only — ids, type tags,
generation flags, extensions, statement form-tags + ids + literal encodings.
**No surface syntax and no spelling string enters the hash.** Spellings are
recomputed at runtime from the descriptor. Therefore this change lowers to the
identical `StatementDescriptor`, identical bytes, identical fingerprint —
existing stores open unchanged. The criteria PROVE this by requiring every
fingerprint pin in the tree to remain byte-identical.

## Work

1. **Grammar** (`crates/bumbledb-macros/src/lib.rs` `parse_statement`): accept
   `key R(field, ...);`. DELETE acceptance of `R(...) -> R`; on encountering
   the old form emit a pointed compile error naming the new spelling
   ("the key RHS is gone — write `key Outage(service, window);`").
2. **Rust renderer** (`crates/bumbledb/src/schema/render.rs`, the Key arm at
   ~line 397 rendering `side -> RelationName`): emit `key R(fields)`. Update
   `schema/render/tests.rs` goldens.
3. **TS renderer** (`ts/src/statements.ts::renderStatement`, key arm ~line
   180): emit the identical string. Update the hand-spelled implied-key
   strings in `ts/test/render-golden.test.ts` (~lines 165, 172, 196). The
   render-golden test (TS render == engine manifest spelling, byte-equal) is
   the referee that both renderers moved together — it must be green at this
   PRD's end even if the wider tree is red.
4. **The sweep** (~130+ sites): every written key statement across
   `docs/**` (architecture chapters, `docs/cookbook.md` schema blocks),
   `crates/**` (tests, doc comments, macro docs), `fuzz/**` (dictionaries or
   seed spellings if any), `ts/COOKBOOK.md`. Find them ALL:
   `grep -rn '\-> *[A-Z][A-Za-z0-9]*;' docs crates fuzz ts` and filter to key
   statements (the pattern `Relname(fields) -> Relname`); also sweep
   error-message examples in `docs/architecture/30-dependencies.md` and
   `70-api.md`.
5. Any test that pinned a rejection/violation message containing the old
   spelling re-pins to the new render.

## Technical direction

- The canonical spelling is derived from the AST/descriptor, so grammar and
  renderer are separate changes — land BOTH in this PRD; a grammar-only change
  violates the render-fixed-point law (renderer output must reparse).
- Do not touch statement lowering, descriptor types, or ordering — this PRD is
  spelling only.
- `bumbledb-theory`'s shared lowering is untouched (names/ids only, no
  spellings live there).

## Passing criteria

- Old spelling: a `schema!` block containing `R(a) -> R;` fails to compile
  with the pointed error (a compile-fail assertion pins this — trybuild-style
  or the macro test suite's existing failure-pinning mechanism).
- New spelling: parses; renderer emits it; **render fixed-point** — for every
  schema in the render test suites, `parse(render(x)) == x`.
- `ts/test/render-golden.test.ts` green: TS `renderStatement` == engine
  manifest spelling, byte-equal, for keys (declared AND implied).
- **Every fingerprint pin in the tree is byte-identical to before this PRD**:
  the `fingerprint.rs` unit goldens, the cross-host lock constant
  (`6120cb…1508` in `ts/crate/src/fingerprint_lock.rs` + `ts/test/fingerprint.test.ts`),
  and the T5 fixture if it has landed. Zero pin edits in this PRD's diff.
- Sweep completeness: `grep -rn ') -> [A-Z]' docs crates fuzz ts` (and the
  looser pattern above) returns zero key-statement hits.
- `cargo test -p bumbledb-macros -p bumbledb` green for the touched suites
  (whole-tree green is the final gate's job). Commit in the repo's voice; push.
