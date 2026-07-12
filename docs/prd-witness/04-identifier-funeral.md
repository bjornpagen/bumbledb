# PRD 04 — The identifier funeral completes: Variant, ordinal, seed

**Depends on:** 03 (renames land on the deduplicated tree).
**Modules:** `crates/bumbledb-macros/src/lib.rs`,
`crates/bumbledb-query/src/lib.rs`, `crates/bumbledb/src/image/cache*`,
`crates/bumbledb/src/exec/sink/aggregate/groups.rs`,
`crates/bumbledb/src/exec/kernel/filter.rs`,
`crates/bumbledb-bench/src/` (non-naive "seed" identifiers),
stale `#[allow]` pair.
**Authority:** the comptime set's vocabulary discipline (recorded in
`docs/architecture/10-data-model.md`'s obituary and the deleted-vocabulary
table): *enum* and *ordinal* are dead engine vocabulary; *seed/seeding* is
banned outside the naive model (a model may seed; the engine never does).
The owner's ruling: the ban is semantic, not lexical — identifiers that
NAME the dead concepts rename; identifiers that merely contain the letters
(query-shape `*Variant` enums, RNG seeds) are examined case by case below.
**Representation move:** names are the representation of intent; a field
called `closed_ordinals` teaches every reader a dead concept.

## Context (decided shape) — the rename ledger, exhaustive

Renames (each verified to name the dead concept):
1. `Literal::Variant` → `Literal::Handle` — `bumbledb-macros/src/lib.rs`
   (three sites; its own doc comment already says "a closed relation's
   handle").
2. `SelValue::Variant { qualifier, variant }` →
   `SelValue::Handle { qualifier, handle }` — `bumbledb-query/src/lib.rs`
   (the parse sites, the emitter arm, and the doc comment that already
   says handle).
3. `ImageCache.closed_ordinals` → `closed_slots` (and the local `ordinal`
   binders in `image/cache/new.rs` / `get_or_build.rs` → `slot`): the
   value is an index into the closed `OnceLock` slot array, and "slot" is
   what the cache's own docs call it.
4. `seed_pack_group` / `seed_arg_group` → `init_pack_group` /
   `init_arg_group` — `exec/sink/aggregate/groups.rs` (engine-side
   "seed").
5. Bench non-naive "seed" identifiers (the model keeps its blessed
   vocabulary; these populate the ENGINE): `differential/tests/closed.rs
   fn seed()` (cross-called from `fold.rs`) and `contradiction.rs
   fn seed()` → `base_delta()`; `witness.rs fn seeded()` →
   `prepared_world()` (or the closest honest name at the site);
   the test name `seeded_200_op_stream_agrees_with_the_engine` →
   `fixed_200_op_stream_agrees_with_the_engine`; `writebench.rs
   seeded_posting` → `prepared_posting`.
6. Comment: `exec/kernel/filter.rs` "the enum/bool column variant" → the
   byte-column arm's honest name ("the bool byte-column arm").

Explicitly NOT renamed (examined, kept, with reasons — record these in
the commit body, not in code):
- querygen's `ClosedVariant`/`ChaseVariant`/`RulesVariant`: query-shape
  flavors, not the dead column type; renaming to `*Kind` is churn without
  a concept fix. KEEP.
- RNG `seed` fields/flags (`GenConfig.seed`, `--seed`, `BENCH_SEED`) and
  everything under `src/naive/`: blessed. KEEP.
- Rust's own `enum` keyword, "variant" in prose about Rust sum types,
  and the obituary/deleted-vocabulary/host-emission contexts. KEEP.
- Loop binders like `|ordinal: u64|` in bench closures where the value
  IS a row id: rename to `id`/`row` only where touched anyway; do not
  sweep for their own sake.

Plus the two stale suppressions (audit-verified against clippy's
thresholds): `image/decode.rs` `#[allow(clippy::too_many_arguments)]` on
`decode_fact` (7 args; fires at 8+) and `verify_store/facts.rs` on
`check_outgoing` (7 args). Delete both attributes. `exec/run/pump.rs`'s
allow is NOT stale (`self` counts; 8 total) — leave it.

## Technical direction

Mechanical, compiler-driven: rename the type/variant/fn/field, fix every
use site the compiler names, fix the doc comments alongside. The
bumbledb-query rename touches its emitter string templates — re-run its
notation/cookbook suites, whose goldens do not contain the Rust-side
identifier (verify: the goldens are query-text, not Rust). No public API
of the engine crate changes except `ImageCache` internals (pub(crate)).
The macro crate's `Literal` is private; the query crate's `SelValue` is
private.

## Passing criteria

- `[shape]` `grep -rn "Variant" crates/bumbledb-macros/src
  crates/bumbledb-query/src` → zero hits outside comments about Rust
  itself; `grep -rn "closed_ordinals\|seed_pack_group\|seed_arg_group"
  crates` → zero hits.
- `[shape]` `grep -rniE "fn seed|fn seeded" crates/bumbledb-bench/src`
  → hits only under `src/naive/`.
- `[shape]` The two named `#[allow(clippy::too_many_arguments)]`
  attributes are gone; `cargo clippy --workspace --all-targets --
  -D warnings` is green (proves they were stale).
- `[test]` bumbledb-query notation + cookbook suites pass with zero
  golden edits (the renames are Rust-side only).
- `[gate]` Workspace gates green at campaign close.

## Doc amendments (rule 5)

None — vocabulary enforcement of rulings already recorded in
10-data-model and the comptime obituary.
