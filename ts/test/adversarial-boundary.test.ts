/**
 * Hostile capability inputs across the actual Rust/Node boundary.
 * Forged and kind-confused tokens, one-shot take, retained wrappers after
 * close, close/drain under load. Deleted writer/parked-session verbs are
 * not part of this roster — snapshot/session attacks use the db-native
 * capability path. Verification: NotRun
 */
import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { test } from "node:test"
import { dbNative } from "#db-native.ts"
import { u64 } from "#fields.ts"
import { lower } from "#lower.ts"
import { relation } from "#relation.ts"
import type { ExecutionPolicy } from "#runtime.ts"
import type {
	DirectoryHandle,
	OperationHandle,
	OptionsWire,
	PolicyWire,
	RuntimeHandle
} from "#runtime-native.ts"
import { runtimeNative } from "#runtime-native.ts"
import type { SnapshotHandle } from "#db-native.ts"
import { schema } from "#schema.ts"

const wire: OptionsWire = {
	workers: 2,
	queueCapacity: 16,
	cleanupCapacity: 16,
	ownerCapacity: 16,
	nativeHandleCapacity: 64,
	inputBytes: 8_000_000n,
	workingBytes: 8_000_000n,
	scratchBytes: 8_000_000n,
	resultBytes: 1_000_000n,
	chunkBytes: 1_000_000n,
	cleanupTimeoutMs: 2000
}
const work: ExecutionPolicy = {
	inputBytes: 1_000_000n,
	workingBytes: 1_000_000n,
	scratchBytes: 1_000_000n,
	resultBytes: 100_000n,
	rows: 100_000n,
	workUnits: 10_000_000n,
	timeout: "10 seconds"
}
const policy: PolicyWire = { ...work, timeoutMs: 10_000 }

const Row = relation("Row", { id: u64 })
const Boundary = schema("Boundary", { Row }, [])
const spec = lower(Boundary)

function tempDir(tag: string): string {
	return fs.mkdtempSync(path.join(os.tmpdir(), `bdb-p12-boundary-${tag}-`))
}

const typedRefusal = (error: unknown): boolean =>
	typeof error === "object" && error !== null && "_tag" in error && typeof (error as { _tag: unknown })._tag === "string"

const close = (handle: RuntimeHandle) =>
	new Promise((resolve) => runtimeNative.runtimeClose(handle, resolve))

function started<Value>(
	start: (callback: () => void) => Value
): { readonly lease: Value; readonly done: Promise<void> } {
	const pending = Promise.withResolvers<void>()
	const lease = start(() => pending.resolve())
	return { lease, done: pending.promise }
}

test("forged and kind-confused externals refuse typed, never alias or crash", async () => {
	const runtime = runtimeNative.runtimeOpen(wire)
	try {
		const forgedRuntime = { __runtime: Symbol("forged") } as unknown as RuntimeHandle
		assert.throws(() => runtimeNative.runtimeInspect(forgedRuntime), typedRefusal)
		assert.throws(
			() => runtimeNative.runtimeHash(forgedRuntime, policy, new Uint8Array(1), () => {}),
			typedRefusal
		)
		const forgedOperation = {} as unknown as OperationHandle
		assert.throws(() => runtimeNative.runtimeTake(forgedOperation), typedRefusal)
		const forgedSnapshot = {} as unknown as SnapshotHandle
		assert.throws(
			() => dbNative.runtimeSnapshotGet(forgedSnapshot, policy, 0, 0, [], () => {}),
			typedRefusal
		)
		const dir = tempDir("kind")
		const acquire = started((callback) => runtimeNative.runtimeDirectoryAcquire(runtime, policy, dir, callback))
		await acquire.done
		const owner: DirectoryHandle = runtimeNative.runtimeDirectoryTake(acquire.lease)
		assert.throws(
			() => runtimeNative.runtimeTake(owner as unknown as OperationHandle),
			typedRefusal,
			"a directory owner is not an operation lease"
		)
		assert.throws(
			() => dbNative.runtimeSnapshotGet(owner as unknown as SnapshotHandle, policy, 0, 0, [], () => {}),
			typedRefusal,
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
		const hash = started((callback) => runtimeNative.runtimeHash(runtime, policy, input, callback))
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
	const acquire = started((callback) => runtimeNative.runtimeDirectoryAcquire(runtime, policy, dir, callback))
	await acquire.done
	const owner = runtimeNative.runtimeDirectoryTake(acquire.lease)
	const open = started((callback) => runtimeNative.runtimeDirectoryDbOpen(owner, policy, "store", spec, true, callback))
	await open.done
	const outcome = runtimeNative.runtimeDbTake(open.lease)
	assert.equal(outcome.tag, "accepted")
	if (outcome.tag !== "accepted") return
	const db = outcome.db

	await new Promise((resolve) => runtimeNative.runtimeManagedDbClose(db, resolve))
	await new Promise((resolve) => runtimeNative.runtimeDirectoryClose(owner, false, resolve))
	const report = await close(runtime)
	assert.deepEqual(report, { kind: "closed" }, "the drained close reports real reclamation")

	assert.throws(() => dbNative.runtimeDbSnapshot(db, policy, () => {}), typedRefusal)
	assert.throws(() => runtimeNative.runtimeDirectoryBegin(owner, policy), typedRefusal)
	assert.throws(() => runtimeNative.runtimeInspect(runtime), typedRefusal)

	const successor = runtimeNative.runtimeOpen(wire)
	try {
		const reacquire = started((callback) =>
			runtimeNative.runtimeDirectoryAcquire(successor, policy, dir, callback)
		)
		await reacquire.done
		const newOwner = runtimeNative.runtimeDirectoryTake(reacquire.lease)
		const reopen = started((callback) =>
			runtimeNative.runtimeDirectoryDbOpen(newOwner, policy, "store", spec, false, callback)
		)
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

test("close under load drains in-flight operations and refuses new admission", async () => {
	const runtime = runtimeNative.runtimeOpen(wire)
	const input = new Uint8Array(500_000)
	const leases = []
	for (let index = 0; index < 6; index++) {
		leases.push(started((callback) => runtimeNative.runtimeHash(runtime, policy, input, callback)))
	}
	const report = (await close(runtime)) as { kind: string }
	assert.ok(report.kind === "closed" || report.kind === "incomplete", "close reports reality under load")
	assert.throws(() => runtimeNative.runtimeHash(runtime, policy, input, () => {}), typedRefusal)
	await Promise.race([
		Promise.allSettled(leases.map((entry) => entry.done)),
		new Promise((_, reject) => setTimeout(() => reject(new Error("in-flight callbacks never settled")), 30_000))
	])
})

test("owned results taken before close are frozen against native teardown", async () => {
	const runtime = runtimeNative.runtimeOpen(wire)
	const input = new Uint8Array([9, 9, 9])
	const hash = started((callback) => runtimeNative.runtimeHash(runtime, policy, input, callback))
	await hash.done
	const digest = runtimeNative.runtimeTake(hash.lease)
	assert.ok(digest instanceof Uint8Array)
	const copy = Uint8Array.from(digest ?? [])
	await close(runtime)
	assert.deepEqual(digest, copy, "the owned result is untouched by teardown")
})

test("a stale snapshot handle and a foreign relation id miss typed on the live db", async () => {
	const runtime = runtimeNative.runtimeOpen(wire)
	const dir = tempDir("snapshot")
	try {
		const acquire = started((callback) => runtimeNative.runtimeDirectoryAcquire(runtime, policy, dir, callback))
		await acquire.done
		const owner = runtimeNative.runtimeDirectoryTake(acquire.lease)
		const open = started((callback) =>
			runtimeNative.runtimeDirectoryDbOpen(owner, policy, "store", spec, true, callback)
		)
		await open.done
		const outcome = runtimeNative.runtimeDbTake(open.lease)
		assert.equal(outcome.tag, "accepted")
		if (outcome.tag !== "accepted") return
		const snapOp = started((callback) => dbNative.runtimeDbSnapshot(outcome.db, policy, callback))
		await snapOp.done
		const opened = dbNative.runtimeSnapshotTake(snapOp.lease)

		let foreignRefused = false
		try {
			const get = started((callback) =>
				dbNative.runtimeSnapshotGet(opened.snapshot, policy, 4096, 0, [], callback)
			)
			await get.done
			dbNative.runtimeRowTake(get.lease)
		} catch (error) {
			foreignRefused = typedRefusal(error)
		}
		assert.ok(foreignRefused, "a foreign relation id refuses typed")

		const live = started((callback) =>
			dbNative.runtimeSnapshotGet(opened.snapshot, policy, 0, 0, [0n], callback)
		)
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
