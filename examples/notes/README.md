# Notes: a server-side Next.js example

Notes demonstrates a database per tenant, authenticated bindings, one
process-lifetime Effect runtime, durable named commands, retained receipts,
and explicit schema initialization. It targets Node, not Edge or the browser.

Local history is the qualified end-to-end setup. Hosted S3 bindings and
Alchemy deployment code use the same explicit creation and ordinary seed
commands. Remote deployment is qualified separately.

## Local setup

This example pins the core and log SDKs to `1.3.1` and Effect to
`4.0.0-rc.112`. The repository's packed-consumer check also tests locally
staged SDK packages in isolation.

From this directory:

```sh
pnpm install
pnpm run snapshot
pnpm run init-tenant local student-a <operation-uuid> <database-uuid> <incarnation-uuid>
```

Supply three canonical UUIDs generated once by the application or deployment
tool. Retain them across retries. The initializer verifies the current schema
and writes its seeds through ordinary commands. The snapshot command writes
`schema.json`; retain it to generate historical bindings
when an application migration is needed.

Start the development server with `SESSION_SECRET` set to a secret of at
least 32 characters. Authentication uses the signed bearer-token format in
`src/auth.ts`; wire a trusted authentication service to that boundary.
Tenant bindings come from the verified registry, never arbitrary request
paths. `src/db/runtime-policy.ts` sets process-wide scheduling overrides
and the open-tenant limit. Database allocation is unrestricted; request
aborts cancel the Effect scope through the framework boundary.

## Code map

| Path | Responsibility |
|---|---|
| `src/db/schema.ts`, `initialize.ts` | Current schema, explicit creation and seed commands. |
| `src/db/queries.ts`, `reads.ts` | Reusable queries and the core `QueryReader`. |
| `src/db/commands.ts` | Sealing commands and resolving retained references. |
| `src/db/server.ts`, `bindings.ts` | Shared runtime and authenticated tenant registry. |
| `scripts/init-tenant.ts` | Explicit initialization and binding adoption. |
| `scripts/backup-restore.ts`, `resolve-command.ts` | Backup, restore, and outcome resolution. |
| `app/api/notes/` | Node route handlers. |
| `alchemy.run.ts`, `next.config.ts` | Deployment and native-package bundling. |

## Verification

`scripts/packed-import.sh --host-only` from the repository root creates a
temporary installed-package copy and runs
the complete TypeScript check, `test/routes.test.ts`, `test/specimens.test.ts`,
and the Next.js production build. These cover local retries, tenant isolation,
witnessed updates, schema initialization, runtime error responses, and public API use.
Dependency declarations remain checked. The pinned Alchemy patch corrects an
optional-attribute declaration to admit its existing deleting-state variant;
it changes no runtime code.

`test/deployed.test.ts` requires `DEPLOYED_URL` and `DEPLOYED_TOKEN`.
Missing credentials are not successful deployment evidence. A green local
route test does not validate Alchemy provisioning, IAM, remote S3, or an
actual deployed server.
