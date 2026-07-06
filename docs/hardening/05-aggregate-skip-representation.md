# PRD 05 — Aggregate skip-legality by representation

Findings fixed (docs/audit/executor.md): **NOTE** "Aggregate skip-legality
rests on a single point of enforcement"; **NOTE** "Phase 1 hashes (and counts)
probes that will never use the hash"; (docs/audit/sink-pipeline.md) **NOTE**
"ProjectionSink's unconditional SkipSuffix is safe only jointly with the
executor" — the same coupling, seen from the other side.

## Purpose

D2's suffix skip is *illegal under aggregation* — today that rule holds only
because `AggregateSink::emit` happens to return `Continue` at both exits. The
executor's `sink_relevant` gating bits are computed from the group key even
for aggregate plans, so any future sink change would be *honored* into
silently wrong sums. The elegant fix is the semantic one: for an aggregate
plan, **every variable is sink-relevant** — the fold is defined over the
distinct full binding set, so no node's bindings are skippable, and the bits
themselves should say so. Encode the illegality in the data the executor
already reads.

## Technical direction

- **`sink_vars` for aggregate plans = all query variables.**
  `api/prepared.rs:352` passes `witness.group_key()` as the plan's sink vars;
  that is correct for projections (the projected set) and *wrong in spirit*
  for aggregates (the group key is the projection of the *output*, not the
  relevance set of the *fold*). When the find list contains any aggregate,
  pass the full variable set (the witness knows every var). Consequence in
  `plan/fj.rs`: every node's `sink_relevant` bit is true → any `SkipSuffix`
  that ever arrives under an aggregate plan is absorbed at the very node that
  produced it — structurally harmless, no matter what any future sink does.
  This is not defensive redundancy; it is the true semantic statement the
  paper's fold requires.
- **The debug tripwire stays cheap:** in `run_node`'s skip-absorption arm,
  `debug_assert!` that the plan's sink kind is projection when a `SkipSuffix`
  actually *crosses* a node (i.e., is not absorbed immediately) — with the
  all-relevant bits this is unreachable for aggregates, and the assert
  documents why.
- **Both coupling comments get the cross-reference:** sink.rs's "safe only
  jointly with run.rs:504-515" and run.rs's absorption comment each name the
  other file AND this PRD's rule ("aggregate plans mark every node
  sink-relevant — the bits encode the illegality").
- **Phase-1 hash waste on pinned rows.** `run.rs:410-426`: branch phase 1 per
  sibling on `matches!(cursor, Cursor::Row(_))` — pinned siblings skip
  `hash_key` and the `probe_hash` counter entirely (one branch per sibling
  per batch, as the audit priced). EXPLAIN's `hashes` statistic becomes what
  its name says; update the CoverStats/NodeStats field doc from "keys
  gathered" ambiguity to "hashes computed for map probes".

## Non-goals

New Flow variants; changing ProjectionSink's first-emit skip signal (audited
sound and load-bearing for D2's win); per-tuple branching in phase 2.

## Passing criteria

- Structural pin: for an aggregate-shaped prepared query, a test-visible
  accessor (or plan Debug inspection in a unit test at the fj level with
  all-vars sink set) shows **every** node `sink_relevant == true`; for the
  same body with a plain projection, the existing narrower bits are
  unchanged (pin one node false — the D2 win still exists for projections).
- Behavioral pin: the aggregate results over a corpus with deep non-projected
  suffixes are unchanged (existing differential + verify-S cover this; add
  one targeted case — an aggregate whose body has a node binding only
  existential vars — asserting equality with the nested-loop reference).
- The pinned-row hash branch: counters test — an FK-walk-shaped plan where a
  sibling is pinned (`Cursor::Row`) reports `hashes == 0` for that subatom in
  `CoverStats` while probes still count; results unchanged.
- Every existing executor/sink test passes verbatim; the D2 skip tests for
  projections are untouched. `scripts/check.sh` green.
