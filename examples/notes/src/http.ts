/**
 * App-owned mapping from typed database errors and submit certainty to
 * HTTP responses (chapter 33: "app code maps their typed errors to HTTP
 * status before its run boundary"). Certainty is data, never an
 * exception family: decided rejection is 409/422 WITH the durable
 * receipt, outcome-unknown is 202 WITH the retained ref — a client
 * retries the identical command or resolves the ref; it never invents a
 * fresh intent because a timeout happened.
 *
 * Responses carry codes and bounded diagnostics only — no tenant facts,
 * no query parameters, no credentials (redaction default).
 */
import { DbError } from "@bjornpagen/bumbledb"
import type { SubmitOutcome, TerminalReceipt } from "@bjornpagen/bumbledb-log"
import { ProtocolError, renderCommandRef, renderDecisionStamp, renderStateStamp } from "@bjornpagen/bumbledb-log"
import { Cause, Effect, Exit, Option } from "effect"
import type { Unauthenticated } from "./auth.ts"
import type { BindingRegistryInvalid, TenantNotProvisioned } from "./db/bindings.ts"

function json(status: number, body: unknown): Response {
	return Response.json(body, { status, headers: { "Cache-Control": "private, no-store" } })
}

/** Status for an operational database error (E-channel). */
function statusOf(error: DbError | ProtocolError): number {
	switch (error.reason._tag) {
		case "ResourceLimit":
		case "QueueFull":
			return 429
		case "DeadlineExceeded":
		case "Cancelled":
			return 504
		case "InvalidArgument":
		case "InvalidPath":
			return 400
		case "DatabaseFrozen":
		case "DatabaseDeleted":
			return 423
		case "NotYetAvailable":
		case "WitnessUnavailable":
			return 503
		case "NotInitialized":
		case "DatabaseMissing":
			return 404
		case "MigrationRequired":
		case "MigrationDrift":
		case "DatabaseAhead":
		case "CommandIdentityConflict":
			return 409
		default:
			return 500
	}
}

export function databaseErrorResponse(error: DbError | ProtocolError): Response {
	return json(statusOf(error), { error: error.code, operation: error.operation })
}

export function authErrorResponse(error: Unauthenticated): Response {
	return json(401, { error: error._tag })
}

export function tenantErrorResponse(error: TenantNotProvisioned | BindingRegistryInvalid): Response {
	return error._tag === "TenantNotProvisioned"
		? json(404, { error: error._tag, tenantId: error.tenantId })
		: json(500, { error: error._tag })
}

function receiptBody(receipt: TerminalReceipt): Record<string, unknown> {
	return {
		command: renderCommandRef(receipt.command),
		decisionAt: renderDecisionStamp(receipt.decisionAt),
		stateAt: renderStateStamp(receipt.stateAt),
		outcome: receipt.outcome.kind
	}
}

/**
 * One submit certainty mapping for every write route. Decided is
 * terminal data (including durable rejection); not-submitted proves this
 * invocation dispatched nothing; outcome-unknown returns the ref the
 * client (and our request record) can resolve.
 */
export function submitResponse(outcome: SubmitOutcome): Response {
	switch (outcome.kind) {
		case "decided": {
			const receipt = outcome.receipt
			switch (receipt.outcome.kind) {
				case "committed":
				case "no-change":
					return json(200, receiptBody(receipt))
				case "precondition-failed":
					return json(409, receiptBody(receipt))
				case "invariant-rejected":
					return json(422, receiptBody(receipt))
			}
			break
		}
		case "not-submitted":
			return json(statusOf(outcome.error), {
				submitted: false,
				command: renderCommandRef(outcome.command),
				error: outcome.error.code
			})
		case "outcome-unknown":
			return json(202, {
				submitted: "unknown",
				command: renderCommandRef(outcome.command),
				error: outcome.error.code
			})
	}
}

/**
 * The outermost Exit mapping: interruption and defects are Cause, never a
 * fabricated database outcome. A client disconnect can interrupt the
 * fiber AFTER publication — the retained command ref in the app's request
 * record remains the recovery coordinate; this response only reports that
 * THIS response is indeterminate.
 */
export function exitResponse(exit: Exit.Exit<Response, Response>): Response {
	if (Exit.isSuccess(exit)) {
		return exit.value
	}
	const failure = Cause.findErrorOption(exit.cause)
	if (Option.isSome(failure)) {
		return failure.value
	}
	if (Cause.hasInterrupts(exit.cause)) {
		return json(499, { error: "Interrupted" })
	}
	return json(500, { error: "Internal" })
}

/** Convenience: catch every typed failure into a Response success channel. */
export function respond<R>(
	effect: Effect.Effect<
		Response,
		DbError | ProtocolError | Unauthenticated | TenantNotProvisioned | BindingRegistryInvalid,
		R
	>
): Effect.Effect<Response, never, R> {
	return effect.pipe(
		Effect.catchTag("DbError", (error) => Effect.succeed(databaseErrorResponse(error))),
		Effect.catchTag("ProtocolError", (error) => Effect.succeed(databaseErrorResponse(error))),
		Effect.catchTag("Unauthenticated", (error) => Effect.succeed(authErrorResponse(error))),
		Effect.catchTag("TenantNotProvisioned", (error) => Effect.succeed(tenantErrorResponse(error))),
		Effect.catchTag("BindingRegistryInvalid", (error) => Effect.succeed(tenantErrorResponse(error)))
	)
}
