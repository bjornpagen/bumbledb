# PRD-M5 — The coherence check: newtype agreement across statement faces

Wave M · Repo: bumbledb (`crates/bumbledb-theory` + macro + `ts/` error arm) ·
depends on: — · OWNER RULING 2026-07-18: the Rust macro's syntax is UNCHANGED
("option 1"); `as NewType`, `fresh`, and every statement form stay exactly as
they are. Macro-side `ref`, projection sorts, and signature blocks were all
REJECTED. What lands is verification: the hand taxonomy stops being an
unchecked claim.

## Objective

Today `Attempt(kind) <= Kind(id)` is accepted even when `kind: u64 as SheetId`
— the newtypes on a statement's paired faces are never compared (verified: the
macro does not cross-check newtypes across containment faces; the shared spec
lowering doesn't either). Add ONE check in the shared lowering: for every
statement with paired faces, the paired columns' newtype labels must cohere.
Surfaced per host idiom: a spanned `compile_error!` at macro expansion; the
engine-side twin of TS's existing `SameDomains` compile wall on the SDK path.

## The rule (matches TS's `SameDomains` strictness exactly)

Positionwise over every paired-face statement — containment (`<=`, including
ψ-selected targets: selection never changes field pairing), `mirrors`,
pointwise `==` faces, and cardinality/window target pairs:

- labeled ↔ labeled: newtypes MUST be equal;
- unlabeled ↔ unlabeled: passes;
- labeled ↔ unlabeled: **an error** (TS's `undefined` pairs only with
  `undefined`; the shared check adopts the same law so the two walls judge
  identically).

Deliberately-bare columns (primer's `task.subject` shape) are untouched — they
appear in no paired-face statement.

## Work

1. **The check** in `crates/bumbledb-theory`'s shared lowering
   (`schema/spec.rs` — where `FieldSpec::newtype` already lives and where
   `SpecIssue`s are raised): a new issue variant, e.g.
   `StatementNewtypeMismatch { statement, position, left, right }` (naming in
   the crate's existing voice), raised while resolving each paired-face
   statement. It runs BEFORE newtype-dropping (newtypes are dropped at
   lowering — the check is authoring-time only; descriptors, fingerprints, and
   stores carry no newtypes and are untouched).
2. **Macro surfacing**: the macro already maps each `SpecIssue` to a
   `compile_error!` at the offending token via its span table — add the new
   variant with a teaching message in the repo's voice, e.g. "the containment
   pairs kind (SheetId) with Kind.id (KindId) — the faces of a dependency
   agree on their newtype, or neither carries one." Span points at the
   statement's offending face.
3. **TS surfacing**: the SDK's typed `Db.create` rejection path gains the
   matching error arm (verify the napi marshal passes the new issue through;
   extend the wire error union in `ts/src/native.ts` and the typed rejection
   in `db.ts`). Note the arm is UNREACHABLE through the typed builder — TS's
   `SameDomains` blocks it at compile time — so the runtime test constructs
   the mismatched spec through a raw-object fixture (test-side laundering is
   legal in `ts/test`), proving the engine twin fires. This is the type-lie
   law applied to a wall: the compile claim has a runtime referee.
4. **The sweep**: run the check over every in-tree schema — engine tests,
   `docs/cookbook.md` recipes, `schema_macro.rs` fixtures, fuzz generators'
   vocabularies. Every hit is a FINDING: fix the mislabel (or label the bare
   face) in the same commit, listed in the commit body. `theorygen`'s hostile
   arm may now legitimately generate mismatched specs — that is the point
   (validator totality); add the new issue to the fuzz suite's exhaustive
   error-arm roster (the `DescriptorMissing` convention).
5. **Docs**: one paragraph in `docs/architecture/30-dependencies.md` beside
   the M1 ratification: the taxonomy is checked, the rule's three cases, and
   the owner ruling that the notation itself stays put.

## Technical direction

- ZERO fingerprint/store surface: newtypes never reach `canonical_bytes`; the
  check runs at authoring surfaces only (macro expansion, `SchemaSpec`
  lowering). Every fingerprint pin in the tree must be byte-identical after
  this PRD — same proof obligation as M1.
- One check, one home: do NOT duplicate the logic in the macro and the spec
  path — the macro's copy IS the spec path's, reached through the shared
  lowering (that was the point of the theory-crate extraction).
- The closed-handle `HandleNewtypeReuse` uniqueness check is adjacent but
  distinct — leave it as is.

## Passing criteria

- Compile-fail pin (macro suite): a mismatched containment fails with the
  spanned teaching message; a labeled↔unlabeled pairing fails; an
  unlabeled↔unlabeled pairing passes — all three pinned.
- Spec-level unit tests for the same three cases through `SchemaSpec` (the
  TS path's entry), plus one ψ-selected-target case proving selection doesn't
  bypass the check.
- TS: the new rejection arm exists, is typed, and the raw-fixture runtime
  test proves it fires; `SameDomains` compile probes unchanged.
- The in-tree sweep is clean (the check passes over every schema in the
  repo) with any fixes named in the commit body; the fuzz exhaustive-arm
  roster includes the new issue.
- Every fingerprint pin byte-identical; `scripts/check.sh` green.
  Commit in the repo's voice; push.
