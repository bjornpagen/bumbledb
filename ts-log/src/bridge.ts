/**
 * Effect-side adaptation of the C09 executor pattern for log operations.
 * Registration returns the native lease before any completion runs in JS;
 * interruption signals native cancellation and JOINS the drain (an
 * incomplete or failed drain surfaces as a structured `CloseFailure` defect
 * in the finalizer `Cause`, never false quiescence). No Promise or libuv
 * job is created; there is no JS critical section, timer, or queue here.
 */
import { DbError, finalizeClose } from "@bjornpagen/bumbledb"
import type { CloseReport, CloseWire, OperationHandle } from "@bjornpagen/bumbledb/internal/log"
import { Cause, Effect } from "effect"
import type { LogError } from "#errors.ts"
import { cancelled, logFailure } from "#errors.ts"

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

function cancelDrain(operation: string, cancel: CancelVerb, lease: OperationHandle): Effect.Effect<void> {
	return drainClose(`${operation}.cancel`, (callback) => cancel(lease, callback)).pipe(
		Effect.flatMap((report) => finalizeClose(`${operation}.cancel`, report))
	)
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
	return Effect.callback<A, LogError>((resume, signal) => {
		let lease: OperationHandle
		try {
			lease = start(() => {
				if (signal.aborted) {
					return
				}
				try {
					resume(Effect.succeed(accept(take(lease))))
				} catch (cause) {
					resume(Effect.fail(logFailure(operation, cause)))
				}
			})
		} catch (cause) {
			resume(Effect.fail(logFailure(operation, cause)))
			return
		}
		return cancelDrain(operation, cancel, lease)
	})
}

/**
 * One certainty-preserving operation: E is `never`, and every failure is
 * mapped into an arm by the caller-supplied classifiers. `beforeDispatch`
 * covers registration refusal (this invocation dispatched nothing);
 * `afterDispatch` covers a completion that cannot be decoded, and also a
 * fiber interrupt after `start()` returned a lease. After dispatch the
 * arm is unknown or decided under the caller-supplied identity — never a
 * new ID, and never a silent Cause drop that would tempt a reseal.
 * Cancellation still joins the native drain. CloseFailure defects stay in
 * Cause. A receipt that already decoded wins over unknown.
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
	return Effect.uninterruptibleMask((restore) => {
		let settled: A | undefined
		const settle = (arm: A): A => {
			if (settled === undefined) {
				settled = arm
			}
			return settled
		}
		return restore(
			Effect.callback<A>((resume) => {
				let lease: OperationHandle
				try {
					lease = start(() => {
						try {
							resume(Effect.succeed(settle(accept(take(lease)))))
						} catch (cause) {
							resume(Effect.succeed(settle(afterDispatch(logFailure(operation, cause)))))
						}
					})
				} catch (cause) {
					resume(Effect.succeed(beforeDispatch(logFailure(operation, cause))))
					return
				}
				return cancelDrain(operation, cancel, lease)
			})
		).pipe(
			Effect.catchCause((cause) => {
				if (settled !== undefined) {
					return Effect.succeed(settled)
				}
				if (Cause.hasInterruptsOnly(cause)) {
					return Effect.succeed(settle(afterDispatch(cancelled(operation))))
				}
				return Effect.failCause(cause)
			})
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
) {
	return Effect.acquireRelease(
		acquire,
		(resource) => close(resource).pipe(Effect.flatMap((report) => finalizeClose(operation, report))),
		{ interruptible: true }
	)
}
