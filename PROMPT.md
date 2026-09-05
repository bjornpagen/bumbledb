# Implement the Bumbledb successor — orchestrated goal prompt

You are the orchestrator for one selected proposal, not a new architecture
brainstorm. Fully implement the contracts in `final-solution/`, including all
known audit obligations, both public SDKs, internal log, proofs, tests, packaging,
examples and performance qualification. Use deep parallel fanout where the
environment supports it. You are accountable for integration and the final
result, not merely assigning work or accepting agents' “done” messages.

This prompt is intended to be given to Claude Fable or another coding
orchestrator. Do not assume tools or unlimited agents exist: discover actual
capacity and use the available delegation mechanisms. If delegation is not
available, execute the same packets yourself and say so; do not simulate agents.

## Start from the actual stopped branch

Read `implementation/06-frozen-implementation-handoff.md` first. Source was
preserved in `4b127782` on `codex/bumbledb-1-0`, on top of `80e6b750`; the later
proposal commit contains this prompt. The tree includes incomplete source and
known failures. It is not a clean 1.0 foundation. Inspect current Git state and
preserve user changes; never reset the branch to the checkpoint. Previous
engine/replication/SDK agents were stopped; do not wait on or assume their work.

Then read all of `final-solution/README.md`, `00`, `01`, `02`, `50`, `60`–`64`,
`70`, `90`, all detailed subsystem chapters, and the complete `audit/`. Read
implementation/04–05 for earlier evidence and native affinity constraints.
Do not treat historical test results or partial scaffolds as current passes.

The executable plan is:

- `61-orchestration-and-dependency-graph.md`: phase machine, dependencies,
  contract-first pipeline, ownership and resumption rules.
- `62-work-packets.md`: P00–P14, exact default source domains, broad deliverables,
  deletions, authored tests, handoffs and all 68 audit IDs' primary owners.
- `63-shared-interface-contracts.md`: C01–C12, one shared meaning/owner per seam.
- `64-final-verification-and-handoff.md`: final-only execution, exact-artifact
  evidence, performance decisions, Git handoff and proposal retirement.

These scheduling documents supersede historical “benchmark early” or serial
milestone wording. Detailed semantic chapters remain binding. Chapter 35 owns
the exact Effect API interpretation; chapters 50/70 are the complete obligation
set, not optional suggestions. Resolve contradictions explicitly instead of
letting different agents choose different architectures.

## Non-negotiable product

Build a small, excellent **set-semantic relational application database**:
LMDB underneath, warm Free Join and selective probes first, disk-backed bounded
fallback when working sets grow. One database per user/student/tenant. Do not
turn this into an analytics warehouse, remote page engine, fleet platform or
generic framework. Representation before casework; each new mechanism must
replace machinery or satisfy an existing selected guarantee.

- Public core: Rust and TypeScript. Delete all C product/code/artifacts.
  Public log: TypeScript only; one internal Rust protocol, not a public Rust log
  SDK. Core Rust must not depend on log/AWS.
- Canonical full bytes decide equality, including under forced hash collision.
  Live text, application-owned Id128, no FreshRef/counter issuance authority.
  One same-command normalized final set effect; complete final-state judgment.
- First-class F64, canonical NaN/zero, total order, explicit casts, deterministic
  exact sum/mean rounded once, parameterized dense float intervals. No implicit
  bigint/number coercion, approximate float capacity or fixed float intervals.
- Exact grouped measures over sets, not weighted bags. Typed nonrecursive
  relation stages including aggregate/computed outputs. Only positive linear,
  projection-only finite recursive feedback; preserve all grain/error/rounding
  boundaries. No new textual query parser.
- Elastic LMDB, safe map resizing, real coherent owned snapshots and one
  RAM/temporary-LMDB scratch facility. No arbitrary 32 GiB/RAM database ceiling.
  Preserve Apple Silicon performance; qualify portable Graviton/x86 Vercel Node.
- LocalHistory facts/receipts/attachment use one LMDB transaction. HostedHistory
  uses one S3 HEAD with multiple competing writers, immutable decisions,
  checkpoint plus bounded tail. No braids, split outcome or second JS machine.
- Owned sealed commands and stable refs; retained receipts distinguish decided,
  not-submitted and unknown. Semilattice properties do not prove keys/capacity
  union-closed or turn deletion/read-dependent business intent into free merge.
- Backup/restore/migration stay in log. Users declare schemas and declarative
  ambiguity/loss intent; generator emits canonical repo-local plans. One final
  staged migration target with required intermediate checks; explicit activation;
  durable abort fences activation/delayed genesis before source thaw.
- Effect **4.0.0-rc.112** is the exact selected required TS peer/dev dependency
  unless deliberately requalified. Both packages are Effect-only. Pure schema/
  query/intent metadata is synchronous; all work is lazy, scoped and bounded.
  No Promise/sync/disposal twin or per-operation hidden Effect runtime.
- Core owns actual schema/query/scalar/ChangeSet/codec/QueryReader/result/runtime
  primitives; log imports them literally. One exact-version native addon/runtime.
  No duplicate log DSL, JS cache/lease authority, untracked AsyncTask or per-row
  fibers/spans. Bounded owned page Streams after complete query evaluation.
- Direct Effect error classes/reasons; no `@superbuilders/errors` anywhere in
  maintained code, scripts, tests, manifests or locks. Preserve causes and
  publication certainty; interruption and finalizer Cause are not rejection.
- Rust nightly is allowed when useful; pin/justify it. Default hash roles are
  16-byte exact-checked local fingerprints and 32-byte authoritative BLAKE3.
  Measure AEGIS/physical layout in the final phase before physical bytes freeze;
  never invent benchmark results or make formats depend on the host CPU.

Read the pinned Effect 4 shipped docs/source cited in chapter 35. Prefer its
actual service/layer/callback/Scope/Stream/Schema/error APIs to remembered Effect
3 patterns. Read `../edullm` bronze and the actual explanation/learner database
consumers for idiom, and `../bumblebench` for performance evidence. Those sibling
repositories are read-only inputs, not authorized edit/deploy targets. If an
input is unavailable, report it and use the documented contract; do not invent
what it contains or silently upgrade dependencies.

## Orchestrate aggressively, with exclusive ownership

Create `implementation/campaign-status.md` with phase, exact write ownership,
live agents, contract handoffs and remaining dependencies. Dispatch P01–P14
according to chapter 61 at maximum **useful actual** concurrency. P00 is your
job: shared hubs, integration, decisions, cross-review and final evidence.

Give agents the entire bounded packet, not one compiler error or test at a time.
Publish C-contract declarations first so dependent SDK/log/migration authors can
implement concurrently while producers finish behavior. Independent tests,
models, docs and performance harnesses can be authored immediately. Do not wait
for the entire engine to finish before beginning TS or log work.

With three workers, use broad engine/history/SDK bundles and schedule independent
proof/test/packaging/performance review as capacity frees; do meaningful local
integration work yourself. With more workers, split disjoint packet domains.
Children may fan out only with exclusive paths and useful independent work.
One writer per file, including root modules/manifests/locks/generated corpora.
P00 resolves overlapping ownership before edits. Do not allow “everyone fixes
the typechecker” to become simultaneous hub rewrites.

Use this dispatch template, filling every field:

```text
Implement packet Pxx from final-solution/62-work-packets.md.
Current phase: F0/F1; execute no tests/typechecks/builds/lint/probes.
Read: [full normative chapters, audit rows, child gates, C-contracts].
Exclusive write paths: [actual files/directories].
Excluded/shared hubs: [owner and how to request edits].
Inputs already declared: [C IDs, symbols and actual source paths].
Deliver the complete packet behavior, deletions and authored tests.
Publish needed interfaces promptly; notify named consumers of changes.
Do not commit/push, expand scope, waive tests or create fake production adapters.
Keep implementation/packets/Pxx.md current using chapter 61's handoff format.
Return exact changed paths, supplied contracts, authored tests/gate mapping,
review concerns, unfinished boundaries and Verification: NotRun.
```

If an agent stops, preserve its source and handoff, then resume that packet from
the actual files. Do not erase another agent's work or repeat completed work
blindly. Source review can reveal defects; fix them in the owning lane. An
interface-only stub, TODO, unchecked cast, no-op adapter or test-only happy path
is not completion. Never invent unsafe Send/lifetimes to satisfy the runtime.

## No execution until the entire implementation is integrated

**Do not run any tests or typechecks until the very end.** Concretely, no builds,
linters, benchmark/probe executions, generating by executing project code,
package lifecycle hooks or CI pushes during F0–F2. Reading source/docs and
writing tests/fixtures/harnesses is allowed and required. No subagent exception.

Enter F3 only after every selected implementation packet and authored suite is
integrated, all declarations have real behavior, deletions are complete and
all 68 audit IDs plus all 17 parent/220 child gate families have coverage owners.
Record and announce the barrier. Then run the **full** final campaign in 64,
repair failures, and repeat affected checks until genuinely qualified. Final-only
means one final verification phase, not one attempt or permission to skip tests.

Use existing batteries/native/Lean/packed consumer machinery plus missing
backend/large-data/platform/performance lanes. Author those missing lanes before
the barrier. Preserve independent oracles, exact float bits, real process
schedules, actual Node artifacts and S3 evidence. Do not disable/skip/suppress
failures, weaken golden expectations, fake resource reclamation or count missing
credentials as success. Performance measurements serialize per host and compare
equal semantics/durability. Format/hash choices remain provisional until those
final probes; a change requires implementation and requalification, not prose.

## Finish honestly

You own the final integrated code/evidence commit and push. No per-agent or
implementation-phase checkpoint churn. Chapter 64 explains the final-stage local
candidate commit, repairs before push, exact source/artifact evidence and an
optional evidence-only handoff commit. Do not force-push published history.
Final implementation pushes must run CI; `[skip ci]` in the old preservation
commits is not permission to skip qualification. Inspect actual CI results.

This goal authorizes repository implementation and Git handoff, **not** package
publication, a 1.0 version bump/tag, production migration, sibling-repo writes,
cloud provisioning or spending. Ask for a specific disposable scope if required
external qualification cannot proceed safely. Missing required external evidence
blocks release; finish all safe local work and report the precise missing gate.

Preserve `audit/` permanently. Keep `final-solution/` until implementation and
required qualification are complete. Before its eventual removal, transfer
normative contracts/obligation inventory into permanent docs and update the
release checker, which currently reads this folder. Qualify that change too.
Never delete the proposal or ledger to conceal unfinished work.

The final report must state what shipped on the branch, exact commit(s), which
verification actually ran and on what artifacts/targets, every unresolved gate,
and whether it is implementation-complete, pre-promotion-qualified or blocked.
“Ready for 1.0” is earned by the full evidence set, not by confidence, a large
diff, agent consensus or a green subset. Do not stop merely because agents
returned; finish integration, review, qualification and the authorized handoff.
