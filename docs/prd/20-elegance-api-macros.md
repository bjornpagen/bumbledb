# PRD 20 — Elegance: api and macros

**Depends on:** 19.
**Binding constraints:** the README's elegance-pass block.
**Modules:** `crates/bumbledb/src/api.rs` + `api/` (db, prepared, stats),
`crates/bumbledb-macros/src/lib.rs`, `crates/bumbledb/src/alloc_counter.rs`,
the engine crate's integration tests (`crates/bumbledb/tests/`).

## Subsystem-specific hunt list (verify, don't assume)

- **The dyn/typed surface pairs:** `insert`/`insert_dyn`, `delete`/`delete_dyn`,
  `get`/`get_dyn`, `alloc`/`alloc_at` (the `SerialField` witness fresh from PRD 06) — the
  typed halves should be thin wrappers over one shared core per operation;
  check for validation logic duplicated between halves, and for encode-scratch
  handling that differs per pair when it should be one discipline.
- **Bind path layering:** scalar params, param sets (`ParamArg`), intern
  resolution, and the staleness pin record (PRD 13) all live on the
  prepared query — check the bind path for sequential re-walks of the same
  param list that could be one pass, and for error-position bookkeeping
  duplicated between arity/type/set-shape checks.
- **The macro's parser:** hand-rolled across two eras (field grammar, then
  statement grammar) — check for token-cursor helper duplication (peek/expect/
  ident-match patterns re-implemented per grammar section) and converge on one
  small cursor vocabulary. Diagnostics: one error-message voice ("expected X
  after Y" style), consistent across field and statement parsing.
- **Codegen output shape:** the generated fact structs, newtypes, and impls —
  check the emission code for near-identical string-building blocks per type
  category that a small emission table would collapse; verify generated-code
  style matches hand-written engine style (the user reads both).
- **Integration tests:** `tests/api.rs` / `tests/edge.rs` / `tests/alloc_gate.rs`
  were rewritten under deadline during the rebuild — normalize fixture
  construction against the converged style from PRDs 12–14 and kill
  triple-coverage where the lib tests already pin the same behavior (merge and
  redirect, never just delete).

## Passing criteria

As PRD 16's, applied to this subsystem. Additionally:
- `[shape]` Each dyn/typed pair shares one core (grep-checkable: the validation
  logic appears once per operation).
- `[gate]` Workspace gates green; the macro's compile_fail doctests still pass
  (they pin the diagnostics voice — if messages were normalized, the doctests
  were updated in the same hunk, which is an allowed assertion change ONLY for
  diagnostic wording, noted in the findings list).
