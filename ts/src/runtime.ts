import { Context, Duration, Effect, Layer } from "effect"
import type { OutstandingWork } from "./errors.ts"
import { DbError, dbError } from "./errors.ts"
import { call, scoped } from "./native/op.ts"
import type { OptionsWire, RuntimeHandle } from "./runtime-native.ts"
import { runtimeNative } from "./runtime-native.ts"

/** Sizing for the native runtime. The addon validates every count and picks the defaults. */
export interface BumbleOptions {
	readonly workers?: number
	readonly queueCapacity?: number
	readonly cleanupCapacity?: number
	readonly ownerCapacity?: number
	readonly nativeHandleCapacity?: number
	readonly cleanupTimeout?: Duration.Input
}

function wire(options: BumbleOptions): OptionsWire {
	const timeout = options.cleanupTimeout === undefined ? undefined : Duration.toMillis(options.cleanupTimeout)
	if (timeout !== undefined && !Number.isFinite(timeout)) {
		throw new DbError({ operation: "Bumble.layer", reason: { _tag: "InvalidArgument" } })
	}
	return {
		workers: options.workers,
		queueCapacity: options.queueCapacity,
		cleanupCapacity: options.cleanupCapacity,
		ownerCapacity: options.ownerCapacity,
		nativeHandleCapacity: options.nativeHandleCapacity,
		cleanupTimeoutMs: timeout === undefined ? undefined : Math.ceil(timeout)
	}
}

/** The live native runtime: one worker pool and handle registry per process. */
export class Runtime {
	readonly #handle: RuntimeHandle
	/** The runtime's outstanding native work, after every operation queued before it settles. */
	readonly inspect: () => Effect.Effect<OutstandingWork, DbError>

	constructor(handle: RuntimeHandle) {
		this.#handle = handle
		this.inspect = Effect.fn("Bumble.inspect")(function* () {
			yield* call("Bumble.inspect", (done) => runtimeNative.runtimeReady(handle, done), runtimeNative.runtimeTake)
			return runtimeNative.runtimeInspect(handle)
		})
	}

	static handle(runtime: Runtime): RuntimeHandle {
		return runtime.#handle
	}
}

const acquire = Effect.fn("Bumble.layer")(function* (options: BumbleOptions) {
	const input = yield* Effect.try({ try: () => wire(options), catch: (cause) => dbError("Bumble.layer", cause) })
	const handle = yield* scoped(
		"Bumble.release",
		Effect.try({ try: () => runtimeNative.runtimeOpen(input), catch: (cause) => dbError("Bumble.layer", cause) }),
		(owner) => (done) => runtimeNative.runtimeClose(owner, done)
	)
	yield* call("Bumble.layer", (done) => runtimeNative.runtimeReady(handle, done), runtimeNative.runtimeTake)
	return new Runtime(handle)
})

/** The native runtime as an Effect service. Provide `Bumble.layer()` once in the application graph. */
export class Bumble extends Context.Service<Bumble, Runtime>()("@bjornpagen/bumbledb/Bumble") {
	static layer(options: BumbleOptions = {}): Layer.Layer<Bumble, DbError> {
		return Layer.effect(Bumble, acquire(options))
	}
}

/** The provided runtime's native handle. */
export const runtimeHandle: Effect.Effect<RuntimeHandle, never, Bumble> = Effect.gen(function* () {
	return Runtime.handle(yield* Bumble)
})
