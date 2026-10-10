/**
 * One outbox pass for a tenant, run by a schedule or an operator:
 *   OUTBOX_WEBHOOK_URL=https://... node scripts/dispatch-outbox.ts <tenant>
 */
import { Bumble, Database } from "@bjornpagen/bumbledb"
import { Effect } from "effect"
import { migrations } from "../migrations/index.ts"
import { runtimePolicy } from "../src/db/runtime-policy.ts"
import { App } from "../src/db/schema.ts"
import { cacheFor, storeFor } from "../src/db/stores.ts"
import { dispatchOutbox } from "../src/outbox.ts"

const tenant = process.argv[2]
if (tenant === undefined) {
	console.error("usage: dispatch-outbox.ts <tenant>")
	process.exit(2)
}

const program = Effect.scoped(
	Effect.flatMap(
		Database.make({ schema: App, migrations, store: storeFor(tenant), cache: cacheFor(tenant), onOpen: "verify" }),
		dispatchOutbox
	)
)
const report = await Effect.runPromise(program.pipe(Effect.provide(Bumble.layer(runtimePolicy.native))))
console.log(`outbox: dispatched ${report.dispatched}${report.stopped === null ? "" : ` (stopped: ${report.stopped})`}`)
