# 16 — `verify_store` findings: structural variants still transcribe; dangling ids ride raw `u64`

- **Status:** OPEN (verified 2026-08-19 17:05 EDT; the tree is hot).
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
