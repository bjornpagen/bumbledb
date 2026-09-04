# Bumbledb 1.0: the design contract

Status: proposed successor, 2026-09-04. This folder specifies implementation work; it does not claim that work or its release tests have passed. The owner explicitly permits breaking every pre-1.0 representation, API, axiom, and storage format. The goal is the best small core, not maximum churn for its own sake.

## The owner's non-negotiables

1. A set-semantic relational application database: a good data model, LMDB underneath, and the essential core implemented extremely well.
2. Representation before casework. Replace collections of loosely related flags, counters, checks and recovery exceptions with data whose legal transitions are apparent.
3. Compatibility is not a protected asset during this redesign. The format counter may restart, but a new format family must make old files unambiguously incompatible; resetting a number must never accidentally admit old bytes.
4. Floats are part of 1.0, including their actual set, key, query, encoding and aggregate semantics—not an unprincipled native-number escape hatch.
5. Keep LMDB and take its larger-than-memory behavior seriously. No arbitrary 32 GiB database ceiling. RAM is a performance resource, not the definition of a supported database.
6. Remain small. Do not construct a fleet orchestrator, a replacement storage engine, a generic plugin platform, or a new distributed service to avoid thinking carefully about the core.
7. Backup, restore, and migration belong to **`bumbledb-log`**, not the core `bumbledb` engine. A consistent snapshot or admitted-state construction primitive is not a migration framework.
8. Nightly Rust is welcome when it materially improves representation or the machine. Pin and test the compiler; do not add an unstable feature merely to advertise it.
9. Every known audit issue gets an explicit successor disposition and regression obligation. All required release gates must pass before 1.0. A proposal, skipped test, or narrowed comment is not a fix.
10. This phase writes and reviews the complete proposal, then commits and pushes the documentation before implementation begins.
11. The public log product is TypeScript-only in 1.0. Its authoritative implementation remains Rust internally; it has no supported public Rust/C log SDK. Core Rust/TypeScript/C remain. Supply repo-local TypeScript migrations and a small server-side Next.js + Alchemy integration; the Expo/Drizzle comparison is workflow inspiration, not a new mobile target.

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
       TypeScript log SDK and migration runner

       Core separately supports Rust / TypeScript / C
```

The dependency arrow never points from the engine to the log. The core does not know what S3, a tenant routing record, a command receipt epoch, or a schema migration is. The log uses the engine; it does not introduce another implementation of relational semantics. TypeScript does not introduce another implementation of the log machine. TypeScript migration transforms are explicit offline/staged application work, outside the ordinary native command transaction.

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
| Fresh entities | Log-local `FreshRef` placeholders resolved only by a winning decision | No exposed pre-publication reservations, counter-object leases, or ID-burn recovery machine |
| Value equality | Canonical full values; hashes accelerate lookup but do not define equality | Exact set identity also under forced hash collision |
| Binary64 | Canonical NaN, canonical zero, total relational order | One equality/hash/key law across languages and storage |
| Float reductions | Exact mergeable finite accumulator with explicit nonfinite states; one final rounding | Answer does not depend on plan, iteration order, or RAM versus disk execution |
| Text | Canonical UTF-8 owned in live facts, not a mandatory immortal global dictionary | Ordinary data lifetime no longer grows with every historical string |
| Query execution | Disk-native ordered LMDB path; bounded RAM acceleration; one LMDB-backed scratch-map mechanism | Larger-than-memory data changes cost, not denotation or support status |
| Ownership | One owner; separately acquired idempotent borrow capabilities; deterministic release | A stale borrow cannot close a successor or keep an invisible native owner alive forever |
| Hosted cleanup | Epoch-qualified staged objects and a durable authority barrier | An old paused publisher cannot resurrect or publish objects concurrently being collected |
| Retention | Current recoverable state and explicit retained restore points | No default time-window PITR claim or clock-driven deletion policy in the small 1.0 core |
| Migration | Repo-local TypeScript manifest/steps; log-layer freeze/export/transform/admit/import/new-incarnation/cutover | Familiar explicit migration runner, no core migration API, automatic fleet planner, or hidden in-place 0.x upgrade |
| Public log surface | TypeScript only; one internal Rust machine | No public Rust/C log compatibility burden; core language surfaces remain independently qualified |

Detailed algorithms, edge conditions, cost, and proof obligations belong to the corresponding chapters; this table does not prove them. In particular, a single HEAD alone does not solve garbage collection, uncertain publication, or receipt retention. Those need the precise restrictions in [20](20-durable-protocol.md) and [21](21-storage-and-retention.md).

## Three coordinates, not one overloaded generation

- **HeadRevision** changes on every authoritative HEAD update, including maintenance. It is a CAS/metadata coordinate.
- **DecisionStamp** identifies a durable terminal command decision by sequence and digest. It supplies history identity and fresh-entity allocation authority.
- **StateStamp** identifies an incarnation and its application-data revision. It changes on net application-state changes; maintenance, rejection and no-op decisions do not invalidate an otherwise applicable read witness.

An engine snapshot has its own local store identity and coherent local generation. The log binds that snapshot to its history coordinates through atomic materialization metadata. Do not place log-specific identifiers in the core's public relational type system merely to make a diagram uniform.

## The subtraction ledger

The redesign earns its complexity budget by removing mechanisms:

- Per-braid vacant-slot arbitration, retired-slot recreation, scalar vector-sum checkpoint ordering, and split-commit result handling disappear from the public 1.0 log contract.
- The independent TypeScript protocol machine disappears; the binding transports data and lifecycle operations to the Rust owner.
- Public Rust/C log bindings disappear from the selected 1.0 product. TypeScript owns migration authoring/ergonomics, not another durability algorithm.
- Numeric writer IDs as fences, separate fresh-ID counter objects, escaped reservation burns, and immortal C callback tombstones disappear.
- Default immortal text interning, full-relation RAM images as a requirement, and an arbitrary database-size cap disappear.
- Expiring filesystem mutation leases and checks immediately before an unconditional rename disappear in favor of ordinary enforced local ownership.
- Time equality sentinels, default 90-day restore claims, and a second clock-based retention machine disappear. Explicit retained roots replace the promise; the capability change is public.
- No new custom page engine, external sort engine, fleet scheduler, always-on migration service, or schema-specific remote index is introduced. A repo-local migration runner is an ordinary finite application tool.

Some inherent cases remain: another writer won; the network outcome is unknown; storage is full; a caller cancelled; a restore point was explicitly released. Represent these as data with explicit transitions. Do not hide them behind a boolean named `valid` or claim types can prevent a remote failure.

## What “everything passes” means

All tests required by the selected supported 1.0 contract pass on the exact release artifacts and supported platform/backend matrix. Superseded 0.x behavior is not retained merely to keep an old golden green; its semantic intent and safety counterexamples must be ported or explicitly accounted for.

No known correctness, isolation, durability, or resource-lifetime defect may be waved through by renaming it. An obsolete mechanism can be removed, but the replacement must pass the adversarial property that exposed it. A supported backend lacking credentials or a runner is **unqualified**, not passed. See [70 — Test and release gates](70-test-and-release-gates.md).

This is not a claim that testing proves the absence of all unknown bugs. It is a refusal to knowingly publish 1.0 with unclosed evidence.
