/**
 * P12 adversarial integration: hostile capability inputs across the ACTUAL
 * Rust/Node boundary (not mocked helpers). Forged and kind-confused
 * externals, one-shot take discipline, retained wrappers after close,
 * close/drain under load and retained owned results — FFI-01/02/05/07,
 * RUN-04/05, SDK-004/SDK-007 boundary halves, G11/G14.
 *
 * Every attack must surface as a TYPED refusal (`_tag`-shaped error) or an
 * honest report — never a crash, an alias into another slot, or a silent
 * success. Verification: NotRun (F2 authors; runs at F3 against the fresh
 * addon built by the package test command).
 */
import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { test } from "node:test"
import { u64 } from "#fields.ts"
import { lower } from "#lower.ts"
import { relation } from "#relation.ts"
import type { ExecutionPolicy } from "#runtime.ts"
import type {
	DirectoryHandle,
	OperationHandle,
	OptionsWire,
	PolicyWire,
	RuntimeHandle,
	SessionHandle
} from "#runtime-native.ts"
import { runtimeNative } from "#runtime-native.ts"
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
		// Plain-object forgeries in every handle position.
		const forgedRuntime = { __runtime: Symbol("forged") } as unknown as RuntimeHandle
		assert.throws(() => runtimeNative.runtimeInspect(forgedRuntime), typedRefusal)
		assert.throws(
			() => runtimeNative.runtimeHash(forgedRuntime, policy, new Uint8Array(1), () => {}),
			typedRefusal
		)
		const forgedOperation = {} as unknown as OperationHandle
		assert.throws(() => runtimeNative.runtimeTake(forgedOperation), typedRefusal)
		const forgedSession = {} as unknown as SessionHandle
		assert.throws(
			() => runtimeNative.runtimeSessionCount(forgedSession, policy, 0, () => {}),
			typedRefusal
		)
		// Kind confusion: a REAL external of the wrong kind in a take slot.
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
			() => runtimeNative.runtimeSessionCount(owner as unknown as SessionHandle, policy, 0, () => {}),
			typedRefusal,
			"a directory owner is not a session"
		)
		await new Promise((resolve) => runtimeNative.runtimeDirectoryClose(owner, false, resolve))
		// The runtime survived every attack.
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
		// The second take must not yield the value again: null or typed.
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

	// Deterministic teardown: db child, then owner, then the runtime.
	await new Promise((resolve) => runtimeNative.runtimeManagedDbClose(db, resolve))
	await new Promise((resolve) => runtimeNative.runtimeDirectoryClose(owner, false, resolve))
	const report = await close(runtime)
	assert.deepEqual(report, { kind: "closed" }, "the drained close reports real reclamation")

	// Every retained wrapper is now a typed refusal, not a live capability.
	assert.throws(() => runtimeNative.runtimeDbSession(db, policy, () => {}), typedRefusal)
	assert.throws(() => runtimeNative.runtimeDirectoryBegin(owner, policy), typedRefusal)
	assert.throws(() => runtimeNative.runtimeInspect(runtime), typedRefusal)

	// Real release: a successor owns the same directory immediately.
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
	// Close while the pool is saturated: the report arrives only when the
	// native work is actually drained or accounted, never a false quiescence.
	const report = (await close(runtime)) as { kind: string }
	assert.ok(
		report.kind === "closed" || report.kind === "incomplete",
		"close reports reality under load"
	)
	// New admission is revoked either way.
	assert.throws(() => runtimeNative.runtimeHash(runtime, policy, input, () => {}), typedRefusal)
	// Every callback settles (drained or cancelled) — no operation leaks a
	// pending JS continuation forever.
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

test("a stale prepared id and a foreign relation id miss typed inside a live session", async () => {
	const runtime = runtimeNative.runtimeOpen(wire)
	const dir = tempDir("session")
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
		const sessionOp = started((callback) => runtimeNative.runtimeDbSession(outcome.db, policy, callback))
		await sessionOp.done
		const opened = runtimeNative.runtimeSessionTake(sessionOp.lease)

		// A never-installed prepared id can only miss — typed, no alias.
		let staleRefused = false
		try {
			const execute = started((callback) =>
				runtimeNative.runtimeSessionExecute(opened.session, policy, 999n, [], callback)
			)
			await execute.done
			runtimeNative.runtimeRowsTake(execute.lease)
		} catch (error) {
			staleRefused = typedRefusal(error)
		}
		assert.ok(staleRefused, "a stale prepared id refuses typed")

		// A relation id outside the schema is a typed refusal, not a read of
		// arbitrary native memory.
		let foreignRefused = false
		try {
			const count = started((callback) => runtimeNative.runtimeSessionCount(opened.session, policy, 4096, callback))
			await count.done
			runtimeNative.runtimeCountTake(count.lease)
		} catch (error) {
			foreignRefused = typedRefusal(error)
		}
		assert.ok(foreignRefused, "a foreign relation id refuses typed")

		// The session survived both attacks and still answers.
		const count = started((callback) => runtimeNative.runtimeSessionCount(opened.session, policy, 0, callback))
		await count.done
		assert.equal(runtimeNative.runtimeCountTake(count.lease), 0n)

		await new Promise((resolve) => runtimeNative.runtimeSessionClose(opened.session, resolve))
		await new Promise((resolve) => runtimeNative.runtimeManagedDbClose(outcome.db, resolve))
		await new Promise((resolve) => runtimeNative.runtimeDirectoryClose(owner, false, resolve))
	} finally {
		await close(runtime)
		fs.rmSync(dir, { recursive: true, force: true })
	}
})
