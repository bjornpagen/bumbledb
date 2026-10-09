/**
 * Creates or migrates each named tenant's database, running every bundled migration. This is the
 * deploy step that precedes traffic: production opens with `verify` and refuses pending migrations.
 *   node scripts/migrate-tenant.ts <tenant>...
 */
import { Bumble, Database } from "@bjornpagen/bumbledb"
import { Effect } from "effect"
import { migrations } from "../migrations/index.ts"
import { runtimePolicy } from "../src/db/runtime-policy.ts"
import { App } from "../src/db/schema.ts"
import { cacheFor, storeFor } from "../src/db/stores.ts"

const tenants = process.argv.slice(2)
if (tenants.length === 0) {
	console.error("usage: migrate-tenant.ts <tenant>...")
	process.exit(2)
}

const program = Effect.forEach(
	tenants,
	(tenant) =>
		Effect.scoped(
			Database.make({ schema: App, migrations, store: storeFor(tenant), cache: cacheFor(tenant), onOpen: "migrate" })
		).pipe(Effect.tap(() => Effect.sync(() => console.log(`migrated ${tenant}`)))),
	{ discard: true }
)
await Effect.runPromise(program.pipe(Effect.provide(Bumble.layer(runtimePolicy.native))))
