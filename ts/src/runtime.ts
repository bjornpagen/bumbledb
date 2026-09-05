import { Context, Duration, Effect, Exit, Layer } from "effect"
import type { CloseReport, OutstandingWork } from "#runtime-errors.ts"
import { CloseFailure, DbError, dbError } from "#runtime-errors.ts"
import type { CloseWire, OperationHandle, OptionsWire, PolicyWire, RepositoryLockHandle, RuntimeHandle } from "#runtime-native.ts"
import { runtimeNative } from "#runtime-native.ts"

export interface ExecutionPolicy {
	readonly inputBytes: bigint
	readonly workingBytes: bigint
	readonly scratchBytes: bigint
	readonly resultBytes: bigint
	readonly rows: bigint
	readonly workUnits: bigint
	readonly timeout: Duration.Input
}

export interface NativeRuntimeOptions {
	readonly workers: number
	readonly queueCapacity: number
	readonly cleanupCapacity: number
	readonly ownerCapacity: number
	readonly nativeHandleCapacity: number
	readonly inputBytes: bigint
	readonly workingBytes: bigint
	readonly scratchBytes: bigint
	readonly resultBytes: bigint
	readonly chunkBytes: bigint
	readonly cleanupTimeout: Duration.Input
}

interface RuntimeService {
	readonly close: () => Effect.Effect<CloseReport>
	readonly inspect: (work: ExecutionPolicy) => Effect.Effect<OutstandingWork, DbError>
}

const owners = new WeakMap<RuntimeService, RuntimeHandle>()

function invalid(operation: string): DbError {
	return new DbError({ operation, reason: { _tag: "InvalidArgument" } })
}

function count(value: number, operation: string): number {
	if (!Number.isSafeInteger(value) || value <= 0 || value > 0xffffffff) throw invalid(operation)
	return value
}

function bytes(value: bigint, operation: string): bigint {
	if (typeof value !== "bigint" || value < 0n || value > 0xffffffffffffffffn) throw invalid(operation)
	return value
}

function millis(value: Duration.Input, operation: string): number {
	const duration = Duration.toMillis(value)
	if (!Number.isFinite(duration) || duration < 0 || duration > 0xffffffff) throw invalid(operation)
	return Math.ceil(duration)
}

export function policyWire(work: ExecutionPolicy, operation: string): PolicyWire {
	return {
		inputBytes: bytes(work.inputBytes, operation),
		workingBytes: bytes(work.workingBytes, operation),
		scratchBytes: bytes(work.scratchBytes, operation),
		resultBytes: bytes(work.resultBytes, operation),
		rows: bytes(work.rows, operation),
		workUnits: bytes(work.workUnits, operation),
		timeoutMs: millis(work.timeout, operation)
	}
}

function options(value: NativeRuntimeOptions): OptionsWire {
	const operation = "NativeRuntime.acquire"
	return {
		workers: count(value.workers, operation),
		queueCapacity: count(value.queueCapacity, operation),
		cleanupCapacity: count(value.cleanupCapacity, operation),
		ownerCapacity: count(value.ownerCapacity, operation),
		nativeHandleCapacity: count(value.nativeHandleCapacity, operation),
		inputBytes: bytes(value.inputBytes, operation),
		workingBytes: bytes(value.workingBytes, operation),
		scratchBytes: bytes(value.scratchBytes, operation),
		resultBytes: bytes(value.resultBytes, operation),
		chunkBytes: bytes(value.chunkBytes, operation),
		cleanupTimeoutMs: count(millis(value.cleanupTimeout, operation), operation)
	}
}

function failure(operation: string, cause: unknown): DbError {
	return cause instanceof DbError ? cause : dbError(operation, cause)
}

function closeReport(operation: string, report: CloseWire): CloseReport {
	// A payload-less native "failed" drain (cleanup capacity exhausted, or
	// owner teardown failed) is bounded diagnostic data per chapter 35's
	// CloseReport policy: it decodes as the core `Internal` reason — the
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

function attachInterruptClose(operation: string, stash: { report?: CloseReport }): <A, E, R>(
	effect: Effect.Effect<A, E, R>
) => Effect.Effect<A, E, R> {
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

/** Independent caps intersect: a delivery request cannot enlarge `work.resultBytes`. */
export function deliveryResultBytes(requested: bigint, work: ExecutionPolicy): bigint {
	if (typeof requested !== "bigint" || requested < 0n) {
		throw invalid("deliveryResultBytes")
	}
	return requested < work.resultBytes ? requested : work.resultBytes
}

function close(handle: RuntimeHandle): Effect.Effect<CloseReport> {
	return drain("NativeRuntime.close", (callback) => runtimeNative.runtimeClose(handle, callback))
}

/** Kick `runtimeCancel` without joining. Used when the completion
 * callback arrives after abort: JS must not `take`; L12 drain reclaims
 * queued Page/Rows. A second call is native-idempotent (Committed
 * mutations stay; only unpublished delivery is dropped).
 */
function startCancelDrain(
	operation: string,
	lease: OperationHandle,
	stash: { report?: CloseReport }
): void {
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
	return attachInterruptClose(cancelOp, stash)(
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
			return joinInterruptDrain(
				cancelOp,
				(callback) => runtimeNative.runtimeCancel(lease, callback),
				stash
			)
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
						const lease = runtimeNative.runtimeReady(
							owner,
							{
								inputBytes: 0n,
								workingBytes: 0n,
								scratchBytes: 0n,
								resultBytes: 0n,
								rows: 0n,
								workUnits: 1n,
								timeoutMs: wire.cleanupTimeoutMs
							},
							() => {
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
										inspect: Effect.fn("NativeRuntime.inspect")(function* (work) {
											yield* nativeOperation(
												"NativeRuntime.inspect",
												(callback) =>
													runtimeNative.runtimeReady(owner, policyWire(work, "NativeRuntime.inspect"), callback),
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
											Effect.flatMap((report) =>
												afterClose("NativeRuntime.acquire", report, Effect.fail(error))
											)
										)
									)
								}
							}
						)
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
	static layer(options: NativeRuntimeOptions): Layer.Layer<NativeRuntime, DbError> {
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

/**
 * Opaque kernel fence (C8). Backed by L14 `log_repository_lock_*` and
 * L12 `mint_repository_lock` (`NativeKind::RepositoryLock`). Native
 * exclusion is the owner — there is no JS occupancy table. `release`
 * is the one close: it joins `logRepositoryLockRelease` then clears
 * `slot.owner` so the Scope finalizer is a no-op. Callers join host
 * I/O first (`joinPendingIo.pipe(Effect.andThen(lock.release))`).
 */
export interface RepositoryLock {
	readonly directory: string
	readonly release: Effect.Effect<void>
}

function joinLockRelease(operation: string, owner: RepositoryLockHandle): Effect.Effect<void> {
	return drain(operation, (callback) => runtimeNative.logRepositoryLockRelease(owner, callback)).pipe(
		Effect.flatMap((report) => finalizeClose(operation, report))
	)
}

/**
 * Internal log seam: stamped mint only. Cleanup is registered before
 * the interruptible acquire (TS-003): `acquireRelease({ interruptible })`
 * installs the finalizer only after acquire succeeds. Interrupt, defect,
 * and acquire failure all run the same slot finalizer — empty slot is
 * a no-op. No `runtimeDirectoryAcquire`, no core `Db`, no JS bookkeeping.
 */
export const internalAcquireRepositoryLock = Effect.fn("internalAcquireRepositoryLock")(function* (
	operation: string,
	directory: string,
	work: ExecutionPolicy
) {
	if (directory.length === 0) {
		return yield* Effect.fail(invalid(operation))
	}
	const runtime = yield* runtimeHandle()
	const wire = yield* Effect.try({
		try: () => policyWire(work, operation),
		catch: () => invalid(operation)
	})
	const closeOp = `${operation}.repositoryLock`
	return yield* Effect.uninterruptibleMask((restore) =>
		Effect.gen(function* () {
			const slot: { owner?: RepositoryLockHandle } = {}
			const release = Effect.suspend(() => {
				const held = slot.owner
				if (held === undefined) {
					return Effect.void
				}
				return joinLockRelease(closeOp, held).pipe(
					Effect.ensuring(
						Effect.sync(() => {
							slot.owner = undefined
						})
					)
				)
			})
			yield* Effect.addFinalizer(() => release)
			const owner = yield* restore(
				nativeOperationWith(
					operation,
					(callback) => runtimeNative.logRepositoryLockAcquire(runtime, wire, directory, callback),
					runtimeNative.logRepositoryLockTake,
					(value) => value
				)
			)
			slot.owner = owner
			return Object.freeze({
				directory,
				release
			})
		})
	)
})

/** Internal first executor consumer; not a replacement row codec or public hash API. */
export const hashChunk = Effect.fn("bumbledb.hashChunk")(function* (input: Uint8Array, work: ExecutionPolicy) {
	const runtime = yield* NativeRuntime
	const handle = owners.get(runtime)
	if (handle === undefined) return yield* Effect.fail(invalid("hashChunk"))
	return yield* nativeOperation(
		"hashChunk",
		(callback) => {
			if (!(input instanceof Uint8Array) || !(input.buffer instanceof ArrayBuffer)) throw invalid("hashChunk")
			return runtimeNative.runtimeHash(handle, policyWire(work, "hashChunk"), input, callback)
		},
		(value) => {
			if (value === null) throw dbError("hashChunk", { _tag: "Internal" })
			return value
		}
	)
})
