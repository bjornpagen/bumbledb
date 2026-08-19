# 16 — `verify_store` findings: structural variants still transcribe; dangling ids ride raw `u64`

- **Status:** **fixed this pass** — `StoreFinding` is `Judgment(Violation) | Corruption(CorruptionError)`; structural twins embed; twinless facts are `CorruptionError` variants first; intern ids are `InternId` (`intern_id: u64` gone; stored `SENTINEL` is `Malformed`). Tests: `intern_id_at_or_beyond_the_counter_is_found_with_fact_context`, `a_referenced_id_without_a_reverse_entry_is_the_finding`, `a_sentinel_intern_id_is_malformed_not_a_named_id`, `a_desynced_weight_slot_is_convicted_never_repaired`.
- **Severity:** later (deliberately after the spine work), but filed fully.
- **Supersedes:** REP-03, REP-07.

## Principle

The one-verdict-shape rule that already landed for judgments
(`StoreFinding::Judgment(Violation)`) applied to the rest of the roster: a
finding that is field-for-field a `CorruptionError` is one semantic fact in
two shapes, and every raw-`u64` intern id at a sweep surface is a value the
`InternId` newtype exists to type (the sentinel `u64::MAX` is representable
in findings today).

## Evidence

- `crates/bumbledb/src/verify_store.rs:267` —
  `DanglingInternId { intern_id: u64 }`, doc'd as "the offline twin of the
  runtime `Corruption(DanglingInternId)`";
  `error.rs:115` — `CorruptionError::DanglingInternId(u64)`.
- ~20 structural finding variants still parallel the sweeper's byte-walk
  vocabulary rather than embedding `CorruptionError`.

## The fix

1. `StoreFinding::Corruption(CorruptionError)` absorbs every structural
   variant with a corruption twin; variants with no twin become
   `CorruptionError` variants first (they are corruption facts — the
   sweeper just found them offline).
2. `InternId` at every sweep/report/error surface; the raw-`u64` fields and
   the representable sentinel go.
3. The sweep's per-fact, insertion-ordered reporting discipline is recorded
   and stays — this is about the *element* shape, not the collection.

## Acceptance

- `StoreFinding` is `Judgment(Violation) | Corruption(CorruptionError)` plus
  only variants that are genuinely neither.
- `grep "intern_id: u64" crates/bumbledb/src/` is empty.
- Sweep tests unchanged in verdicts; only payload shapes move.
