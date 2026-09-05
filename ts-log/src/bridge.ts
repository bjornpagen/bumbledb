/**
 * Effect-side adaptation of the C09 executor pattern for log operations.
 * Registration returns the native lease before any completion runs in JS;
 * interruption signals native cancellation and JOINS the drain (an
 * incomplete or failed drain surfaces as a structured `CloseFailure` defect
 * in the finalizer `Cause`, never false quiescence). No Promise or libuv
 * job is created; there is no JS critical section, timer, or queue here.
 */
import { DbError } from "@bjornpagen/bumbledb"
import type { CloseReport, CloseWire, OperationHandle } from "@bjornpagen/bumbledb/internal/log"
import { finalizeClose } from "@bjornpagen/bumbledb/internal/log"
import { Effect, Exit, type Scope } from "effect"
import type { LogError } from "#errors.ts"
import { logFailure } from "#errors.ts"

export type CancelVerb = (operation: OperationHandle, callback: (report: CloseWire) => void) => void

export function closeReportOf(operation: string, report: CloseWire): CloseReport {
	if (report.kind === "failed") {
		return { kind: "failed", error: new DbError({ operation, reason: { _tag: "Internal" } }) }
	}
	return report
}

/**
 * Run one native close/release transition to completion. Uninterruptible:
 * teardown uses the runtime's reserved cleanup envelope, and abandoning the
 * callback would fake quiescence.
 */
export function drainClose(
	operation: string,
	start: (callback: (report: CloseWire) => void) => void
): Effect.Effect<CloseReport> {
	return Effect.callback<CloseReport>((resume) => {
		try {
			start((report) => resume(Effect.succeed(closeReportOf(operation, report))))
		} catch (cause) {
			resume(Effect.succeed({ kind: "failed", error: toDbError(operation, cause) }))
		}
	}).pipe(Effect.uninterruptible)
}

function toDbError(operation: string, cause: unknown): DbError {
	const typed = logFailure(operation, cause)
	return typed instanceof DbError ? typed : new DbError({ operation, reason: { _tag: "Internal" } })
}

/**
 * One bounded log operation with typed E. Registration failures and
 * completion decode failures both fail with the typed union; interruption
 * cancels and joins the native lease, including a late successful result.
 */
export function logOperation<Value, A>(
	operation: string,
	cancel: CancelVerb,
	start: (callback: () => void) => OperationHandle,
	take: (operation: OperationHandle) => Value,
	accept: (value: Value) => A
): Effect.Effect<A, LogError> {
	return nativeOperation(operation, cancel, start, take, accept, Effect.fail, Effect.fail)
}

/**
 * Transport completion/refusal is data in the certainty union. Fiber
 * interruption stays in Cause: it joins native cancellation, but cannot prove
 * nonpublication. Resolve the command/operation ref retained before dispatch.
 */
export function certaintyOperation<Value, A>(
	operation: string,
	cancel: CancelVerb,
	start: (callback: () => void) => OperationHandle,
	take: (operation: OperationHandle) => Value,
	accept: (value: Value) => A,
	beforeDispatch: (error: LogError) => A,
	afterDispatch: (error: LogError) => A
): Effect.Effect<A> {
	return nativeOperation(
		operation,
		cancel,
		start,
		take,
		accept,
		(error) => Effect.succeed(beforeDispatch(error)),
		(error) => Effect.succeed(afterDispatch(error))
	)
}

/** The one lease lifecycle shared by typed-error and certainty operations. */
function nativeOperation<Value, A, E>(
	operation: string,
	cancel: CancelVerb,
	start: (callback: () => void) => OperationHandle,
	take: (operation: OperationHandle) => Value,
	accept: (value: Value) => A,
	beforeDispatch: (error: LogError) => Effect.Effect<A, E>,
	afterDispatch: (error: LogError) => Effect.Effect<A, E>
): Effect.Effect<A, E> {
	return Effect.suspend(() => {
		let report: CloseReport | undefined
		return Effect.callback<A, E>((resume, signal) => {
			let lease: OperationHandle
			try {
				lease = start(() => {
					if (signal.aborted) return
					try {
						resume(Effect.succeed(accept(take(lease))))
					} catch (cause) {
						resume(afterDispatch(logFailure(operation, cause)))
					}
				})
			} catch (cause) {
				resume(beforeDispatch(logFailure(operation, cause)))
				return
			}
			// Callback finalizers must succeed so Effect retains the interrupt.
			return drainClose(`${operation}.cancel`, (callback) => cancel(lease, callback)).pipe(
				Effect.tap((closed) => {
					report = closed
					return Effect.void
				}),
				Effect.asVoid
			)
		}).pipe(
			Effect.onExit((exit) =>
				Exit.hasInterrupts(exit) && report !== undefined ? finalizeClose(`${operation}.cancel`, report) : Effect.void
			)
		)
	})
}

/**
 * Scoped ownership of a native resource: interruptible acquisition through
 * the cancellation-safe bridge above, and a finalizer that runs the stored
 * native close and dies with `CloseFailure` on incomplete/failed drain.
 */
export function scopedResource<A, E>(
	operation: string,
	acquire: Effect.Effect<A, E>,
	close: (resource: A) => Effect.Effect<CloseReport>
): Effect.Effect<A, E, Scope.Scope> {
	return Effect.acquireRelease(
		acquire,
		(resource) => close(resource).pipe(Effect.flatMap((report) => finalizeClose(operation, report))),
		{ interruptible: true }
	)
}
