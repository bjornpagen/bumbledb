# PRD 08 — The late-stager sweep: σ-literals and flags into `Resolved`

**Depends on:** baseline only (independent of Phases A/B; may run any time).
**Modules:** `schema.rs` (`Resolved`, `Statement`), `schema/validate.rs`,
`storage/commit/judgment.rs` (`Selections::encode`), `storage/commit/plan.rs`.
**Authority:** the staging audit (2026-07-10), `30-dependencies.md`.
**Representation move:** the staging law applied to the checker. The audit
found the commit path re-computing stage-1-fixed data every commit; this PRD
moves each item to open-time and leaves the commit path consuming
constants.

## Context (decided shape) — the audit's items, each with its verdict

1. **σ-selection literal encodings** (`Selections::encode` re-runs
   `encode_literal` for bool/int/interval literals every commit —
   `judgment.rs:147-151` at audit time). **Fix:** validate pre-encodes every
   non-interned selection literal into the sealed `Statement` (a
   `Box<[SelectionCheck]>` computed once); `Selections::encode` shrinks to
   resolving ONLY `str`/`bytes<N>`… note `bytes<N>` is inline (never
   interned) post-algebra — so only `str` literals remain commit-resolved
   (dictionary state is per-database); everything else is a precomputed
   compare against sealed bytes.
2. **`pointwise`/`coverage` booleans** re-derived per fact per commit from
   `interval_position` (`plan.rs:196/230/282` at audit). **Fix:** two `bool`
   fields on the `Resolved` arms, set at validate, read at plan/judgment.
3. **`FactLayout` rebuilt per open.** **Verdict: stays** — open is rare,
   the rebuild is pure and cheap; record the ruling in the doc amendment so
   the audit line is discharged, not forgotten.
4. **fresh→FD materialization at validate though inputs fix at expansion.**
   **Verdict: stays** — materialized ORDER is a fingerprint input and the
   fingerprint contract pins it at validate; record the ruling.

## Technical direction

1. Extend the sealed `Statement` (schema.rs) with
   `checks: Box<[CompiledCheck]>` where
   `CompiledCheck = Encoded { field, bytes: Box<[u8]> } | Interned { field,
   tag: (), text: Box<str> }` — validate builds it by walking each side's σ;
   `Selections::encode` becomes: copy `Encoded` as-is; resolve `Interned`
   through delta-then-committed dict (the existing miss ⇒
   `SelectionCheck::Never` behavior preserved verbatim).
2. `Resolved::Functionality` gains `pointwise: bool`;
   `Resolved::Containment` gains `coverage: bool`; delete every
   `interval_position.is_some()` at the consumption sites (grep-driven; the
   audit cites plan.rs:196, :230, :282, :105 — re-locate by mechanism name).
3. Do NOT change judgment semantics anywhere: this PRD is
   behavior-preserving by construction, and the differential suite is its
   safety net — no oracle changes are needed or permitted.

## Passing criteria

- `[test]` A commit against a σ-bearing theory produces byte-identical
  verdicts before/after (fixture replay: same op stream, same typed
  verdicts including statement ids — captured as a unit test with a
  hand-built stream, not a smoke test).
- `[shape]` `encode_literal` is unreachable from the commit path for
  non-str literals (grep: its commit-path caller list is exactly the
  `Interned` arm); `interval_position.is_some()` appears only in validate.
- `[test]` The `Interned`-miss path still yields `SelectionCheck::Never`
  (existing test relocated, asserted unchanged).
- `[shape]` The two stays-verdicts (items 3–4) are recorded in
  `30-dependencies.md`'s enforcement notes.
- `[gate]` Workspace gates green at campaign close.

## Doc amendments (rule 5)

`30-dependencies.md`: the enforcement summary notes compiled checks
("selection literals are sealed at validate; only interned text resolves at
commit") and carries the two recorded stays.
