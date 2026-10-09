/**
 * Hostile inputs across the actual addon boundary: forged and kind-confused handles, one-shot
 * takes, retained wrappers after close, and close under load.
 */
import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { test } from "node:test"
import { compiledOf } from "../src/compile.ts"
import { u64 } from "../src/fields.ts"
import type { OperationRef, RuntimeRef, SnapshotRef } from "../src/native/addon.ts"
import { addon } from "../src/native/addon.ts"
import type { CloseOut } from "../src/native/binding.d.ts"
import { relation } from "../src/relation.ts"
import { schema } from "../src/schema.ts"
import { key } from "../src/statements.ts"

const wire = JSON.stringify({
	workers: 2,
	queueCapacity: 16,
	cleanupCapacity: 16,
	ownerCapacity: 16,
	nativeHandleCapacity: 64,
	cleanupTimeoutMs: 2000
})
const Row = relation("Row", { id: u64 })
const Boundary = schema("Boundary", { Row }, [key(Row, ["id"])])
const compiled = compiledOf(Boundary).handle

function tempDir(tag: string): string {
	return fs.mkdtempSync(path.join(os.tmpdir(), `bumbledb-boundary-${tag}-`))
}

const typedRefusal = (error: unknown): boolean =>
	typeof error === "object" && error !== null && "_tag" in error && typeof error._tag === "string"

const close = (handle: RuntimeRef) => new Promise<CloseOut>((resolve) => addon.runtimeClose(handle, resolve))

function started<Value>(start: (callback: () => void) => Value): {
	readonly lease: Value
	readonly done: Promise<void>
} {
	const pending = Promise.withResolvers<void>()
	const lease = start(() => pending.resolve())
	return { lease, done: pending.promise }
}

async function openDb(runtime: RuntimeRef, dir: string, create: boolean) {
	const acquire = started((callback) => addon.runtimeDirectoryAcquire(runtime, dir, callback))
	await acquire.done
	const owner = addon.runtimeDirectoryTake(acquire.lease)
	const open = started((callback) => addon.runtimeDirectoryDbOpen(owner, "store", compiled, create, callback))
	await open.done
	const opened = addon.runtimeDbTake(open.lease)
	assert.equal(opened._tag, "Opened")
	if (opened._tag !== "Opened") throw new Error("unreachable")
	return { owner, db: opened.db }
}

test("forged and kind-confused handles are refused at conversion", async () => {
	const runtime = addon.runtimeOpen(wire)
	try {
		assert.throws(() => addon.runtimeInspect({} as unknown as RuntimeRef))
		assert.throws(() => addon.runtimeReady({} as unknown as RuntimeRef, () => {}))
		assert.throws(() => addon.runtimeTake({} as unknown as OperationRef))
		assert.throws(() => addon.runtimeSnapshotGet({} as unknown as SnapshotRef, 0, 0, [], () => {}))
		const dir = tempDir("kind")
		const acquire = started((callback) => addon.runtimeDirectoryAcquire(runtime, dir, callback))
		await acquire.done
		const owner = addon.runtimeDirectoryTake(acquire.lease)
		assert.throws(() => addon.runtimeTake(owner as unknown as OperationRef), "a directory owner is not an operation")
		assert.throws(
			() => addon.runtimeSnapshotGet(owner as unknown as SnapshotRef, 0, 0, [], () => {}),
			"a directory owner is not a snapshot"
		)
		await new Promise((resolve) => addon.runtimeDirectoryClose(owner, false, resolve))
		assert.equal(addon.runtimeInspect(runtime).phase, "Open")
		fs.rmSync(dir, { recursive: true, force: true })
	} finally {
		await close(runtime)
	}
})

test("takes are one-shot: a completed operation yields its value exactly once", async () => {
	const runtime = addon.runtimeOpen(wire)
	try {
		const encode = started((callback) => addon.runtimeEncodeRows(runtime, compiled, 0, 1n, [7n], callback))
		await encode.done
		const first = addon.runtimeBytesTake(encode.lease)
		assert.ok(first.length > 0)
		assert.throws(() => addon.runtimeBytesTake(encode.lease), typedRefusal)
		assert.equal(addon.runtimeInspect(runtime).retained, 0n)
	} finally {
		await close(runtime)
	}
})

test("retained wrappers cannot reach native resources after close", async () => {
	const runtime = addon.runtimeOpen(wire)
	const dir = tempDir("retained")
	const { owner, db } = await openDb(runtime, dir, true)
	await new Promise((resolve) => addon.runtimeManagedDbClose(db, resolve))
	await new Promise((resolve) => addon.runtimeDirectoryClose(owner, false, resolve))
	assert.deepEqual(await close(runtime), { _tag: "Closed" })

	assert.throws(() => addon.runtimeDbSnapshot(db, () => {}), typedRefusal)
	assert.throws(() => addon.runtimeDirectoryBegin(owner), typedRefusal)
	assert.equal(addon.runtimeInspect(runtime).phase, "Closed")

	const successor = addon.runtimeOpen(wire)
	try {
		const reopened = await openDb(successor, dir, false)
		await new Promise((resolve) => addon.runtimeManagedDbClose(reopened.db, resolve))
		await new Promise((resolve) => addon.runtimeDirectoryClose(reopened.owner, false, resolve))
	} finally {
		await close(successor)
	}
	fs.rmSync(dir, { recursive: true, force: true })
})

test("close under load reports what it drained and refuses new admission", async () => {
	const runtime = addon.runtimeOpen(wire)
	const leases = Array.from({ length: 6 }, () => started((callback) => addon.runtimeReady(runtime, callback)))
	const report = await close(runtime)
	assert.ok(report._tag === "Closed" || report._tag === "Incomplete")
	assert.throws(() => addon.runtimeReady(runtime, () => {}), typedRefusal)
	await Promise.allSettled(leases.map((entry) => entry.done))
})

test("bytes taken before close are owned and untouched by teardown", async () => {
	const runtime = addon.runtimeOpen(wire)
	const encode = started((callback) => addon.runtimeEncodeRows(runtime, compiled, 0, 1n, [9n], callback))
	await encode.done
	const bytes = addon.runtimeBytesTake(encode.lease)
	const copy = Uint8Array.from(bytes)
	await close(runtime)
	assert.deepEqual(Uint8Array.from(bytes), copy)
})

test("a foreign relation id is refused typed; an absent key is null", async () => {
	const runtime = addon.runtimeOpen(wire)
	const dir = tempDir("snapshot")
	try {
		const { owner, db } = await openDb(runtime, dir, true)
		const snapshot = started((callback) => addon.runtimeDbSnapshot(db, callback))
		await snapshot.done
		const opened = addon.runtimeSnapshotTake(snapshot.lease)

		let foreignRefused = false
		try {
			const get = started((callback) => addon.runtimeSnapshotGet(opened.snapshot, 4096, 0, [], callback))
			await get.done
			addon.runtimeRowTake(get.lease)
		} catch (error) {
			foreignRefused = typedRefusal(error)
		}
		assert.ok(foreignRefused)

		const live = started((callback) => addon.runtimeSnapshotGet(opened.snapshot, 0, 0, [0n], callback))
		await live.done
		assert.equal(addon.runtimeRowTake(live.lease), null)

		await new Promise((resolve) => addon.runtimeSnapshotClose(opened.snapshot, resolve))
		await new Promise((resolve) => addon.runtimeManagedDbClose(db, resolve))
		await new Promise((resolve) => addon.runtimeDirectoryClose(owner, false, resolve))
	} finally {
		await close(runtime)
		fs.rmSync(dir, { recursive: true, force: true })
	}
})
