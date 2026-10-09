/**
 * D18 / TS-003: every partial acquisition is owned before the next
 * interruptible step. Directory acquire is finalized before Db open;
 * operation output is registered before the next yield. JS tokens stay
 * reachable.
 */
import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, Exit, Fiber, ManagedRuntime } from "effect"
import { ChangeSet } from "../src/changes.ts"
import { Db } from "../src/db.ts"
import { addon } from "../src/native/addon.ts"
import { query } from "../src/query/lower.ts"
import { v } from "../src/query/scope.ts"
import { Bumble } from "../src/runtime.ts"
import { Attempt, Learning, runtimeOptions, Student, storeDir } from "./fixtures/learning.ts"

const allAttempts = query(Learning).rule((r) => {
	const { id, student, score, units, active } = v(Attempt)
	return r.match(Attempt, { id, student, score, units, active }).find({ id, score })
})

function runtime() {
	return ManagedRuntime.make(Bumble.layer(runtimeOptions))
}

test("interrupt after directory acquire and before db output adoption drains both owners (D18)", async function directoryThenDb() {
	const rt = runtime()
	try {
		const original = (await import("../src/native/addon.ts")).addon
		const open = original.runtimeDirectoryDbOpen
		const completed = Promise.withResolvers<() => void>()
		original.runtimeDirectoryDbOpen = ((directory, child, spec, create, callback) =>
			open.call(original, directory, child, spec, create, () => completed.resolve(callback))) as typeof open
		try {
			const path = storeDir("dir-then-db")
			const fiber = rt.runFork(Effect.scoped(Db.create(path, Learning)))
			const lateCallback = await completed.promise
			await rt.runPromise(Fiber.interrupt(fiber))
			const exit = await rt.runPromise(Fiber.await(fiber))
			assert.ok(Exit.hasInterrupts(exit), "interruption is Cause")
			lateCallback()
			original.runtimeDirectoryDbOpen = open
			const successor = await rt.runPromiseExit(Effect.scoped(Db.open(path, Learning).pipe(Effect.asVoid)))
			assert.equal(successor._tag, "Success", "the same directory is unlocked and its created database can reopen")
		} finally {
			original.runtimeDirectoryDbOpen = open
		}
	} finally {
		await Effect.runPromise(rt.disposeEffect)
	}
})

test("abort after publication drains without take; retained wrappers cannot pin output (D18)", async function abortAfterPublication() {
	const rt = runtime()
	const kept: object[] = []
	const collect = addon.runtimeResultCollect
	const published = Promise.withResolvers<() => void>()
	addon.runtimeResultCollect = ((handle, callback) =>
		collect.call(addon, handle, () => {
			published.resolve(callback)
		})) as typeof collect
	try {
		await rt.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.create(storeDir("abort-after-pub"), Learning)
					kept.push(db)
					const studentId = yield* Effect.sync(() => crypto.randomUUID())
					const draft = yield* ChangeSet.builder(Learning)
					yield* draft.insert(Student, [{ id: studentId, name: "Ada", budget: 1000n }])
					const attemptId = yield* Effect.sync(() => crypto.randomUUID())
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
					yield* db.apply(changes)
					const snapshot = yield* db.snapshot()
					kept.push(snapshot)
					const result = yield* snapshot.execute(allAttempts, {})
					kept.push(result)
					const before = yield* (yield* Bumble).inspect()
					const fiber = yield* Effect.forkChild(result.collect())
					const lateCallback = yield* Effect.promise(() => published.promise)
					yield* Fiber.interrupt(fiber)
					const exit = yield* Fiber.await(fiber)
					assert.ok(Exit.hasInterrupts(exit), "interruption is Cause")
					lateCallback()
					const after = yield* (yield* Bumble).inspect()
					assert.equal(after.retained, before.retained, "queued output reclaimed without JS take")
				})
			)
		)
		assert.equal(kept.length, 3, "db, snapshot, and result wrappers stayed reachable")
	} finally {
		addon.runtimeResultCollect = collect
		await Effect.runPromise(rt.disposeEffect)
	}
})

test("draft interruption spends the draft and joins drain", async function interruptSpendsDraft() {
	const rt = runtime()
	const insert = addon.runtimeDraftInsert
	const completed = Promise.withResolvers<() => void>()
	addon.runtimeDraftInsert = ((handle, relation, rows, cells, callback) =>
		insert.call(addon, handle, relation, rows, cells, () => completed.resolve(callback))) as typeof insert
	try {
		await rt.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const draft = yield* ChangeSet.builder(Learning)
					const id = yield* Effect.sync(() => crypto.randomUUID())
					let visited = 0
					let closed = 0
					function* source() {
						try {
							for (let index = 0; index < 20_000; index++) {
								visited++
								yield { id, name: "n", budget: 1n }
							}
						} finally {
							closed++
						}
					}
					const fiber = yield* Effect.forkChild(draft.insert(Student, source()))
					const lateCallback = yield* Effect.promise(() => completed.promise)
					assert.ok(visited > 0 && visited <= 4097, "only one batch plus carry was pulled")
					assert.equal(closed, 0, "iterator remains live at the native boundary")
					yield* Fiber.interrupt(fiber)
					assert.equal(closed, 1, "interruption closes the partially consumed iterator")
					const interrupted = yield* Fiber.await(fiber)
					assert.ok(Exit.hasInterrupts(interrupted), "the insert was interrupted after native completion")
					lateCallback()
					const late = yield* Effect.exit(draft.finish())
					assert.equal(late._tag, "Failure", "interrupt spends the draft")
				})
			)
		)
	} finally {
		addon.runtimeDraftInsert = insert
		await Effect.runPromise(rt.disposeEffect)
	}
})
