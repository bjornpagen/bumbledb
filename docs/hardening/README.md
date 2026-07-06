# The hardening PRD suite

The deep correctness audit (docs/audit/, 2026-07-06, ten reports) returned:
1 CRITICAL, 1 HIGH, 12 MEDIUM, 16 LOW, 35 NOTE — with the core algorithm clean
and every finding carrying a verified scenario. This suite fixes all of it the
representation-first way: where the audit offered a checked-at-runtime fix and
an unrepresentable-by-construction fix, we take the second. Findings the audit
explicitly closed as "no action / recorded as designed" are honored as such and
folded into the final documentation sweep rather than re-litigated.

`docs/audit/` is the evidence base — every PRD cites its findings by report and
title. `docs/architecture/` stays the design authority; PRD 10 reconciles it
and closes the audit ledger.

## The fixes, in order

| PRD | Title | Kills |
|---|---|---|
| 00 | Environment identity: branding, locking, honest create | the CRITICAL (cross-env memo aliasing), api-schema's unbound-PreparedQuery LOW, storage's foreign-env create LOW, the multi-process corruption NOTE, the nested-write deadlock LOW |
| 01 | Counter escape discipline: no-op serial flush, mint-free deletes | api-schema MEDIUM (serial re-issue), api-schema LOW (delete-side interning) |
| 02 | Unbound views: prepare pins nothing | the parked-COLT image-pinning MEDIUM (sink-pipeline + concurrency, same finding) |
| 03 | The plan boundary rejects what the executor cannot run | plan MEDIUM (dropped gate occurrence), plan LOW (unknown OccId panic) |
| 04 | COLT state defenses: tagged tokens, hard start | colt MEDIUM (token reinterpretation), colt LOW ×2, colt NOTEs (position_matches, Row/max) |
| 05 | Aggregate skip-legality by representation | executor NOTE (single-point skip enforcement), executor NOTE (phase-1 hash waste) |
| 06 | Corruption is a typed error, everywhere | storage LOW (short-key panics), image LOW (row-count trust), api-schema LOW (RelationId panics), sink-pipeline LOW (u32 offset panics), counter asserts |
| 07 | The stamp knows what it vouches for | the HIGH (stamp code identity), oracle LOW (divergence-by-error panics), oracle LOW (NUL literals), oracle NOTEs (param functions outside digest, zero-case stamps) |
| 08 | Three new families: spread, triangle, true balance | oracle MEDIUMs (no cross-atom residuals, no cyclic join), oracle NOTE (balance is not a balance) |
| 09 | The generator earns its coverage contract | oracle MEDIUMs (gates never false / no empty relations, U64 aggregates + the Sum-range hole, the per-(op,type) matrix), oracle LOW (boundary maxima) |
| 10 | Reconciliation: docs true, ledger closed | every doc/comment finding across all ten reports, the planner DP micro-fixes, the alloc-gate one-test guard, the audit resolution ledger |

## Rules

1. **No smoke-test or end-to-end PRDs.** Humans own e2e. Every passing
   criterion is a unit/integration test in the workspace, a `scripts/check.sh`
   gate, or a structural assertion. (In-crate concurrency and differential test
   *families* are regression nets, not e2e — they are in scope.)
2. **No migrations, ever.** Format- or corpus-affecting changes re-baseline
   pinned digests in the same PRD; stores are regenerated, never migrated.
3. **No transitional shims, no atomic-passing-state ceremony.** PRDs are work
   organization. Cut straight to the end state; if the tree does not typecheck
   between PRDs, that is fine — converge where the work converges.
4. **Representation first.** Prefer making the illegal state unrepresentable
   (branding, `Option`, type-level) over checking for it; prefer deleting a
   hazard over asserting around it. Runtime checks are the fallback, typed
   errors the floor, panics only for programmer invariants the docs sanction.
5. **Every verified repro becomes a test.** The audit demonstrated the CRITICAL
   end-to-end; that repro (and every scenario concrete enough to encode) must
   be a regression test in the fixing PRD.
6. **Verify stays green; re-baselines are deliberate.** PRDs 07–09 change the
   stamp, the family digest, and the corpus contract — each re-pin is named in
   its commit. The full-S verify test gates every commit via `check.sh`.
7. **No wall-clock assertions.** Same as ever.

Humans own after 10: re-running `scripts/bench.sh`, the L-scale claim, and any
decision to publish. The audit reports in `docs/audit/` remain as the record,
closed out by PRD 10's resolution ledger.
