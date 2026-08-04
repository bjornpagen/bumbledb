## interval_measure trusts start <= end: an inverted general interval tail underflows instead of convicting corruption

bug | low | CONFIRMED | lean-capacity-drift
outcome: fixed c9a78a67

### Summary

`interval_measure` (crates/bumbledb/src/storage/commit/judgment.rs:164-182) computes `Ok(end - start)` after checking only the tail width (via `IntervalTail::words`) and the ray (`end == u64::MAX`). The general 16-byte arm of `IntervalTail::words` (crates/bumbledb/src/schema.rs:214-229) never validates `start <= end` — only the fixed 8-byte arm is fully checked (`decode_fixed_interval_start`, crates/bumbledb/src/encoding/decode.rs:72-81, `checked_add` + the Q2 bound). A stored fact whose general interval tail is inverted (`end < start`, `end != MAX`) therefore panics the judgment in debug builds and wraps to a near-2^64 measure in release (the workspace profile sets no `overflow-checks`), instead of the typed `MalformedValue` corruption the same function raises for a wrong-width tail two lines earlier.

### Evidence (verified)

- crates/bumbledb/src/storage/commit/judgment.rs:170-181 — `tail.words(bytes)` miss → `Error::Corruption(MalformedValue("capacity interval field"))`; `end == u64::MAX` → `CapacityRayMeasure`; then bare `Ok(end - start)`. No ordering check anywhere between decode and subtraction.
- crates/bumbledb/src/schema.rs:218-223 — the general arm: two `u64::from_be_bytes` reads, `Some((start, end))` unconditionally after the width check. The doc comment (schema.rs:209-213) lists only "wrong width, or a fixed start at or past the Q2 bound" as malformed shapes.
- Root Cargo.toml — no `[profile.release] overflow-checks`, so release subtraction wraps; debug panics.
- Reachability with on-disk bytes (the write path cannot mint an inverted interval — `decode_interval_u64`/`Interval::new` reject `start >= end`, decode.rs:40-47, and encode goes through the checked host type — so this is purely the corruption class):
  - **Live commit, parent side**: `Checker::check_capacity` → `resolve_hi` (judgment.rs:1199-1210, `CapacityBound::TargetDuration`) calls `interval_measure` on parent fact bytes fetched from LMDB (judgment.rs:1130). A bit-flipped parent row inverts a live commit's capacity verdict: the wrapped `hi` (~2^64) makes the ceiling effectively infinite, silently accepting what should have been judged — or the judgment panics in debug.
  - **verify_store, child side**: `check_marks` (crates/bumbledb/src/verify_store/facts.rs:229) calls `expected_slot_weight` → `child_weight` → `interval_measure` on the stored fact's `DurationOf` field. The `Err(_)` arm there (facts.rs:239-242) catches only the ray refusal; an inverted tail returns `Ok(wrapped)` and produces a garbage `derived` weight.
  - **verify_store, parent side**: `check_marks` (facts.rs:269) runs the same `check_capacity`/`resolve_hi` path per ψ-selected parent; the `Ok(()) | Err(Error::Corruption(_))` arm (facts.rs:294) never sees an error because none is raised.
- The sweeper's convict-not-crash discipline is genuinely breached: the F pass DOES convict the inverted field first — `decode_field` returns `CorruptionError::InvalidInterval` and the pass records `Malformed { what: "F fact interval" }` (facts.rs:62-81) — but the comment says "Keep walking after a finding" and the loop does not `continue`, so `check_marks` still executes on the same corrupted fact bytes.

### Corrections to the original claim

- An **R key** never reaches `interval_measure`: R-key interval tails feed only the coverage/intersection comparisons (`ss < te && ts < se`, judgment.rs:588-593), which are order-blind. The underflow surface is F-row fact bytes only (weight field, bound field).
- In a release-build `verify_store`, the report is not *missing* the malformed-content finding — the F pass records `Malformed("F fact interval")` naming the row, and the garbage-measure `CapacityViolation`/`ReverseEdgeWeightDesync` lands **beside** it, not instead of it.

### Failure scenario / impact

A bit-flip inverts the halves of a stored general interval in a `DurationOf`-weighed child row or a `TargetDuration`-bounded parent row.

- **Debug build**: `verify_store`'s F pass records the `Malformed` finding, then `check_marks` panics on `end - start` underflow — the sweep aborts mid-run and no report is returned. A debug-build commit touching the corrupted parent panics the same way.
- **Release build**: `verify_store` returns the legitimate `Malformed` finding plus a spurious finding carrying a near-2^64 measure (`CapacityViolation` or `ReverseEdgeWeightDesync` with garbage `derived`). A release-build **commit** whose capacity judgment resolves `hi` from the corrupted parent gets a silently wrong window — a ceiling of ~2^64 accepts everything — with no error and no finding at all, since the online path has no `decode_field` sweep in front of it.

Severity stays low: reachable only via on-disk corruption. But the online commit-path exposure (silent wrong verdict, no conviction anywhere) is strictly worse than the sweeper exposure the finding led with.

### Suggested fix

Convict the inverted shape where the data enters, matching the module's own trust-boundary doctrine: either in `IntervalTail::words`' general arm (`(start <= end).then_some((start, end))` — making the existing `MalformedValue("capacity interval field")` arm in `interval_measure` catch it, and tightening every other `words` caller for free) or as an explicit `end < start` check in `interval_measure` beside the ray check. One register compare, zero allocation, zero cost on the hot path. Note the `words` route also hardens the coverage walk's tails (currently order-blind but harmless); if that breadth is unwanted, the `interval_measure` route is the minimal fix. Land with a test: a store fixture with a hand-inverted interval tail must yield `Malformed`/`MalformedValue` — not a panic, not a wrapped measure — through both `verify_store` and a commit judging the corrupted parent.