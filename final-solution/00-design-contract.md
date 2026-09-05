# Bumbledb 1.0: the design contract

Status: proposed successor, 2026-09-04. This folder specifies implementation work; it does not claim that work or its release tests have passed. The owner explicitly permits breaking every pre-1.0 representation, API, axiom, and storage format. The goal is the best small core, not maximum churn for its own sake.

## The owner's non-negotiables

1. A set-semantic relational application database: a good data model, LMDB underneath, and the essential core implemented extremely well.
2. Representation before casework. Replace collections of loosely related flags, counters, checks and recovery exceptions with data whose legal transitions are apparent.
3. Compatibility is not a protected asset during this redesign. The format counter may restart, but a new format family must make old files unambiguously incompatible; resetting a number must never accidentally admit old bytes.
4. Floats are part of 1.0, including sum, mean, parameterized float intervals, and their actual set, key, query and encoding semantics—not an unprincipled native-number escape hatch.
5. Keep LMDB and take its larger-than-memory behavior seriously. No arbitrary 32 GiB database ceiling. RAM is a performance resource, not the definition of a supported database.
6. Remain small. Do not construct a fleet orchestrator, a replacement storage engine, a generic plugin platform, or a new distributed service to avoid thinking carefully about the core.
7. Backup, restore, and migration belong to **`bumbledb-log`**, not the core `bumbledb` engine. A consistent snapshot or admitted-state construction primitive is not a migration framework.
8. Nightly Rust is welcome when it materially improves representation or the machine. Pin and test the compiler; do not add an unstable feature merely to advertise it.
9. Every known audit issue gets an explicit successor disposition and regression obligation. All required release gates must pass before 1.0. A proposal, skipped test, or narrowed comment is not a fix.
10. This resumed phase updates/reviews the proposal before further implementation. Preserve already-started implementation changes without extending them; commit/push only the reviewed documentation, never release/tag/publish from this phase.
11. Rust and TypeScript are the only public languages. Hard-delete the entire C API, not merely deprecate it. The public log product is TypeScript-only; its authoritative implementation remains Rust internally. Internal Node native-boundary safety remains required.
12. Users declare schemas/types, not migrations. A high-level TypeScript schema SDK generates canonical migration plans and repo-local history, in the same AST-first style as the query SDK. No SQL/textual query parser, authored migration callbacks, arbitrary JavaScript transformation runner, or new compiler framework.
13. This is a per-student/per-user application database, not an analytics warehouse. Preserve excellent warm Free Join execution. Apple Silicon is the first optimization target; ARM Graviton and x86 Vercel are canonical portable targets, with specialized tuning deferred. The M2 Max ledger in `../bumblebench` supplies regime-specific evidence and methodology, not constants valid on every chip.
14. Supply a small server-side Next.js + Alchemy integration and qualify Vercel's Node deployment. The Expo/Drizzle comparison is workflow inspiration, not a new mobile target. Serverless local disk and cold materialization are measured workload constraints, not reasons to invent a remote page engine.
15. Revisit the in-repo README/benchmarks, policy constants and storage amplification. Distinguish justified indexed-admission cost from redundant representation; size each hash by its role, population and threat model. Hardware-accelerated candidates must win actual workloads, not reputation.
16. Keep exact grouped-measure constraints: count is unit weight, not a separate weighted-relation semantics. Normalize harmless supported spellings instead of policing them across languages. Preserve nonnegative exact measures, scalar grouping and meaningful domain restrictions; do not add time-varying occupancy, arbitrary query assertions or a weighted-bag engine.
17. Queries produce typed relations that compose, including nonrecursive aggregate outputs used by later queries. Names do not mandate materialization. Preserve distinctness, group, rounding and error boundaries; retain only the positive finite-active-domain linear recursive fragment, with no value creation, negation or aggregation through its cycle.
18. The usage layers are porous by reuse, not by bypassing authority. The log imports the core's values, schema/query/change types, canonical codecs, read interface, results and execution policy. It adds history identity and publication, never a parallel fact/query/change API. Rust and TypeScript syntax receive a side-by-side review before implementation.

19. Both TypeScript packages are Effect 4-only, using the inspected exact RC dependency. Pure schema/query/intent metadata stays synchronous; database/codec/ingestion/hash work is bounded and off the event loop. Scope owns resources, completed answers use bounded page Streams, core primitives/runtime are shared by log. No Promise/sync/disposal compatibility twin or optional Effect adapter.
20. Be idiomatic Effect and performance-aware together: operation/page granularity, stable V8 record shapes and bounded conversion; no per-row fibers/spans, redundant validation, layer-per-tenant workers or second cache. Measure overhead on the actual app workload.

The two supplied copies of the representation-first essay are byte-identical (SHA-256 `a931bb20a66d732fa66961fac6e1e249f1fee1166f920f313ce46b943fd663c3`). Its principle is the design method here. Its historical quotations are user-supplied reference material, not newly verified scholarship.

## The smallest coherent successor

```text
Application facts and declared laws
                 ↓
       bumbledb: the small engine
       canonical values / admitted theory
       final-state judgment / query evaluation
       LMDB transactions and snapshots
                 ↑
       bumbledb-log: optional durable history
       one tenant authority / immutable commands
       receipts / materialization / retained roots
       checkpoint, backup, restore, migration
                 ↑
       TypeScript schema/query SDK and generated plans
       TypeScript log SDK and migration runner

       Core separately supports Rust / TypeScript
```

The dependency arrow never points from the engine to the log. The core does not know what S3, a tenant routing record, a command receipt epoch, or a schema migration is. The log uses the engine; it does not introduce another implementation of relational semantics. TypeScript constructs schema/query/plan data; it does not introduce another implementation of the log machine. Migration execution consumes generated data at the log layer, outside an ordinary live application command. The generator must refuse ambiguous business intent rather than guess it.

The core owns `SchemaId` and ordinary application `Id128` values as well as its schema/query AST, sealed `ChangeSet` and `CompleteResult`. A log command is an envelope around that same change set; a published log snapshot satisfies the same core query-read interface while adding log stamps. Shared helpers need no row lifting, remarshal DSL or log-specific query/result class. Porosity never grants a writable core owner through a log handle. The [syntax review](34-sdk-syntax-and-composition.md) makes this boundary concrete.

## Binding successor decisions

These decisions coordinate the detailed chapters. They are proposed requirements, not descriptions of current 0.x behavior.

| Decision | Selected representation | Consequence |
| --- | --- | --- |
| Hosted durable authority | One never-deleted conditional-update HEAD per logical database incarnation | Many competing writers, tenant-wide atomic decisions; no vacant per-braid slot as publication authority |
| Hosted durable history | Immutable single-parent decision objects over a coherent checkpoint and bounded tail | No remote page-storage engine or universal history DAG |
| LocalHistory authority | Facts, receipts and log attachment in one durable LMDB transaction | No S3 object epochs, replay envelope or mandatory full checkpoint merely to reopen an embedded database |
| Application command | Owned immutable canonical data, with stable command identity and optional expected state | No replay of arbitrary host callbacks; mutation of caller buffers cannot alter meaning |
| Local candidate | Uncommitted LMDB write transaction on its owning worker | Existing committed readers cannot see a losing candidate; no full database copy per attempt |
| Read-dependent intent | Explicit expected `StateStamp` for the whole observed tenant state | No implied serializable host read/compute/write from schema validity alone |
| Publication outcomes | Named terminal decisions, plus explicit unresolved transport/lifecycle outcomes | Byte equality and timeout do not manufacture ownership or rejection |
| Entity identity | Application-owned nominal 128-bit values, generated once before command sealing | No FreshRef, allocator, issuance receipt, reserved/burned range, or history-dependent entity codec; UUID uniqueness is probabilistic, not a protocol theorem |
| Value equality | Canonical full values; hashes accelerate lookup but do not define equality | Exact set identity also under forced hash collision |
| Grouped constraints | Exact nonnegative measure of distinct matching child facts per selected parent; count uses unit weight | One measure family, canonical internal windows; zero total is not absence, and total duration is not simultaneous occupancy |
| Query composition | Typed relation-expression dependencies, with explicit restricted linear closure | Nonrecursive aggregate results can feed further queries; no projection-only CTE product or compulsory full materialization merely because an expression is named |
| Hash width | 16-byte local fingerprints with exact comparison; 32-byte authoritative BLAKE3 commitments; transient routing hashes remain separate | Save repeated index bytes without weakening content-identity commitments; AEGIS is evaluated before format freeze, not silently substituted by CPU |
| Binary64 | Canonical NaN, canonical zero, total relational order | One equality/hash/key law across languages and storage |
| Float reductions | Exact mergeable finite accumulator with explicit nonfinite states; one final rounding | Answer does not depend on plan, iteration order, or RAM versus disk execution |
| Text | Canonical UTF-8 owned in live facts, not a mandatory immortal global dictionary | Ordinary data lifetime no longer grows with every historical string |
| Query execution | Warm Free Join and selective indexed access as primary app paths; complete bounded LMDB fallback and one scratch-map mechanism | Preserve the performance experiment's strengths; larger-than-memory data changes cost, not denotation or support status |
| Ownership | One owner; separately acquired idempotent borrow capabilities; deterministic release | A stale borrow cannot close a successor or keep an invisible native owner alive forever |
| Hosted cleanup | Epoch-qualified staged objects and a durable authority barrier | An old paused publisher cannot resurrect or publish objects concurrently being collected |
| Retention | Current recoverable state and explicit retained restore points | No default time-window PITR claim or clock-driven deletion policy in the small 1.0 core |
| Migration | Schema SDK → canonical schema diff → generated plan/history → log-layer staged execution and explicit cutover | No user-authored migration code; one final destination per pending batch, with necessary intermediate validation but no needless intermediate publication |
| Public log surface | TypeScript only; one internal Rust machine | No public Rust/C log compatibility burden; core language surfaces remain independently qualified |
| TypeScript execution | Exact Effect 4 dependency, core NativeRuntime service, scoped capabilities, completed-result page Streams | No manual wrapper layer, Promise/sync/AsyncDisposable twin or ambient transaction; schemas stay pure, native work stays bounded |
| SDK reuse | Core primitives imported by log; one shared native Node artifact/runtime per supported platform/version | No log row builder, scalar codec, query dialect, result class or second addon engine; Rust core remains log/AWS-independent |

Detailed algorithms, edge conditions, cost, and proof obligations belong to the corresponding chapters; this table does not prove them. In particular, a single HEAD alone does not solve garbage collection, uncertain publication, or receipt retention. Those need the precise restrictions in [20](20-durable-protocol.md) and [21](21-storage-and-retention.md).

## Three coordinates, not one overloaded generation

- **HeadRevision** changes on every authoritative HEAD update, including maintenance. It is a CAS/metadata coordinate.
- **DecisionStamp** identifies a durable terminal command decision by sequence and digest. It supplies history identity, never entity-allocation authority.
- **StateStamp** identifies an incarnation and its application-data revision. It changes on net application-state changes; maintenance, rejection and no-op decisions do not invalidate an otherwise applicable read witness.

An engine snapshot has its own local store identity and coherent local generation. The log binds that snapshot to its history coordinates through atomic materialization metadata. Do not place log-specific identifiers in the core's public relational type system merely to make a diagram uniform.

## The subtraction ledger

The redesign earns its complexity budget by removing mechanisms:

- Promise/synchronous TS data APIs, AsyncDisposable ownership, generic consumer Effect wrappers, separate TS cursor classes and duplicated cancellation channels disappear. The Effect runtime is supplied by the app; the native runtime still owns actual resources/work.
- Per-braid vacant-slot arbitration, retired-slot recreation, scalar vector-sum checkpoint ordering, and split-commit result handling disappear from the public 1.0 log contract.
- The independent TypeScript protocol machine disappears; the binding transports data and lifecycle operations to the Rust owner.
- The entire public C product disappears: crate, headers, exports, examples, packaging and dedicated release workflow. Public Rust log bindings also disappear; internal Rust remains tested. TypeScript owns schema authoring and generated migration ergonomics, not another durability algorithm.
- Numeric writer IDs as fences, FreshRef placeholders, 28-byte issued IDs, separate counter objects, reservation burns and issuance-only receipt cases disappear. Native Node owners still need bounded lifetime handling; deleting C is not a proof of Node safety.
- Handwritten migration modules, user-maintained relation-coverage lists and helper-closure/purity machinery disappear. Generated canonical data is the deployable artifact; ambiguous intent is declared in the schema evolution metadata or rejected at generation.
- Separate log change recording/value encoding and SDK-level row lifting disappear; commands wrap the core's sealed changes, and migrations execute the core's typed operators. Core public exports do not acquire log-only protocol types.
- Capacity spelling-ban tables and projection-only named-query special cases disappear where canonical normalization and typed composition express the same supported meaning. Real arithmetic, ownership and recursion restrictions remain; no generic assertion interpreter is introduced.
- Default immortal text interning, full-relation RAM images as a requirement, and an arbitrary database-size cap disappear.
- Expiring filesystem mutation leases and checks immediately before an unconditional rename disappear in favor of ordinary enforced local ownership.
- Time equality sentinels, default 90-day restore claims, and a second clock-based retention machine disappear. Explicit retained roots replace the promise; the capability change is public.
- No new custom page engine, external sort engine, fleet scheduler, always-on migration service, or schema-specific remote index is introduced. A repo-local migration runner is an ordinary finite application tool.

Some inherent cases remain: another writer won; the network outcome is unknown; storage is full; a caller cancelled; a restore point was explicitly released. Represent these as data with explicit transitions. Do not hide them behind a boolean named `valid` or claim types can prevent a remote failure.

## What “everything passes” means

All tests required by the selected supported 1.0 contract pass on the exact release artifacts and supported platform/backend matrix. Superseded 0.x behavior is not retained merely to keep an old golden green; its semantic intent and safety counterexamples must be ported or explicitly accounted for.

No known correctness, isolation, durability, or resource-lifetime defect may be waved through by renaming it. An obsolete mechanism can be removed, but the replacement must pass the adversarial property that exposed it. A supported backend lacking credentials or a runner is **unqualified**, not passed. See [70 — Test and release gates](70-test-and-release-gates.md).

This is not a claim that testing proves the absence of all unknown bugs. It is a refusal to knowingly publish 1.0 with unclosed evidence.
