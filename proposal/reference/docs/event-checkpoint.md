# Event revision checkpoint — pause and resumption record

Branch: `codex/event-algebra`. No release, tag or version bump.
The full M0–M8 objective is unfinished and active again. The sections below
preserve the pause; the final section records the user-authorized resumption.

## Current changes

- Fixed finite conditioning, nonnegative likelihood/Pearl revision, Jeffrey
  replacement, owned receipts and explicit posterior-to-prior translation maps.
- Signed finite-function expectations retaining payoff, evidence and exact masses.
- Nine new core tests and a persisted Coup posterior/utility query consumer.
- Revisions.lean: 30 checked rational reports; the native suite has 334 reports
  across 19 modules, alongside the unchanged 246 proposal reports.
- Proposal/status reconciliation, including stale import/cache/Pack claims.
- A Node runtime race found during qualification: cancelled-operation reclamation
  now wakes cleanup workers, including when the removed output is an error.
  The targeted regression passes with the fix and fails without the notification.

## Validation at the pause

The native semantic run and proposal readiness audit pass. Core revisions,
persisted source integration and targeted Node wakeup tests pass. The initial
full qualification exposed the runtime race; its failed record is preserved.
A first negative-control test missed the wakeup because `wait_timeout_while`
can discover a changed predicate only at timeout yet report success. The corrected
regression checks the direct wait timeout; its negative control fails as intended.

The latest full qualification was stopped at the user's checkpoint request.
Its completed commands are recorded in
[event-evidence/native-revision-qualification-wakeup/check.json](event-evidence/native-revision-qualification-wakeup/check.json).
It is not a fully passing qualification. The earlier interrupted attempt and both
negative-control runs remain preserved. No native source edits should be needed
merely to resume validation.

## Resume sequence

1. Rerun the full `check-event-maps.py --relations --queries` qualification into
   a new evidence directory. Do not overwrite prior attempts. Inspect every failure.
2. Reconcile current evidence links/statuses and refresh the proposal package if
   its documents change. Re-run readiness for changed native/proposal documents.
3. Run the staged-only checkout proof/package/readiness verification, which has
   not yet run for this checkpoint, and verify qualification hashes against staged
   sources before making any completed-checkpoint claim.
4. Continue the full implementation gates from `proposal/implementation-plan.md`:
   parameterized sources/solver capabilities, TypeSafe adapters and provider
   receipts, owned query observations and expectation aggregation, function/kernel/
   receipt/SDK transport, remaining dependency placements, general fixed-point
   query binders, certified factoring, belief memory and complete M8 qualification.

Current host revision receipts are mathematical records, not provider import
receipts or persisted provenance. Posterior laws and translation maps already
use BEVT v2 and BEDC. A total FiniteFunction's zero default does not satisfy the
separate query-roster coverage contract. Do not mark M6, M7 or the goal complete.


## Resumed from 8378ae1f0

The user explicitly resumed the full implementation goal. The
[resumed qualification](event-evidence/resumed-revision-qualification/check.json)
passed all fourteen steps, and the
[fresh staged checkout](event-evidence/resumed-revision-portable/check.json)
passed package, proposal, native proof and readiness checks. These records qualify
the checkpoint's source state, before subsequent source-transport implementation.
The historical interrupted runs above remain preserved.

Implementation continued with [BESC v1 source descriptors](event-source-descriptors.md).
The active ledger tracks current evidence and remaining gates; this pause record
does not redefine completion around an intermediate commit.
