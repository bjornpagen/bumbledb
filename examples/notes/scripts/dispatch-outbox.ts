/**
 * The outbox dispatcher job (OPS-003's idempotent effect dispatcher):
 * an explicit provisioned Node job — a schedule target or an operator
 * command — never a public request hook.
 *
 *   OUTBOX_WEBHOOK_URL=https://... \
 *   node --experimental-strip-types scripts/dispatch-outbox.ts <tenantId>
 *
 * Reads pending outbox facts from a published snapshot, delivers each with
 * the row id as the receiver's idempotency key, and retires delivered rows
 * in separate deterministic commands. Crash anywhere replays safely: the
 * receiver deduplicates by key, and the retire command's identity derives
 * from the row id.
 */
import { NativeRuntime } from "@bjornpagen/bumbledb"
import { LocalHistory, HostedHistory } from "@bjornpagen/bumbledb-log"
import { Effect } from "effect"
import { bindingFor } from "../src/db/bindings.ts"
import { App } from "../src/db/schema.ts"
import { adminWork, runtimePolicy } from "../src/db/runtime-policy.ts"
import { dispatchOutbox } from "../src/outbox.ts"

const tenantId = process.argv[2]
if (tenantId === undefined) {
	console.error("usage: dispatch-outbox.ts <tenantId>")
	process.exit(2)
}

const program = Effect.scoped(
	Effect.gen(function* () {
		const binding = yield* bindingFor(tenantId)
		const history =
			binding.kind === "local"
				? yield* LocalHistory.open(binding, App, adminWork)
				: yield* HostedHistory.open(binding, App, adminWork)
		return yield* dispatchOutbox(history, tenantId, adminWork)
	})
)

const report = await Effect.runPromise(program.pipe(Effect.provide(NativeRuntime.layer(runtimePolicy.native))))
console.log(`outbox: retired ${report.retired}${report.stopped === null ? "" : ` (stopped: ${report.stopped})`}`)
