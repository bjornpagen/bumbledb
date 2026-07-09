# 11 — The elegance pass (rebuild-seam refactor)

**Kind:** behavior-preserving refactor — the codebase was rebuilt by twenty-five
isolated work units in sequence; no single unit could see the seams between them.
This item is the deliberate cross-cutting read that removes them. Quality bar:
utmost — the standard is that the code reads as if one author wrote it in one
sitting, in the house style (terse, doc-comment-heavy, representation-first).

**Constraint: strictly behavior-preserving.** No semantics change, no new
features, no error-shape changes. The unsafe-allowlisted hot modules
(`exec/kernel.rs`, `exec/colt.rs` gather/probe, `exec/wordmap.rs`, `exec/run.rs`
leaf/batch, `image.rs` decode, `obs.rs` fast clock) are touch-only-with-cause:
a hot-path refactor needs a reason stronger than taste, and anything that could
plausibly move a measured number gets flagged for the re-bench rather than
assumed neutral.

## The known seam classes (hunt these first)

- **Near-duplicate helpers across crate boundaries built by different units:**
  e.g. commit's `satisfies` selection check vs the naive model's selection
  evaluation (deliberately independent — algorithms must not be shared with the
  model; but *within* the engine, duplicated selection/guard/slicing logic is a
  defect); scratch-harness test utilities re-invented per module's test dir.
- **Idiom drift between modules:** error-construction patterns, iterator vs
  index-loop styles, `expect` message conventions, doc-comment voice, test naming
  and fixture-construction styles that differ per work unit.
- **Altitude misplacements:** logic living in the caller that belongs in the
  callee's type (or vice versa) because two units negotiated an interface without
  a third view; over-wide function signatures threading state a struct should
  own.
- **Dead weight:** parameters no caller varies, enum variants no site constructs,
  pub items with one internal caller, comments narrating the obvious.
- **Test overlap:** the per-unit criteria tests plus later integration tests may
  triple-cover some behaviors while the seam between them covers nothing — merge
  and redirect, never just delete.

## Method

Subsystem-sized passes, each read whole before any edit: (1) schema + encoding +
error, (2) storage (delta/commit/judgment/keys/env/read), (3) ir + plan,
(4) exec + image, (5) api + macros, (6) bench crate (naive, translate, querygen,
families, harness). Per pass: read, list findings, apply, gates green
(`fmt`/`clippy -D warnings`/`cargo test --workspace`), commit — one commit per
subsystem so the diff stays reviewable.

## Acceptance

- Gates green after every subsystem commit.
- Zero behavior changes: no test's *assertion* changes (test code may be
  restructured; expected values may not).
- A findings summary per subsystem in the commit body: what was deduplicated,
  what moved, what died — so the review is of decisions, not diffs.
- Hot-module changes, if any, individually justified in the commit body and
  listed for the closing re-bench.
