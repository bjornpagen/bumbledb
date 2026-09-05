import { Context, Duration, Effect, Layer } from "effect"
import type { CloseReport, OutstandingWork } from "#runtime-errors.ts"
import { CloseFailure, DbError, dbError } from "#runtime-errors.ts"
import type { CloseWire, OperationHandle, OptionsWire, PolicyWire, RuntimeHandle } from "#runtime-native.ts"
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

function policy(work: ExecutionPolicy, operation: string): PolicyWire {
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
	return report.kind === "failed" ? { kind: "failed", error: dbError(operation, { _tag: "QueueFull" }) } : report
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

function close(handle: RuntimeHandle): Effect.Effect<CloseReport> {
	return drain("NativeRuntime.close", (callback) => runtimeNative.runtimeClose(handle, callback))
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
	return Effect.callback((resume, signal) => {
		let lease: OperationHandle
		try {
			lease = start(() => {
				if (signal.aborted) return
				try {
					resume(Effect.succeed(accept(runtimeNative.runtimeTake(lease))))
				} catch (cause) {
					resume(Effect.fail(failure(operation, cause)))
				}
			})
		} catch (cause) {
			resume(Effect.fail(failure(operation, cause)))
			return
		}
		return drain(`${operation}.cancel`, (callback) => runtimeNative.runtimeCancel(lease, callback)).pipe(
			Effect.flatMap((report) => finalizeClose(`${operation}.cancel`, report))
		)
	})
}

const acquire = Effect.fn("NativeRuntime.acquire")(function* (configuration: NativeRuntimeOptions) {
	return yield* Effect.acquireRelease(
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
						if (signal.aborted) return
						try {
							runtimeNative.runtimeTake(lease)
							const service: RuntimeService = {
								close: () => close(owner),
								inspect: Effect.fn("NativeRuntime.inspect")(function* (work) {
									yield* nativeOperation(
										"NativeRuntime.inspect",
										(callback) => runtimeNative.runtimeReady(owner, policy(work, "NativeRuntime.inspect"), callback),
										() => undefined
									)
									return runtimeNative.runtimeInspect(owner)
								})
							}
							owners.set(service, owner)
							resume(Effect.succeed(service))
						} catch (cause) {
							resume(
								close(owner).pipe(
									Effect.flatMap((report) => finalizeClose("NativeRuntime.acquire", report)),
									Effect.andThen(Effect.fail(failure("NativeRuntime.acquire", cause)))
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
								Effect.flatMap((report) => finalizeClose("NativeRuntime.acquire", report)),
								Effect.andThen(Effect.fail(error))
							)
				)
			}
			return handle === undefined
				? Effect.void
				: close(handle).pipe(Effect.flatMap((report) => finalizeClose("NativeRuntime.acquire.cancel", report)))
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

/** Internal first executor consumer; not a replacement row codec or public hash API. */
export const hashChunk = Effect.fn("bumbledb.hashChunk")(function* (input: Uint8Array, work: ExecutionPolicy) {
	const runtime = yield* NativeRuntime
	const handle = owners.get(runtime)
	if (handle === undefined) return yield* Effect.fail(invalid("hashChunk"))
	return yield* nativeOperation(
		"hashChunk",
		(callback) => {
			if (!(input instanceof Uint8Array) || !(input.buffer instanceof ArrayBuffer)) throw invalid("hashChunk")
			return runtimeNative.runtimeHash(handle, policy(work, "hashChunk"), input, callback)
		},
		(value) => {
			if (value === null) throw dbError("hashChunk", { _tag: "Internal" })
			return value
		}
	)
})
