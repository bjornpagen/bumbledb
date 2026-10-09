# Notes: a server-side Next.js example

Notes is a multi-tenant notes app over bumbledb. Each tenant has its own
database, opened on demand from one `Database.pool` and closed when idle.
Migrations live in the repository beside the schema, and each write is a
command with a request id, so a retried request is decided once. External
effects go through an outbox. The app runs on Node, not on Edge or in the
browser.

## Local setup

The example installs `@bjornpagen/bumbledb` and its platform packages from
`../../ts` as copies, the way an app installs the published packages. Build
the package and the host's addon first, then run these commands from this
directory:

```sh
(cd ../../ts && pnpm install && node scripts/build.ts release)
pnpm install
SESSION_SECRET=<32+ characters> pnpm dev
```

Run `pnpm install` again after rebuilding `../../ts`.

In development each tenant's log is a directory under `BUMBLEDB_DATA_DIR`
(default `.bdb/`). A tenant's database is created and migrated the first
time a request names it. In production (`NODE_ENV=production`) databases open
with `onOpen: "verify"`, which refuses a tenant whose migrations have not run.
`pnpm migrate <tenant>...` creates or migrates tenants before they receive
traffic. Migration `0001_init` seeds the `inbox` and `archive` tags once, when
a tenant's database is created.

Authentication uses the signed bearer token in `src/auth.ts`, and
`scripts/mint-session.ts` mints a token for local use. Connect a trusted
identity service at that boundary. The tenant always comes from the verified
session and never from the request path.

## Changing the schema

Edit `src/db/schema.ts` and run:

```sh
pnpm migrations:generate <name>
```

This writes `migrations/NNNN_<name>/` (`schema.json`, generated `schema.ts`,
and `migration.ts`) and updates `migrations/index.ts`. When a relation is
renamed or reshaped, add a `populate` step to the new `migration.ts`.
`pnpm migrations:check` fails if the schema and the migrations directory
disagree, or if a migration was edited after it was generated.

## Code map

| Path | Responsibility |
|---|---|
| `src/db/schema.ts` | The current schema. |
| `migrations/` | Bundled migrations. `0001_init` seeds the tags. |
| `src/db/stores.ts` | Each tenant's object store (filesystem, or S3 under `tenants/<tenant>/`) and its local cache. |
| `src/db/server.ts` | The process runtime and the tenant pool. |
| `src/db/queries.ts`, `reads.ts` | Queries and reads at a revision. |
| `src/db/commands.ts` | Commands and their request ids. |
| `src/http.ts` | How errors and submit outcomes map to HTTP responses. |
| `src/outbox.ts`, `scripts/dispatch-outbox.ts` | Outbox dispatch and retirement. |
| `scripts/migrate-tenant.ts` | Creating or migrating tenants before traffic. |
| `scripts/resolve-command.ts` | Resolving a request id whose response was lost. |
| `app/api/notes/` | Route handlers. |
| `alchemy.run.ts`, `next.config.ts` | Deployment and native-package bundling. |

## Responses

A decided command is returned as a receipt (`request`, `seq`, `revision`,
`outcome`). `Committed` and `NoChange` return 200, `PreconditionFailed`
returns 409 and `InvariantRejected` returns 422. If the database cannot say
whether a command was decided, the response is 202, and the client either
retries the identical request or resolves its request id. A request id that
is reused for a different command returns 409 `RequestReused`. A tenant
without a database returns 404 `TenantNotProvisioned`.

## Deployment

`alchemy.run.ts` provisions the Next.js server function, the checkpoint
bucket, the blob bucket and the server's IAM policy. The log bucket
(`BUMBLEDB_LOG_BUCKET`) is an S3 Express directory bucket, which must be
created outside Alchemy. Checkpoints go in `BUMBLEDB_CKPT_BUCKET`. Run
`pnpm migrate <tenant>...` with the same bucket environment before a tenant
receives traffic.

The build ships the platform package named by `BUMBLEDB_TARGET` (default
`linux-arm64`), so `../../ts/npm/linux-arm64/bdb.node` must hold that
platform's addon (the CI artifact `bdb.linux-arm64.node`) and the install
must include that platform. The function runs on `nodejs24.x`, the newest
runtime Alchemy offers, while bumbledb requires Node 26.

## Verification

`pnpm typecheck` and `pnpm test` cover tenant creation on first use,
idempotent retries, tenant isolation, pins checked against a read revision,
the seeded tags, and how errors map to responses. `pnpm build` runs the
Next.js production build. `pnpm test:deployed` needs `DEPLOYED_URL` and
`DEPLOYED_TOKEN`, and without them it fails rather than skipping. Passing
local tests do not show that provisioning, IAM, remote S3 or a deployed
server work.
