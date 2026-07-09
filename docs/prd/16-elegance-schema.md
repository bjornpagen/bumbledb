# PRD 16 — Elegance: schema, encoding, error

**Depends on:** 01–15 (the elegance passes run last, over settled code).
**Binding constraints:** the README's elegance-pass block — strictly
behavior-preserving; no assertion changes; findings summary in the commit body.
**Modules:** `crates/bumbledb/src/schema.rs` + `schema/` (all),
`crates/bumbledb/src/encoding.rs` + `encoding/`,
`crates/bumbledb/src/error.rs` + `error/`, `crates/bumbledb/src/digest.rs`,
`crates/bumbledb/src/lib.rs`.

## Method (identical for PRDs 16–21; stated once fully here)

1. **Read the whole subsystem before any edit** — every file, tests included.
   Build the findings list first; then apply. The commit body carries the list:
   what was deduplicated, what moved, what died, what was left alone
   deliberately.
2. Hunt the seam classes in the README's priority order. This subsystem's
   likely finds, from the rebuild's shape (verify, don't assume):
   - **Validation/materialization split:** PRD-02/03-era code split descriptor
     materialization, roster validation, and `Resolved` computation across
     `schema.rs`/`validate.rs`/`runtime.rs` — check for duplicated
     field-lookup/type-equality helpers and for roster checks that re-derive
     what materialization already computed.
   - **Value-collapse residue:** PRDs 01–03 landed the shared `Value`, the
     decl-layer deletion, and the materialized mirror — sweep for their
     residue: helper functions or match arms that survived the collapse with
     one caller, conversion shims someone left "temporarily," and doc comments
     still describing the old three-type world.
   - **Error enum ergonomics:** the schema/validation error enums grew ~30
     variants across three PRDs — check Display arms for copy-paste drift,
     payload field-name inconsistency (`statement` vs `statement_id`), and
     variants no site constructs (dead weight — delete, don't deprecate).
   - **`render.rs` vs `Debug`:** two renderings of statements exist; confirm
     Debug derives haven't been shadowed by hand-impls that duplicate render
     logic.
3. **Idiom normalization:** one error-construction idiom, one doc-comment voice
   (the house voice: terse, cites the owning chapter), one test-fixture style
   per module (the schema tests grew three fixture-construction styles across
   PRDs 02/03/05 — converge on the best one and port the others).
4. Gates after every functional grouping of edits; one commit for the PRD.

## Passing criteria

- `[shape]` The findings summary in the commit body, per the README block.
- `[shape]` No behavior change: `git diff` over `tests/` shows no expected-value
  changes (restructuring allowed).
- `[shape]` Zero dead `pub` items, unconstructed variants, or single-caller
  indirections remain in the subsystem (each either deleted or justified in the
  findings list).
- `[gate]` Workspace gates green.
