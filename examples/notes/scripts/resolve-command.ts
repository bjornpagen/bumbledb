/**
 * Resolve a retained command ref after a lost ack. The ref is the app's
 * request record — never a reminted identity. Unknown stays resolvable
 * under the original command; absence after rotation is not proved loss.
 *
 *   node --experimental-strip-types scripts/resolve-command.ts <tenantId> <requestKeyHex>
 *
 * Verification: NotRun until F3.
 */
import { Id128, NativeRuntime } from "@bjornpagen/bumbledb"
import { parseCommandRef } from "@bjornpagen/bumbledb-log"
import { Effect, Result } from "effect"
import { bindingFor } from "../src/db/bindings.ts"
import { resolveCommand } from "../src/db/commands.ts"
import { App } from "../src/db/schema.ts"
import { adminWork, runtimePolicy } from "../src/db/runtime-policy.ts"
import { storedRef } from "../src/requests.ts"
import { LocalHistory, HostedHistory } from "@bjornpagen/bumbledb-log"

const [tenantId, requestKeyHex] = process.argv.slice(2)
if (tenantId === undefined || requestKeyHex === undefined) {
	console.error("usage: resolve-command.ts <tenantId> <requestKeyHex>")
	process.exit(2)
}

const requestKey = Id128.fromHex(requestKeyHex)
if (Result.isFailure(requestKey)) {
	console.error("request key must be 32 lowercase hex characters")
	process.exit(2)
}

const rendered = storedRef(tenantId, requestKey.success)
if (rendered === undefined) {
	console.error("no retained command ref for that request — dispatch never recorded a coordinate")
	process.exit(1)
}

const parsed = parseCommandRef(rendered)
if (Result.isFailure(parsed)) {
	console.error("stored command ref refused to parse")
	process.exit(1)
}

const outcome = await Effect.runPromise(
	Effect.scoped(
		Effect.gen(function* () {
			const binding = yield* bindingFor(tenantId)
			const history =
				binding.kind === "local"
					? yield* LocalHistory.open(binding, App, adminWork)
					: yield* HostedHistory.open(binding, App, adminWork)
			return yield* resolveCommand(history, parsed.success, adminWork)
		})
	).pipe(Effect.provide(NativeRuntime.layer(runtimePolicy.native)))
)

console.log(`resolve: ${outcome.kind}`)
