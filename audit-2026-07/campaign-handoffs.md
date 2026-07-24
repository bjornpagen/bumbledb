# Campaign cross-lane handoffs (live)

Interface notices between concurrent campaign lanes. If you are fixing an integration
failure whose suspect paths appear below, this file is your spec.

## R16 — one id allocator (Lane S → Lane B, exec lane) — 2026-07-24

Lane S has landed the R16 representation (FORMAT_VERSION 6):

1. `KeyStatement` gains `fresh_row: bool` (the relation's first fresh field's auto-key);
   `Relation` gains `fresh_row_field() -> Option<FieldId>` (crate-visible). On a
   fresh-keyed relation the first fresh field's value IS the F row id and that auto-key
   maintains NO U tree.
2. **READ SIDE OWED (lane B, `storage/read/**`):** `determinant_row` / `fact_for_key`
   must dispatch on the key statement's `fresh_row` — the determinant bytes ARE the
   8-byte BE row id, so probe `F | rel | determinant` directly (one descent; the F value
   is the fact itself, no second fetch). Known-failing until then:
   `storage::read::tests::key_probe_hit_and_miss` (probes StatementId(0), the fresh
   auto-key, via U and gets None). Commit-side precedent to copy:
   `storage/commit/judgment.rs::Checker::check_scalar`'s `fresh_row` arm.
3. **EXEC SIDE OWED (exec lane):** `exec/dispatch/key_probe_fact.rs` / `classify.rs`
   key-statement probes need the same dispatch — a plan key probe against a fresh
   auto-key must read F directly, or classification must route around U.
4. `ImageCache::advance` now takes `(generation, dirty, floors)` — floors from
   `WriteDelta::inserted_floors()`; already wired in `api/db/write.rs`.
5. `S RowIdHighWater` exists only for fresh-less relations (fresh-keyed relations mint
   from Q).
