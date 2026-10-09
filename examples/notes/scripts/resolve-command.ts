/**
 * Looks up a request's receipt after an interrupted or ambiguous submit:
 *   node scripts/resolve-command.ts <tenant> <request key uuid>
 */
import { Bumble, Database, Uuid } from "@bjornpagen/bumbledb"
import { Effect, Option, Result } from "effect"
import { migrations } from "../migrations/index.ts"
import { requestIdOf } from "../src/db/commands.ts"
import { runtimePolicy } from "../src/db/runtime-policy.ts"
import { App } from "../src/db/schema.ts"
import { cacheFor, storeFor } from "../src/db/stores.ts"

const [tenant, key] = process.argv.slice(2)
const parsed = key === undefined ? undefined : Uuid.parse(key)
if (tenant === undefined || parsed === undefined || Result.isFailure(parsed)) {
	console.error("usage: resolve-command.ts <tenant> <request key uuid>")
	process.exit(2)
}

const program = Effect.scoped(
	Effect.flatMap(
		Database.make({ schema: App, migrations, store: storeFor(tenant), cache: cacheFor(tenant), onOpen: "verify" }),
		(db) => db.resolve(requestIdOf(parsed.success))
	)
)
const receipt = await Effect.runPromise(program.pipe(Effect.provide(Bumble.layer(runtimePolicy.native))))
console.log(
	Option.match(receipt, {
		onNone: () => "not decided",
		onSome: (decided) => `decided at ${decided.seq}: ${decided.outcome._tag}`
	})
)
