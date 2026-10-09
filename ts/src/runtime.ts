import { Context, Duration, Effect, Layer } from "effect"
import type { OutstandingWork } from "./errors.ts"
import { DbError, dbError } from "./errors.ts"
import type { RuntimeRef } from "./native/addon.ts"
import { addon } from "./native/addon.ts"
import type { RuntimeOptionsIn } from "./native/binding.d.ts"
import { call, scoped } from "./native/op.ts"

/** Sizing for the native runtime. The addon validates every count and picks the defaults. */
export interface BumbleOptions {
	readonly workers?: number
	readonly queueCapacity?: number
	readonly cleanupCapacity?: number
	readonly ownerCapacity?: number
	readonly nativeHandleCapacity?: number
	readonly cleanupTimeout?: Duration.Input
}

function wire(options: BumbleOptions): string {
	const timeout = options.cleanupTimeout === undefined ? undefined : Duration.toMillis(options.cleanupTimeout)
	if (timeout !== undefined && !Number.isFinite(timeout)) {
		throw new DbError({ operation: "Bumble.layer", reason: { _tag: "InvalidArgument" } })
	}
	const input: RuntimeOptionsIn = {
		...(options.workers === undefined ? {} : { workers: options.workers }),
		...(options.queueCapacity === undefined ? {} : { queueCapacity: options.queueCapacity }),
		...(options.cleanupCapacity === undefined ? {} : { cleanupCapacity: options.cleanupCapacity }),
		...(options.ownerCapacity === undefined ? {} : { ownerCapacity: options.ownerCapacity }),
		...(options.nativeHandleCapacity === undefined ? {} : { nativeHandleCapacity: options.nativeHandleCapacity }),
		...(timeout === undefined ? {} : { cleanupTimeoutMs: Math.ceil(timeout) })
	}
	return JSON.stringify(input)
}

/** The live native runtime: one worker pool and handle registry per process. */
export class Runtime {
	readonly #handle: RuntimeRef
	/** The runtime's outstanding native work, after every operation queued before it settles. */
	readonly inspect: () => Effect.Effect<OutstandingWork, DbError>

	constructor(handle: RuntimeRef) {
		this.#handle = handle
		this.inspect = Effect.fn("Bumble.inspect")(function* () {
			yield* call("Bumble.inspect", (done) => addon.runtimeReady(handle, done), addon.runtimeTake)
			return addon.runtimeInspect(handle) as OutstandingWork
		})
	}

	static handle(runtime: Runtime): RuntimeRef {
		return runtime.#handle
	}
}

const acquire = Effect.fn("Bumble.layer")(function* (options: BumbleOptions) {
	const input = yield* Effect.try({ try: () => wire(options), catch: (cause) => dbError("Bumble.layer", cause) })
	const handle = yield* scoped(
		"Bumble.release",
		Effect.try({ try: () => addon.runtimeOpen(input), catch: (cause) => dbError("Bumble.layer", cause) }),
		(owner) => (done) => addon.runtimeClose(owner, done)
	)
	yield* call("Bumble.layer", (done) => addon.runtimeReady(handle, done), addon.runtimeTake)
	return new Runtime(handle)
})

/** The native runtime as an Effect service. Provide `Bumble.layer()` once in the application graph. */
export class Bumble extends Context.Service<Bumble, Runtime>()("@bjornpagen/bumbledb/Bumble") {
	static layer(options: BumbleOptions = {}): Layer.Layer<Bumble, DbError> {
		return Layer.effect(Bumble, acquire(options))
	}
}

/** The provided runtime's native handle. */
export const runtimeHandle: Effect.Effect<RuntimeRef, never, Bumble> = Effect.gen(function* () {
	return Runtime.handle(yield* Bumble)
})
