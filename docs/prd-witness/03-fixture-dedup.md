# PRD 03 — Fixture deduplication: the bench fixture module

**Depends on:** baseline only (run before 04 — the renames land on the
deduplicated tree, not vice versa).
**Modules:** `crates/bumbledb-bench/src/differential/tests.rs` and its
children, `crates/bumbledb-bench/src/` (new `fixture` module),
`crates/bumbledb-query/tests/`, `crates/bumbledb/src/storage/keys.rs`,
`storage/commit/tests.rs`, `verify_store/tests.rs`.
**Authority:** the settled house convention,
`crates/bumbledb/src/storage/commit/tests.rs` header: shared *shorthands*
centralize in the parent tests module; *schemas* stay per-file ("each
judgment matrix wants its own statement shapes"). The owner's ruling:
extend that convention to the bench crate.
**Representation move:** ~300 lines of copy-paste shorthands across the
bench crate are the residue of parallel authorship — the convention
already exists; this PRD applies it. Fixture *builders* (corpora, seeds)
are genuinely distinct fixtures and are NOT merged.

## Context (decided shape)

The duplication census (2026-07-12 audit):
- `TempDir`: four hand-rolled copies inside one module tree
  (`differential/tests/{measure,identity_bytes,pack,chase}.rs`) while
  siblings import `super::TempDir`; two more in
  `bumbledb-query/tests/{notation,cookbook}.rs`. The bumbledb crate's
  integration-test twin (`crates/bumbledb/tests/common/mod.rs`) is
  justified by its own comment and stays.
- `string()` Value helper: identical in `differential/tests/closed.rs`
  and `contradiction.rs`.
- `key()` KeyBuf closure: three identical copies inside the engine crate
  (`storage/keys.rs` tests, `storage/commit/tests.rs`,
  `verify_store/tests.rs`).
- Shorthand fns across the bench crate with no shared home: `var()` ~16
  copies, `field()`/`fresh()` ~15, `atom()` ~8 (two shapes), `side()` ~5.
- The era-narration comment cluster (provenance prose pointing at deleted
  work-plan docs): `storage/commit/tests.rs` "(the PRD 07/08/09 eras each
  grew their own copies)", `schema/fingerprint.rs` "(the funeral,
  PRD 05)", `ir/render.rs` "the query-notation PRD owns the normative
  block", `ir/validate/context.rs` "PRD 12 §1", provenance headers in
  `ir/normalize/fold/tests.rs`, `bumbledb-query/tests/notation.rs`
  (×5 sites), `bumbledb-query/tests/compile_fail.rs`.

## Technical direction

1. New `crates/bumbledb-bench/src/fixture.rs`, compiled in every target,
   holding exactly the shared query shorthands used by both production
   benchmark families and tests: `var(u16) -> Term`,
   `atom(RelationId, &[(u16, Term)]) -> Atom` (fold the two shapes into
   one; callers adapt), `field`/`fresh` descriptor helpers, `side`,
   `string`, `TempDir` (the test-only `string`/`TempDir` items are gated
   individually with `#[cfg(test)]`; one definition; the per-file
   `bumbledb-*-{tag}`
   temp naming becomes a `TempDir::new(tag)` argument exactly as the
   engine crate does). Register as `mod fixture;` in
   `lib.rs`. Every duplicate definition in the bench crate is deleted and
   replaced by the import. Schemas, corpora, seed deltas, and family
   fixtures are NOT moved.
2. `bumbledb-query/tests/common/mod.rs` gains the crate's one `TempDir`;
   both integration files import it (the integration-twin pattern the
   engine crate documents).
3. Engine crate: `key()` becomes one `#[cfg(test)] pub(crate) fn` beside
   `KeyBuf` in `storage/keys.rs`; the two other copies import it.
4. The era-narration comments are rewritten to state the invariant they
   decorate, or deleted where the invariant is already stated: e.g. the
   commit-tests header keeps "shared shorthands live here; schemas stay
   per-file — each judgment matrix wants its own statement shapes" and
   loses the era sentence. Bare "(PRD NN)" cohort labels stay (set
   refusal); only narration dies. The distinction: a label after a test
   name is vocabulary; a sentence explaining what some era did is
   narration.

## Passing criteria

- `[shape]` One `TempDir` definition per crate boundary that needs one:
  `grep -rn "struct TempDir" crates` → exactly four (bumbledb
  `testutil`, bumbledb `tests/common`, bumbledb-query `tests/common`, and
  the bench `fixture` module), none inside
  `differential/tests/`.
- `[shape]` `grep -rn "fn var(" crates/bumbledb-bench/src | wc -l` → 1;
  same for `fn atom(`, `fn side(`, `fn string(` (within the bench crate);
  `grep -rn "fn key(" crates/bumbledb/src` → 1.
- `[shape]` `grep -rn "PRD 07/08/09 eras\|PRD 12 §1\|query-notation PRD"
  crates` → zero hits.
- `[test]` No test's assertions change — this PRD moves definitions only;
  the full workspace suite passes identically (same counts as baseline,
  minus nothing).
- `[gate]` Workspace gates green at campaign close.

## Doc amendments (rule 5)

None — test-internal convention, already documented at the convention's
definition site.
