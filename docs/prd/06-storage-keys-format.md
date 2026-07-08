# PRD 06 — Storage keys and format version

**Depends on:** 02.
**Modules:** `crates/bumbledb/src/storage/keys.rs`, `crates/bumbledb/src/storage/env.rs`.
**Authority:** `docs/architecture/50-storage.md` (§ Key layout).

## Goal

The `_data` key layout matches `50-storage.md`: `U` keyed by statement id, `R`
statement-scoped; the store format version bumps so pre-redesign stores fail at
open.

## Technical direction

1. `env.rs`: `FORMAT_VERSION` 0 → 1. Nothing else in the open path changes (the
   version check already hard-fails on mismatch). Do not write any fallback or
   migration read path.
2. `keys.rs` layouts (widths unchanged where the component survives; all
   big-endian):
   ```
   U | relation_id(u32) | statement_id(u16) | guard_bytes            -> row_id(u64)
   R | statement_id(u16) | key_bytes | source_rel(u32) | source_row(u64) -> ()
   ```
   `F`, `M`, `Q`, `S` are unchanged. The old `R` layout
   (`R | target_rel | constraint | key | source_rel | source_row`) is deleted —
   note the target relation id is gone from `R` (the statement id determines it;
   storing it twice was transcription).
3. Rename every `constraint`-named component and function to `statement`
   (`restrict_prefix` → `reverse_prefix` or similar containment vocabulary).
   Recompute the overhead constant: `MAX_GUARD_WIDTH = MAX_KEY − R_OVERHEAD` where
   `R_OVERHEAD` = 1 (tag) + 2 (statement) + 4 (source_rel) + 8 (source_row) — the
   binding constraint is still the `R` embedding, since a target-key value embeds
   whole in `R`. Add a doc comment deriving the number.
4. Guard-byte builders: guard keys are built by slicing the statement's projection
   fields out of `fact_bytes` in **statement projection order** (for `U`) or in the
   **target key's guard order via `key_permutation`** (for `R` key bytes) — the
   slicing helpers gain a 16-byte width case (interval fields copy their whole 16
   bytes; never split start/end here — the contiguity is what makes the B-tree
   start-ordered within a prefix group).
5. Key builders/parsers get exhaustive unit tests in the existing keys-test style:
   byte-level golden assertions for each namespace (construct, then assert the
   exact byte sequence), plus parse-back round trips including a 16-byte-guard `U`
   key and an `R` key with an interval-bearing key_bytes segment.

## Out of scope

Commit logic using these keys (PRDs 07–09), point reads (PRD 10).

## Passing criteria

- `[shape]` `FORMAT_VERSION == 1`; no code path reads or converts version-0 stores.
- `[shape]` The `R` layout contains no target relation id; grep for `target_rel`
  in `keys.rs` returns nothing.
- `[shape]` `rg -i 'restrict|constraint' crates/bumbledb/src/storage/keys.rs`
  returns no identifier hits.
- `[test]` Golden byte tests for `U` and `R` keys as specified, including the
  16-byte interval cases and the permutation-ordered `R` key.
- `[test]` A `MAX_GUARD_WIDTH` boundary test: a guard exactly at the limit builds;
  the validation-side rejection (PRD 03) references this same constant (assert the
  constant is imported there, not duplicated).
