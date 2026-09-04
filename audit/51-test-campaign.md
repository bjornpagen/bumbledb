# Regression campaign: test the seams, not only the features

This is proposed follow-up work. These tests have not all been implemented or executed. The existing corpus remains valuable; this campaign extends it with small, discriminating failure schedules.

## First blocking matrix

| Schedule | Required assertion | Primary findings |
| --- | --- | --- |
| Failed publication → new commit on same live handle | Previous pending outcome resolved; no local/log divergence | SDK-001 |
| Encode → yield → mutate caller byte buffer | Logged/applied facts remain identical | SDK-003 |
| Apply candidate → read elsewhere → publication rejection | Ordinary read never sees rejected candidate | SDK-014 |
| Open old writer → other writer/checkpoint/GC → old commit | No published success in retired history | REP-001 |
| Publish incomparable checkpoint vectors after collection | Recovery floor never regresses a component | REP-002 |
| Open checkpointer against recent existing checkpoints | Retention preserves young restore points | REP-003 |
| Two allocators write same next counter; one response ambiguous | Returned ranges never overlap | REP-004 |
| High writer identity then lower identity, repeated blocks | Bounded progress or explicit fencing refusal | REP-006 |
| Pause local holder before rename; successor completes | Old holder cannot overwrite successor | REP-005/010 |
| Crash after checkpoint publication, advance head, reopen | Historical reachable checkpoint not swept as orphan | REP-007 |
| Two delayed checkpoint candidates share predecessor | Retained published history remains discoverable | REP-008 |
| Second open while first owns live scratch | Failed opener has no destructive side effects | REP-009 |
| Case-distinct logical keys on supported local filesystems | Keys remain distinct or backend refuses deployment | REP-011 |
| Reuse same-schema/equal-vector cache under another namespace | No old facts, pending commands, or cleanup cross the identity boundary | SDK-016 |
| Lease handoff racing old tenant disposal | Old owner cannot delete successor state | REP-017 |
| Concurrent source writes during compact | Copied state and metadata share one snapshot | ENG-003 |
| Public value construction → admitted write → ordinary read | No successfully admitted invalid representation | ENG-001/002 |
| Aggregate valid groups then error | No ambiguous partial output contract | QRY-001 |
| Small budget crossed during materialization | Peak work/memory bounded before excessive growth | QRY-002/003 |
| Double/stale release, pool close during open | Borrow isolation and deterministic cleanup | SDK-004/005/007 |
| C read → destroy | Native engine ownership and lock released | SDK-013 |

## Expand each schedule across the relevant axes

- Rust driver and TS driver; shared codec does not remove machine divergence.
- Memory, filesystem and S3 adapter semantics; do not let a forgiving mock be the only oracle.
- Before-write failure, after-write response loss, definitive conflict, ambiguous conflict, failed verification GET, failed response body.
- Reuse the live handle, reopen the same directory, start from a clean directory, and open after retention.
- One and several braids; compare componentwise vectors, not just sums.
- Pending local versus published acknowledgment; provisional state must not acquire a published-read contract accidentally.
- Ordinary and heap-backed engine sources; typed and dynamic input; scalar, string, bytes and intervals.
- Native operation error, domain rejection, cancellation and process interruption.

Use tiny fixtures to keep these tests fast and diagnostic. More random iterations over the same narrow state shape will not discover a missing axis.

## Oracles

1. **Semantic oracle:** independent naive set/constraint evaluator, plus Lean where its fragment applies.
2. **History oracle:** simple state machine tracking calls, outcomes, visible reads, and durable objects.
3. **Resource oracle:** counted growth/work with strict small budgets; actual process disk/RSS checks for lifecycle tests.
4. **Adapter oracle:** conditional-create/swap/fence contract checked independently of the log driver.
5. **Application oracle:** named command effects and request receipts, not merely final row counts.

Do not generate expected histories by executing the same production transition helper. Shared production grammar is fine; independent expected behavior is essential.

## Test instrumentation principles

- Place deterministic barriers before and after consequential operations, not only after large named phases.
- Preserve exact failed schedules as small fixtures with stable finding IDs.
- Record which call was acknowledged and what each reader observed before final convergence.
- On every error path, inspect reusable buffers, pending state, held locks, timers and native owners.
- Model real clock separately from injected logical time. Test wall-clock jumps and actual suspension as different conditions.
- Do not assert only that a function returned an error. Assert no unauthorized state transition and a recoverable next action.
- For destructive maintenance, verify the retained dependency graph before and after partial failure.
- Make skip reporting explicit: a credential-gated test returning early is not evidence of a successful S3 operation.

## Release acceptance

Close the release blockers only when their regressions fail on the audited implementation and pass on the corrected implementation, the opposite-language path is checked, and a fresh replica/restore verifies every published receipt. Then run the full source/build/packed-artifact matrix on a controlled tree.

After correctness closure, run the workload program in `40-performance.md` and retain raw results. Performance sign-off and correctness sign-off are distinct: neither can be inferred from the other.
