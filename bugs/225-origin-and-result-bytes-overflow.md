# OriginOverflow and ResultBytesOverflow are runtime errors Lean does not denote
- id: 225
- severity: low
- confidence: confirmed
- area: spec-docs-rust
- wrong-side: split
- components: crates/bumbledb/src/error.rs, crates/bumbledb/src/exec/run.rs, crates/bumbledb/src/api/prepared/resolve_memo.rs, lean/Bumbledb/Query/Denotation.lean, docs/architecture/70-api.md, docs/architecture/40-execution.md
- status: open (do not fix)

## Summary
The executor can abort a well-typed query with `Overflow(OriginCapacity)` when a D2 origin counter would cross `u32`, and with `ResultBytesOverflow` when answer-byte offsets do not fit `u32`. Lean `eval_sound` / `ruleAnswers` have no such errors — every denotational answer is a tuple. 70-api lists `Overflow` as “aggregate range check” and omits both `OriginCapacity` and `ResultBytesOverflow`. `40-execution.md` still says resource limits are “none in v0” except the fixpoint budget, while result buffers are supposed to grow until the OS is the backstop. These are representation ceilings that make the engine incomplete on large-but-finite answer sets the spec still denotes.

## Lean spec
Silent. Answer tuples are lists of values; no origin counter, no byte-offset width. `eval_sound` (`Denotation.lean:1656+`) equates list evaluation with the set denotation under safety and measure-free bindings.

## Normative docs
`70-api.md:832-837`: runtime query errors include `Overflow` (aggregate range check), `FixpointBudgetExceeded`, `Corruption` — not origin capacity, not `ResultBytesOverflow`. `40-execution.md:969-985`: “Resource limits: none in v0” except the fixpoint budget; “result buffers grow with output; … the OS is the backstop.” D2 skip / origin machinery is described as mechanism; the overflow as a typed query error is easy to miss.

## Rust implementation
`exec/run.rs` `Poison::OriginOverflow` → `Error::Overflow(OverflowKind::OriginCapacity)` (`execute.rs:411-412`, `probe_pass.rs:579-594`). `resolve_memo.rs:73-87` `ResultBytesOverflow` on `u32::try_from` of answer-buffer offsets.

## Why this matters
A query whose Lean denotation is a large finite set can fail with Overflow/ResultBytesOverflow. Hosts distinguishing "aggregate overflow" (documented, Lean `checkedSum`) from "origin/buffer overflow" (undocumented in the API roster) will mis-handle the latter. Rare at validated scale S; the mismatch is structural (typed errors exist; spec and API roster do not).

## Verification (2026-08-12)
Re-read `eval_sound`, the 70-api/40-execution resource stance, and the two raise sites. **Confirmed** (was likely). `wrong-side` corrected to **split**: Lean silent, docs understate the roster/stance, engine aborts.

**Lean:** Silent. Answer tuples are lists of values; no origin counter, no byte-offset width. `eval_sound` (`lean/Bumbledb/Query/Denotation.lean:1656-1663`) equates list evaluation with the set denotation under safety and measure-free bindings.

**Docs:** `70-api.md:832-837` as above. `40-execution.md:969-972` claims result buffers grow until the OS backstop, then amends only for fixpoints (`:973-985`).

**Rust:** `exec/run.rs:681` `Poison::OriginOverflow` → `Error::Overflow(OverflowKind::OriginCapacity)` (`execute.rs:411-412`; mint check `probe_pass.rs:574-594`). `resolve_memo.rs:73-87` `ResultBytesOverflow` on `u32::try_from` of answer-buffer offsets. Comments call both “beyond any validated workload” but “valid input.”

## Related
- 206 (another incompleteness vs Lean eval)
- 210 (runtime error roster)
