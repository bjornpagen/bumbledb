# PRD 15 — CommitPlan: compute, don't accumulate

**Depends on:** 05 (the net-disposition delta is what makes the derivation
pure), 10 (the sweeper exists; its shared helpers are consumers).
**Modules:** new `crates/bumbledb/src/storage/commit/plan.rs`,
`crates/bumbledb/src/storage/commit/{applier.rs,judgment.rs,write.rs}`.
**Authority:** `50-storage.md` (the commit pipeline), `00-product.md`
(representation over control flow — SICP's "control flow into data" applied to
the write path).

## Context (decided)

The commit currently *accumulates* its bookkeeping imperatively while applying:
`deleted_guards`/`inserted_guards` sets are inserted into mid-loop
(`applier.rs:54,142`) and threaded through phase signatures; the judgment phase
then consumes the accumulation. But with the net-disposition delta (PRD 05),
every one of those sets is a **pure function of (delta, schema)**: guard bytes,
reverse-edge keys, per-statement check sets, and the source-probe list are all
derivable from fact bytes and statement descriptors before a single LMDB page
is touched. Accumulation-during-apply is control flow standing in for a value.

Honest boundary, stated up front: row ids are **not** derivable (deletes need
the `M` lookup; inserts mint from the high-water) and judgment probe *results*
obviously need final-state reads. The plan precomputes every derivable **key
material and check set**; the applier keeps the id plumbing and the probes.

## Technical direction

1. **The derivation:** `fn plan_commit(delta: &WriteDelta, schema: &Schema) ->
   CommitPlan` — pure, no LMDB imports, no `heed` types. Per delete: fact
   bytes + its guard keys per key statement + its reverse-edge keys per
   satisfied containment. Per insert: the same, plus which containment
   statements' source probes it owes (with pre-permuted target key bytes) and
   which pointwise keys need neighbor probes. Aggregated: the per-statement
   disestablished-guard check sets (deleted − inserted, with the ψ-qualified
   re-establishment inputs marked for the judgment phase). Selection literals
   pre-encoded once here (this subsumes the commit-local literal scratch —
   through the one canonical encode path, PRD 01).
2. **The applier becomes a dumb executor:** iterate the plan's ops, do LMDB
   puts/dels with id plumbing, run the desync probes. No set-building, no
   guard derivation, no selection evaluation remains in the loop — if the
   applier computes anything derivable, the derivation is incomplete. The
   desync checks' meaning sharpens and their comments should say so: storage
   disagreeing with what the plan *proved* is unambiguously corruption.
3. **Judgment consumes the plan:** `check_source` iterates the plan's
   source-probe list; `check_target` iterates the plan's per-statement check
   sets. The phase signatures collapse (the threaded-accumulator parameters
   die — this discharges the "phase-output struct" seam PRD 17 was told to
   hunt; note it there).
4. **Unit-testable without LMDB:** the derivation gets direct tests — delta in,
   plan out, byte-level assertions on guard/edge keys and check sets — the
   class of test the accumulate-during-apply shape could never have.
5. **Perf note:** the derivation allocates into commit-scratch exactly as the
   accumulated sets did (same data, computed earlier); commits are not the
   warm path, but keep the arena discipline the delta already uses. Flag the
   commit-path hunks for the closing re-bench (write families are timed).

## Passing criteria

- `[shape]` `plan.rs` has no LMDB/`heed` imports; the applier contains no
  guard/edge derivation and no set-accumulation (grep `guard_bytes`,
  `permuted_guard_bytes`, selection evaluation under `applier.rs` — zero
  hits); judgment's inputs are plan fields.
- `[test]` Derivation unit tests: byte-level plan assertions for a delta
  covering scalar keys, pointwise keys, satisfied and unsatisfied selections,
  `==` pairs, and the delete+insert re-establishment shape.
- `[test]` Every existing commit/judgment test green **unchanged** — the
  refactor is behavior-preserving by construction and the suite is the proof.
- `[gate]` Workspace gates green; hunks flagged for re-bench.
