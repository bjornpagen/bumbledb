# 33 — Write TypeScript schemas; generate the migrations

Status: proposed 1.0 contracts and release obligations. No source implementation, generated migration, package, cloud deployment or platform qualification is performed by this document. It supersedes the earlier handwritten TypeScript migration-callback design.

## The application product

A developer writes a high-level TypeScript schema/type definition, as they already write typed query values. The tooling generates schema snapshots, migration plans, a manifest and the small deployment expectation. The app keeps importing its ordinary typed schema declarations and gets query/row inference directly; there is no required generated runtime-type/Prisma-style layer. **The user does not write migration files or transform callbacks.** Ambiguous business intent is a typed declarative input to generation, or generation refuses.

The core is public Rust/TypeScript only; the entire C product is removed during implementation. The log product is public TypeScript only, implemented internally in Rust. Preserve the small, fast set-semantic application database: one database per student, user or tenant, embedded locally or materialized beside a Node app with S3 authority. Do not build a warehouse, fleet platform or new query language.

~~~text
schema.ts  →  canonical schema value  →  generated diff-plan data  →  native log executor
authoring     existing typed lowering    checked repo artifacts      freeze/build/validate/cutover
~~~

Generation evaluates the schema SDK's ordinary values; the migration runner loads only inert generated plan/snapshot data. Normal compiled app modules can import/construct schema and query values, as they do today—this is not a runtime TypeScript compiler or permission to run migration logic on open. “No parser” means no SQL/schema/query source-language parser; bounded canonical data decoding remains mandatory at storage and transport boundaries.

## Build on the existing AST-first SDK

This is an extension of the repository's existing approach, not a replacement DSL:

- ts/src/relation.ts constructs frozen named field descriptors.
- ts/src/schema.ts assembles relations, laws and field equivalence classes into a schema value.
- ts/src/spec.ts defines tagged SchemaSpec/value/statement data; lowering produces the engine representation.
- ts/src/query/atom.ts and query/lower.ts build typed rule/expression values and lower them to query IR. Reusing typed variable references creates joins.

Keep those useful structural/law-derived types and the native checked boundary. Current u64.fresh/reserve behavior is removed; application-owned Id128 values are ordinary 16-byte schema values. Float scalar/sum/mean and Interval<F64> follow chapter 11. Generation must not erase law information merely to make a schema diff easier.

## The repository stays ordinary

~~~text
src/db/schema.ts                    authored high-level schema and optional typed intent
src/db/runtime-contract.json        generated exact schema/history expectation
src/db/server.ts                    server-only owner/cache policy
bumbledb/migrations/
  0000-initialize.plan.json         generated canonical data, not executable TS
  0001-note-pinned.plan.json
  meta/0000.schema.json             generated immutable schema snapshots
  meta/0001.schema.json
  manifest.json                    ordered identities/digests
  index.ts                         generated static data imports
scripts/migrate.ts                  small explicit admin runner, not migration logic
next.config.ts
alchemy.run.ts                      when AWS/Alchemy is the chosen deployment
~~~

The high-level schema module is shared by generation and ordinary application queries. It builds typed values without database I/O or migration execution; the application's normal TypeScript build handles it. Generation-only intent metadata can live in a separate export/module and be excluded from the app bundle when useful. Generated plan/snapshot/manifest artifacts are reviewed and checked into the repository; historical ones are immutable. No generated runtime handles or extra type-codegen product is required.

Two ordinary commands are enough for authorship:

1. A proposed bumbledb-log generate command loads schema.ts using the normal TypeScript build tool, compares its canonical schema against the last checked snapshot, and emits the new plan/manifest/snapshot files and small runtime expectation. Structural inference is automatic; ambiguity is reported with the exact typed intent required. A generation-time generateMigrations({ schema, hints }) API is the same operation with repository configuration supplying the prior checked snapshot.
2. A proposed bumbledb-log check command repeats canonical generation in memory and compares outputs without changing files. CI rejects source/generated drift and edits to previously recorded plan identities.

Use the existing TypeScript/bundler toolchain, not a custom compiler, parser, import-closure security framework or JavaScript purity checker. Generation may fail if authoring code cannot be evaluated; deployment never tries to evaluate it as a recovery strategy. Normal package integrity and release provenance still apply, but there is no separate executable migration bundle/source-helper hash/production transform launcher.

## Example: add a field, generate the rest

The constructors below are proposed extensions around the existing relation/schema/key API. Their names are illustrative until implemented and compile-tested. The developer authors schema/type values, not migration procedures:

~~~ts
// src/db/schema.ts — ordinary typed schema declarations shared by app and generator
import { bool, id128, key, relation, schema, str } from "@bjornpagen/bumbledb"
import { backfill, literal, migrationIntent } from "@bjornpagen/bumbledb-log/schema"

export const Note = relation("Note", { id: id128, body: str, pinned: bool })
export const App = schema("App", { Note }, [key(Note, ["id"])])

export const evolution = migrationIntent(App, [
  backfill(Note, "pinned", literal(false))
])
~~~

Given a checked previous schema with Note(id, body), generation emits a native MapRelation plan whose target projection is id = old.id, body = old.body, pinned = false, followed by the required target-law validation. Unchanged relations are automatically copied/preserved in the plan. The developer does not maintain reads/writes/copy/drop/empty lists or write loops over rows.

The expression is a typed literal AST, not a function returning values during execution. More complex supported backfills use the same typed field/expression/query IR vocabulary: field references, literals, checked casts and the explicitly supported deterministic expression fragment. This does not authorize arbitrary JavaScript, SQL text, network fetches, clocks, random values, plugin functions or opaque “run this code” nodes.

Application code imports App/Note from the ordinary schema module. Runtime queries retain the existing query(App).rule(...) construction style and inferred parameter/row types. The generated runtime contract checks that this application's expected canonical schema/history is deployed; it does not replace the schema SDK with generated classes.

### Generation must know what it is allowed to infer

| Change | Generated behavior |
| --- | --- |
| Unchanged relation/field/law | Preserve automatically; no author-maintained coverage list |
| New empty relation | Create as empty, with complete target-law validation; required seed data must be declaratively supplied |
| New required field | Require a typed backfill/default expression unless the generator can prove no source rows exist; no fabricated zero/null |
| Rename | Require explicit old-to-new typed identity intent when it is not already unambiguous metadata; do not guess from matching shapes |
| Remove relation/field | Refuse without explicit destructive intent/acknowledgement; expose what data is discarded in generated review |
| Type change | Use only an explicit supported checked conversion; narrowing/overflow/refusal follows the value algebra |
| Change keys/references/capacity/closed facts | Generate exact validation and necessary declared data changes; existing data may fail, never silently repair/drop it |
| Data-only change | Generate from explicit typed declarative data intent; a schema diff cannot invent business meaning |
| Unsupported transform | Refuse generation with a finite explanation; do not fall back to a handwritten callback or general scripting runtime |

Typed rename/drop/backfill/seed intent is metadata or expression AST attached to the schema-evolution input. It is not an imperative migration file under another name. Fixed seed facts are canonical checked data, with explicitly supplied application IDs where needed. A migration cannot call an ID generator per row; any necessary mapping must be explicit and representable by the supported deterministic plan or generation refuses.

Generation checks complete source/target coverage internally and includes it in the plan. Removing handwritten coverage does not permit forgotten relations to disappear. Ambiguous ID remapping must cover every reference; ordinary changes preserve Id128 bytes. Changes in history incarnation never invalidate application IDs.

## Canonical plans are the executable contract

A generated step contains a plan codec version, stable step ID/sequence, exact source/target SchemaIds, typed operations/expressions, explicit destructive acknowledgements and required validation boundaries. Its meaning is finite canonical data that the native engine can validate before writing. It contains no module paths, closures or executable source text.

Illustrative generated plan data; the exact codec uses the same checked scalar/schema/query representations:

~~~json
{
  "planVersion": 1,
  "id": "0001-note-pinned",
  "fromSchemaId": "<previous-schema-digest>",
  "toSchemaId": "<target-schema-digest>",
  "operations": [
    {
      "kind": "map-relation",
      "source": "Note",
      "target": "Note",
      "fields": {
        "id": { "kind": "field", "name": "id" },
        "body": { "kind": "field", "name": "body" },
        "pinned": { "kind": "literal", "type": "bool", "value": false }
      }
    },
    { "kind": "validate-schema", "schemaId": "<target-schema-digest>" }
  ]
}
~~~

Angle-bracket digests are explanatory placeholders, not valid inputs. Native validation checks plan version, exact schema/field ownership, expression types, complete coverage, allowed operations, size/work bounds and acknowledgements. A well-shaped JSON object alone is not a checked plan.

The manifest records the ordered chain of seq/id/fromSchemaId/toSchemaId/planDigest/prefixDigest. A planDigest is the full 32-byte authoritative content hash over the versioned canonical plan and referenced schema identities, excluding its own digest field. No raw TypeScript text or helper-source closure participates in migration identity.

Prefix hashing is acyclic: begin with a domain-separated base containing the manifest/plan codec and empty-base SchemaId; each next prefix hashes the previous prefix plus the canonical entry excluding its own prefixDigest. A manifest digest, if stored, excludes its own field. planSetDigest identifies the exact ordered pending suffix plus its source/target schema and source-prefix expectation, not a particular tenant's arbitrary local directory. Hash framing, field inclusion and domain/version separation are fixed codec rules with golden vectors. App-owned 128-bit IDs are independent of these 32-byte content hashes.

A generation refactor producing identical canonical plan/schema values does not rewrite applied migration identity. A changed plan under the same ID is drift. Upgrading the executor does not change a plan's meaning; compatible implementation releases must replay its golden fixtures identically. Unsupported old plan codecs refuse before mutation and require an explicit qualified transition.

The generated static index only imports plan/manifest data. It performs no database I/O. The app's small runtime contract contains the exact current schema and applied-prefix expectation; regular runtime code need not bundle historical plans or generation dependencies.

## One authoritative history; one final target

Chapter 22 defines one Applied record for the executed suffix:

~~~text
Applied {
  operationId, planSetDigest,
  sourceIdentity, sourceStamp,
  targetIdentity, targetSchemaId, targetDigest,
  steps: [{ seq, id, fromSchemaId, toSchemaId, planDigest }, ...]
}
~~~

Flattening the batch's steps extends the exact contiguous applied manifest prefix. The final target genesis binds inherited history, that record and the admitted final data together. There are no fabricated intermediate database identities, public roots or per-step “applied” records for private scratch. targetDigest is the canonical application-state digest, not a circular reference to genesis.

History is authoritative local LMDB/log metadata or reachable hosted metadata, retained through checkpoint/backup/restore independently of command receipt retirement. A file beside a disposable cache is not authority. Exact schema plus applied-prefix mismatch yields MigrationRequired, DatabaseAhead or MigrationDrift; open does not auto-repair or auto-migrate.

Initialization executes the generated chain from the declared empty base, including all declarative seeds/closed data. Creating an empty newest-schema database cannot falsely claim skipped plans ran. Explicit baseline/adoption is a separate verified operator claim, recorded as Baseline rather than Applied. Restore preserves prior history and adds restore provenance without rerunning seeds.

### Coalesce without changing semantics

The native executor plans the whole pending suffix as one operation and produces **one final destination and one final genesis**, not a full published database for each numbered file. Consecutive row projections/default additions can fuse; unchanged data can pass through once. Five simple pending schema changes must not cause five compulsory whole-database copies, uploads and incarnation changes.

Fusion preserves the declared step semantics, including intermediate errors and laws. A later drop cannot conceal a narrowing-cast failure or an invariant violation that an earlier step must report. Where a dependency or validation genuinely requires an intermediate, use bounded private scratch/checks rather than a published intermediate history. Do not promise every arbitrary plan is single-pass; explain actual passes, temporary bytes and validation work.

Before target publication, interruption restarts the incomplete operation from its original pinned source under the same planSetDigest. This sacrifices incremental work reuse for a smaller state machine; there is no JavaScript stack journal or per-page migration execution log. Resolve uncertain final publication before restarting. A proven completed target is reused, never regenerated merely because its response was lost.

## Explicit migrate and cutover

~~~ts
// scripts/migrate.ts — proposed admin API, no migration logic here
import { migrate, migrationStatus } from "@bjornpagen/bumbledb-log/migrations"
import plans from "../bumbledb/migrations"
import { loadAdminBinding, loadStableOperationId, migrationPolicy } from "./admin-config"

const binding = loadAdminBinding()
const options = {
  operationId: loadStableOperationId(),
  to: "0001-note-pinned",
  ...migrationPolicy
}
const status = await migrationStatus(binding, plans, options)
const outcome = await migrate(binding, plans, options)
// Persist the structured result in the restricted deployment job.
// ReadyToSwitch contains deploymentBinding + activationRef; it does not activate.
~~~

The configuration helpers/policy are application-owned. The runner consumes generated data and calls the native executor; it does not load schema.ts or any migration callback. migrationStatus is bounded/read-only. Deployment never runs generation against whatever files happen to exist on the server.

1. Verify the manifest, stored prefix, source/target schemas, planSetDigest and resource feasibility. Rehearse the plan against a verified backup.
2. Persist the operation/plan intent and durably freeze source admission: hosted HEAD CAS or local LMDB metadata transaction. Capture/pin its exact final source. A crash does not thaw it.
3. Execute the checked plan natively into isolated staging in bounded batches, with no JS callback in a transaction. Validate target laws, expected logical results, IDs/references and complete history.
4. Publish one complete target in Frozen/AwaitingCutover state. Return ReadyToSwitch with the final deployment binding and activation reference bound to operation/planSetDigest/identity/genesis.
5. Configure/deploy the application's new binding while the target remains frozen, perform authorized read-only checks, explicitly activateMigration, verify activation and re-enable traffic. The old source stays frozen.

Activation persists its one-time marker with the authority change. A lost response resolves by activationRef; repeated activation returns the recorded outcome/current access mode without thawing a subsequently Frozen or Deleted target. Applied/activation history does not expire with ordinary command receipts.

Status distinguishes UpToDate, Pending, InProgress/Paused, ReadyToSwitch, Activated, OutcomeUnknown and typed drift/refusal. Unknown publication is never resolved by assuming a timeout means failure. Same operation with a different plan/source refuses.

Before activation, explicit abort may discard the unused target and thaw the matching source after proving activation did not occur. After activation, do not auto-rollback: even unchanged data state can have new receipts and external effects. Require an explicit decision/effect audit and reverse/repair plan or documented loss acceptance.

The application supplies its existing authenticated tenant-binding registry or one deployment environment value. Bumbledb creates no mutable router/alias service and claims no atomic transaction with external deployment configuration. Migrations use an appropriately provisioned Node admin job, not a request hook, Next build import or every worker startup. Native execution can still be expensive and needs disk/CPU/deadline budgets.

## Next.js: a small server-only module

Use the same Node integration on local Apple Silicon, AWS Graviton or qualified Vercel Node x64. Schema/query values come from the ordinary typed SDK. A process-local bounded cache keeps independent request borrows; it is not a tenant discovery service or durable authority.

~~~ts
// src/db/server.ts — proposed API; runtimePolicy is app-owned measured configuration
import "server-only"
import { TenantCache } from "@bjornpagen/bumbledb-log"
import contract from "./runtime-contract.json"
import { runtimePolicy } from "./runtime-policy"

const options = { ...runtimePolicy, expected: contract }
const key = JSON.stringify(options)
const state = globalThis as typeof globalThis & {
  __bumbledb?: { key: string; cache: TenantCache }
}
if (state.__bumbledb && state.__bumbledb.key !== key) {
  throw new Error("Database runtime settings changed; restart the development server")
}
state.__bumbledb ??= { key, cache: new TenantCache(options) }
export const databases = state.__bumbledb.cache
~~~

Construction is inert; acquisition opens a trusted binding. The policy declares cache directory, owner/operation limits, memory/disk/output/work budgets and refreshable credential source. It is ordinary finite configuration, not a plugin framework. Platform/request policy numbers are measured settings, not new hard-coded engine size limits.

~~~ts
// app/api/notes/[id]/route.ts — proposed Bumbledb API
import { databases } from "@/src/db/server"
import { noteById } from "@/src/db/queries"
import { requirePrincipal, bindingFor, parseId128 } from "@/src/auth"
import { requestPolicy } from "@/src/db/runtime-policy"
import { encodeRows } from "@bjornpagen/bumbledb-log"

export const runtime = "nodejs"
export const dynamic = "force-dynamic"

export async function GET(request: Request, context: {
  params: Promise<{ id: string }>
}) {
  const principal = await requirePrincipal(request)
  const binding = await bindingFor(principal)
  const id = parseId128((await context.params).id)
  const work = requestPolicy(request)
  await using db = await databases.acquire(binding, work)
  await using snapshot = await db.snapshot({ ...work, consistency: { kind: "latest" } })
  await using result = await snapshot.execute(noteById, { id }, work)
  const rows = await result.collect({ maxBytes: work.outputBytes })
  return Response.json(encodeRows(noteById.outputSchema, rows), {
    headers: { "Cache-Control": "private, no-store" }
  })
}
~~~

Auth, binding resolution, ID parsing and deadline policy are application-owned helpers, not invented SDK authentication. The policy includes request/platform cancellation and a cleanup margin. Match actual host cancellation behavior; an HTTP abort never undoes a published command.

Writes use an application-owned Id128 generated once before the original request/command is sealed and retained with its stable request ID. A helper can use 16 cryptographically random bytes; UUIDv4 is also a 128-bit representation but has 122 random bits because of version/variant fields. Neither promises absolute uniqueness. Ordinary schema keys/references handle conflicts. No allocator, FreshRef or generated-ID receipt exists.

Do not share tenant rows, snapshots or owners through Next/React/CDN caches keyed only by query text. Keep dynamic authenticated requests as the default; explicit app caching needs identity/schema/parameters/published-stamp keys and an invalidation policy. Writes also need the app's CSRF/origin/session protections and typed decided/unknown-outcome handling.

CompleteResult/intoCursor provide bounded delivery after full execution, not early ordered feed pagination. Current exact-key get/internal range probes do not establish a public seek/order/limit API. Measure intended small per-student/per-user queries; do not add a lazy general query language or claim a giant result can yield its first ordered page cheaply.

## Native packaging on AWS and Vercel

Next.js serverExternalPackages and outputFileTracingIncludes are actual external APIs. For an AWS arm64 build, the trace includes the arm64 package; for Vercel's selected x64 deployment, it includes the x64 package. The first-party configuration generates the appropriate explicit target, not an import-time guess:

~~~ts
// next.config.ts — example selected Vercel/Linux x64 build
import type { NextConfig } from "next"
export default {
  serverExternalPackages: ["@bjornpagen/bumbledb", "@bjornpagen/bumbledb-log"],
  outputFileTracingIncludes: {
    "/*": ["./node_modules/@bjornpagen/bumbledb-linux-x64/**/*"]
  }
} satisfies NextConfig
~~~

Externalization alone does not prove the correct .node file is shipped. Inspect the emitted server unit and execute it in the target environment. Test optional dependency installation, pnpm/workspace tracing, ABI/libc/CPU compatibility and cross-building from Apple Silicon. Never copy a macOS binary into Linux. No native library, database file, secret or admin plan enters a client/public bundle.

Apple Silicon is the canonical local performance focus. AWS Graviton and Vercel Node x86-64 are portable correctness targets before specialized tuning. Preserve the same Free Join/set semantics and float behavior on all three. Node 24 is the common initial deployment baseline; other Node majors require explicit host support and qualification.

### Vercel is supported inside a measured envelope

Official Vercel docs inspected for this proposal support the full Node API surface and list Node 24.x as the default available version; they do not establish Node 26 availability. Current documented standard function bundle size is 250 MB uncompressed, with separately conditional larger-function support; file descriptors are 1,024 shared including runtime usage. Memory/duration/payload limits depend on the selected plan/runtime and are deployment inputs, not engine constants.

These facts do not independently prove Bumbledb's native file-lock/mmap/durability behavior. APP-04/06 require a real deployed Vercel test of the selected x64 artifact, observed Node/libc/CPU, actual writable temporary space and concurrent function behavior. Do not infer undocumented temporary-disk capacity from generic AWS Lambda limits or assume provider-owned AWS environment variables are exposed.

HostedHistory's local directory is disposable. A cold instance still downloads/imports/validates the full selected checkpoint plus retained tail before serving the requested frontier. A warm instance may reuse it, but eviction/new instances can repeat the cost. Configure owner/FD/memory/disk reservations across concurrent requests; the same process may handle multiple users.

A per-user database can exceed RAM when sufficient local disk exists; it cannot exceed the actual worker's local materialization/scratch budget. InsufficientLocalDisk is a typed refusal. LocalHistory is not durable on ephemeral function storage. Large tenants can use an ordinary Node host with adequate owned local disk; no new router, placement platform, EFS fallback or remote-demand-paging engine is introduced.

Hosted writes still wait for S3 publication and Latest still observes remote authority. Report cold/warm latency separately. “Turso-style per-user application database” describes the product experience, not a claim of Turso's replication mechanism, cold-start behavior or latency.

## Alchemy: ordinary infrastructure, attached permissions

The existing examples/lambda stack is historical evidence, not the production template: its intended IAM role is not attached to the function, and its package/tooling versions are old. Replace its static credentials, forever-memoized writer and request-triggered admin duty design during implementation.

The inspected Alchemy AWS.Website.Nextjs supports env/runtime/architecture/memorySize/timeout and returns the actual server Lambda. The inspected source uses server.bind with policyStatements on that server. The exact external version still must compile/deploy this recipe before release:

~~~ts
// alchemy.run.ts — observed external API shape; binding/policy are app deploy inputs
import * as Alchemy from "alchemy"
import * as AWS from "alchemy/AWS"
import * as Effect from "effect/Effect"
import { deployedBinding, dataObjectArn } from "./deploy-config"

export const Website = AWS.Website.Nextjs("Website", {
  runtime: "nodejs24.x",
  architecture: "arm64",
  env: { BUMBLEDB_DEPLOYMENT_BINDING: deployedBinding }
})
export default Alchemy.Stack(
  "Notes",
  { providers: AWS.providers(), state: AWS.state() },
  Effect.gen(function* () {
    const site = yield* Website
    if (site.server) {
      yield* site.server.bind`BumbledbDataWriter(${site.server})`({
        policyStatements: [{
          Effect: "Allow",
          Action: ["s3:GetObject", "s3:PutObject"],
          Resource: [dataObjectArn]
        }]
      })
    }
    return { url: site.url }
  })
)
~~~

The log bucket/prefix is explicitly provisioned durable data, separate from website asset/ISR buckets. Initialization runs the generated plan chain and returns a verified binding. Subsequent website deployments consume that binding; a failed HEAD read never recreates an empty database.

Attach the real scoped role and qualify bucket policy, conditional writes, protected never-deleted HEAD, TLS/encryption and region/account/prefix configuration. The code holding a writer role is trusted to run the protocol; S3 IAM cannot interpret every HEAD field as application authorization. Refresh credentials through the supported provider chain. Normal app code does not expose migrations/GC.

Admin/GC/backup identities have only their required additional prefix-constrained list/delete/source-target/backup permissions. Returning an unattached role ARN is not permission configuration. Vercel-to-AWS authentication needs an explicitly configured supported credential source; do not assume Vercel automatically supplies the AWS role/variables from the Alchemy deployment. No committed static keys.

Alchemy's examined Next.js composite ships the OpenNext server unit and wires its own framework cache resources; these are not extra Bumbledb authorities. That source exposes no ephemeral-storage prop, so do not invent one. Its web Function URL is public: app routes must authenticate even when callers bypass the CDN.

## Required generated-migration and app gates

These rows replace the earlier callback/purity/artifact-closure obligations under the same gate IDs. All are proposed tests, not passes. Pre-promotion uses exact staged artifacts; actual registry verification remains PKG-07B after separately authorized publication.

| Gate | Required assertion |
| --- | --- |
| TS-MIG-01 Canonical generation | Same canonical schema/typed intent generates identical plans/manifest despite nonsemantic TS refactors. Plan/schema/prefix/planSet hash golden vectors, self-field exclusions, unknown codecs and corrupted/reordered entries. check writes nothing. |
| TS-MIG-02 Applied history | Applied batches flatten to the exact manifest prefix across local/hosted checkpoint, receipt retirement, backup and restore. Schema/data-only database-ahead/drift refuses app open; cache files cannot forge history. |
| TS-MIG-03 Initialization/baseline | Empty initialization actually executes declarative seeds/closed data; no latest-schema shortcut marks skipped plans applied. Baseline is explicit/verified/distinct. Restore never repeats seeds. |
| TS-MIG-04 Diff intent/admission | Unchanged relations preserved automatically; ambiguous rename/drop/backfill/type/ID mapping refuses without typed intent. Unsupported AST, loss acknowledgement, collisions, reference/float interval/closed/key/capacity failures checked; no guessed repairs. |
| TS-MIG-05 Native bounds | Source/target larger than RAM, tiny batches, disk exhaustion, cancellation and process kill. Oversized plans/manifests/node counts refuse within explicit admission budgets; no requirement that an arbitrary AST exceed RAM. Migration execution evaluates no authoring code/JS callback; partial destination stays private with bounded buffers. |
| TS-MIG-06 Resume | Crash every freeze/capture/execute/validate/final-genesis boundary. Same operation/plan restarts only unpublished work from fixed original source; resolves/reuses a completed target. No JS stack journal or intermediate published incarnation. |
| TS-MIG-07 Fusion semantics | Fused pending suffix equals ordered reference-plan evaluation, including intermediate errors/laws, seeds, f64 sum/mean and deterministic output. Five simple maps use one final materialization/publication; genuinely required private passes are explained/measured. |
| TS-MIG-08 Concurrency/ambiguity | Same/different operations, changed planSetDigest, lost freeze/genesis replies and competing staging owners resolve to matching authority or typed refusal. No duplicate lineage or silent source substitution. |
| TS-MIG-09 Cutover | Final target frozen until explicit activation; old writers fenced; activation marker survives receipt retirement. Same reference cannot thaw later Frozen/Deleted state. No automatic rollback after activation/receipts/effects. |
| TS-MIG-10 Tooling boundary | User writes schema/typed intent only; migration runner consumes generated plans/index without source paths/authoring/compiler. Ordinary app schema/query inference remains direct SDK use, with no mandatory runtime-type codegen. No handwritten migration callback/helper-purity framework/parser. CLI/direct API identical native outcomes; app bundle excludes admin plans. |
| APP-01 Server boundary | Dev/build/production app imports can construct typed schema/query values but perform no opens/migrations/schema generation. Native/admin artifacts never enter client/Edge/public bundles. Ordinary schema imports preserve AST-first inference. |
| APP-02 Auth/isolation | Anonymous/forged tenant/binding/stamp/direct-origin calls refuse before open. Concurrent per-user databases and Next/CDN caching never cross identity; writes demonstrate CSRF/auth and stable command/Id128 retry semantics. |
| APP-03 Request ownership | Warm/cold/concurrent invocations, HMR, abort/deadline, owner fault and process death. Independent borrows release, result conversion stays bounded, no stale forever-cached writer or hidden native work. |
| APP-04 Actual targets | Packed native packages run on Apple Silicon, AWS Graviton and deployed Vercel Node x64 through pinned framework packaging. Verify .node/ABI/libc/CPU, tracing, optional deps and wrong-target rejection. No unsupported Node-version claim. |
| APP-05 Real credentials/IAM | Actual AWS server role/bucket policy and separately configured Vercel credential path work and refresh. Cross-prefix/admin/list/delete denial and protected HEAD tested. Missing deployment access or unattached intended role is incomplete. |
| APP-06 Host envelope | Measure full cold materialization, warm reuse, tenant churn, shared FDs/concurrency, disk/memory/output/deadline budgets and cleanup on actual hosts. Insufficient disk refuses; no ephemeral LocalHistory durability or network-filesystem escape claim. |
| APP-07 Deployment rehearsal | Generate/verify/init, coalesced schema/data plans, interrupted admin job, frozen rollout, lost activation and rollback boundaries with exact staged artifacts. No migrations/generation on public request paths. |
| APP-08 Minimal integration | Generated files/config preserve existing app code, compile/run on qualified versions, expose app-owned auth/binding/policy, and need no C SDK/public Rust log/mobile runtime. Common per-user workloads—not analytics-only benchmarks—exercise the example. |

## Evidence and limits

Read-only source/doc inspection on 2026-09-04; no install/build/deployment is claimed. Pin external versions and test actual artifacts before release.

- Existing ts/src/relation.ts, schema.ts, spec.ts, query/atom.ts, query/lower.ts and ts/README.md establish the typed AST/lowering approach. Old fresh allocation/callback lifetime APIs are evidence to replace, not examples for 1.0.
- [Drizzle Expo SQLite](https://orm.drizzle.team/docs/connect-expo-sqlite) supplies the familiar generated/bundled migration-asset workflow, not mobile support or a request-hook execution model.
- [Next serverExternalPackages](https://nextjs.org/docs/app/api-reference/config/next-config-js/serverExternalPackages) and [file tracing](https://nextjs.org/docs/app/api-reference/config/next-config-js/output) define the actual bundling keys.
- [Vercel Node runtime](https://vercel.com/docs/functions/runtimes/node-js), [available Node versions](https://vercel.com/docs/functions/runtimes/node-js/node-js-versions), and [function limits](https://vercel.com/docs/functions/limitations) substantiate Node API/version/host-envelope facts above. They are not native Bumbledb qualification evidence.
- [Alchemy AWS Next.js](https://alchemy.run/aws/frontend/nextjs) and [inspected Nextjs source](https://github.com/alchemy-run/alchemy/blob/a3349e20baa92611a80a08ca581855f4709a5482/packages/alchemy/src/AWS/Website/Nextjs.ts) establish the scaffold shape, returned server/bind, framework packaging and absence of an ephemeral-storage prop at that revision.

The “magic” is generated correct data, a small typed API and precise refusal when intent is missing—not hidden schema execution, guessed data conversion or impossible platform promises.
