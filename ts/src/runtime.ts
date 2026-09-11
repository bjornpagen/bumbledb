import { Context, Duration, Effect, Exit, Layer } from "effect"
import type { CloseReport, OutstandingWork } from "#runtime-errors.ts"
import { CloseFailure, DbError, dbError } from "#runtime-errors.ts"
import type { CloseWire, OperationHandle, OptionsWire, RuntimeHandle } from "#runtime-native.ts"
import { runtimeNative } from "#runtime-native.ts"

export interface NativeRuntimeOptions {
	readonly workers?: number
	readonly queueCapacity?: number
	readonly cleanupCapacity?: number
	readonly ownerCapacity?: number
	readonly nativeHandleCapacity?: number
	readonly cleanupTimeout?: Duration.Input
}

interface RuntimeService {
	readonly close: () => Effect.Effect<CloseReport>
	readonly inspect: () => Effect.Effect<OutstandingWork, DbError>
}

const owners = new WeakMap<RuntimeService, RuntimeHandle>()

function invalid(operation: string): DbError {
	return new DbError({ operation, reason: { _tag: "InvalidArgument" } })
}

function count(value: number, operation: string): number {
	if (!Number.isSafeInteger(value) || value <= 0 || value > 0xffffffff) throw invalid(operation)
	return value
}

function millis(value: Duration.Input, operation: string): number {
	const duration = Duration.toMillis(value)
	if (!Number.isFinite(duration) || duration < 0 || duration > 0xffffffff) throw invalid(operation)
	return Math.ceil(duration)
}

function options(value: NativeRuntimeOptions): OptionsWire {
	const operation = "NativeRuntime.acquire"
	return {
		workers: value.workers === undefined ? undefined : count(value.workers, operation),
		queueCapacity: value.queueCapacity === undefined ? undefined : count(value.queueCapacity, operation),
		cleanupCapacity: value.cleanupCapacity === undefined ? undefined : count(value.cleanupCapacity, operation),
		ownerCapacity: value.ownerCapacity === undefined ? undefined : count(value.ownerCapacity, operation),
		nativeHandleCapacity:
			value.nativeHandleCapacity === undefined ? undefined : count(value.nativeHandleCapacity, operation),
		cleanupTimeoutMs:
			value.cleanupTimeout === undefined ? undefined : count(millis(value.cleanupTimeout, operation), operation)
	}
}

function failure(operation: string, cause: unknown): DbError {
	return cause instanceof DbError ? cause : dbError(operation, cause)
}

function closeReport(operation: string, report: CloseWire): CloseReport {
	// A payload-less native "failed" drain (cleanup capacity exhausted, or
	// owner teardown failed) decodes as the core `Internal` reason — the
	// same mapping the log bridge applies, so one wire arm has one meaning.
	// It is deliberately NOT `QueueFull`: a failed drain is never retryable
	// submit backpressure.
	return report.kind === "failed" ? { kind: "failed", error: dbError(operation, { _tag: "Internal" }) } : report
}

function drain(operation: string, start: (callback: (report: CloseWire) => void) => void): Effect.Effect<CloseReport> {
	return Effect.callback<CloseReport>((resume) => {
		try {
			start((report) => resume(Effect.succeed(closeReport(operation, report))))
		} catch (cause) {
			resume(Effect.succeed({ kind: "failed", error: failure(operation, cause) }))
		}
	}).pipe(Effect.uninterruptible)
}

export function finalizeClose(operation: string, report: CloseReport): Effect.Effect<void> {
	return report.kind === "closed" ? Effect.void : Effect.die(new CloseFailure({ operation, report }))
}

/**
 * Effect.callback's returned finalizer is interruption cleanup only
 * (`asyncFinalizer` runs solely on interrupt Cause). If that finalizer
 * dies, the original interrupt is dropped. Join the native drain here and
 * always succeed so the interrupt Cause is kept; attach CloseFailure via
 * `onExit` so both appear in the final Cause.
 */
function joinInterruptDrain(
	operation: string,
	start: (callback: (report: CloseWire) => void) => void,
	stash: { report?: CloseReport }
): Effect.Effect<void> {
	return drain(operation, start).pipe(
		Effect.tap((report) => {
			stash.report = report
			return Effect.void
		}),
		Effect.asVoid
	)
}

function attachInterruptClose(
	operation: string,
	stash: { report?: CloseReport }
): <A, E, R>(effect: Effect.Effect<A, E, R>) => Effect.Effect<A, E, R> {
	return (effect) =>
		effect.pipe(
			Effect.onExit((exit) => {
				if (!Exit.hasInterrupts(exit)) {
					return Effect.void
				}
				const report = stash.report
				if (report === undefined || report.kind === "closed") {
					return Effect.void
				}
				return Effect.die(new CloseFailure({ operation, report }))
			})
		)
}

function afterClose<A, E, R>(
	operation: string,
	report: CloseReport,
	effect: Effect.Effect<A, E, R>
): Effect.Effect<A, E, R> {
	if (report.kind === "closed") {
		return effect
	}
	return effect.pipe(Effect.onExit(() => Effect.die(new CloseFailure({ operation, report }))))
}

function close(handle: RuntimeHandle): Effect.Effect<CloseReport> {
	return drain("NativeRuntime.close", (callback) => runtimeNative.runtimeClose(handle, callback))
}

/** Kick `runtimeCancel` without joining. Used when the completion
 * callback arrives after abort: JS must not `take`; L12 drain reclaims
 * queued Page/Rows. A second call is native-idempotent (Committed
 * mutations stay; only unpublished delivery is dropped).
 */
function startCancelDrain(operation: string, lease: OperationHandle, stash: { report?: CloseReport }): void {
	try {
		runtimeNative.runtimeCancel(lease, (report) => {
			stash.report = closeReport(operation, report)
		})
	} catch (cause) {
		stash.report = { kind: "failed", error: failure(operation, cause) }
	}
}

/** Registration returns the native lease before any completion can run in JS.
 * Interruption cancels and joins that lease, including a late successful result.
 * No Promise or libuv job is created for ordinary native work.
 */
export function nativeOperation<A>(
	operation: string,
	start: (callback: () => void) => OperationHandle,
	accept: (value: Uint8Array | null) => A
): Effect.Effect<A, DbError> {
	return nativeOperationWith(operation, start, runtimeNative.runtimeTake, accept)
}

export function nativeOperationWith<A, Value>(
	operation: string,
	start: (callback: () => void) => OperationHandle,
	take: (operation: OperationHandle) => Value,
	accept: (value: Value) => A
): Effect.Effect<A, DbError> {
	const stash: { report?: CloseReport } = {}
	const cancelOp = `${operation}.cancel`
	return attachInterruptClose(
		cancelOp,
		stash
	)(
		Effect.callback((resume, signal) => {
			let lease: OperationHandle
			try {
				lease = start(() => {
					if (signal.aborted) {
						startCancelDrain(cancelOp, lease, stash)
						return
					}
					try {
						resume(Effect.succeed(accept(take(lease))))
					} catch (cause) {
						resume(Effect.fail(failure(operation, cause)))
					}
				})
			} catch (cause) {
				resume(Effect.fail(failure(operation, cause)))
				return
			}
			return joinInterruptDrain(cancelOp, (callback) => runtimeNative.runtimeCancel(lease, callback), stash)
		})
	)
}

const acquire = Effect.fn("NativeRuntime.acquire")(function* (configuration: NativeRuntimeOptions) {
	return yield* Effect.acquireRelease(
		Effect.suspend(() => {
			const stash: { report?: CloseReport } = {}
			return attachInterruptClose(
				"NativeRuntime.acquire.cancel",
				stash
			)(
				Effect.callback<RuntimeService, DbError>((resume, signal) => {
					let handle: RuntimeHandle | undefined
					try {
						const wire = options(configuration)
						handle = runtimeNative.runtimeOpen(wire)
						const owner = handle
						const lease = runtimeNative.runtimeReady(owner, () => {
							if (signal.aborted) {
								try {
									runtimeNative.runtimeClose(owner, (report) => {
										stash.report = closeReport("NativeRuntime.acquire.cancel", report)
									})
								} catch (cause) {
									stash.report = { kind: "failed", error: failure("NativeRuntime.acquire.cancel", cause) }
								}
								return
							}
							try {
								runtimeNative.runtimeTake(lease)
								const service: RuntimeService = {
									close: () => close(owner),
									inspect: Effect.fn("NativeRuntime.inspect")(function* () {
										yield* nativeOperation(
											"NativeRuntime.inspect",
											(callback) => runtimeNative.runtimeReady(owner, callback),
											() => undefined
										)
										return runtimeNative.runtimeInspect(owner)
									})
								}
								owners.set(service, owner)
								resume(Effect.succeed(service))
							} catch (cause) {
								const error = failure("NativeRuntime.acquire", cause)
								resume(
									close(owner).pipe(
										Effect.flatMap((report) => afterClose("NativeRuntime.acquire", report, Effect.fail(error)))
									)
								)
							}
						})
					} catch (cause) {
						const error = failure("NativeRuntime.acquire", cause)
						resume(
							handle === undefined
								? Effect.fail(error)
								: close(handle).pipe(
										Effect.flatMap((report) => afterClose("NativeRuntime.acquire", report, Effect.fail(error)))
									)
						)
					}
					return handle === undefined
						? Effect.void
						: joinInterruptDrain(
								"NativeRuntime.acquire.cancel",
								(callback) => runtimeNative.runtimeClose(handle, callback),
								stash
							)
				})
			)
		}),
		(service) => service.close().pipe(Effect.flatMap((report) => finalizeClose("NativeRuntime.release", report))),
		{ interruptible: true }
	)
})

export class NativeRuntime extends Context.Service<NativeRuntime, RuntimeService>()(
	"@bjornpagen/bumbledb/NativeRuntime"
) {
	static layer(options: NativeRuntimeOptions = {}): Layer.Layer<NativeRuntime, DbError> {
		return Layer.effect(NativeRuntime, acquire(options))
	}
}

/** Private core/log integration: captures the already acquired shared service. */
export const runtimeHandle = Effect.fn("NativeRuntime.handle")(function* () {
	const runtime = yield* NativeRuntime
	const handle = owners.get(runtime)
	if (handle === undefined) return yield* Effect.fail(invalid("NativeRuntime.handle"))
	return handle
})

/** Internal first executor consumer; not a replacement row codec or public hash API. */
export const hashChunk = Effect.fn("bumbledb.hashChunk")(function* (input: Uint8Array) {
	const runtime = yield* NativeRuntime
	const handle = owners.get(runtime)
	if (handle === undefined) return yield* Effect.fail(invalid("hashChunk"))
	return yield* nativeOperation(
		"hashChunk",
		(callback) => {
			if (!(input instanceof Uint8Array) || !(input.buffer instanceof ArrayBuffer)) throw invalid("hashChunk")
			return runtimeNative.runtimeHash(handle, input, callback)
		},
		(value) => {
			if (value === null) throw dbError("hashChunk", { _tag: "Internal" })
			return value
		}
	)
})
