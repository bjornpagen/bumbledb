# Frozen implementation handoff — 2026-09-04

The owner stopped the implementation campaign and requested a preserved source
checkpoint plus a refactored, parallel execution proposal for a different
orchestrator. Engine, SDK and replication agents are stopped. Do not interpret
this checkpoint as a working successor or a release candidate.

The source parent is `80e6b750584f0bfde1cfbc8d2ea27291824ea09d` on
`codex/bumbledb-1-0`. The preservation commit containing this note intentionally
includes incomplete source. No tests, typechecks, builds or performance probes
were run for this handoff. Its `[skip ci]` message is deliberate: verification
was deferred by the owner. A final implementation candidate must not skip CI.

## What exists, and what does not

| Area | Preserved work | Missing or unsafe to assume |
| --- | --- | --- |
| Foundations | Canonical F64 kernel; checked row codec; WorkContext; Arc-backed ChangeSet; writer prepare/seal/commit substrate; direct Effect error classes; release-result inventory | Whole successor representation, complete bounded execution, final history machine and release qualification |
| Engine packet | Exact F64 sum/mean integration; compact separate float accumulator bank; independent rounding fixtures; shared ScalarExpr/NumericCast/ScalarError/ScalarEvaluator; partial computed sink | Computed module declaration, FindSpec/EitherSink preparation/execution, whole-query numeric guard, exhaustive matches; Interior.rules still projection-only; no completed float intervals, storage rewrite or owned snapshots |
| Native ownership | Shared runtime owner/operation/FS scaffolding; managed DB access; bounded owner/handle registry draft | Worker-affine persistent sessions, safe owned snapshots, complete close/drain, complete cancellation/byte accounting; legacy AsyncTask and JS owners remain |
| TS core | Typed F64 fields/literals/parameters and mean; stricter mixed numeric typing; authored float tests | Effect-only public API, ScalarExpr export, final schema/codec/result API, complete pure metadata versus bounded work separation |
| Filesystem store | Rust WorkContext/Accounted operations; bounded reads/hash/staging/lock waits; cross-language regressions | TS still uses a different mutation authority. All five complete FS operations must delegate to native; no JS-held lock across await |
| TS migrations | Draft schema.ts and migrations/{types,intent}.ts | Draft imports nonexistent core ScalarExpr; no generator, canonical codec, repository writer, native executor, exports or qualification |
| Log product | Existing 0.x machinery plus groundwork | LocalHistory/HostedHistory successor, receipts, checkpoint/GC/recovery rewrite, Effect log/tenant surface and migration/backup implementation |

The unfinished files are design inputs, not frozen interfaces. Complete or
replace them according to `final-solution/`; do not preserve a bad scaffold for
compatibility. Do not discard independent regressions or historical evidence.

## Concrete integration hazards

- `crates/bumbledb/src/api/prepared/computed.rs` is not yet declared as a module.
  Computed finds/sinks and native/macros/bench/Lean matches need coordinated work.
- `ts/crate/src/runtime_wire.rs` accesses private `Operation.external`; there may
  also be a public DbLease/private DbInner visibility mismatch. These are known
  source-review concerns, not current compiler diagnostics.
- `DbHandle` has transitional `DbOwner::Legacy` and `Managed` variants. Managed
  legacy `db_close` starts close but does not constitute a joined callback close.
  Delete the transitional surface in the completed cutover.
- Engine/Db and OwnedInstance are Send+Sync; InstanceBuilder is Send, not Sync.
  PreparedQuery is !Send/!Sync through Rc<PipeTables>; ReadInstance is !Send/!Sync;
  WriterSession is !Send through its guard and thread identity. Preserve actual
  ownership constraints. Never add unsafe Send to move transactions between
  interchangeable runtime workers.
- A hosted attempt that retains a prepared writer over remote I/O needs a real
  owner-affine session. One registry does not itself supply affinity.
- TS and Rust filesystem CAS currently do not share authority. Rust uses
  `~lease/<key>/mutation.lock`; TS uses numeric tokens, `~head`, and unconditional
  rename. A paused TS old-value read followed by Rust CAS and resumed TS CAS can
  acknowledge both updates. This is not merely TTL expiry.
- Draft migration labels (for example `0001-note-pinned`) are stable human IDs,
  not digests. Plan/schema/prefix commitments are separate 32-byte identities.
  Draft hash-domain names and private wire declarations are not frozen bytes.

## Historical evidence, not qualification of this tree

Earlier foundations are recorded in [04](04-successor-foundations.md) and native
ownership constraints in [05](05-native-ownership-resume.md). Preserve both.

- Earlier core TS run: 430 passed. Earlier log TS run: 167 passed, one failed,
  six S3 skips. The failure was renewable tenant directory ownership loss.
- Earlier workspace nextest: 2,153 passed, one failed, 30 skipped; run
  `ba8ab1f4-03d2-42d2-b787-ab2b6ee701db`. The mixed-fleet CAS counter ended at 31
  after 32 acknowledged swaps.
- Additional deterministic interop regressions failed before the no-testing
  instruction: paused real TS read/Rust swap/TS resume, and a poisoned or
  symlinked mutation lock accepted only by TS. Run
  `e4f0698c-c8dc-40d1-b71c-ae45d9241e5b`; local historical log
  `/tmp/bumbledb-interop-deterministic-red.log` is not a durable checkout artifact.
- Earlier float core checks and four focused tests passed, including 317
  independent reduction fixtures. Later scalar/sink/native/migration changes
  were not verified. Current source is expected to require integration repair.
- Earlier Lean corpus: 277 cases without disagreement. This does not prove new
  derived-stage or mutable-support contracts.
- Some earlier nextest runs reported LEAK (stdio not reaching EOF promptly).
  A separate Darwin pipe/CLOEXEC inheritance reproduction exists, but it does
  not attribute those historical incidents. Do not suppress leaks, enlarge
  timeouts or serialize tests to manufacture green. TOKIO_WORKER_THREADS=1 also
  changes child test runtime behavior and is not a runner-only solution.
- The two CI runs on `80e6b750` (33936275547 and 33936275523) were canceled when
  verification was deferred. That commit fixes the reviewed optional
  msgpackr-extract install-script policy, not successor correctness.

Both packages remain 0.20.3 with Effect exactly `4.0.0-rc.112`. Owned TS sources,
scripts, tests, manifests and locks had their `@superbuilders/errors` dependency
removed in the foundations checkpoint; final qualification must verify absence.
`implementation/release-results.json` remains the single evidence ledger. No
required successor obligation is qualified by this note.

## Resumption

Use [the execution plan](../final-solution/60-implementation-and-release-plan.md)
and the root [PROMPT](../PROMPT.md) after the proposal refactor is committed.
Read the current source rather than assuming old agent work is complete. Author
tests alongside implementation, but execute the verification campaign only
after all implementation lanes are integrated. There are no active agents to
wait for and no background implementation to inherit.
