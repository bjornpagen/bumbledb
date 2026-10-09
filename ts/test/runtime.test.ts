import assert from "node:assert/strict"
import { test } from "node:test"
import { Cause, Effect, Exit, Fiber, Layer, ManagedRuntime } from "effect"
import { CloseFailure, DbError, dbError, runtimeErrorCodes } from "../src/errors.ts"
import { call, release } from "../src/native/op.ts"
import { native } from "../src/native.ts"
import type { BumbleOptions } from "../src/runtime.ts"
import { Bumble, runtimeHandle } from "../src/runtime.ts"
import type { CloseWire, OptionsWire, RuntimeHandle } from "../src/runtime-native.ts"
import { runtimeNative } from "../src/runtime-native.ts"

const configuration: BumbleOptions = {
	workers: 2,
	queueCapacity: 8,
	cleanupCapacity: 16,
	ownerCapacity: 16,
	nativeHandleCapacity: 32,
	cleanupTimeout: "1 second"
}
const wire: OptionsWire = { ...configuration, cleanupTimeoutMs: 1000 }
const close = (handle: RuntimeHandle) =>
	new Promise<CloseWire>((resolve) => runtimeNative.runtimeClose(handle, resolve))

const hashChunk = (input: Uint8Array) =>
	Effect.gen(function* () {
		const handle = yield* runtimeHandle
		return yield* call("hashChunk", (done) => runtimeNative.runtimeHash(handle, input, done), runtimeNative.runtimeTake)
	})

test("native error roster matches; structured backpressure preserves exact counters", async () => {
	assert.deepEqual(runtimeNative.runtimeErrorCodes(), runtimeErrorCodes)
	const exact = (1n << 63n) + 1n
	const error = dbError("test.handles", {
		_tag: "ResourceLimit",
		dimension: "nativeHandleCount",
		used: exact,
		requested: 1n,
		limit: exact
	})
	assert.deepEqual(error.reason, {
		_tag: "ResourceLimit",
		dimension: "nativeHandleCount",
		used: exact,
		requested: 1n,
		limit: exact
	})
	const recovered = Effect.fail(error).pipe(
		Effect.catchReason("DbError", "ResourceLimit", (reason) => Effect.succeed(reason.limit))
	)
	assert.equal(await Effect.runPromise(recovered), exact)
	const runtime = ManagedRuntime.make(Bumble.layer({ workers: 0 }))
	try {
		const exit = await runtime.runPromiseExit(Bumble)
		assert.ok(Exit.isFailure(exit))
		const reason = exit.cause.reasons.find(Cause.isFailReason)
		assert.ok(reason?.error instanceof DbError)
		assert.equal(reason.error.code, "InvalidArgument")
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
	}
})

test("layer and hash effects are lazy, repeatable and accept independently owned input", async () => {
	const layer = Bumble.layer(configuration)
	const input = new Uint8Array([1, 2, 3])
	const effect = hashChunk(input)
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
				return yield* (yield* Bumble).inspect()
			})
		)
		assert.equal(inspection.retained, 0n)
		assert.equal(inspection.active, 0n)
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
	}
})

test("one reused Layer shares the runtime, independent layers refuse", async () => {
	const layer = Bumble.layer(configuration)
	const runtime = ManagedRuntime.make(Layer.merge(layer, layer))
	try {
		const [left, right] = await Promise.all([runtime.runPromise(Bumble), runtime.runPromise(Bumble)])
		assert.equal(left, right)
		const other = ManagedRuntime.make(Bumble.layer(configuration))
		try {
			const exit = await other.runPromiseExit(Bumble)
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

test("native admission bounds outstanding jobs and releases their slots on take", async () => {
	const handle = runtimeNative.runtimeOpen(wire)
	try {
		const operations = []
		// Outstanding operations include the worker slots plus the queue.
		// Wait for each completion, retaining outputs so queue timing is irrelevant.
		for (let byte = 0; byte < 10; byte++) {
			const input = new Uint8Array([byte])
			const ready = Promise.withResolvers<void>()
			const operation = runtimeNative.runtimeHash(handle, input, ready.resolve)
			await ready.promise
			operations.push({ operation, input })
		}
		assert.equal(runtimeNative.runtimeInspect(handle).retained, 10n)
		assert.throws(() => runtimeNative.runtimeReady(handle, () => {}), { _tag: "QueueFull" })
		for (const { operation, input } of operations) {
			assert.deepEqual(runtimeNative.runtimeTake(operation), native.blake3Hash(input))
		}
		assert.equal(runtimeNative.runtimeInspect(handle).retained, 0n)
	} finally {
		assert.equal((await close(handle)).kind, "closed")
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
			assert.throws(() => runtimeNative.runtimeHash(handle, input, () => assert.fail("invalid input ran")), {
				_tag: "InvalidArgument"
			})
		}
		assert.equal(runtimeNative.runtimeInspect(handle).retained, 0n)
	} finally {
		assert.equal((await close(handle)).kind, "closed")
	}
})

test("scheduling defaults work, invalid counts refuse, and hash inputs have no chunk quota", async () => {
	for (const workers of [-1, 0, 1.5, Number.NaN, 0x100000000]) {
		assert.throws(() => runtimeNative.runtimeOpen({ workers }), { _tag: "InvalidArgument" })
	}
	const runtime = ManagedRuntime.make(Bumble.layer())
	try {
		const input = new Uint8Array(1_000_001).fill(17)
		assert.deepEqual(await runtime.runPromise(hashChunk(input)), native.blake3Hash(input))
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
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
			const fiber = Effect.runFork(Effect.scoped(Layer.build(Bumble.layer(configuration))))
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
			const effect = call(
				"interruption-test",
				(done) => {
					const lease = runtimeNative.runtimeHash(handle, input, done)
					started.resolve()
					return lease
				},
				runtimeNative.runtimeTake
			)
			const fiber = Effect.runFork(effect)
			await started.promise
			await Effect.runPromise(Fiber.interrupt(fiber))
			const exit = await Effect.runPromise(Fiber.await(fiber))
			assert.equal(Exit.hasInterrupts(exit), true)
			const inspection = runtimeNative.runtimeInspect(handle)
			assert.equal(inspection.retained, 0n)
			assert.equal(inspection.active, 0n)
		}
	} finally {
		assert.equal((await close(handle)).kind, "closed")
	}
})

test("scope close reclaims workers with wrappers retained and permits a successor", async () => {
	const retained = []
	for (let count = 0; count < 30; count++) {
		const runtime = ManagedRuntime.make(Bumble.layer(configuration))
		const service = await runtime.runPromise(Bumble)
		retained.push(service)
		await runtime.runPromise(hashChunk(new Uint8Array([count])))
		await Effect.runPromise(runtime.disposeEffect)
		const exit = await Effect.runPromiseExit(service.inspect())
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
					release("test.close", (done) =>
						done({
							kind: "incomplete",
							outstanding: {
								phase: "closing",
								active: 1n,
								queued: 0n,
								retained: 1n,
								owners: 0n,
								databases: 0n,
								natives: 0n
							}
						})
					)
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

test("an interrupted operation whose cancel drain is incomplete keeps the interrupt and adds CloseFailure", async () => {
	const outstanding = {
		phase: "closing" as const,
		queued: 0n,
		active: 1n,
		retained: 1n,
		owners: 0n,
		databases: 0n,
		natives: 1n
	}
	const started = Promise.withResolvers<void>()
	const effect = Effect.callback<never>(() => {
		started.resolve()
		return release("test.cancel", (done) => done({ kind: "incomplete", outstanding }))
	})
	const fiber = Effect.runFork(effect)
	await started.promise
	await Effect.runPromise(Fiber.interrupt(fiber))
	const exit = await Effect.runPromise(Fiber.await(fiber))
	assert.equal(Exit.hasInterrupts(exit), true)
	assert.ok(Exit.isFailure(exit))
	assert.ok(exit.cause.reasons.some((reason) => Cause.isDieReason(reason) && reason.defect instanceof CloseFailure))
})
