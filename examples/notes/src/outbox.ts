/**
 * The idempotent outbox dispatcher (OPS-003): pending external effects
 * are FACTS committed atomically with their domain change; this module
 * reads them from a published snapshot, performs the effect, and retires
 * the row in a separate command. Safety comes from the effect target's
 * idempotency key (the outbox row id) plus the retire command's
 * deterministic identity — a crash between "performed" and "retired"
 * replays the delivery with the SAME key, and the receiver deduplicates.
 * The database never promises exactly-once external networking.
 */
import type { ExecutionPolicy, Id128 } from "@bjornpagen/bumbledb"
import type { History, HistoryBorrow } from "@bjornpagen/bumbledb-log"
import { Effect, Schema } from "effect"
import { retireOutbox } from "./db/commands.ts"
import { listPendingOutbox } from "./db/reads.ts"
import type { App } from "./db/schema.ts"

export class WebhookFailed extends Schema.TaggedError<WebhookFailed>()("WebhookFailed", {
	status: Schema.Number
}) {}

export class WebhookUnconfigured extends Schema.TaggedError<WebhookUnconfigured>()("WebhookUnconfigured", {}) {}

interface OutboxRow {
	readonly id: Id128
	readonly note: Id128
	readonly kind: string
}

/** Deliver one effect with the row id as the receiver's idempotency key. */
const deliver = Effect.fn("outbox.deliver")(function* (row: OutboxRow) {
	const target = process.env.OUTBOX_WEBHOOK_URL
	if (target === undefined) {
		return yield* new WebhookUnconfigured({})
	}
	const response = yield* Effect.callback<Response, WebhookFailed>((resume, signal) => {
		fetch(target, {
			method: "POST",
			signal,
			headers: { "content-type": "application/json", "idempotency-key": row.id },
			body: JSON.stringify({ kind: row.kind, note: row.note })
		})
			.then((value) => resume(Effect.succeed(value)))
			.catch(() => resume(Effect.fail(new WebhookFailed({ status: 0 }))))
	})
	if (!response.ok) {
		return yield* new WebhookFailed({ status: response.status })
	}
})

/**
 * One bounded dispatcher pass over a tenant: read pending rows from a
 * published snapshot, deliver each, retire delivered rows. Failures stop
 * the pass with the row retained — the next pass retries with the same
 * idempotency key. Returns the number retired.
 */
export const dispatchOutbox = Effect.fn("outbox.dispatch")(
	function* (history: History<typeof App> | HistoryBorrow<typeof App>, tenantId: string, work: ExecutionPolicy) {
		const rows = yield* Effect.scoped(
			Effect.gen(function* () {
				const snapshot = yield* history.snapshot({ ...work, consistency: { kind: "latest" } })
				return yield* listPendingOutbox(snapshot, work)
			})
		)
		let retired = 0
		for (const row of rows) {
			yield* deliver(row)
			const outcome = yield* retireOutbox(history, tenantId, row, work)
			if (outcome.kind === "decided") {
				retired += 1
				continue
			}
			// not-submitted / outcome-unknown: stop; the retained ref and the
			// still-present row drive the next pass.
			return { retired, stopped: outcome.kind } as const
		}
		return { retired, stopped: null } as const
	}
)
