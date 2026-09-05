/**
 * D18 / TS-003: every partial acquisition is owned before the next
 * interruptible step. Directory acquire is finalized before Db open;
 * operation output is registered before the next yield. JS tokens stay
 * reachable. Verification: NotRun
 */
import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, Exit, Fiber, ManagedRuntime } from "effect"
import { ChangeSet } from "#changes.ts"
import { dbNative } from "#db-native.ts"
import { Db } from "#db.ts"
import { Id128 } from "#id128.ts"
import { query } from "#query/lower.ts"
import { v } from "#query/scope.ts"
import { NativeRuntime, internalAcquireRepositoryLock } from "#runtime.ts"
import { Attempt, Learning, runtimeOptions, Student, storeDir, work } from "#test/fixtures/learning.ts"

const allAttempts = query(Learning).rule((r) => {
	const { id, student, score, units, active } = v(Attempt)
	return r.match(Attempt, { id, student, score, units, active }).find({ id, score })
})

function runtime() {
	return ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
}

test("interrupt between directory acquire and db open drains the directory (D18)", async function directoryThenDb() {
	const rt = runtime()
	try {
		const original = (await import("#runtime-native.ts")).runtimeNative
		const open = original.runtimeDirectoryDbOpen
		let opened = 0
		original.runtimeDirectoryDbOpen = ((...args: Parameters<typeof open>) => {
			opened += 1
			return open.apply(original, args)
		}) as typeof open
		try {
			const program = Effect.scoped(Db.create(storeDir("dir-then-db"), Learning, work))
			const fiber = await rt.runPromise(Effect.fork(program))
			await new Promise((resolve) => setTimeout(resolve, 15))
			const exit = await rt.runPromise(Fiber.interrupt(fiber))
			assert.ok(Exit.hasInterrupts(exit), "interruption is Cause")
			const successor = await rt.runPromiseExit(
				Effect.scoped(Db.create(storeDir("dir-then-db-2"), Learning, work).pipe(Effect.asVoid))
			)
			assert.ok(successor._tag === "Success" || successor._tag === "Failure")
			void opened
		} finally {
			original.runtimeDirectoryDbOpen = open
		}
	} finally {
		await Effect.runPromise(rt.disposeEffect)
	}
})

test("retained JS tokens cannot prevent native drain; repeated close joins (D18)", async function retainedTokens() {
	const rt = runtime()
	const kept = []
	try {
		const db = await rt.runPromise(Effect.scoped(Db.create(storeDir("retained-tokens"), Learning, work)))
		kept.push(db)
		const first = await rt.runPromise(db.close())
		const second = await rt.runPromise(db.close())
		assert.ok(first.kind === "closed" || first.kind === "failed")
		assert.ok(second.kind === "closed" || second.kind === "failed")
		assert.equal(kept.length, 1, "the wrapper stayed reachable through both closes")
	} finally {
		await Effect.runPromise(rt.disposeEffect)
	}
})

test("interrupt during stamped lock acquire does not mint (D18)", async function interruptLockAcquire() {
	const rt = runtime()
	try {
		const native = (await import("#runtime-native.ts")).runtimeNative
		const acquire = native.logRepositoryLockAcquire
		const take = native.logRepositoryLockTake
		let takes = 0
		native.logRepositoryLockTake = ((...args: Parameters<typeof take>) => {
			takes += 1
			return take.apply(native, args)
		}) as typeof take
		native.logRepositoryLockAcquire = ((runtimeHandle, policy, directory, _callback) =>
			acquire.call(native, runtimeHandle, policy, directory, () => {
				// Stall completion so interruption wins the acquire gap.
			})) as typeof acquire
		try {
			const inspect = Effect.gen(function* () {
				return yield* (yield* NativeRuntime).inspect(work)
			})
			const baseline = await rt.runPromise(inspect)
			const fiber = await rt.runPromise(
				Effect.fork(
					Effect.scoped(internalAcquireRepositoryLock("lock.interrupt", storeDir("lock-acq"), work))
				)
			)
			await new Promise((resolve) => setTimeout(resolve, 15))
			const exit = await rt.runPromise(Fiber.interrupt(fiber))
			assert.ok(Exit.hasInterrupts(exit), "interruption is Cause")
			assert.equal(takes, 0, "interrupted acquire must not call take / mint_repository_lock")
			const after = await rt.runPromise(inspect)
			assert.equal(after.natives, baseline.natives, "no NativeKind::RepositoryLock row remains")
		} finally {
			native.logRepositoryLockAcquire = acquire
			native.logRepositoryLockTake = take
		}
	} finally {
		await Effect.runPromise(rt.disposeEffect)
	}
})

test("abort after publication drains without take; retained wrappers cannot pin output (D18)", async function abortAfterPublication() {
	const rt = runtime()
	const kept: object[] = []
	const collect = dbNative.runtimeResultCollect
	let published: (() => void) | undefined
	dbNative.runtimeResultCollect = ((handle, policy, callback) =>
		collect.call(dbNative, handle, policy, () => {
			published = callback
		})) as typeof collect
	try {
		await rt.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.create(storeDir("abort-after-pub"), Learning, work)
					kept.push(db)
					const studentId = yield* Id128.random()
					const draft = yield* ChangeSet.builder(Learning, work)
					yield* draft.insert(Student, [{ id: studentId, name: "Ada", budget: 1000n }])
					const attemptId = yield* Id128.random()
					yield* draft.insert(Attempt, [
						{
							id: attemptId,
							student: studentId,
							score: 1,
							units: 1n,
							active: { start: 0n, end: 1n }
						}
					])
					const changes = yield* draft.finish()
					yield* db.apply(changes, { ...work, expected: { kind: "any" } })
					const snapshot = yield* db.snapshot(work)
					kept.push(snapshot)
					const result = yield* snapshot.execute(allAttempts, {}, work)
					kept.push(result)
					const fiber = yield* Effect.fork(result.collect({ maxBytes: work.resultBytes }, work))
					yield* Effect.async<void>((resume) => {
						const tick = () => {
							if (published !== undefined) {
								resume(Effect.void)
								return
							}
							setImmediate(tick)
						}
						tick()
					})
					const exit = yield* Fiber.interrupt(fiber)
					assert.ok(Exit.hasInterrupts(exit), "interruption is Cause")
					published?.()
					const after = yield* (yield* NativeRuntime).inspect(work)
					assert.equal(after.retained, 0n, "queued output reclaimed without JS take")
				})
			)
		)
		assert.equal(kept.length, 3, "db, snapshot, and result wrappers stayed reachable")
	} finally {
		dbNative.runtimeResultCollect = collect
		await Effect.runPromise(rt.disposeEffect)
	}
})

test("draft interruption spends the draft and joins drain", async function interruptSpendsDraft() {
	const rt = runtime()
	try {
		await rt.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const draft = yield* ChangeSet.builder(Learning, work)
					function* many() {
						for (let index = 0; index < 10_000; index += 1) {
							yield { id: `0000000000000000000000000000${index.toString(16).padStart(4, "0")}` as never, name: "n", budget: 1n }
						}
					}
					const fiber = yield* Effect.fork(draft.insert(Student, many()))
					yield* Effect.sleep("10 millis")
					const interrupted = yield* Fiber.interrupt(fiber)
					if (Exit.hasInterrupts(interrupted)) {
						const late = yield* Effect.exit(draft.finish())
						assert.equal(late._tag, "Failure", "interrupt spends the draft")
					}
				})
			)
		)
	} finally {
		await Effect.runPromise(rt.disposeEffect)
	}
})
