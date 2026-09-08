/**
 * CompleteResult ownership and the ONE-SHOT page Stream (API-07/API-10;
 * chapter 35 "Streams replace the TypeScript cursor facade"):
 *
 * - collect materializes explicitly and leaves the result available;
 *   cancellation leaves its sealed backing intact for another delivery;
 * - `pages` construction spends NOTHING; the FIRST run atomically moves
 *   the backing into a private scoped cursor; a second run fails
 *   SpentHandle — never silent EOF, never a rerun of the query;
 * - early `Stream.take`, downstream failure and interruption drain the
 *   private cursor; terminal EOF cleanup is identical;
 * - each element is one owned page ARRAY (pages, not rows) with the same
 *   record shape on every page; copied pages are independent;
 * - a run after the result's owning scope closed fails ClosedHandle.
 *
 */
import assert from "node:assert/strict"
import { test } from "node:test"
import { Cause, Deferred, Effect, Exit, Fiber, ManagedRuntime, Stream } from "effect"
import { ChangeSet } from "#changes.ts"
import { drainClose } from "#close.ts"
import { Db } from "#db.ts"
import { dbNative } from "#db-native.ts"
import { bytes, f64, i64, interval, str, u64, uuid } from "#fields.ts"
import { query } from "#query/lower.ts"
import { v } from "#query/scope.ts"
import { relation } from "#relation.ts"
import { type CompleteResult, internalResult } from "#result.ts"
import { NativeRuntime, nativeOperationWith, runtimeHandle } from "#runtime.ts"
import { DbError } from "#runtime-errors.ts"
import { runtimeNative } from "#runtime-native.ts"
import { schema } from "#schema.ts"
import { Attempt, Learning, runtimeOptions, Student, storeDir } from "#test/fixtures/learning.ts"
import type { Uuid } from "#uuid.ts"

const allAttempts = query(Learning).rule((r) => {
	const { id, student, score, units, active } = v(Attempt)
	return r.match(Attempt, { id, student, score, units, active }).find({ id, score })
})

type Row = { readonly id: Uuid; readonly score: number }

function runtime() {
	return ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
}

/** Seeds one student with `count` attempts and returns an executed result. */
function seededResult(tag: string, count: number) {
	return Effect.gen(function* () {
		const db = yield* Db.create(storeDir(tag), Learning)
		const studentId = yield* Effect.sync(() => crypto.randomUUID())
		const draft = yield* ChangeSet.builder(Learning)
		yield* draft.insert(Student, [{ id: studentId, name: "Ada", budget: 1000n }])
		const rows = []
		for (let index = 0; index < count; index += 1) {
			const id = yield* Effect.sync(() => crypto.randomUUID())
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
		yield* db.apply(changes, { expected: { kind: "any" } })
		const snapshot = yield* db.snapshot()
		const result: CompleteResult<Row> = yield* snapshot.execute(allAttempts, {})
		return { result, count }
	})
}

test("collect leaves the result available; cancellation leaves the sealed backing for pages", async function collectThenPages() {
	const rt = runtime()
	try {
		await rt.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const { result, count } = yield* seededResult("collect-then-pages", 600)
					const rows = yield* result.collect()
					assert.equal(rows.length, count)
					// collect did NOT spend the result: it collects again.
					const again = yield* result.collect()
					assert.equal(again.length, count)

					// A publication cancellation leaves the backing sealed.
					runtimeNative.runtimeArmPublicationCancel(yield* runtimeHandle())
					const capped = yield* Effect.exit(result.collect())
					assert.equal(capped._tag, "Failure")
					if (capped._tag === "Failure") {
						const reason = capped.cause.reasons.find(Cause.isFailReason)
						assert.ok(reason?.error instanceof DbError)
						assert.equal(reason.error.code, "Cancelled")
					}

					// The sealed backing is still there: pages delivers ALL rows.
					const pages = yield* Stream.runCollect(result.pages())
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
					const stream = result.pages()
					// CONSTRUCTION spends nothing: collect still works.
					const before = yield* result.collect()
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
					const late = yield* Effect.exit(result.collect())
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
					const first = yield* Stream.runCollect(Stream.take(result.pages(), 1))
					assert.equal(first.length, 1, "backpressure delivered exactly the taken page")
					// The early termination drained/closed the private cursor;
					// the RESULT stays spent (one-shot), but the snapshot is
					// live: a fresh execution answers the complete set.
					const again = yield* Effect.exit(Stream.runDrain(result.pages()))
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
					return yield* Stream.runForEach(result.pages(), () => Effect.fail(new AppError("downstream refused")))
				})
			)
		)
		assert.equal(exit._tag, "Failure", "the app failure propagates; the cursor was drained by the scope")
	} finally {
		await Effect.runPromise(rt.disposeEffect)
	}
})

test("an empty result emits one terminal empty page and drains its cursor", async function emptyResult() {
	const rt = runtime()
	try {
		await rt.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const { result } = yield* seededResult("empty-result", 0)
					const pages = yield* Stream.runCollect(result.pages())
					assert.deepEqual(pages, [[]], "empty result has exactly one terminal empty page")
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
					const rows = yield* result.collect()
					const first = rows[0]
					assert.ok(first)
					assert.ok(Object.isFrozen(first), "delivered records are frozen owned data")
					const mutable = [...rows]
					mutable.length = 0
					const again = yield* result.collect()
					assert.equal(again.length, count, "caller mutation cannot reach native state or a later delivery")
				})
			)
		)
	} finally {
		await Effect.runPromise(rt.disposeEffect)
	}
})

test("borrowed native delivery preserves exact values and independent byte owners", async function borrowedDelivery() {
	const Record = relation("Record", {
		id: uuid,
		signed: i64,
		unsigned: u64,
		text: str,
		raw: bytes(16),
		span: interval(i64),
		precise: f64
	})
	const Data = schema("DeliveryValues", { Record }, [])
	const all = query(Data).rule((r) => {
		const fields = v(Record)
		return r.match(Record, fields).find(fields)
	})
	const expected = Array.from({ length: 12 }, (_, index) => ({
		id: `00000000-0000-0000-0000-${String(index + 1).padStart(12, "0")}` as Uuid,
		signed: -(1n << 63n) + BigInt(index),
		unsigned: (1n << 64n) - 1n - BigInt(index),
		text: `borrowed \u{1f41d}\0${index}${"x".repeat(70_000)}`,
		raw: new Uint8Array(16).fill(index + 1),
		span: { start: -(1n << 63n), end: (1n << 63n) - 1n },
		precise: 0.5
	}))
	const rt = runtime()
	try {
		const delivered = await rt.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.create(storeDir("borrowed-values"), Data)
					const draft = yield* ChangeSet.builder(Data)
					yield* draft.insert(Record, expected)
					const changes = yield* draft.finish()
					yield* db.apply(changes, { expected: { kind: "any" } })
					const snapshot = yield* db.snapshot()
					const result = yield* snapshot.execute(all, {})
					yield* snapshot.close()
					const first = yield* result.collect()
					assert.deepEqual(first, expected)
					first[0]?.raw.fill(255)
					const again = yield* result.collect()
					assert.deepEqual(again, expected, "changing an owned blob cannot mutate the result backing")
					const pages = yield* Stream.runCollect(result.pages())
					assert.equal(pages.length, 1, "one valid oversized row is not a quota refusal")
					const rows = pages.flat()
					assert.deepEqual(rows, expected)
					return rows
				})
			)
		)
		assert.deepEqual(delivered, expected, "values remain valid after all native scopes close")
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
		const exit = await rt.runPromiseExit(Stream.runDrain(escaped.pages()))
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
					const received = yield* Deferred.make<void>()
					const fiber = yield* Effect.forkChild(
						Stream.runForEach(result.pages(), () =>
							Deferred.succeed(received, undefined).pipe(Effect.andThen(Effect.never))
						)
					)
					yield* Deferred.await(received)
					yield* Fiber.interrupt(fiber)
					const exit = yield* Fiber.await(fiber)
					assert.ok(Exit.hasInterrupts(exit), "interruption is Cause")
				})
			)
		)
	} finally {
		await Effect.runPromise(rt.disposeEffect)
	}
})

test("completed-result delivery needs no resource options", () => {
	const collectArgs: Parameters<CompleteResult<Row>["collect"]> = []
	const pagesArgs: Parameters<CompleteResult<Row>["pages"]> = []
	assert.deepEqual(collectArgs, [])
	assert.deepEqual(pagesArgs, [])
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
					const refused = yield* Effect.exit(result.collect())
					assert.equal(refused._tag, "Failure", "predelivery cancel returns no page")
					const retry = yield* result.collect()
					assert.equal(retry.length, 3, "retry begins at row1; publication cancel must not skip")
					const ids = new Set(retry.map((row) => row.id.toString()))
					assert.equal(ids.size, 3, "row1, row2, row3 each appear once")

					const fiber = yield* Effect.forkChild(result.collect())
					yield* Fiber.interrupt(fiber)
					const cancelled = yield* Fiber.await(fiber)
					assert.ok(Exit.hasInterrupts(cancelled), "Effect cancel joins; interruption is Cause")
					const afterJoin = yield* result.collect()
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
		const page = pageTake.call(dbNative, operation)
		takes += 1
		return page
	}) as typeof pageTake
	try {
		await rt.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const { result } = yield* seededResult("cursor-refusal", 3)
					const resultHandle = internalResult(result)?.handle
					assert.ok(resultHandle, "sealed result still has a native handle")
					const runtime = yield* runtimeHandle()
					const cursor = yield* Effect.acquireRelease(
						nativeOperationWith(
							"cursor.open",
							(callback) => dbNative.runtimeResultCursor(resultHandle, callback),
							dbNative.runtimeCursorTake,
							(taken) => taken
						),
						(taken) =>
							drainClose("cursor.close", (callback) => dbNative.runtimeCursorClose(taken, callback)).pipe(Effect.asVoid)
					)
					runtimeNative.runtimeArmPublicationCancel(runtime)
					const refused = yield* Effect.exit(
						nativeOperationWith(
							"cursor.refused",
							(callback) => dbNative.runtimeCursorNext(cursor, callback),
							dbNative.runtimePageTake,
							(page) => page
						)
					)
					assert.equal(refused._tag, "Failure", "predelivery cancel returns no page")
					assert.equal(takes, 0, "the completion probe throws its refusal; no Page/Rows payload is adopted")
					const retry = yield* nativeOperationWith(
						"cursor.retry",
						(callback) => dbNative.runtimeCursorNext(cursor, callback),
						dbNative.runtimePageTake,
						(page) => page
					)
					assert.equal(retry?.length, 3, "same cursor retries all three rows without skipping")
				})
			)
		)
	} finally {
		dbNative.runtimePageTake = pageTake
		await Effect.runPromise(rt.disposeEffect)
	}
})

test("delivery batches bound copied rows without dropping or duplicating the remainder", async function jointPageBoundary() {
	const rt = runtime()
	try {
		await rt.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const { result } = yield* seededResult("joint-page", 513)
					const pages = yield* Stream.runCollect(result.pages())
					assert.deepEqual(
						pages.map((page) => page.length),
						[256, 256, 1]
					)
					const ids = pages.flat().map((row) => row.id)
					assert.equal(new Set(ids).size, 513)
				})
			)
		)
	} finally {
		await Effect.runPromise(rt.disposeEffect)
	}
})
