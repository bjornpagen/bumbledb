import { Context, Duration, Effect, Layer } from "effect"
import { call, drain, scoped } from "./native/op.ts"
import type { CloseReport, OutstandingWork } from "./runtime-errors.ts"
import { DbError, dbError } from "./runtime-errors.ts"
import type { OptionsWire, RuntimeHandle } from "./runtime-native.ts"
import { runtimeNative } from "./runtime-native.ts"

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

const acquire = Effect.fn("NativeRuntime.acquire")(function* (configuration: NativeRuntimeOptions) {
	const wire = yield* Effect.try({
		try: () => options(configuration),
		catch: (cause) => dbError("NativeRuntime.acquire", cause)
	})
	const owner = yield* scoped(
		"NativeRuntime.release",
		Effect.try({
			try: () => runtimeNative.runtimeOpen(wire),
			catch: (cause) => dbError("NativeRuntime.acquire", cause)
		}),
		(handle) => (done) => runtimeNative.runtimeClose(handle, done)
	)
	yield* call("NativeRuntime.acquire", (done) => runtimeNative.runtimeReady(owner, done), runtimeNative.runtimeTake)
	const service: RuntimeService = {
		close: () => drain("NativeRuntime.close", (done) => runtimeNative.runtimeClose(owner, done)),
		inspect: Effect.fn("NativeRuntime.inspect")(function* () {
			yield* call("NativeRuntime.inspect", (done) => runtimeNative.runtimeReady(owner, done), runtimeNative.runtimeTake)
			return runtimeNative.runtimeInspect(owner)
		})
	}
	owners.set(service, owner)
	return service
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
