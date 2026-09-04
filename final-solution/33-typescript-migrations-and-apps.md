# 33 — Repository migrations and a small, excellent Next.js integration

Status: proposed 1.0 API, authoring workflow and required release tests. No migration runner, generated file, application scaffold, cloud deployment or new SDK export is implemented by this document. Bumbledb names in examples are proposed contracts; the external Next.js/Alchemy capabilities verified below are identified separately.

## The product decision

Make **`bumbledb-log` public TypeScript only**. Keep the database and durable log implementation in Rust, but do not maintain a public Rust log SDK, C log header, or Rust migration-authoring interface. The core remains public Rust/TypeScript/C. Local and hosted log history expose one TypeScript command/read/migration vocabulary.

The user experience should be familiar: schema code in the app, ordered migration files checked into the repository, an immutable checked manifest, `migrationStatus()`, and an explicit `migrate()` call. The inspiration from Drizzle/Expo is **bundled ordered migration assets**, not a promise to run a native LMDB database in Expo/Hermes, the browser, Next.js Edge, or Cloudflare Workers.

The deliberately small contract is an offline per-tenant transformation to a checked new incarnation. There is no schema-language interpreter, fleet rollout scheduler, mutable routing alias, automatically replayed business callback, or online dual-write engine. The application/deployment tool supplies authentication, configuration cutover and infrastructure.

## What lives in the application repository

```text
src/db/schema.ts                      current application schema exports
src/db/server.ts                      server-only runtime/cache configuration
src/db/runtime-contract.json          generated expected schema + history prefix
bumbledb/migrations/
  0000-initialize.ts
  0001-add-note-pinned.ts
  schemas/v0000.ts                    immutable historical schema view
  schemas/v0001.ts
  schemas/empty.ts                    declared empty base schema
  manifest.json                      ordered identities/checksums, reviewed in git
  index.ts                           generated explicit imports; no runtime glob
scripts/migrate.ts                    explicit administrative entry point
next.config.ts
alchemy.run.ts
```

Historical schemas are immutable canonical descriptors with typed TypeScript views. A historical migration must not import today's mutable `src/db/schema.ts`, arbitrary application modules, environment configuration, or an untracked helper whose behavior can change underneath its checksum. Local helper modules are permitted only within the checked immutable migration dependency closure. App schema exports can point to the newest snapshot; old migrations still point to their original snapshots.

Three small proposed authoring operations are sufficient:

1. `bumbledb-log migrations new add-note-pinned` scaffolds the next ordered file and an explicit source/target schema snapshot. It can suggest structural changes but does not invent a correct data transformation.
2. `bumbledb-log migrations seal` computes the new entry's source identity, updates the checked manifest/explicit import index and emits the small runtime expectation. The author reviews and commits these files. It refuses to silently rewrite previously sealed entries; correcting an old applied mistake is a new migration.
3. `bumbledb-log migrations check` verifies the complete closure and manifest without modifying them. Release bundling emits a separate immutable execution artifact and provenance outside the source tree. Normal application builds consume the small runtime contract, not all historical migration code.

The directory name is convention, not runtime discovery. Explicit imports keep migration code available when bundled and keep unrelated files out. The npm package need not invent another build system: use one documented, pinned build pipeline and verify its resulting artifact. The migration CLI is an adapter around the same TypeScript functions used by `scripts/migrate.ts`.

## A migration is a checked source-to-target transformation

The example assumes `v0.Note = { id: EntityId, body: text }`, `v1.Note = { id: EntityId, body: text, pinned: bool }`, and unchanged `Settings = { key: text, enabled: bool }`. The schemas include their complete laws, not just TypeScript row interfaces.

```ts
// bumbledb/migrations/0000-initialize.ts — proposed API
import { defineMigration } from "@bjornpagen/bumbledb-log/migrations"
import { empty } from "./schemas/empty"
import * as v0 from "./schemas/v0000"

export default defineMigration({
  id: "0000-initialize",
  from: empty,
  to: v0.schema,
  coverage: { writes: [v0.Settings], empty: [v0.Note] },
  async transform({ target }) {
    await target.insert(v0.Settings, [{ key: "notes-enabled", enabled: true }])
  }
})
```

```ts
// bumbledb/migrations/0001-add-note-pinned.ts — proposed API
import { defineMigration } from "@bjornpagen/bumbledb-log/migrations"
import * as v0 from "./schemas/v0000"
import * as v1 from "./schemas/v0001"

export default defineMigration({
  id: "0001-add-note-pinned",
  from: v0.schema,
  to: v1.schema,
  coverage: {
    reads: [v0.Note],
    writes: [v1.Note],
    copy: [[v0.Settings, v1.Settings]]
  },
  async transform({ source, target }) {
    for await (const page of source.pages(v0.Note, { pageBytes: 256 * 1024 })) {
      await target.insert(v1.Note, page.map(note => ({ ...note, pinned: false })))
    }
  }
})
```

`coverage` is a finite completeness declaration, not a second query/migration DSL. Every source relation is covered by `reads`, an exact-schema `copy`, or an explicit `drop`; every target relation is covered by `writes`, `copy`, or explicit `empty`. Missing, duplicate or incompatible classifications refuse before freeze. Omitted lists mean empty lists. The example explicitly preserves Settings instead of silently producing a new database containing only transformed Notes. `reads`/`writes` can contain multiple relations for joins/splits; same-schema data migrations use the same mechanism.

The runner streams declared copies natively and grants the transform only its declared read/write capabilities. A target begins empty; transformed rows are set facts, so duplicate equal output facts collapse normally. Distinct rows that violate a key/other target law fail final validation; the runner never chooses a row or drops it to make the migration succeed. Coverage proves declared intent, not that an arbitrary transform implemented that intent correctly—expected counts/digests and application assertions still matter.

`source.pages()` returns bounded **owned** pages from the fixed read-only source. It does not give arbitrary LMDB pointers to JavaScript. A transform may use bounded source queries where needed; the full source schema is fixed and every accessed relation must be declared. Returned sets have no accidental physical-order guarantee: output must be independent of source iteration order and page partitioning. IDs based on row arrival order and naive order-dependent floating reductions are not reproducible; use explicit stable application mappings and the engine's deterministic aggregates.

`target.insert()` copies and checks a bounded owned batch, performs its private staging transaction, and ends that transaction before resolving the promise. **No transform callback, getter, iterator, or JavaScript `await` runs inside an LMDB write transaction.** Page conversion is also bounded; the runner cannot stop arbitrary synchronous JavaScript that never yields. Trusted migration code must cooperate with limits, and a host requiring a hard memory/CPU kill boundary runs the admin job in a process with OS limits. A killed transform leaves a resumable frozen operation, not an active partial database.

Preserve existing entity bytes by default. A new incarnation changes command/witness authority, not the validity of old-born application IDs. Explicit ID remapping needs a checked mapping covering every reference. This first API does not expose command fresh-ID reservation inside a migration; deterministic new seed identifiers use a schema-chosen application domain, or a separately specified checked mapping. Do not mint IDs from the clock/randomness or pretend that migration row order is a decision coordinate.

## Checksums are about meaning, not filenames

Illustrative manifest shape; angle-bracket values below stand for generated complete digests, not accepted encodings:

```json
{
  "formatVersion": 1,
  "semanticCodec": 1,
  "baseSchemaId": "<empty-schema-digest>",
  "entries": [
    {
      "seq": 0,
      "id": "0000-initialize",
      "fromSchemaId": "<empty-schema-digest>",
      "toSchemaId": "<v0-schema-digest>",
      "sourceDigest": "<closure-and-metadata-digest>",
      "prefixDigest": "<prefix-through-0000>"
    },
    {
      "seq": 1,
      "id": "0001-add-note-pinned",
      "fromSchemaId": "<v0-schema-digest>",
      "toSchemaId": "<v1-schema-digest>",
      "sourceDigest": "<closure-and-metadata-digest>",
      "prefixDigest": "<prefix-through-0001>"
    }
  ],
  "manifestDigest": "<entire-checked-manifest-digest>"
}
```

`sourceDigest` covers normalized local module paths and raw source bytes for the restricted transitive helper/schema closure, canonical from/to schema descriptors, coverage and migration metadata, and the semantic codec version. Nonsemantic-looking edits still change raw source identity: do not edit sealed historical files. External imports are restricted to the declared SDK and explicitly allowed deterministic runtime helpers; arbitrary package imports and dynamic code loading are refused by the supported tooling. All sizes, path normalization, duplicate IDs and graph cycles are bounded/validated.

`prefixDigest` commits to the ordered chain through that entry. The application expects an **exact** schema and history-prefix identity, including data-only migrations. A filename match, newest schema hash, or developer's local lockfile is insufficient. An app ahead of the database receives `MigrationRequired`; a database ahead of the app receives `DatabaseAhead`; an altered/missing/reordered shared prefix receives `MigrationDrift`. Ordinary open neither mutates nor repairs any of these cases.

Hash preimages are acyclic and domain-separated, using the protocol's selected content hash and versioned canonical length-framed encoding:

- `sourceDigest` hashes the author-declared metadata and immutable closure rooted at that migration module. Generated manifest/index/runtime-contract files, emitted bundles and derived digest fields are excluded from that source closure; adding a later migration cannot rewrite an earlier source identity.
- `P[-1] = H("migration-prefix-base/v1", formatVersion, semanticCodec, baseSchemaId)`. For entry `i`, `P[i] = H("migration-prefix-entry/v1", P[i-1], entryWithoutPrefixDigest)`. The entry includes its already computed `sourceDigest`, never its own `prefixDigest`; its stored `prefixDigest` must equal `P[i]`.
- `manifestDigest = H("migration-manifest/v1", canonicalManifestWithoutManifestDigest)`. Stored entry prefix digests are included and independently checked; the manifest does not hash its own final digest field.
- `artifactDigest` hashes ordered emitted module bytes plus declared execution metadata under `"migration-artifact/v1"`, excluding its own digest/signature fields. Its external execution manifest is not also included as a file containing that same digest. The production launcher verifies this acyclic description before module import.

The codec fixes all field inclusion, ordering, path normalization and framing rules. Golden vectors include empty and multi-entry manifests, one-byte source changes and deliberately self-referential/incorrect digest fields; string concatenation with ambiguous delimiters is not a hash encoding.

The actual executable bundle has a separate `artifactDigest`. Its execution manifest binds the checked source identities to the emitted module bytes, SDK/native version, semantic codec, bundler/compiler versions, options and dependency locks. Verify the bytes **before importing/running** the artifact. A small trusted production launcher checks that execution manifest and its files before importing the script entry point; verification inside `migrate()` alone is too late for JavaScript's eager module evaluation. The script below illustrates the source compiled into that verified artifact, not permission to run arbitrary unchecked `.ts` modules in production. Keep that exact artifact for operation resume and backup rehearsal. Matching source under a newly qualified compiler need not invalidate an already applied history; it also does not authorize changing the executable halfway through an unfinished operation. Completed history compares source/schema/prefix identity, while resume pins the recorded execution artifact.

Checksums detect drift/corruption and bind evidence. They do not prove arbitrary JavaScript pure or protect against an administrator intentionally substituting both code and its trust configuration. The administrative runner is trusted code, not a hostile-code sandbox. It never loads a migration URL or executable supplied by a database row/request body.

The generated index uses static imports:

```ts
// bumbledb/migrations/index.ts — proposed generated artifact
import { defineMigrations } from "@bjornpagen/bumbledb-log/migrations"
import manifest from "./manifest.json"
import initialize from "./0000-initialize"
import addNotePinned from "./0001-add-note-pinned"

export default defineMigrations({ manifest, modules: [initialize, addNotePinned] })
```

Importing definitions performs no database I/O or migrations. `defineMigrations` verifies the finite definition/manifest mapping; it is not an auto-migrate hook.

## Database history is authoritative

Chapter 22 specifies `Applied` and `Baseline` records. `Applied` records the sequence, ID, source/target schemas, source/artifact digests, operation ID, source/target identities, source stamp and canonical target application-state digest. A genesis binds the complete target plus inherited history and the new entry in one publication. The state digest excludes the system history record itself, avoiding a circular hash.

This history survives checkpointing, receipt retirement, backup and writable restore. Hosted history references it from authoritative reachable metadata; LocalHistory stores it transactionally in LMDB. A loose `migrations.json` next to a cache is not authority. Receipt epochs are command deduplication policy and never erase migration history.

Initialization really runs the declared chain from the empty base, including seeds. `create` cannot initialize the latest schema and mark every skipped file applied. The explicit `initialize()` administrative convenience may compose empty creation plus this same migration runner, returning the same frozen cutover result. It must not expose an empty writable latest-schema store between those steps.

Explicit adoption of an already populated validated snapshot can use `baseline()` with the claimed prefix, target digest and an operator reason. It records **Baseline, not Applied**: this asserts the adopted state satisfies the chosen history contract without claiming those transforms/seeds ran. A nonempty backup restore preserves original applied/baseline evidence and adds restore provenance; it does not rerun old seed code. Baseline is not a checksum-error bypass in normal `migrate()`.

## One explicit runner; no migrations on request paths

```ts
// scripts/migrate.ts — proposed API; trusted CI/admin input
import { migrate, migrationStatus } from "@bjornpagen/bumbledb-log/migrations"
import migrations from "../bumbledb/migrations"
import { loadAdminBinding, loadStableOperationId } from "./admin-config"

const binding = loadAdminBinding()
const options = {
  operationId: loadStableOperationId(), // persisted by the deploy job, reused on retry
  to: "0001-add-note-pinned",
  limits: {
    workingBytes: 64 * 1024 * 1024,
    batchBytes: 256 * 1024,
    maxLocalDiskBytes: 20 * 1024 * 1024 * 1024
  },
  signal: AbortSignal.timeout(30 * 60 * 1000)
}

const status = await migrationStatus(binding, migrations, options)
const outcome = await migrate(binding, migrations, options)
// Persist the structured result in the CI job's restricted artifact store.
// A ready-to-switch result contains deploymentBinding + activationRef.
// It does not change the app's configuration or activate writes.
```

`admin-config` is application-owned configuration/authentication code, not an SDK export. Limits are illustrative workload policy, not universal defaults; disk feasibility includes source/target/intermediate overlap and scratch. The runner may need far more disk than RAM. Run long production migrations in a suitable Node admin process/job with persistent operation evidence, not a short-lived Lambda request or Next build import. Lambda-hosted application requests do not imply Lambda-hosted migration jobs.

The public status/outcome vocabulary must distinguish:

| Result | Meaning |
| --- | --- |
| `UpToDate` | Exact schema/history expectation already satisfied; no freeze or transform |
| `Pending` | Verified unapplied suffix exists; status inspection is read-only |
| `MigrationDrift` / `DatabaseAhead` | Repository and authoritative history disagree; no guessed repair |
| `InProgress` / `Paused` | Durable operation exists, with source/target identity, phase and next safe action |
| `ReadyToSwitch` | Final target data/history verified and published, still frozen awaiting explicit activation |
| `Activated` | This operation's activation is durably proven; returns its deployment binding and separately reports current access mode |
| `OutcomeUnknown` | A freeze/genesis/activation publication may have occurred; operation reference resolves it |
| `Refused` | Specific schema, identity, permission, resource or validation failure; existing source/operation state reported |

Status metadata is bounded. Progress can report rows/bytes/phase and last error without logging tenant facts. State includes the original source and actual last completed intermediate, not only a path or count of files. `migrate()` called with the same operation/plan recovers that operation; changed plan/artifact/source cannot silently take it over. Concurrent runners either join/refuse the same recorded operation or discover the same completed target; local staging directory locks prevent two processes using one attempt directory.

### Crash resume is deliberately simple

1. Verify the manifest/source/artifact plan before freeze where possible. Record a stable operation ID, plan digest and planned target identities.
2. Durably freeze source admission with that operation identity: one HEAD CAS for hosted history, one local LMDB metadata transaction for LocalHistory. A process crash does not thaw it. Capture/pin its final published source.
3. Build an isolated target in bounded batches. No partial target serves normal application queries or writes. Validate the whole target theory, application digest, history extension and ID references after all batches.
4. Publish each completed target as `Frozen { AwaitingCutover, operationId, planDigest }`. Reuse a verified completed intermediate as the next migration's read-only source; never activate an intermediate between files.
5. On interruption, resume from the last verified completed step. **Restart an incomplete transform step from its fixed source into fresh isolated attempt staging.** There is no persisted JS stack, arbitrary callback checkpoint, or promise of incremental per-page resume in 1.0. This may redo expensive work; the cost is visible and avoids a second staging journal/state machine.
6. Resolve uncertain target publication from the planned identity, operation, genesis/data/history hashes before deciding to run any transform again. A lost response never justifies blindly repeating a completed migration or seed.

A transform is deterministic data conversion, not an ordinary business callback. It may repeat under the rule above, so network calls, payments, email, `Date.now()`, random IDs, environment-dependent branching and unversioned external inputs are forbidden. Materialize any necessary external input as explicitly versioned source data in a separate authorized workflow first. Test reproducibility across page sizes/order, low memory and interrupted attempts. Identical operation/source/artifact with conflicting completed output is `MigrationOutputMismatch`, not overwrite permission. Partial targets/attempt markers remain owned cleanup work until actual close; source data is never modified by a transform.

### Cutover is explicit and fenced

`ReadyToSwitch` returns the final `deploymentBinding` and an `activationRef` bound to the operation, plan, target identity and genesis. For one database, CI supplies the new binding to the Alchemy-deployed server environment. For many tenants, application code updates its **existing authenticated binding registry**. Bumbledb does not introduce a separate mutable alias service.

The production flow is: build/qualify app and migration artifacts → enter application maintenance mode → run/resume migration → deploy/configure the new binding while target remains frozen → perform authorized read-only validation → explicitly `activateMigration(activationRef, adminOptions)` → verify activation and re-enable application traffic. Source remains frozen throughout. Old app instances pointed at the source refuse writes; new ones cannot write until activation. The two history authorities and external deployment are not claimed to switch atomically; the safe unavailable interval is deliberate.

Activation durably records a one-time marker in the same local transaction/HEAD CAS that makes the target active. Its evidence remains available independently of command receipt retirement. Lost activation response is resolved through `activationRef`; do not infer success from a deployment tool exit code or repeat under a new operation. Retrying the same completed migration/activation reports `Activated` with that evidence, not a false still-frozen `ReadyToSwitch`. If later maintenance froze/deleted the destination, return its current access mode separately: repeating an old activation reference never thaws or reactivates it. New commands use the new identity/receipt epoch; existing business/entity identifiers remain ordinary data.

Before activation, explicit abort may discard the unused frozen target and thaw the source after matching operation checks. **After activation, no automatic config rollback is safe.** Even unchanged `StateStamp` can hide no-change/fresh decisions and external business effects. Require an explicit decision/effect audit and tested reverse/repair migration or documented loss acceptance. There is no public `down()` that silently rewinds production history.

## Next.js: server-only native ownership, ordinary app authentication

The default integration is one small server-only module and a bounded request borrow. `TenantCache` is chapter 31's optional local owner registry, not a tenant service. Module construction is inert; acquisition opens the exact trusted binding. Development HMR must reuse or explicitly close the old process registry; do not create a new owner per hot reload, silently clear an active lock, or open at build time. Process eviction/termination is expected, so hosted correctness never relies on this cache surviving.

```ts
// src/db/server.ts — proposed Bumbledb API, application-owned policy
import "server-only"
import { TenantCache } from "@bjornpagen/bumbledb-log"
import contract from "./runtime-contract.json"

const options = {
  cacheDirectory: process.env.BUMBLEDB_CACHE_DIR ?? "/tmp/bumbledb",
  maxOwners: 4,
  maxConcurrentOperations: 8,
  expected: contract,
  credentials: { kind: "aws-default-chain" as const }
}
const processState = globalThis as typeof globalThis & {
  __bumbledbRuntime?: { key: string; cache: TenantCache }
}
const key = JSON.stringify(options)
if (processState.__bumbledbRuntime && processState.__bumbledbRuntime.key !== key) {
  throw new Error("Database runtime settings changed; restart the dev server")
}
processState.__bumbledbRuntime ??= { key, cache: new TenantCache(options) }
export const databases = processState.__bumbledbRuntime.cache
```

```ts
// app/api/notes/[id]/route.ts — proposed Bumbledb API
import { databases } from "@/src/db/server"
import { noteById } from "@/src/db/queries"
import { requirePrincipal, bindingFor, parseNoteId } from "@/src/auth"
import { encodeRows } from "@bjornpagen/bumbledb-log"

export const runtime = "nodejs"
export const dynamic = "force-dynamic"

export async function GET(request: Request, context: {
  params: Promise<{ id: string }>
}) {
  const principal = await requirePrincipal(request)
  const binding = await bindingFor(principal) // authorized server configuration
  const id = parseNoteId((await context.params).id)
  const work = {
    signal: AbortSignal.any([request.signal, AbortSignal.timeout(1800)]),
    workingBytes: 8 * 1024 * 1024,
    outputBytes: 64 * 1024
  }
  await using db = await databases.acquire(binding, work)
  await using snapshot = await db.snapshot({
    ...work, consistency: { kind: "latest" }
  })
  await using result = await snapshot.execute(noteById, { id }, work)
  const rows = await result.collect({ maxBytes: 64 * 1024 })
  return Response.json(encodeRows(noteById.outputSchema, rows), {
    headers: { "Cache-Control": "private, no-store" }
  })
}
```

The authentication/query helpers are app-owned, not supplied authentication by Bumbledb. Real middleware/route error handling maps typed database errors and must not expose facts/credentials. The `encodeRows` sketch means the schema-tagged canonical value codec from chapter 30, including bigint and all f64 values; ordinary `JSON.stringify` is not a universal database serializer.

Read/write routes remain dynamic and request-scoped by default. Never put a tenant database owner, result or witness in a Next/React/CDN shared cache keyed only by query text. If an app opts into caching, keys include authenticated binding/identity/schema, query parameters and relevant published stamp, and its invalidation policy is explicit. Authenticated writes also need the app's CSRF/origin/session policy, stable command IDs and receipt/unknown-outcome handling from chapter 30. An HTTP disconnect does not undo a submitted command. Do not run `migrate()` in a route, server action, React hook, import initializer, cache acquisition or every replica's startup.

### Native bundling is a tested configuration, not a hope

The following Next.js options are real external API. Package names refer to the proposed 1.0 artifacts using the repository's current package family; exact released versions are pinned by the scaffold/lockfile.

```ts
// next.config.ts — Linux arm64 deployment recipe
import type { NextConfig } from "next"

const config: NextConfig = {
  serverExternalPackages: [
    "@bjornpagen/bumbledb",
    "@bjornpagen/bumbledb-log"
  ],
  outputFileTracingIncludes: {
    "/*": ["./node_modules/@bjornpagen/bumbledb-linux-arm64/**/*"]
  }
}
export default config
```

Externalizing JavaScript packages does **not** prove that the matching `.node` library is in the final OpenNext artifact. Build/install for the target OS/architecture/libc, inspect the emitted server unit, and execute it in the real selected Node/Lambda image. Cross-building on an Apple laptop cannot copy a macOS binary into a Linux deployment. pnpm symlinks/workspaces and optional-dependency omission need actual packed-artifact tests. Trace includes must target the selected package layout, not a broad repository glob. No binary, credential, database file or migration admin artifact enters a client bundle or public directory.

The initial support policy is chapter 32's Node 24/26 and qualified macOS arm64/Linux arm64+x64 glibc targets. The AWS recipe selects Node 24, Linux arm64 and the glibc 2.34 floor. No musl/Alpine, Edge, Worker, browser or mobile compatibility is inferred from TypeScript syntax. A normal Node server can embed directly; other hosts may call an authenticated application API without loading native code there.

## Alchemy: a verified small AWS scaffold

The existing `examples/lambda/alchemy.run.ts` is useful evidence, not a production template: it creates an intended IAM role that is not attached to the actual function. Its README explicitly predicts `AccessDenied`; it also targets 0.19.0 artifacts and beta.74 tooling. Do not copy its static-credential handling, forever-memoized writer or administrative duty route as the new contract.

The current external Alchemy docs/source inspected for this proposal support `AWS.Website.Nextjs`, `Alchemy.Stack`, `env`, `runtime`, `architecture`, `memorySize` and `timeout`. The composite returns its actual `server` Lambda (undefined during local dev), and its implementation attaches `policyStatements` using `server.bind`. The shape below follows that inspected source; **the selected released Alchemy version must compile and deploy this exact integration before it is advertised**. It is not evidence that beta.74 has every capability.

```ts
// alchemy.run.ts — external Alchemy shape plus app-owned environment input
import * as Alchemy from "alchemy"
import * as AWS from "alchemy/AWS"
import * as Effect from "effect/Effect"

const deployedBinding = process.env.BUMBLEDB_DEPLOYMENT_BINDING
const dataObjectArn = process.env.BUMBLEDB_DATA_OBJECT_ARN
if (!deployedBinding || !dataObjectArn) throw new Error("Missing deploy inputs")

export const Website = AWS.Website.Nextjs("Website", {
  runtime: "nodejs24.x",
  architecture: "arm64",
  memorySize: 2048,
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
```

This deliberately starts from an explicitly provisioned durable log bucket/prefix and a verified deployment binding. Data storage is not the website's disposable asset/ISR bucket. `dataObjectArn` is a deploy-time validated, narrowly scoped object ARN pattern for the authorized database namespace, not caller input. Do not use `forceDestroy` or an age-based lifecycle rule on active database objects. First-time provisioning is a separate explicit storage declaration plus initialization job; subsequent website deploys consume the resulting binding and never recreate authority because a HEAD read failed.

The scaffold's object Get/Put grant is the normal writer's **capability envelope**, not a complete bucket-security policy. Require bucket policy/IAM conditions appropriate to the supported conditional-write/no-delete authority protocol, TLS, encryption/key permissions when configured, and explicit region/account/prefix validation. S3 IAM cannot by itself recognize every semantic HEAD field: code holding a writer role is trusted to run the protocol, while untrusted HTTP callers only reach authenticated app methods. Credentials use the refreshable provider chain, not committed/static user keys.

Administration/GC requires a separate identity with the exact additional permissions: prefix-constrained `ListBucket` on the bucket ARN, deletion only in collectible object areas, creation/freeze/activation of allowed source/target namespaces, and backup destinations/keys as selected. Normal app roles do not need object deletion or migrations. `HEAD` authority must never be deleted by GC, bucket lifecycle or the app's delete grants. The full supported bucket policy is part of G08/G10's actual S3 qualification and APP-05, not manufactured by returning an unattached role ARN.

Alchemy's Next.js composite currently builds the OpenNext server unit, ships that unit as-is, and wires its own ISR S3/SQS/DynamoDB resources. Those are framework resources, not new Bumbledb coordination authorities. At the inspected source revision it does not expose an ephemeral-storage prop; do not invent one in the snippet. Qualify the actual configured local disk, or use a verified provider configuration/ordinary Node host with sufficient local disk. A per-tenant database larger than the available `/tmp` cannot mount there, even though it can exceed RAM and run on an appropriately provisioned host. Network filesystems are not a supported LMDB escape hatch without separate lock/durability qualification.

The examined composite uses a public Function URL for the web server. Therefore every sensitive route must enforce app authentication/authorization even when callers bypass the CDN. CloudFront/Next cache settings are not authentication. Large migrations run in the separately credentialed Node admin job; no request-triggered duty subprocess or always-on migration service is needed.

### What the tiny integration should generate

A first-party example/scaffolder should emit the server-only module, migration folder/manifest, explicit admin script, native tracing configuration, pinned package versions, workload limits and the verified Alchemy role attachment. It must preserve existing app code/config and show the diff. It should require the user's actual auth/binding function instead of generating an insecure fake tenant resolver. The result is ordinary editable repo code, not an opaque plugin or framework lock-in.

Development may explicitly initialize/migrate a configured local history before `next dev` begins accepting requests; it uses the same checked manifest and visible runner. Production deployment executes the explicit maintenance/migrate/cutover/activate flow. `open()` never means “create if missing and migrate whatever happens to be there.” The desired magic is good defaults, few files and precise errors—not hidden writes.

## Required migration and application gates

All are executable implementation/release obligations, not tests performed by this proposal. Pre-promotion application/cloud qualification uses the exact staged artifacts; public registry downloads are the separately post-promotion PKG-07B distribution check.

| Gate | Required assertion |
| --- | --- |
| TS-MIG-01 Manifest integrity | Mutated top-level/helper/schema source, missing/reordered/duplicate entries, wrong base/from/to/prefix, path aliases, dynamic imports, oversized/cyclic dependency graphs and unsupported codecs refuse before mutation where knowable. Golden source/prefix/manifest/artifact digest vectors prove exact framing and self-field exclusions; appending an entry preserves prior source/prefix identities. `check` writes nothing. |
| TS-MIG-02 Applied history | Exact prefix survives local/hosted checkpoint, receipt retirement, backup and restore. Database-ahead/data-only divergence refuses ordinary app open. Forged local cache manifest cannot replace authoritative evidence. |
| TS-MIG-03 Initialization/baseline | Empty creation runs seed chain exactly as recorded; latest-schema creation cannot falsely mark seeds applied. Baseline requires explicit verified claim/reason and remains distinguishable from Applied. Restore never implicitly repeats seeds. |
| TS-MIG-04 Coverage/admission | Missing source/target relation coverage, incompatible copy, undeclared query/write, key collisions, malformed rows, lost/remapped references, f64 edge values and cross-batch law failures refuse activation. No silent relation/row loss. |
| TS-MIG-05 Bounded transform | Source and target larger than RAM, tiny pages, low memory, disk exhaustion, cancellation and process kill. No JS callback/getter/iterator runs in native write transactions; incomplete target remains private. Measure batch conversion/event-loop cost and native buffers. |
| TS-MIG-06 Resume semantics | Crash each freeze/capture/copy/transform/validate/genesis boundary; same operation reuses completed steps and restarts only incomplete pure step from fixed source/artifact. No skipped/repeated applied seed, JS-stack journal, in-place source mutation or orphan-lock takeover. |
| TS-MIG-07 Determinism/artifacts | Vary row order/page partition/worker/memory; exact logical output matches. Check emitted artifact before execution, pin artifact on resume, permit qualified rebuild for later unapplied work without falsifying prior provenance. Conflicting output under one operation refuses. |
| TS-MIG-08 Concurrency/ambiguity | Same/different concurrent operation IDs, lost freeze/genesis replies and competing staging owners. Recover same recorded authority or typed refusal; never duplicate target lineage or blindly reexecute after completed publication. |
| TS-MIG-09 Multi-step/cutover | Every intermediate/final target remains frozen until intended explicit activation. Old writers stay fenced, binding change is explicit, activation loss resolves from durable marker even after receipt retirement. Same-reference retry reports prior activation without thawing a subsequently frozen/deleted target. No automatic rollback after activation/no-change decisions. |
| TS-MIG-10 Packaging/status | Static migration import bundle runs outside repo paths; runtime app bundle excludes transform/admin code. Status is read-only/bounded/redacted and distinguishes drift/pending/paused/unknown/ready. CLI and direct TS API produce identical history/outcomes. |
| APP-01 Server boundary | Next development/build/production imports perform no opens/migrations. Native modules and admin code never enter client/Edge/static asset bundles. Server-only restrictions and unsupported-runtime messages execute as documented. |
| APP-02 Authentication/cache isolation | Anonymous, forged tenant/binding/stamp and direct-origin calls refuse before open. Concurrent tenants and Next/CDN cache behavior cannot share data or owners. Write routes demonstrate CSRF/auth plus stable command/unknown-outcome handling. |
| APP-03 Request ownership | Warm/cold/concurrent invocations, HMR, deadline/abort, owner fault and process death. Independent borrows release, bounded query conversion holds, no forever-memoized broken writer or undisclosed native task survives completed close. |
| APP-04 Native deployment | Actual staged packages build through pinned Next/OpenNext/Alchemy and boot on selected Node24 Linux arm64 image. Inspect `.node` inclusion/ABI/glibc/architecture; test pnpm tracing, missing optional deps and cross-build failure. No Mac binary in Linux output. |
| APP-05 Attached IAM | Inspect/test actual server role and bucket policy, credential refresh, denied list/delete/admin actions, source/target prefix scope, protected HEAD and separately authorized backup/GC. An unattached intended role or skipped credentialed test is not a pass. |
| APP-06 Storage/runtime fit | Cold recovery plus query/checkpoint/target overlap on actual local disk; >RAM correctness, insufficient-disk refusal, configured platform timeout and cleanup margin. No unsupported EFS/shared-filesystem fallback or invented Nextjs ephemeral-storage property. |
| APP-07 Deployment rehearsal | New initialization, existing-schema data migration, interrupted admin job, frozen app rollout, lost activation response and rollback boundaries rehearsed with exact staged app/migration artifacts. No migrations run from public requests or build-time imports. |
| APP-08 Scaffold/contracts | Generated minimal app compiles/runs with exact qualified external versions and preserves existing files. All auth/binding helpers are identified as app-owned; examples promise no public Rust/C log or mobile API. Upgrade the Alchemy recipe only with fresh compile/deploy proof. |

## External evidence and scope of verification

Read on 2026-09-04; this proposal performed documentation/source inspection only, not an install/build/deployment. Release qualification must pin and test exact external versions rather than depend on mutable docs or `main`.

- [Drizzle Expo SQLite migrations](https://orm.drizzle.team/docs/connect-expo-sqlite): generated/bundled migration assets and explicit startup migration state are the ergonomic inspiration. Its SQLite/Expo hook is not transplanted into Bumbledb's Node request lifecycle.
- [Next.js serverExternalPackages](https://nextjs.org/docs/app/api-reference/config/next-config-js/serverExternalPackages) and [output tracing](https://nextjs.org/docs/app/api-reference/config/next-config-js/output): actual configuration keys and native-asset inclusion behavior. Inspected docs identified version 16.3.4.
- [Alchemy AWS Next.js guide](https://alchemy.run/aws/frontend/nextjs) and [Nextjs resource reference](https://alchemy.run/providers/aws/website/nextjs): OpenNext topology, required build dependencies, server env and framework deployment shape. Install/pin `@alchemy.run/frontend-frameworks` and `@opennextjs/aws` as documented.
- [Alchemy Nextjs source at inspected commit](https://github.com/alchemy-run/alchemy/blob/a3349e20baa92611a80a08ca581855f4709a5482/packages/alchemy/src/AWS/Website/Nextjs.ts): actual props, returned server, native server-unit packaging, public Function URL, `server.bind` role policy mechanism and absence of an ephemeral-storage prop. This is source evidence, not a tested promise for the repository's older beta.74.
- [Alchemy binding model](https://alchemy.run/infrastructure-as-effects/binding) and [Lambda Function reference](https://alchemy.run/providers/aws/lambda/function): role/config attachment, native package installation for standalone functions, and explicit lifecycle constraints. A standalone Lambda `build.install` option is not assumed to exist on `AWS.Website.Nextjs`.

This chapter closes no audit finding by itself. It gives the TypeScript-only product a concrete small application workflow, and makes migration correctness and the “plop it into Next.js” experience measured release requirements instead of marketing promises.
