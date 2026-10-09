/**
 * Maps typed database errors and submit outcomes to HTTP responses. A decided command is terminal
 * data, including a rejection; an `Unknown` refusal is 202 and the client resolves its request id
 * or retries the identical request. Bodies carry codes and receipts only, never tenant rows.
 */
import type { DbError, ReceiptOut, SubmitOutcome } from "@bjornpagen/bumbledb"
import { Cause, Effect, Exit, Option } from "effect"
import type { Unauthenticated } from "./auth.ts"

function json(status: number, body: unknown): Response {
	return Response.json(body, { status, headers: { "Cache-Control": "private, no-store" } })
}

function statusOf(error: DbError): number {
	const reason = error.reason
	switch (reason._tag) {
		case "ResourceLimit":
		case "QueueFull":
			return 429
		case "Cancelled":
			return 504
		case "InvalidArgument":
		case "InvalidValue":
		case "InvalidPath":
			return 400
		case "Engine":
			switch (reason.kind) {
				case "NotFound":
					return 404
				case "Frozen":
					return 423
				case "MigrationPending":
				case "SchemaAdvanced":
				case "MigrationsDiverged":
				case "MigrationRejected":
					return 503
				default:
					return 500
			}
		default:
			return 500
	}
}

export function databaseErrorResponse(error: DbError): Response {
	const notProvisioned = error.reason._tag === "Engine" && error.reason.kind === "NotFound"
	return json(statusOf(error), {
		error: notProvisioned ? "TenantNotProvisioned" : error.code,
		operation: error.operation
	})
}

export function authErrorResponse(error: Unauthenticated): Response {
	return json(401, { error: error._tag })
}

function receiptBody(receipt: ReceiptOut): Record<string, unknown> {
	return {
		request: receipt.request,
		seq: receipt.seq.toString(),
		revision: receipt.revision.toString(),
		outcome: receipt.outcome._tag
	}
}

/** One mapping for every write route. */
export function submitResponse(outcome: SubmitOutcome): Response {
	if (outcome._tag === "Decided") {
		switch (outcome.receipt.outcome._tag) {
			case "Committed":
			case "NoChange":
				return json(200, receiptBody(outcome.receipt))
			case "PreconditionFailed":
				return json(409, receiptBody(outcome.receipt))
			case "InvariantRejected":
				return json(422, receiptBody(outcome.receipt))
		}
	}
	switch (outcome.refusal._tag) {
		case "Unknown":
			return json(202, { submitted: "unknown" })
		case "Frozen":
			return json(423, { submitted: false, error: "Frozen" })
		case "RequestReused":
			return json(409, { submitted: false, error: "RequestReused" })
		case "MigrationPending":
		case "SchemaAdvanced":
			return json(503, { submitted: false, error: outcome.refusal._tag })
		default:
			return json(500, { submitted: false, error: outcome.refusal._tag })
	}
}

/**
 * The outermost Exit mapping: interruption and defects stay causes, never a made-up database
 * outcome. A disconnect can interrupt after the command was decided; the request id still resolves it.
 */
export function exitResponse(exit: Exit.Exit<Response, Response | DbError>): Response {
	if (Exit.isSuccess(exit)) return exit.value
	const failure = Cause.findErrorOption(exit.cause)
	if (Option.isSome(failure)) {
		return failure.value instanceof Response ? failure.value : databaseErrorResponse(failure.value)
	}
	if (Cause.hasInterrupts(exit.cause)) return json(499, { error: "Interrupted" })
	return json(500, { error: "Internal" })
}

/** Turns every typed failure into a response. */
export function respond<R>(
	effect: Effect.Effect<Response, DbError | Unauthenticated, R>
): Effect.Effect<Response, never, R> {
	return effect.pipe(
		Effect.catchTag("DbError", (error) => Effect.succeed(databaseErrorResponse(error))),
		Effect.catchTag("Unauthenticated", (error) => Effect.succeed(authErrorResponse(error)))
	)
}
