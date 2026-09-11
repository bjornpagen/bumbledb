/**
 * Hostile capability inputs across the actual Rust/Node boundary.
 * Forged and kind-confused tokens, one-shot take, retained wrappers after
 * close, close/drain under load. Deleted writer/parked-session verbs are
 * not part of this roster — snapshot/session attacks use the db-native
 * capability path.
 */
import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { test } from "node:test"
import type { SnapshotHandle } from "#db-native.ts"
import { dbNative } from "#db-native.ts"
import { u64 } from "#fields.ts"
import { lower } from "#lower.ts"
import { relation } from "#relation.ts"
import type { DirectoryHandle, OperationHandle, OptionsWire, RuntimeHandle } from "#runtime-native.ts"
import { runtimeNative } from "#runtime-native.ts"
import { schema } from "#schema.ts"
import { key } from "#statements.ts"

const wire: OptionsWire = {
	workers: 2,
	queueCapacity: 16,
	cleanupCapacity: 16,
	ownerCapacity: 16,
	nativeHandleCapacity: 64,
	cleanupTimeoutMs: 2000
}
const Row = relation("Row", { id: u64 })
const Boundary = schema("Boundary", { Row }, [key(Row, ["id"])])
const spec = lower(Boundary)

function tempDir(tag: string): string {
	return fs.mkdtempSync(path.join(os.tmpdir(), `bdb-p12-boundary-${tag}-`))
}

const typedRefusal = (error: unknown): boolean =>
	typeof error === "object" &&
	error !== null &&
	"_tag" in error &&
	typeof (error as { _tag: unknown })._tag === "string"

const close = (handle: RuntimeHandle) => new Promise((resolve) => runtimeNative.runtimeClose(handle, resolve))

function started<Value>(start: (callback: () => void) => Value): {
	readonly lease: Value
	readonly done: Promise<void>
} {
	const pending = Promise.withResolvers<void>()
	const lease = start(() => pending.resolve())
	return { lease, done: pending.promise }
}

test("forged externals fail conversion and kind-confused capabilities refuse typed", async () => {
	const runtime = runtimeNative.runtimeOpen(wire)
	try {
		const forgedRuntime = { __runtime: Symbol("forged") } as unknown as RuntimeHandle
		const conversionRefusal = { message: "Failed to get external value" }
		assert.throws(() => runtimeNative.runtimeInspect(forgedRuntime), conversionRefusal)
		assert.throws(() => runtimeNative.runtimeHash(forgedRuntime, new Uint8Array(1), () => {}), conversionRefusal)
		const forgedOperation = {} as unknown as OperationHandle
		assert.throws(() => runtimeNative.runtimeTake(forgedOperation), conversionRefusal)
		const forgedSnapshot = {} as unknown as SnapshotHandle
		assert.throws(() => dbNative.runtimeSnapshotGet(forgedSnapshot, 0, 0, [], () => {}), conversionRefusal)
		const dir = tempDir("kind")
		const acquire = started((callback) => runtimeNative.runtimeDirectoryAcquire(runtime, dir, callback))
		await acquire.done
		const owner: DirectoryHandle = runtimeNative.runtimeDirectoryTake(acquire.lease)
		assert.throws(
			() => runtimeNative.runtimeTake(owner as unknown as OperationHandle),
			{ message: /OperationHandle.*not the type of wrapped object/ },
			"a directory owner is not an operation lease"
		)
		assert.throws(
			() => dbNative.runtimeSnapshotGet(owner as unknown as SnapshotHandle, 0, 0, [], () => {}),
			{ message: /SnapshotHandle.*not the type of wrapped object/ },
			"a directory owner is not a snapshot"
		)
		await new Promise((resolve) => runtimeNative.runtimeDirectoryClose(owner, false, resolve))
		assert.equal(runtimeNative.runtimeInspect(runtime).phase, "open")
	} finally {
		await close(runtime)
	}
})

test("takes are one-shot: a completed operation yields its value exactly once", async () => {
	const runtime = runtimeNative.runtimeOpen(wire)
	try {
		const input = new Uint8Array([1, 2, 3, 4])
		const hash = started((callback) => runtimeNative.runtimeHash(runtime, input, callback))
		await hash.done
		const first = runtimeNative.runtimeTake(hash.lease)
		assert.ok(first instanceof Uint8Array && first.length > 0, "the completed take yields the digest")
		let second: Uint8Array | null | "refused" = "refused"
		try {
			second = runtimeNative.runtimeTake(hash.lease)
		} catch (error) {
			assert.ok(typedRefusal(error))
		}
		assert.ok(second === "refused" || second === null, "a one-shot take never double-delivers")
		assert.equal(runtimeNative.runtimeInspect(runtime).retained, 0n)
	} finally {
		await close(runtime)
	}
})

test("retained wrappers cannot reach native resources after close", async () => {
	const runtime = runtimeNative.runtimeOpen(wire)
	const dir = tempDir("retained")
	const acquire = started((callback) => runtimeNative.runtimeDirectoryAcquire(runtime, dir, callback))
	await acquire.done
	const owner = runtimeNative.runtimeDirectoryTake(acquire.lease)
	const open = started((callback) => runtimeNative.runtimeDirectoryDbOpen(owner, "store", spec, true, callback))
	await open.done
	const outcome = runtimeNative.runtimeDbTake(open.lease)
	assert.equal(outcome.tag, "accepted")
	if (outcome.tag !== "accepted") return
	const db = outcome.db

	await new Promise((resolve) => runtimeNative.runtimeManagedDbClose(db, resolve))
	await new Promise((resolve) => runtimeNative.runtimeDirectoryClose(owner, false, resolve))
	const report = await close(runtime)
	assert.deepEqual(report, { kind: "closed" }, "the drained close reports real reclamation")

	assert.throws(() => dbNative.runtimeDbSnapshot(db, () => {}), typedRefusal)
	assert.throws(() => runtimeNative.runtimeDirectoryBegin(owner), typedRefusal)
	assert.equal(runtimeNative.runtimeInspect(runtime).phase, "closed")

	const successor = runtimeNative.runtimeOpen(wire)
	try {
		const reacquire = started((callback) => runtimeNative.runtimeDirectoryAcquire(successor, dir, callback))
		await reacquire.done
		const newOwner = runtimeNative.runtimeDirectoryTake(reacquire.lease)
		const reopen = started((callback) => runtimeNative.runtimeDirectoryDbOpen(newOwner, "store", spec, false, callback))
		await reopen.done
		const reopened = runtimeNative.runtimeDbTake(reopen.lease)
		assert.equal(reopened.tag, "accepted", "the released store reopens under a successor")
		if (reopened.tag === "accepted") {
			await new Promise((resolve) => runtimeNative.runtimeManagedDbClose(reopened.db, resolve))
		}
		await new Promise((resolve) => runtimeNative.runtimeDirectoryClose(newOwner, false, resolve))
	} finally {
		await close(successor)
	}
	fs.rmSync(dir, { recursive: true, force: true })
})

test("close under load drains in-flight operations and refuses new admission", { timeout: 30_000 }, async () => {
	const runtime = runtimeNative.runtimeOpen(wire)
	const input = new Uint8Array(500_000)
	const leases = []
	for (let index = 0; index < 6; index++) {
		leases.push(started((callback) => runtimeNative.runtimeHash(runtime, input, callback)))
	}
	const report = (await close(runtime)) as { kind: string }
	assert.ok(report.kind === "closed" || report.kind === "incomplete", "close reports reality under load")
	assert.throws(() => runtimeNative.runtimeHash(runtime, input, () => {}), typedRefusal)
	await Promise.allSettled(leases.map((entry) => entry.done))
})

test("owned results taken before close are frozen against native teardown", async () => {
	const runtime = runtimeNative.runtimeOpen(wire)
	const input = new Uint8Array([9, 9, 9])
	const hash = started((callback) => runtimeNative.runtimeHash(runtime, input, callback))
	await hash.done
	const digest = runtimeNative.runtimeTake(hash.lease)
	assert.ok(digest instanceof Uint8Array)
	const copy = Uint8Array.from(digest ?? [])
	await close(runtime)
	assert.deepEqual(Uint8Array.from(digest), copy, "the owned result is untouched by teardown")
})

test("a stale snapshot handle and a foreign relation id miss typed on the live db", async () => {
	const runtime = runtimeNative.runtimeOpen(wire)
	const dir = tempDir("snapshot")
	try {
		const acquire = started((callback) => runtimeNative.runtimeDirectoryAcquire(runtime, dir, callback))
		await acquire.done
		const owner = runtimeNative.runtimeDirectoryTake(acquire.lease)
		const open = started((callback) => runtimeNative.runtimeDirectoryDbOpen(owner, "store", spec, true, callback))
		await open.done
		const outcome = runtimeNative.runtimeDbTake(open.lease)
		assert.equal(outcome.tag, "accepted")
		if (outcome.tag !== "accepted") return
		const snapOp = started((callback) => dbNative.runtimeDbSnapshot(outcome.db, callback))
		await snapOp.done
		const opened = dbNative.runtimeSnapshotTake(snapOp.lease)

		let foreignRefused = false
		try {
			const get = started((callback) => dbNative.runtimeSnapshotGet(opened.snapshot, 4096, 0, [], callback))
			await get.done
			dbNative.runtimeRowTake(get.lease)
		} catch (error) {
			foreignRefused = typedRefusal(error)
		}
		assert.ok(foreignRefused, "a foreign relation id refuses typed")

		const live = started((callback) => dbNative.runtimeSnapshotGet(opened.snapshot, 0, 0, [0n], callback))
		await live.done
		assert.equal(dbNative.runtimeRowTake(live.lease), null)

		await new Promise((resolve) => dbNative.runtimeSnapshotClose(opened.snapshot, resolve))
		await new Promise((resolve) => runtimeNative.runtimeManagedDbClose(outcome.db, resolve))
		await new Promise((resolve) => runtimeNative.runtimeDirectoryClose(owner, false, resolve))
	} finally {
		await close(runtime)
		fs.rmSync(dir, { recursive: true, force: true })
	}
})
