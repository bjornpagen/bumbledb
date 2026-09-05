# Notes — the server-only Next.js + Alchemy example

The chapter 33 application shape, end to end, against the actual public
SDKs: one database per tenant, authenticated tenant bindings, one app
NativeRuntime layer behind one process-lifetime ManagedRuntime, the
LocalHistory development flow, HostedHistory S3 materialization in
production, retained command refs with outbox idempotency, explicit
generated migrations, and typed deadlines/errors at every boundary. No
generic Promise wrappers exist anywhere: the only Promises are the
framework's own run boundaries.

This replaces the retired `examples/lambda` stack, whose intended IAM
role was never attached to the function and whose request-triggered admin
duty and static-credential design were audited defects.

## Layout

```
src/db/schema.ts            current typed schema (shared by app + generator)
src/db/evolution-stages.ts  the staged history behind bumbledb/migrations/
src/db/queries.ts           reusable typed query templates
src/db/commands.ts          sealed commands, retained refs, idempotency
src/db/server.ts            Databases service + the ONE ManagedRuntime
src/db/bindings.ts          authenticated tenant → verified binding registry
src/db/runtime-policy.ts    measured work budgets (deployment inputs)
src/auth.ts                 app-owned authentication (HMAC session tokens)
src/http.ts                 typed errors / submit certainty → HTTP
src/outbox.ts               idempotent external-effect dispatcher
src/blob.ts                 blob-first attachment uploads (app-owned S3)
src/requests.ts             durable request records (refs before dispatch)
app/api/notes/**            route handlers (Node runtime, force-dynamic)
scripts/generate-history.ts regenerate bumbledb/migrations from the stages
scripts/migrate.ts          explicit admin runner (status/migrate/activate)
scripts/init-tenant.ts      provision a tenant from the generated chain
scripts/dispatch-outbox.ts  the outbox job
scripts/mint-session.ts     dev tokens and fresh Id128s
bumbledb/migrations/        generated canonical plans (PENDING F3 generation)
alchemy.run.ts              deployment: attached IAM, provisioned buckets
next.config.ts              native tracing for the selected target
```

## Local development (LocalHistory)

```sh
pnpm install
pnpm run generate                       # F3: emit bumbledb/migrations/
ID=$(pnpm run --silent mint-session id)
DB=$(pnpm run --silent mint-session id)
INC=$(pnpm run --silent mint-session id)
pnpm run init-tenant local student-a "$ID" "$DB" "$INC"
SESSION_SECRET=... pnpm run dev
TOKEN=$(SESSION_SECRET=... node --experimental-strip-types scripts/mint-session.ts token student-a)
curl -H "authorization: Bearer $TOKEN" localhost:3000/api/notes
```

Local tenants live under `.bumbledb/tenants/<tenant>` — genuinely durable
app-owned directories. The dev server holds ONE native runtime; changing
`runtime-policy.ts` requires a restart (hot reload refuses to replace
live native owners).

## Deployment

`alchemy.run.ts` provisions the log bucket and blob bucket as explicit
durable data and ATTACHES the prefix-scoped data-writer policy to the
actual server function. Credentials refresh through the provider chain;
no static keys. Migrations/initialization run from an operator machine or
a provisioned admin job (`scripts/migrate.ts`), never from request
handlers or build steps.

Support floors, the measured Vercel envelope (PENDING F3 measurement),
unsupported runtimes (Edge/browser/Expo) and the backup/restore and
cutover runbooks live in `docs/reference/deployment.md` and
`docs/reference/operations-runbook.md`.

## Tests

- `test/routes.test.ts` — the local LocalHistory request matrix (auth
  refusal, idempotent create, tenant isolation, witnessed pin, 404s).
- `test/deployed.test.ts` — the deployed request lane; requires
  `DEPLOYED_URL`/`DEPLOYED_TOKEN` and FAILS without them (missing
  credentials are NotRun, never green).

Both execute only in F3 under the campaign's final verification rules.
