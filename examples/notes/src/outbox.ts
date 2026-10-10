/**
 * The outbox dispatcher: pending effects are rows committed with the change that needs them. A pass
 * reads them, delivers each with the row id as the receiver's idempotency key, and retires it in a
 * separate command whose request id derives from the row id. A crash between delivery and retirement
 * redelivers with the same key; the receiver deduplicates.
 */
import type { Database, Uuid } from "@bjornpagen/bumbledb"
import { Effect, Schema } from "effect"
import { retireOutbox } from "./db/commands.ts"
import { listPendingOutbox } from "./db/reads.ts"
import type { App } from "./db/schema.ts"

export class WebhookFailed extends Schema.TaggedError<WebhookFailed>()("WebhookFailed", {
	status: Schema.Number
}) {}

export class WebhookUnconfigured extends Schema.TaggedError<WebhookUnconfigured>()("WebhookUnconfigured", {}) {}

interface OutboxRow {
	readonly id: Uuid
	readonly note: Uuid
	readonly kind: string
}

const deliver = Effect.fn("outbox.deliver")(function* (row: OutboxRow) {
	const target = process.env.OUTBOX_WEBHOOK_URL
	if (target === undefined) return yield* new WebhookUnconfigured({})
	const response = yield* Effect.tryPromise({
		try: (signal) =>
			fetch(target, {
				method: "POST",
				signal,
				headers: { "content-type": "application/json", "idempotency-key": row.id },
				body: JSON.stringify({ kind: row.kind, note: row.note })
			}),
		catch: () => new WebhookFailed({ status: 0 })
	})
	if (!response.ok) return yield* new WebhookFailed({ status: response.status })
})

/** One pass: delivers and retires every pending row, stopping at the first that does not retire. */
export const dispatchOutbox = Effect.fn("outbox.dispatch")(function* (db: Database<typeof App>) {
	const rows = yield* Effect.scoped(Effect.flatMap(db.read("latest"), listPendingOutbox))
	let dispatched = 0
	for (const row of rows) {
		yield* deliver(row)
		const outcome = yield* retireOutbox(db, row)
		if (outcome._tag !== "Decided") return { dispatched, stopped: outcome.refusal._tag } as const
		dispatched += 1
	}
	return { dispatched, stopped: null } as const
})
