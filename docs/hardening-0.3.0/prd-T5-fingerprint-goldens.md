# PRD-T5 — Cross-language fingerprint goldens, per recipe

Wave T · Repo: bumbledb (both `ts/` and `crates/`) · depends on: — · note: K7
regenerates the fixture values after the cookbook rewrite; this PRD builds the
MECHANISM against the current 29 recipes

## Objective

The "same theory in TS and Rust → byte-identical fingerprint" guarantee is
by-construction (TS never hashes: it lowers a names-only `SchemaSpec` through
the napi bridge into the SAME Rust resolution/validation/blake3 code) and was
verified experimentally for one recipe — but per-recipe equality is pinned
nowhere. Both cookbook suites already construct all 29 recipe schemas; wire
them to one shared fixture so drift anywhere upstream of the hasher (JS
property-enumeration order, the duplicated name→id resolution in
`spec::descriptor()` vs macro expansion) fails a test on the side that moved.

## Context (verified)

- TS path: `ts/src/lower.ts` → `ts/crate/src/marshal.rs::schema_spec` →
  `crates/bumbledb-theory/src/schema/spec.rs::descriptor()` →
  `crates/bumbledb/src/schema/fingerprint.rs` (blake3 over `canonical_bytes`,
  label `bumbledb-schema-v4`).
- `ts/test/cookbook.test.ts` already computes each recipe's fingerprint (its
  reopen-stability assertion) — it just pins them to nothing.
- `crates/bumbledb-query/tests/cookbook.rs` compiles all 29 recipe schemas
  (`recipe!`), byte-held to `docs/cookbook.md` — it validates but never
  fingerprints.
- A cross-host single-theory lock already exists (`ts/crate/src/fingerprint_lock.rs`
  + `ts/test/fingerprint.test.ts`, pinned constant `6120cb…1508`) — leave it
  untouched; it is the deep composite; this PRD adds breadth.

## Work

1. **The fixture**: `ts/test/fixtures/cookbook-fingerprints.txt` — one line per
   recipe, format `rNN <64-hex>`, sorted by recipe number, with a header
   comment documenting: what it pins, that BOTH suites read it, and the
   regeneration procedure (run the TS side with an env flag, e.g.
   `REGEN_FINGERPRINTS=1 node --test test/cookbook.test.ts`, which rewrites the
   file; the Rust side never writes it).
2. **TS side**: in `cookbook.test.ts`'s recipe helper (where the fingerprint is
   already in hand), assert equality with the fixture line; implement the
   regeneration flag.
3. **Rust side**: a new test in `crates/bumbledb-query/tests/cookbook.rs` (or a
   sibling file in that suite) that, for each recipe schema already compiled
   there, validates and fingerprints it (`SchemaDescriptor::validate` +
   `fingerprint`) and asserts equality with the same fixture, read via
   `include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../ts/test/fixtures/cookbook-fingerprints.txt"))`.
   Parse the two-column format in the test; unknown/missing recipe ids are
   failures, not skips.
4. Generate the initial fixture from the TS side, then run the Rust side to
   prove equality. If ANY recipe disagrees: STOP — that is a live drift trophy;
   report it with both hex values and the recipe number rather than "fixing"
   either side to match.

## Technical direction

- The fixture is plain text on purpose (no serde, no JSON) — two columns, one
  authority file, both consumers.
- Recipe identity: use the cookbook's stable recipe numbers (r01–r29), which
  both suites already carry.
- Do not reformat or re-derive the schemas in either suite — fingerprint
  exactly what each suite already builds; the point is pinning the EXISTING
  two constructions to each other.
- 29 lines exactly at time of writing; K7 will regenerate values for recipes
  it rewrites (its criteria own that update — note it in the fixture header).

## Passing criteria

- The fixture exists with 29 entries; both suites read the SAME file (no
  second copy anywhere; grep for the filename yields the fixture + exactly two
  readers).
- TS: editing any recipe schema fails the TS assertion; the regen flag
  rewrites the file deterministically (two consecutive regens produce
  byte-identical output).
- Rust: `cargo test -p bumbledb-query` fails if any recipe's Rust-side
  fingerprint differs from the fixture; missing/extra fixture lines fail.
- Initial state: both suites green against one generated fixture (or a
  reported drift trophy — which is a STOP, not a pass).
- Commit in the repo's voice; push.
