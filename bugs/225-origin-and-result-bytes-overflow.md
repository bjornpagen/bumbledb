# OriginOverflow and ResultBytesOverflow are runtime errors Lean does not denote
- id: 225
- severity: low
- confidence: likely
- area: spec-docs-rust
- wrong-side: unspecified
- components: crates/bumbledb/src/error.rs, crates/bumbledb/src/exec/run.rs, crates/bumbledb/src/api/prepared/resolve_memo.rs, lean/Bumbledb/Query/Denotation.lean, docs/architecture/70-api.md, docs/architecture/40-execution.md
- status: open (do not fix)

## Summary
The executor can abort a well-typed query with `Overflow(OriginCapacity)` when a D2 origin counter would cross `u32`, and with `ResultBytesOverflow` when answer-byte offsets do not fit `u32`. Lean `eval_sound` / `ruleAnswers` have no such errors — every denotational answer is a tuple. 70-api lists `Overflow` but not `ResultBytesOverflow` or the origin-capacity kind. These are representation ceilings that make the engine incomplete on large-but-finite answer sets the spec still denotes.

## Lean spec
Silent. Answer tuples are lists of values; no origin counter, no byte-offset width. `eval_sound` (`Denotation.lean:1656+`) equates list evaluation with the set denotation under safety and measure-free bindings.

## Normative docs
`70-api.md:832-833`: runtime query errors include `Overflow` (aggregate range check) — not origin capacity, not `ResultBytesOverflow`. `40-execution.md` D2 skip / origin machinery is mechanism; the overflow as a typed query error is easy to miss.

## Rust implementation
`exec/run.rs` `Poison::OriginOverflow` → `Error::Overflow(OverflowKind::OriginCapacity)` (`execute.rs:411-412`, `probe_pass.rs:579-594`). `resolve_memo.rs:73-87` `ResultBytesOverflow` on `u32::try_from` of answer-buffer offsets.

## Why this matters
A query whose Lean denotation is a large finite set can fail with Overflow/ResultBytesOverflow. Hosts distinguishing "aggregate overflow" (documented, Lean `checkedSum`) from "origin/buffer overflow" (undocumented in the API roster) will mis-handle the latter. Likely rather than confirmed as a user-visible bug at scale S; the mismatch is structural.

## Related
- 206 (another incompleteness vs Lean eval)
- 210 (runtime error roster)
