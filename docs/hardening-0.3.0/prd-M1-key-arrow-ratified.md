# PRD-M1 — The key arrow ratified: the FD reading documented, never respelled

Wave M · Repo: bumbledb (docs + macro error message) · depends on: — ·
OWNER RULING 2026-07-18: this PRD REPLACES an earlier draft that respelled
`R(a, b) -> R` as `key R(a, b);`. The owner overruled it. The arrow stays.

## Objective

`Outage(service, window) -> Outage` is not ceremony — it is the
dependency-theoretic utterance: the key projection DETERMINES the tuple, and
the arrow closing over its own relation is what makes a key a key (a
functional dependency `X -> R` over R's own attributes). Ratify this reading
in the docs and the macro's own voice so no future cleanup pass mistakes the
RHS for noise again, and sharpen the one place the parser treats it as
redundant input rather than meaning.

## Context (verified)

- `parse_statement` in `crates/bumbledb-macros/src/lib.rs` (~lines 777–799)
  parses the RHS and then `assert_eq!(right, left.relation)` — mechanically
  the RHS is checked, but the FAILURE is a bare assertion, not a teaching
  error.
- The canonical spelling (`crates/bumbledb/src/schema/render.rs`, Key arm
  ~line 397) and the TS mirror (`ts/src/statements.ts::renderStatement`) both
  emit the arrow form, byte-pinned by `ts/test/render-golden.test.ts`. NOTHING
  about the renderers, spellings, manifests, or fingerprints changes in this
  PRD.

## Work

1. **The mismatch error becomes a teaching error**: replace the bare
   `assert_eq!` with a proper macro diagnostic (`compile_error!` at the RHS
   token span, matching the macro's existing error style): when the RHS names
   a different relation than the LHS, say what the arrow MEANS — e.g.
   "the key arrow closes over its own relation: `Outage(service, window) ->
   Outage` — the projection determines the tuple; `-> {other}` is not a key
   statement." Wording in the repo's voice; the span must point at the RHS.
2. **Document the reading** where the schema notation is normative:
   - the macro's module doc (`crates/bumbledb-macros/src/lib.rs` grammar
     block): one sentence on the FD reading of the key form;
   - `docs/architecture/30-dependencies.md` (the dependency calculus chapter):
     a short paragraph ratifying the arrow as the functional-dependency
     spelling and noting the owner ruling date;
   - `docs/architecture/70-api.md` where the TS `key(R, [...])` is described:
     one sentence that the TS free function is the host-idiomatic FLAVOR and
     the canonical spelling remains the arrow (the render-golden pin is the
     referee) — the semantic-parity law in miniature.
3. **A pinned probe** for the teaching error: a compile-fail test (the macro
   suite's existing failure-pinning mechanism) asserting the wrong-RHS
   message.

## Technical direction

- ZERO changes to: grammar acceptance (the arrow form is already the only
  form), renderers, manifest spellings, fingerprints, TS code. If any test
  golden changes in this PRD's diff, the PRD has exceeded its scope.
- The docs additions are short and declarative — record the reading and the
  ruling; do not relitigate the alternatives.

## Passing criteria

- The wrong-RHS diagnostic is a spanned macro error with the teaching message
  (compile-fail pin green); the happy path is untouched (existing macro tests
  green unmodified).
- The three doc sites carry the FD reading; `docs/architecture/30-dependencies.md`
  names it a ratified owner ruling.
- `git diff` contains NO renderer, golden, spelling, or fingerprint-pin
  changes.
- `cargo test -p bumbledb-macros` green. Commit in the repo's voice; push.
