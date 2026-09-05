# Notes — the server-only Next.js + Alchemy example

One database per tenant, authenticated tenant bindings, one process-lifetime
`ManagedRuntime`, LocalHistory development, HostedHistory S3 materialization,
retained command/admin refs, generated migrations, backup/restore, and typed
deadlines. The only Promises are the framework run boundary.

Verification: **NotRun** until packed artifacts and F3.

## Layout

```
src/db/schema.ts            current typed schema
src/db/evolution-stages.ts  staged history behind bumbledb/migrations/
src/db/queries.ts           reusable typed query templates
src/db/reads.ts             QueryReader collect/pages helpers
src/db/commands.ts          sealed commands, retained refs, resolve
src/db/server.ts            Databases service + the ONE ManagedRuntime
src/db/bindings.ts          authenticated tenant → verified binding registry
src/db/runtime-policy.ts    measured work budgets
scripts/generate-history.ts regenerate bumbledb/migrations
scripts/migrate.ts          status / migrate / activate
scripts/init-tenant.ts      generated initialize
scripts/backup-restore.ts   backup / verify / restore
scripts/resolve-command.ts  resolve a retained command ref
app/api/notes/**            Node runtime, force-dynamic
alchemy.run.ts              attached IAM, provisioned buckets
next.config.ts              serverExternalPackages + platform native
```

## Local development

```sh
pnpm install
pnpm run generate
ID=$(pnpm run --silent mint-session id)
DB=$(pnpm run --silent mint-session id)
INC=$(pnpm run --silent mint-session id)
pnpm run init-tenant local student-a "$ID" "$DB" "$INC"
SESSION_SECRET=... pnpm run dev
```

Bindings come from the provisioned registry, never from a request path.
Field-arithmetic backfill (`Scalar.add(Scalar.field("units"), Scalar.u64(1n))`)
is demonstrated on the shared Learning consumer, not by changing Note fields.

## Tests

- `test/routes.test.ts` — same-ID retry, tenant isolation, witnessed pin
- `test/specimens.test.ts` — consumer field-arithmetic convert, generated `{ manifest, plans, snapshots }`, binding refusal, Node runtime
- `test/deployed.test.ts` — requires `DEPLOYED_URL`/`DEPLOYED_TOKEN`; missing
  credentials fail (NotRun), never skip green

## Deployment

`alchemy.run.ts` provisions the log and blob buckets and attaches the
prefix-scoped data-writer policy to the Node server. Both native packages
are externalized; the matching `@bjornpagen/bumbledb-<target>` addon is
traced into the server unit. Edge/browser/Expo are unsupported.
