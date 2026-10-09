import assert from "node:assert/strict"
import { test } from "node:test"
import { Cause, Effect, Exit, Fiber, Layer, ManagedRuntime } from "effect"
import { compiledOf } from "../src/compile.ts"
import { CloseFailure, DbError, dbError } from "../src/errors.ts"
import { u64 } from "../src/fields.ts"
import type { RuntimeRef } from "../src/native/addon.ts"
import { addon } from "../src/native/addon.ts"
import type { CloseOut } from "../src/native/binding.d.ts"
import { call, release } from "../src/native/op.ts"
import { relation } from "../src/relation.ts"
import type { BumbleOptions } from "../src/runtime.ts"
import { Bumble, runtimeHandle } from "../src/runtime.ts"
import { schema } from "../src/schema.ts"

const configuration: BumbleOptions = {
	workers: 2,
	queueCapacity: 8,
	cleanupCapacity: 16,
	ownerCapacity: 16,
	nativeHandleCapacity: 32,
	cleanupTimeout: "1 second"
}
const wire = JSON.stringify({
	workers: 2,
	queueCapacity: 8,
	cleanupCapacity: 16,
	ownerCapacity: 16,
	nativeHandleCapacity: 32,
	cleanupTimeoutMs: 1000
})
const close = (handle: RuntimeRef) => new Promise<CloseOut>((resolve) => addon.runtimeClose(handle, resolve))

const ready = Effect.gen(function* () {
	const handle = yield* runtimeHandle
	return yield* call("ready", (done) => addon.runtimeReady(handle, done), addon.runtimeTake)
})

test("structured backpressure keeps exact counters and is recoverable by reason", async () => {
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
})

test("a runtime the addon refuses to size fails the layer with a typed reason", async () => {
	const runtime = ManagedRuntime.make(Bumble.layer({ workers: 0 }))
	try {
		const exit = await runtime.runPromiseExit(Bumble)
		assert.ok(Exit.isFailure(exit))
		const reason = exit.cause.reasons.find(Cause.isFailReason)
		assert.ok(reason?.error instanceof DbError)
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
	}
})

test("the layer is lazy and its effects are repeatable", async () => {
	const layer = Bumble.layer(configuration)
	const proof = addon.runtimeOpen(wire)
	assert.equal((await close(proof))._tag, "Closed", "constructing the layer started no runtime")
	const runtime = ManagedRuntime.make(layer)
	try {
		await runtime.runPromise(ready)
		await runtime.runPromise(ready)
		const inspection = await runtime.runPromise(Effect.flatMap(Bumble, (service) => service.inspect()))
		assert.equal(inspection.retained, 0n)
		assert.equal(inspection.active, 0n)
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
	}
})

test("one reused Layer shares the runtime; an independent layer is refused", async () => {
	const layer = Bumble.layer(configuration)
	const runtime = ManagedRuntime.make(Layer.merge(layer, layer))
	try {
		const [left, right] = await Promise.all([runtime.runPromise(Bumble), runtime.runPromise(Bumble)])
		assert.equal(left, right)
		const other = ManagedRuntime.make(Bumble.layer(configuration))
		try {
			const exit = await other.runPromiseExit(Bumble)
			assert.ok(Exit.isFailure(exit))
			const reason = exit.cause.reasons.find(Cause.isFailReason)
			assert.ok(reason?.error instanceof DbError)
			assert.equal(reason.error.code, "RuntimeAlreadyLive")
		} finally {
			await Effect.runPromise(other.disposeEffect)
		}
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
	}
})

test("admission bounds outstanding operations and frees their slots on take", async () => {
	const handle = addon.runtimeOpen(wire)
	try {
		const operations = []
		for (let index = 0; index < 10; index++) {
			const done = Promise.withResolvers<void>()
			const operation = addon.runtimeReady(handle, done.resolve)
			await done.promise
			operations.push(operation)
		}
		assert.equal(addon.runtimeInspect(handle).retained, 10n)
		assert.throws(() => addon.runtimeReady(handle, () => {}), { _tag: "QueueFull" })
		for (const operation of operations) addon.runtimeTake(operation)
		assert.equal(addon.runtimeInspect(handle).retained, 0n)
	} finally {
		assert.equal((await close(handle))._tag, "Closed")
	}
})

test("the addon refuses shared and detached byte buffers before reading them", async () => {
	const Row = relation("Row", { id: u64 })
	const theory = schema("Buffers", { Row }, [])
	const handle = addon.runtimeOpen(wire)
	try {
		const shared = new Uint8Array(new SharedArrayBuffer(8))
		const detached = new Uint8Array(8)
		structuredClone(detached.buffer, { transfer: [detached.buffer] })
		for (const input of [shared, detached]) {
			assert.throws(
				() =>
					addon.runtimeDecodeRows(handle, compiledOf(theory).handle, 0, input, () => assert.fail("invalid input ran")),
				{ _tag: "InvalidArgument" }
			)
		}
		assert.equal(addon.runtimeInspect(handle).retained, 0n)
	} finally {
		assert.equal((await close(handle))._tag, "Closed")
	}
})

test("malformed runtime options are refused with their JSON path", () => {
	assert.throws(() => addon.runtimeOpen(JSON.stringify({ workers: -1 })), { _tag: "Malformed" })
	assert.throws(() => addon.runtimeOpen(JSON.stringify({ workerz: 1 })), { _tag: "Malformed" })
})

test("interrupting runtime acquisition after the runtime opened still closes it", async () => {
	const original = addon.runtimeReady
	const held = Promise.withResolvers<() => void>()
	addon.runtimeReady = (handle, done) => original(handle, () => held.resolve(done))
	try {
		const fiber = Effect.runFork(Effect.scoped(Layer.build(Bumble.layer(configuration))))
		const late = await held.promise
		await Effect.runPromise(Fiber.interrupt(fiber))
		assert.equal(Exit.hasInterrupts(await Effect.runPromise(Fiber.await(fiber))), true)
		late()
	} finally {
		addon.runtimeReady = original
	}
	const successor = addon.runtimeOpen(wire)
	assert.equal((await close(successor))._tag, "Closed", "the interrupted runtime was released")
})

test("interruption cancels and joins a native operation, and its late completion is ignored", async () => {
	const handle = addon.runtimeOpen(wire)
	try {
		const held = Promise.withResolvers<() => void>()
		const effect = call("held", (done) => addon.runtimeReady(handle, () => held.resolve(done)), addon.runtimeTake)
		const fiber = Effect.runFork(effect)
		const late = await held.promise
		await Effect.runPromise(Fiber.interrupt(fiber))
		assert.equal(Exit.hasInterrupts(await Effect.runPromise(Fiber.await(fiber))), true)
		late()
		const inspection = addon.runtimeInspect(handle)
		assert.equal(inspection.retained, 0n)
		assert.equal(inspection.active, 0n)
	} finally {
		assert.equal((await close(handle))._tag, "Closed")
	}
})

test("closing the layer's scope releases the runtime, so a later layer can start", async () => {
	for (let count = 0; count < 3; count++) {
		const runtime = ManagedRuntime.make(Bumble.layer(configuration))
		const service = await runtime.runPromise(Bumble)
		await runtime.runPromise(ready)
		await Effect.runPromise(runtime.disposeEffect)
		assert.ok(Exit.isFailure(await Effect.runPromiseExit(service.inspect())))
	}
})

const outstanding = {
	phase: "Closing" as const,
	queued: 0n,
	active: 1n,
	retained: 1n,
	owners: 0n,
	databases: 0n,
	natives: 1n
}

test("an incomplete release is a CloseFailure defect beside the scope's result", async () => {
	let observed: string | undefined
	const exit = await Effect.runPromiseExit(
		Effect.scoped(
			Effect.gen(function* () {
				yield* Effect.acquireRelease(Effect.void, () =>
					release("test.close", (done) => done({ _tag: "Incomplete", outstanding }))
				)
				observed = "ran"
				return observed
			})
		)
	)
	assert.equal(observed, "ran")
	assert.ok(Exit.isFailure(exit))
	assert.ok(exit.cause.reasons.some((reason) => Cause.isDieReason(reason) && reason.defect instanceof CloseFailure))
})

test("an interrupted operation whose cancel drain is incomplete keeps the interrupt and adds CloseFailure", async () => {
	const started = Promise.withResolvers<void>()
	const effect = Effect.callback<never>(() => {
		started.resolve()
		return release("test.cancel", (done) => done({ _tag: "Incomplete", outstanding }))
	})
	const fiber = Effect.runFork(effect)
	await started.promise
	await Effect.runPromise(Fiber.interrupt(fiber))
	const exit = await Effect.runPromise(Fiber.await(fiber))
	assert.equal(Exit.hasInterrupts(exit), true)
	assert.ok(Exit.isFailure(exit))
	assert.ok(exit.cause.reasons.some((reason) => Cause.isDieReason(reason) && reason.defect instanceof CloseFailure))
})
