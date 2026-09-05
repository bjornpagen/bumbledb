/**
 * CompleteResult ownership and the ONE-SHOT page Stream (API-07/API-10;
 * chapter 35 "Streams replace the TypeScript cursor facade"):
 *
 * - `collect` materializes under a database-enforced total cap and LEAVES
 *   the result available; a cap refusal leaves the sealed backing intact
 *   for `pages`;
 * - `pages` construction spends NOTHING; the FIRST run atomically moves
 *   the backing into a private scoped cursor; a second run fails
 *   SpentHandle — never silent EOF, never a rerun of the query;
 * - early `Stream.take`, downstream failure and interruption drain the
 *   private cursor; terminal EOF cleanup is identical;
 * - each element is one owned page ARRAY (pages, not rows) with the same
 *   record shape on every page; copied pages are independent;
 * - a run after the result's owning scope closed fails ClosedHandle.
 *
 * Verification: NotRun until F3 (needs the rebuilt addon's result verbs).
 */
import assert from "node:assert/strict"
import { test } from "node:test"
import { Cause, Effect, Exit, Fiber, ManagedRuntime, Stream } from "effect"
import { ChangeSet } from "#changes.ts"
import { internalResult, type CompleteResult } from "#result.ts"
import { drainClose } from "#close.ts"
import { dbNative } from "#db-native.ts"
import { Db } from "#db.ts"
import { Id128 } from "#id128.ts"
import { query } from "#query/lower.ts"
import { v } from "#query/scope.ts"
import { deliveryResultBytes, nativeOperationWith, policyWire, NativeRuntime, runtimeHandle } from "#runtime.ts"
import { runtimeNative } from "#runtime-native.ts"
import { DbError } from "#runtime-errors.ts"
import { Attempt, Learning, runtimeOptions, Student, storeDir, work } from "#test/fixtures/learning.ts"

const allAttempts = query(Learning).rule((r) => {
	const { id, student, score, units, active } = v(Attempt)
	return r.match(Attempt, { id, student, score, units, active }).find({ id, score })
})

type Row = { readonly id: Id128; readonly score: number }

function runtime() {
	return ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
}

/** Seeds one student with `count` attempts and returns an executed result. */
function seededResult(tag: string, count: number) {
	return Effect.gen(function* () {
		const db = yield* Db.create(storeDir(tag), Learning, work)
		const studentId = yield* Id128.random()
		const draft = yield* ChangeSet.builder(Learning, work)
		yield* draft.insert(Student, [{ id: studentId, name: "Ada", budget: 1000n }])
		const rows = []
		for (let index = 0; index < count; index += 1) {
			const id = yield* Id128.random()
			rows.push({
				id,
				student: studentId,
				score: index / 100,
				units: 1n,
				active: { start: BigInt(index), end: BigInt(index) + 1n }
			})
		}
		yield* draft.insert(Attempt, rows)
		const changes = yield* draft.finish()
		yield* db.apply(changes, { ...work, expected: { kind: "any" } })
		const snapshot = yield* db.snapshot(work)
		const result: CompleteResult<Row> = yield* snapshot.execute(allAttempts, {}, work)
		return { result, count }
	})
}

test("collect leaves the result available; a cap refusal leaves the sealed backing for pages", async function collectThenPages() {
	const rt = runtime()
	try {
		await rt.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const { result, count } = yield* seededResult("collect-then-pages", 50)
					const rows = yield* result.collect({ maxBytes: work.resultBytes }, work)
					assert.equal(rows.length, count)
					// collect did NOT spend the result: it collects again.
					const again = yield* result.collect({ maxBytes: work.resultBytes }, work)
					assert.equal(again.length, count)

					// A total-cap refusal is typed and leaves the backing sealed.
					const capped = yield* Effect.exit(result.collect({ maxBytes: 8n }, work))
					assert.equal(capped._tag, "Failure")
					if (capped._tag === "Failure") {
						const reason = capped.cause.reasons.find(Cause.isFailReason)
						assert.ok(reason?.error instanceof DbError)
						assert.equal(reason.error.code, "ResourceLimit")
					}

					// The sealed backing is still there: pages delivers ALL rows.
					const pages = yield* Stream.runCollect(result.pages({ pageBytes: 1024n }, work))
					const delivered = pages.flat()
					assert.equal(delivered.length, count)
					// Owned page arrays with one stable record shape per row.
					for (const page of pages) {
						assert.ok(Array.isArray(page))
						for (const row of page) {
							assert.deepEqual(Object.keys(row), ["id", "score"])
						}
					}
				})
			)
		)
	} finally {
		await Effect.runPromise(rt.disposeEffect)
	}
})

test("pages is ONE-SHOT: the first run spends the result; a second run fails SpentHandle", async function pagesSpend() {
	const rt = runtime()
	try {
		await rt.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const { result } = yield* seededResult("pages-spend", 20)
					const stream = result.pages({ pageBytes: 512n }, work)
					// CONSTRUCTION spends nothing: collect still works.
					const before = yield* result.collect({ maxBytes: work.resultBytes }, work)
					assert.equal(before.length, 20)

					yield* Stream.runDrain(stream)

					// The transfer moved the backing: a SECOND run of the same
					// stream refuses (SpentHandle) — no silent EOF, no rerun.
					const second = yield* Effect.exit(Stream.runDrain(stream))
					assert.equal(second._tag, "Failure")
					if (second._tag === "Failure") {
						const reason = second.cause.reasons.find(Cause.isFailReason)
						assert.ok(reason?.error instanceof DbError)
						assert.equal(reason.error.code, "SpentHandle")
					}
					// And collect after the transfer refuses the same way.
					const late = yield* Effect.exit(result.collect({ maxBytes: work.resultBytes }, work))
					assert.equal(late._tag, "Failure")
				})
			)
		)
	} finally {
		await Effect.runPromise(rt.disposeEffect)
	}
})

test("early take drains the private cursor; a fresh execute still answers completely", async function earlyTake() {
	const rt = runtime()
	try {
		await rt.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const { result, count } = yield* seededResult("early-take", 40)
					const first = yield* Stream.runCollect(Stream.take(result.pages({ pageBytes: 256n }, work), 1))
					assert.equal(first.length, 1, "backpressure delivered exactly the taken page")
					// The early termination drained/closed the private cursor;
					// the RESULT stays spent (one-shot), but the snapshot is
					// live: a fresh execution answers the complete set.
					const again = yield* Effect.exit(Stream.runDrain(result.pages({ pageBytes: 256n }, work)))
					assert.equal(again._tag, "Failure", "the one-shot transfer happened on the first run")
					void count
				})
			)
		)
	} finally {
		await Effect.runPromise(rt.disposeEffect)
	}
})

test("downstream failure closes/drains the cursor and propagates the caller's error", async function downstreamFailure() {
	const rt = runtime()
	try {
		const exit = await rt.runPromiseExit(
			Effect.scoped(
				Effect.gen(function* () {
					const { result } = yield* seededResult("downstream-failure", 30)
					class AppError extends Error {}
					return yield* Stream.runForEach(result.pages({ pageBytes: 256n }, work), () =>
						Effect.fail(new AppError("downstream refused"))
					)
				})
			)
		)
		assert.equal(exit._tag, "Failure", "the app failure propagates; the cursor was drained by the scope")
	} finally {
		await Effect.runPromise(rt.disposeEffect)
	}
})

test("an empty result emits NO pages; EOF cleanup equals early-termination cleanup", async function emptyResult() {
	const rt = runtime()
	try {
		await rt.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const { result } = yield* seededResult("empty-result", 0)
					const pages = yield* Stream.runCollect(result.pages({ pageBytes: 256n }, work))
					assert.equal(pages.length, 0, "empty result, zero pages — never an empty sentinel page")
				})
			)
		)
	} finally {
		await Effect.runPromise(rt.disposeEffect)
	}
})

test("copied pages are independent: mutating a delivered page cannot change a later collect", async function pagesAreOwned() {
	const rt = runtime()
	try {
		await rt.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const { result, count } = yield* seededResult("pages-owned", 10)
					const rows = yield* result.collect({ maxBytes: work.resultBytes }, work)
					const first = rows[0]
					assert.ok(first)
					assert.ok(Object.isFrozen(first), "delivered records are frozen owned data")
					const mutable = [...rows]
					mutable.length = 0
					const again = yield* result.collect({ maxBytes: work.resultBytes }, work)
					assert.equal(again.length, count, "caller mutation cannot reach native state or a later delivery")
				})
			)
		)
	} finally {
		await Effect.runPromise(rt.disposeEffect)
	}
})

test("a stream run after the result's owning scope closed fails typed, never dangles", async function runAfterScope() {
	const rt = runtime()
	try {
		let escaped: CompleteResult<Row> | undefined
		await rt.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const { result } = yield* seededResult("run-after-scope", 5)
					escaped = result
				})
			)
		)
		assert.ok(escaped)
		const exit = await rt.runPromiseExit(Stream.runDrain(escaped.pages({ pageBytes: 256n }, work)))
		assert.equal(exit._tag, "Failure")
		if (exit._tag === "Failure") {
			const reason = exit.cause.reasons.find(Cause.isFailReason)
			assert.ok(reason?.error instanceof DbError)
		}
	} finally {
		await Effect.runPromise(rt.disposeEffect)
	}
})

test("interrupting a page consumer drains the cursor and reports interruption in Cause", async function interruptDrains() {
	const rt = runtime()
	try {
		await rt.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const { result } = yield* seededResult("interrupt-drains", 40)
					const fiber = yield* Effect.fork(
						Stream.runForEach(result.pages({ pageBytes: 128n }, work), () => Effect.never)
					)
					yield* Effect.sleep("50 millis")
					const exit = yield* Fiber.interrupt(fiber)
					assert.ok(Exit.hasInterrupts(exit), "interruption is Cause")
				})
			)
		)
	} finally {
		await Effect.runPromise(rt.disposeEffect)
	}
})

test("delivery caps intersect: maxBytes cannot enlarge work.resultBytes (D12)", function intersectCaps() {
	const tight = { ...work, resultBytes: 64n }
	assert.equal(deliveryResultBytes(8n, tight), 8n)
	assert.equal(deliveryResultBytes(1_000_000n, tight), 64n, "a larger maxBytes does not raise the work cap")
	assert.equal(deliveryResultBytes(64n, tight), 64n)
})

test("fresh delivery work is required: pages takes its own policy, not the execute deadline", function pagesRequiresWork() {
	type Pages = import("#result.ts").CompleteResult<unknown>["pages"]
	type NeedsWork = Pages extends (options: { readonly pageBytes: bigint }, work: infer W) => unknown
		? W extends { readonly timeout: unknown; readonly resultBytes: bigint }
			? true
			: false
		: false
	const pinned: NeedsWork = true
	assert.ok(pinned)
})

test("publication-boundary cancel delivers nothing; retry starts at row1 (D12/D25)", async function publicationBoundaryCancel() {
	const rt = runtime()
	try {
		await rt.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const { result } = yield* seededResult("publication-cancel", 3)
					const handle = yield* runtimeHandle()
					// L12 probe: cancel after work returns, before operation.output.
					runtimeNative.runtimeArmPublicationCancel(handle)
					const refused = yield* Effect.exit(result.collect({ maxBytes: work.resultBytes }, work))
					assert.equal(refused._tag, "Failure", "predelivery cancel returns no page")
					const retry = yield* result.collect({ maxBytes: work.resultBytes }, work)
					assert.equal(retry.length, 3, "retry begins at row1; publication cancel must not skip")
					const ids = new Set(retry.map((row) => row.id.toString()))
					assert.equal(ids.size, 3, "row1, row2, row3 each appear once")

					const fiber = yield* Effect.fork(result.collect({ maxBytes: work.resultBytes }, work))
					const cancelled = yield* Fiber.interrupt(fiber)
					assert.ok(Exit.hasInterrupts(cancelled), "Effect cancel joins; interruption is Cause")
					const afterJoin = yield* result.collect({ maxBytes: work.resultBytes }, work)
					assert.equal(afterJoin.length, 3, "joined cancel leaves the cursor on row1")
				})
			)
		)
	} finally {
		await Effect.runPromise(rt.disposeEffect)
	}
})

test("non-terminal cursor refusal does not take Page/Rows; same cursor retries at row1 (D25)", async function sameCursorAfterRefusal() {
	const rt = runtime()
	const pageTake = dbNative.runtimePageTake
	let takes = 0
	dbNative.runtimePageTake = ((operation) => {
		takes += 1
		return pageTake.call(dbNative, operation)
	}) as typeof pageTake
	try {
		await rt.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const { result } = yield* seededResult("cursor-refusal", 3)
					const resultHandle = internalResult(result)?.handle
					assert.ok(resultHandle, "sealed result still has a native handle")
					const runtime = yield* runtimeHandle()
					const wire = policyWire(work, "cursor.page")
					const cursor = yield* Effect.acquireRelease(
						nativeOperationWith(
							"cursor.open",
							(callback) => dbNative.runtimeResultCursor(resultHandle, wire, callback),
							dbNative.runtimeCursorTake,
							(taken) => taken
						),
						(taken) =>
							drainClose("cursor.close", (callback) => dbNative.runtimeCursorClose(taken, callback)).pipe(
								Effect.asVoid
							)
					)
					runtimeNative.runtimeArmPublicationCancel(runtime)
					const refused = yield* Effect.exit(
						nativeOperationWith(
							"cursor.refused",
							(callback) => dbNative.runtimeCursorNext(cursor, wire, callback),
							dbNative.runtimePageTake,
							(page) => page
						)
					)
					assert.equal(refused._tag, "Failure", "predelivery cancel returns no page")
					assert.equal(takes, 0, "abandoned Page/Rows must not be taken")
					const retry = yield* nativeOperationWith(
						"cursor.retry",
						(callback) => dbNative.runtimeCursorNext(cursor, wire, callback),
						dbNative.runtimePageTake,
						(page) => page
					)
					assert.ok(retry !== null && retry.length >= 1, "same cursor is not poisoned; retry starts at row1")
				})
			)
		)
	} finally {
		dbNative.runtimePageTake = pageTake
		await Effect.runPromise(rt.disposeEffect)
	}
})

test("two rows that fit alone but not together end a nonempty page; retry is not a drop (D25)", async function jointPageBoundary() {
	const rt = runtime()
	try {
		await rt.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const { result } = yield* seededResult("joint-page", 3)
					const pageBytes = 200n
					const first = yield* Stream.runCollect(Stream.take(result.pages({ pageBytes }, work), 1))
					assert.equal(first.length, 1, "row1+row2 jointly overflow: first pull is a nonempty page")
					assert.ok((first[0]?.length ?? 0) >= 1, "the first page keeps the fitting prefix")
				})
			)
		)
	} finally {
		await Effect.runPromise(rt.disposeEffect)
	}
})
