/**
 * Effect over the addon's callback protocol. A native verb registers work and returns its lease
 * before any completion can reach JS; the addon then calls back exactly once. Interruption cancels
 * the lease and joins its drain, and a drain that does not close cleanly is a `CloseFailure`
 * defect beside the interrupt.
 */
import type { Scope } from "effect"
import { Effect } from "effect"
import type { CloseReport, DbError } from "../runtime-errors.ts"
import { CloseFailure, dbError } from "../runtime-errors.ts"
import type { CloseWire, OperationHandle } from "../runtime-native.ts"
import { runtimeNative } from "../runtime-native.ts"

/** Registers one native operation; the addon calls `done` once its result can be taken. */
type Start = (done: () => void) => OperationHandle

/** Starts one native close or cancel transition; the addon reports how it drained. */
type Drain = (done: (report: CloseWire) => void) => void

function reportOf(operation: string, wire: CloseWire): CloseReport {
	return wire.kind === "failed" ? { kind: "failed", error: dbError(operation, { _tag: "Internal" }) } : wire
}

/** Runs a native close or cancel transition to completion. It cannot be interrupted and never fails. */
function drain(operation: string, start: Drain): Effect.Effect<CloseReport> {
	return Effect.callback<CloseReport>((resume) => {
		try {
			start((wire) => resume(Effect.succeed(reportOf(operation, wire))))
		} catch (cause) {
			resume(Effect.succeed({ kind: "failed", error: dbError(operation, cause) }))
		}
	}).pipe(Effect.uninterruptible)
}

/** Drains, then dies with `CloseFailure` unless the transition closed cleanly. */
function release(operation: string, start: Drain): Effect.Effect<void> {
	return Effect.flatMap(drain(operation, start), (report) =>
		report.kind === "closed" ? Effect.void : Effect.die(new CloseFailure({ operation, report }))
	)
}

/**
 * One native operation. A completion that arrives after interruption is left to the cancel drain
 * the interruption already started.
 */
function call<A>(operation: string, start: Start, take: (lease: OperationHandle) => A): Effect.Effect<A, DbError> {
	return Effect.callback<A, DbError>((resume, signal) => {
		let lease: OperationHandle
		try {
			lease = start(() => {
				if (signal.aborted) return
				try {
					resume(Effect.succeed(take(lease)))
				} catch (cause) {
					resume(Effect.fail(dbError(operation, cause)))
				}
			})
		} catch (cause) {
			resume(Effect.fail(dbError(operation, cause)))
			return
		}
		return release(`${operation}.cancel`, (done) => runtimeNative.runtimeCancel(lease, done))
	})
}

/** A native resource owned by the current scope, released by its native close transition. */
function scoped<A, E, R>(
	operation: string,
	acquire: Effect.Effect<A, E, R>,
	close: (resource: A) => Drain
): Effect.Effect<A, E, R | Scope.Scope> {
	return Effect.acquireRelease(acquire, (resource) => release(operation, close(resource)), { interruptible: true })
}

export type { Drain, Start }
export { call, drain, release, scoped }
