## estimates() carries the doc comment of the deleted skip_free() eligibility method

category: incoherence | severity: low | verdict: CONFIRMED | finder: engine:plan-ir
outcome: fixed 73215a30

### Summary

The public accessor `ValidatedPlan::estimates()` in `crates/bumbledb/src/plan/fj.rs` is documented as "Whether a suffix skip can never cross a node — the pipelined executor's eligibility." That is the doc comment of `skip_free()`, a boolean method that no longer exists. When `skip_free()` was deleted, its doc lines were left behind and silently reattached to the next item, `estimates()`. The sealed plan witness (`ValidatedPlan`, "the plan witness execution trusts", fj.rs:297-298) now publishes a wrong contract for its introspection reader in rustdoc.

### Evidence

- `crates/bumbledb/src/plan/fj.rs:368-373`:
  ```rust
  /// Whether a suffix skip can never cross a node — the pipelined
  /// executor's eligibility.
  #[must_use]
  pub fn estimates(&self) -> &[u64] {
      &self.estimates
  }
  ```
- Git history pins the mechanism exactly:
  - Commit `92eb25cb` ("PRD 09: the pipelined executor") added `pub fn skip_free(&self) -> bool` with this doc comment, placed immediately above `estimates()`.
  - Commit `936ede53` ("chore: delete dead configuration", explicitly "delete the plan-wide skip_free statistic") removed the `#[must_use]`, signature, and body of `skip_free()` — but the diff hunk kept the two doc-comment lines as unchanged context, so they now document `estimates()`.
- The real suffix-skip eligibility is per-node, not plan-wide: `crates/bumbledb/src/exec/run/leaf.rs:96` reads `node.suffix_skip == crate::plan::fj::SuffixSkip::Licensed` directly; no `ValidatedPlan` method is involved.
- The actual consumers of `estimates()` are introspection (`crates/bumbledb/src/exec/introspection/into_stats.rs:53`, `plan.estimates().get(node_idx)`) and sink presizing (`crates/bumbledb/src/api/prepared/build.rs:479` and `:485`, `plan.estimates().last()`).
- The field's own doc gives the correct contract: `crates/bumbledb/src/plan/fj.rs:314` — "The planner's per-step estimates (introspection's reader, the 40-execution doc)."
- Contract-doc cross-check: docs/architecture/40-execution.md is the estimates field's cited home; the stale comment instead points at the pipelined-executor eligibility concern, which the per-node `SuffixSkip` enum (fj.rs:291-295) now represents — the representation replaced the plan-wide boolean, exactly the representation-over-control-flow move, and the stranded doc is the only residue.

### Failure scenario

Documentation-only. A reader of the public API (or rendered rustdoc) is told `estimates()` answers a boolean skip-eligibility question, on a type whose entire design premise is that downstream code trusts it without re-checking. No runtime behavior is affected.

### Suggested fix

Replace the doc comment on `estimates()` with the field's own doc at fj.rs:314, e.g.:

```rust
/// The planner's per-step estimates — introspection's reader and the
/// sink-presizing hint (the 40-execution doc).
#[must_use]
pub fn estimates(&self) -> &[u64] {
```
