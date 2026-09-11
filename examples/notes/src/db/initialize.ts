import { ChangeSet } from "@bjornpagen/bumbledb"
import type { HistoryBinding, OperationId } from "@bjornpagen/bumbledb-log"
import { Command, HostedHistory, LocalHistory, ReceiptEpoch, RequestId } from "@bjornpagen/bumbledb-log"
import { schemaSnapshot } from "@bjornpagen/bumbledb-log/schema"
import { Effect } from "effect"
import { submitOptionsOf } from "./commands.ts"
import { App, Tag } from "./schema.ts"

/** Creation and application seeds use retained identities on every retry. */
export const initializeTenant = (binding: HistoryBinding, operationId: OperationId) =>
	Effect.scoped(
		Effect.gen(function* () {
			const options = { creation: { operationId, artifact: new TextEncoder().encode(yield* schemaSnapshot(App)) } }
			const history =
				binding.kind === "local"
					? yield* LocalHistory.create(binding, App, options)
					: yield* HostedHistory.create(binding, App, options)
			const builder = yield* ChangeSet.builder(App)
			yield* builder.insert(Tag, [
				{ id: "00000000-0000-0000-0000-000000000001", name: "inbox" },
				{ id: "00000000-0000-0000-0000-000000000002", name: "archive" }
			])
			const command = yield* Command.seal({
				scope: history.identity,
				id: {
					receiptEpoch: yield* Effect.fromResult(ReceiptEpoch.from(1n)),
					requestId: yield* Effect.fromResult(RequestId.from(operationId))
				},
				changes: yield* builder.finish(),
				precondition: { kind: "blind" },
				result: {}
			})
			const outcome = yield* history.submit(command, submitOptionsOf())
			if (
				outcome.kind !== "decided" ||
				(outcome.receipt.outcome.kind !== "committed" && outcome.receipt.outcome.kind !== "no-change")
			) {
				return yield* Effect.die(
					new Error(`initialization seeds: ${outcome.kind}; retry with the retained creation identity`)
				)
			}
			return binding
		})
	)
