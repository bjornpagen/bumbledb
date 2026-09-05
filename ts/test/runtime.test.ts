import assert from "node:assert/strict"
import { test } from "node:test"
import { Cause, Effect, Exit, Fiber, Layer, ManagedRuntime } from "effect"
import { native } from "#native.ts"
import type { ExecutionPolicy, NativeRuntimeOptions } from "#runtime.ts"
import { finalizeClose, hashChunk, NativeRuntime, nativeOperation } from "#runtime.ts"
import { CloseFailure, DbError, runtimeErrorCodes } from "#runtime-errors.ts"
import type { CloseWire, OptionsWire, PolicyWire, RuntimeHandle } from "#runtime-native.ts"
import { runtimeNative } from "#runtime-native.ts"

const configuration: NativeRuntimeOptions = {
	workers: 2,
	queueCapacity: 8,
	cleanupCapacity: 16,
	ownerCapacity: 16,
	nativeHandleCapacity: 32,
	inputBytes: 8_000_000n,
	workingBytes: 8_000_000n,
	scratchBytes: 0n,
	resultBytes: 4096n,
	chunkBytes: 1_000_000n,
	cleanupTimeout: "1 second"
}
const wire: OptionsWire = { ...configuration, cleanupTimeoutMs: 1000 }
const work: ExecutionPolicy = {
	inputBytes: 1_000_000n,
	workingBytes: 1_000_000n,
	scratchBytes: 0n,
	resultBytes: 32n,
	rows: 0n,
	workUnits: 1_000_000n,
	timeout: "5 seconds"
}
const policy: PolicyWire = { ...work, timeoutMs: 5000 }
const inspectWork: ExecutionPolicy = {
	inputBytes: 0n,
	workingBytes: 0n,
	scratchBytes: 0n,
	resultBytes: 0n,
	rows: 0n,
	workUnits: 1n,
	timeout: "5 seconds"
}

const close = (handle: RuntimeHandle) =>
	new Promise<CloseWire>((resolve) => runtimeNative.runtimeClose(handle, resolve))

test("native runtime error roster is complete and structured reasons preserve exact counters", async () => {
	assert.deepEqual(runtimeNative.runtimeErrorCodes(), runtimeErrorCodes)
	const runtime = ManagedRuntime.make(NativeRuntime.layer(configuration))
	try {
		const exit = await runtime.runPromiseExit(hashChunk(new Uint8Array(10), { ...work, workingBytes: 9n }))
		assert.equal(exit._tag, "Failure")
		if (exit._tag !== "Failure") return
		const reason = exit.cause.reasons.find(Cause.isFailReason)
		assert.ok(reason)
		assert.ok(reason.error instanceof DbError)
		assert.deepEqual(reason.error.reason, {
			_tag: "ResourceLimit",
			dimension: "workingBytes",
			used: 0n,
			requested: 10n,
			limit: 9n
		})
		const recovered = hashChunk(new Uint8Array(10), { ...work, workingBytes: 9n }).pipe(
			Effect.catchReason("DbError", "ResourceLimit", (reason) => Effect.succeed(reason.limit))
		)
		// catchReason retains DbError in E in the pinned RC; don't erase it.
		const typed: Effect.Effect<Uint8Array | bigint, DbError, NativeRuntime> = recovered
		assert.equal(await runtime.runPromise(typed), 9n)
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
	}
})

test("layer and hash effects are lazy, repeatable and accept independently owned input", async () => {
	const layer = NativeRuntime.layer(configuration)
	const input = new Uint8Array([1, 2, 3])
	const effect = hashChunk(input, work)
	// Merely constructing the layer/effect has not opened the singleton.
	const proof = runtimeNative.runtimeOpen(wire)
	assert.equal((await close(proof)).kind, "closed")
	const runtime = ManagedRuntime.make(layer)
	try {
		input[0] = 7
		const first = await runtime.runPromise(effect)
		assert.deepEqual(first, native.blake3Hash(input))
		input[0] = 8
		const second = await runtime.runPromise(effect)
		assert.deepEqual(second, native.blake3Hash(input))
		assert.notDeepEqual(first, second)
		const inspection = await runtime.runPromise(
			Effect.gen(function* () {
				return yield* (yield* NativeRuntime).inspect(inspectWork)
			})
		)
		assert.equal(inspection.retained, 0n)
		assert.equal(inspection.workingBytes, 0n)
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
	}
})

test("one reused Layer shares the runtime, independent layers refuse", async () => {
	const layer = NativeRuntime.layer(configuration)
	const runtime = ManagedRuntime.make(Layer.merge(layer, layer))
	try {
		const [left, right] = await Promise.all([runtime.runPromise(NativeRuntime), runtime.runPromise(NativeRuntime)])
		assert.equal(left, right)
		const other = ManagedRuntime.make(NativeRuntime.layer(configuration))
		try {
			const exit = await other.runPromiseExit(NativeRuntime)
			assert.equal(exit._tag, "Failure")
			if (exit._tag === "Failure") {
				const reason = exit.cause.reasons.find(Cause.isFailReason)
				assert.ok(reason?.error instanceof DbError)
				assert.equal(reason.error.code, "RuntimeAlreadyLive")
			}
		} finally {
			await Effect.runPromise(other.disposeEffect)
		}
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
	}
})

test("native ownership rejects shared, detached and forged-buffer typed arrays before reading", async () => {
	const handle = runtimeNative.runtimeOpen(wire)
	try {
		const shared = new Uint8Array(new SharedArrayBuffer(8))
		Object.defineProperty(shared, "buffer", { value: new ArrayBuffer(8) })
		const detached = new Uint8Array(8)
		structuredClone(detached.buffer, { transfer: [detached.buffer] })
		for (const input of [shared, detached]) {
			assert.throws(() => runtimeNative.runtimeHash(handle, policy, input, () => assert.fail("invalid input ran")), {
				_tag: "InvalidArgument"
			})
		}
		assert.equal(runtimeNative.runtimeInspect(handle).retained, 0n)
	} finally {
		assert.equal((await close(handle)).kind, "closed")
	}
})

test("native finite integer/chunk checks reject without reserving work", async () => {
	for (const workers of [-1, 0, 1.5, Number.NaN, 0x100000000]) {
		assert.throws(() => runtimeNative.runtimeOpen({ ...wire, workers }), { _tag: "InvalidArgument" })
	}
	const handle = runtimeNative.runtimeOpen(wire)
	try {
		for (const value of [-1n, 1n << 64n]) {
			assert.throws(
				() => runtimeNative.runtimeHash(handle, { ...policy, workUnits: value }, new Uint8Array(), () => {}),
				{ _tag: "InvalidArgument" }
			)
		}
		assert.throws(() => runtimeNative.runtimeHash(handle, policy, new Uint8Array(1_000_001), () => {}), {
			_tag: "ResourceLimit",
			dimension: "chunkBytes"
		})
		assert.equal(runtimeNative.runtimeInspect(handle).retained, 0n)
	} finally {
		assert.equal((await close(handle)).kind, "closed")
	}
})

test("interruption during actual native runtime acquisition reclaims late success", async () => {
	const original = runtimeNative.runtimeOpen
	for (let count = 0; count < 30; count++) {
		const started = Promise.withResolvers<void>()
		runtimeNative.runtimeOpen = (options) => {
			const handle = original(options)
			started.resolve()
			return handle
		}
		try {
			const fiber = Effect.runFork(Effect.scoped(Layer.build(NativeRuntime.layer(configuration))))
			await started.promise
			await Effect.runPromise(Fiber.interrupt(fiber))
			assert.equal(Exit.hasInterrupts(await Effect.runPromise(Fiber.await(fiber))), true)
			const successor = original(wire)
			assert.equal((await close(successor)).kind, "closed")
		} finally {
			runtimeNative.runtimeOpen = original
		}
	}
})

test("Effect interruption cancels and joins native work before the fiber finishes", async () => {
	const handle = runtimeNative.runtimeOpen(wire)
	const input = new Uint8Array(1_000_000)
	try {
		for (let count = 0; count < 25; count++) {
			const started = Promise.withResolvers<void>()
			const effect = nativeOperation(
				"interruption-test",
				(callback) => {
					const lease = runtimeNative.runtimeHash(handle, policy, input, callback)
					started.resolve()
					return lease
				},
				(value) => value
			)
			const fiber = Effect.runFork(effect)
			await started.promise
			await Effect.runPromise(Fiber.interrupt(fiber))
			const exit = await Effect.runPromise(Fiber.await(fiber))
			assert.equal(Exit.hasInterrupts(exit), true)
			const inspection = runtimeNative.runtimeInspect(handle)
			assert.equal(inspection.retained, 0n)
			assert.equal(inspection.workingBytes, 0n)
		}
	} finally {
		assert.equal((await close(handle)).kind, "closed")
	}
})

test("scope close reclaims workers with wrappers retained and permits a successor", async () => {
	const retained = []
	for (let count = 0; count < 30; count++) {
		const runtime = ManagedRuntime.make(NativeRuntime.layer(configuration))
		const service = await runtime.runPromise(NativeRuntime)
		retained.push(service)
		await runtime.runPromise(hashChunk(new Uint8Array([count]), work))
		await Effect.runPromise(runtime.disposeEffect)
		assert.equal((await Effect.runPromise(service.close())).kind, "closed")
		const exit = await Effect.runPromiseExit(service.inspect(inspectWork))
		assert.equal(exit._tag, "Failure")
	}
	assert.equal(retained.length, 30)
})

test("incomplete finalization remains a structured defect alongside a known result", async () => {
	const receipt = { kind: "decided", sequence: 7n }
	let observed: typeof receipt | undefined
	const exit = await Effect.runPromiseExit(
		Effect.scoped(
			Effect.gen(function* () {
				yield* Effect.acquireRelease(Effect.void, () =>
					finalizeClose("test.close", {
						kind: "incomplete",
						outstanding: {
							phase: "closing",
							active: 1n,
							queued: 0n,
							retained: 1n,
							inputBytes: 1n,
							workingBytes: 1n,
							scratchBytes: 0n,
							resultBytes: 0n
						}
					})
				)
				observed = receipt
				return receipt
			})
		)
	)
	assert.equal(observed, receipt)
	assert.equal(exit._tag, "Failure")
	if (exit._tag === "Failure")
		assert.ok(exit.cause.reasons.some((reason) => Cause.isDieReason(reason) && reason.defect instanceof CloseFailure))
})
