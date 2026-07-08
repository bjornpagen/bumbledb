# PRD 24 — The new ledger benchmark schema and families

**Depends on:** 05 (macro), 20 (bind surface), 22 (translator).
**Modules:** `crates/bumbledb-bench/src/{schema.rs,gen/,families/,corpus.rs,tripwires.rs}`.
**Authority:** `docs/architecture/60-validation.md` (§ the primary benchmark — the schema and family list are normative there).

## Goal

The bench crate's ledger matches `60-validation.md`: the statement-notation schema
with the `Mandate` temporal surface, and the new families (param-set, negation,
Arg-restriction, interval membership/overlap) alongside the ported originals.

## Technical direction

1. **Schema** (`schema.rs`): transcribe the `60-validation.md` schema block
   exactly — nine relations, the eight containments, and the pointwise key
   `Mandate(account, active) -> Mandate`. Serial fields per the block. Delete the
   old constraint-based construction.
2. **Corpus generation** (`gen/`, `corpus.rs`): existing distributions carry
   over; `Mandate` generation must emit per-account interval histories that are
   **valid under the pointwise key** (sequential non-overlapping segments, a mix
   of abutting and gapped, some ending at `MAX_END` — the "currently active"
   convention) — since invalid corpora abort at load, generate validity by
   construction. Seeded and reproducible as today.
3. **Families** (`families/`): port the existing ten; add, per the doc's list:
   - `entries_for_account_set` — the param-set family (replaces the old
     host-side-union convention; delete that convention's plumbing);
   - `postings_without_tag` — negation;
   - `latest_posting_per_account` — ArgMax;
   - `mandate_at_instant` — membership probe (param point);
   - `mandate_overlap` — Overlaps join between Mandate intervals and a generated
     query window (or Mandate×Mandate across orgs — pick the shape that joins,
     not just filters, and record it in the family's doc comment).
   Each family: the IR constructor, its SQL via the translator (goldens pinned),
   its parameter rotation, and its per-family SQLite index DDL (the honest
   opponent: `(account, active_start, active_end)` composite for the interval
   families — `60-validation.md` protocol).
4. **Tripwires** (`tripwires.rs`): the structural assertions extend to the new
   machinery where the doc's perf decisions require them (selection levels
   engaged for the param-set family; the fast path NOT taken for membership
   families). Do not add wall-clock tripwires.
5. **Empty-store pass:** every new family joins the zero-row verify pass
   (gates false, scans empty, aggregates folding nothing).

## Out of scope

Running verify/bench; L-scale claims; report/viz changes (report code adapts
mechanically to the family list — include only the mechanical adaptation).

## Passing criteria

- `[shape]` `schema.rs` transcribes the doc's schema block statement-for-
  statement (reviewable diff against the doc).
- `[shape]` The host-side-union convention for account sets no longer exists in
  the bench crate.
- `[test]` Corpus validity: a generated S-scale corpus loads without a judgment
  violation (unit-scale variant: 10³ facts in-test); Mandate histories contain
  all three shapes (abutting, gapped, sentinel — asserted structurally).
- `[test]` Family construction unit tests: each new family's IR validates against
  the schema and its SQL golden is byte-pinned.
