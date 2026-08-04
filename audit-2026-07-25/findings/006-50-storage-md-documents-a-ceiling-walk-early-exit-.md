## 50-storage.md documents a ceiling-walk early exit the C14 ruling and the code both reject

incoherence | medium | CONFIRMED | capacity-judge
outcome: fixed f720787f

### Summary

The commit-step-3 capacity bullet in `docs/architecture/50-storage.md` describes the child-group measure walk as "stopped as soon as the verdict is decided — sound early exit because non-negative weights make the running sum monotone: a ceiling walk exits at sum > hi, a floor walk at sum ≥ lo". This contradicts both the recorded C14 ruling and the shipped judge: only a FLOOR-ONLY walk (no ceiling present) clips early; any walk with a ceiling always completes, including its floor check, so the witnessed measure on conviction is the full, walk-order-independent group sum. This is live drift, not a stale page — the same bullet cites the C17 measured slot law (resolved 2026-08-01), so it was written or touched after the capacity cutover, and the campaign's audit doctrine treats in-repo architecture docs as the spec of record.

### Evidence

- **The doc's claim** — `docs/architecture/50-storage.md:390-392`: "stopped as soon as the verdict is decided — sound early exit because non-negative weights make the running sum monotone: a ceiling walk exits at sum > hi, a floor walk at sum ≥ lo". The bullet cites "the C17 measured law" at line 389, dating it post-cutover.
- **The C14 ruling** — `docs/design/capacity-laws.md:395-397` (C14, measure parity): "on conviction the judge completes the full walk so the reported measure is walk-order-independent (the clip serves the verdict, the full sum serves the witness)." Also `docs/design/capacity-cutover.md:21`: "both differential twins carry the witnessed measure whole (C14)."
- **The code** — `crates/bumbledb/src/storage/commit/judgment.rs` (note: the file is `storage/commit/judgment.rs`, not `exec/judgment.rs` as the raw finding said; the cited line numbers match this file exactly):
  - Doc comment 1219-1225: "a FLOOR-only walk exits the moment `sum ≥ lo` ... a CEILING walk always completes — deciding `sum ≤ hi` needs the whole group anyway, and on conviction the full sum IS the witness ... (ruled 2026-07-24, C14)".
  - 1257-1258, the only clip predicate: `let floor_only_decided = |measure: u128| hi.is_none() && measure >= u128::from(statement.lo);`
  - 1264-1289, the walk loop: the only exits are the prefix-end `break` (1266-1268) and `floor_only_decided` (1286). No `sum > hi` exit exists; a floor check under a present ceiling never clips either.
  - 1169-1175: the violation record carries this `measure` — so the engine's reported witness on a ceiling conviction is the full group sum.

### Failure scenario / impact

A reader, the TS/Lean wall, or a re-implementation following 50-storage.md builds a ceiling walk that exits at first exceedance and reports the partial running sum as the violation measure. That partial sum depends on walk order over weighted entries — exactly the witness instability C14 was ruled to prevent. Conformance fixtures or differential twins pinned against the doc's semantics would disagree with the engine's reported `measure` on every weighted ceiling conviction. The doc also silently misdescribes the floor-under-ceiling case (no early exit there either), compounding the drift.

### Suggested fix

Rewrite the sentence in `docs/architecture/50-storage.md:390-392` to the C14 form, e.g.: "clipped only when the walk is floor-only — with no ceiling, `sum ≥ lo` is final and no witness is owed; any walk with a ceiling completes the whole group, because deciding `sum ≤ hi` needs the full sum and on conviction the full sum IS the witness (C14: the clip serves the verdict, the full sum serves the witness; walk-order-independent measure)." Keep the existing Lean citations (`capacity_plan_decides`, `capacity_plan_consultations`).