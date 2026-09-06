# Notes: a server-side Next.js example

Notes demonstrates a database per tenant, authenticated bindings, one
process-lifetime Effect runtime, durable named commands, retained receipts,
and generated schema evolution. It targets Node, not Edge or the browser.

Local history is the supported end-to-end setup. Hosted S3 bindings and
Alchemy deployment code are present, but generated hosted initialization and
migration are not wired through the current TypeScript/native bridge.
Do not treat this example as a qualified one-command hosted deployment.

## Local setup

The example depends on matching `0.20.3` packages. Until those are published,
use the repository's packed-consumer check, which installs local tarballs in
isolation; an ordinary package install does not substitute unreleased code.

After installing the matching packages, from this directory:

```sh
pnpm install
pnpm run generate
pnpm run init-tenant local student-a <operation-uuid> <database-uuid> <incarnation-uuid>
```

Supply three canonical UUIDs generated once by the application or deployment
tool. Retain them across retries. Review and commit the generated migration
repository before deploying your application; generation is not a server
startup operation.

Start the development server with `SESSION_SECRET` set to a secret of at
least 32 characters. Authentication uses the signed bearer-token format in
`src/auth.ts`; wire a trusted authentication service to that boundary.
Tenant bindings come from the verified registry, never arbitrary request
paths. The policies in `src/db/runtime-policy.ts` are example budgets to size
for your deployment, not measured universal defaults.

## Code map

| Path | Responsibility |
|---|---|
| `src/db/schema.ts`, `evolution-stages.ts` | Current schema and generated-history inputs. |
| `src/db/queries.ts`, `reads.ts` | Reusable queries and the core `QueryReader`. |
| `src/db/commands.ts` | Sealing commands and resolving retained references. |
| `src/db/server.ts`, `bindings.ts` | Shared runtime and authenticated tenant registry. |
| `scripts/generate-history.ts` | Generate migration data for review. |
| `scripts/init-tenant.ts`, `migrate.ts` | Explicit initialization and migration administration. |
| `scripts/backup-restore.ts`, `resolve-command.ts` | Backup, restore, and outcome resolution. |
| `app/api/notes/` | Node route handlers. |
| `alchemy.run.ts`, `next.config.ts` | Deployment and native-package bundling. |

## Verification

`scripts/packed-import.sh --host-only` from the repository root creates a
temporary installed-package copy, generates its migration chain, and runs
`test/routes.test.ts` and `test/specimens.test.ts`. These cover local retries,
tenant isolation, witnessed updates, generated input, and public API use.

`test/deployed.test.ts` requires `DEPLOYED_URL` and `DEPLOYED_TOKEN`.
Missing credentials are not successful deployment evidence. A green local
route test does not validate Alchemy provisioning, IAM, remote S3, or an
actual deployed server.
