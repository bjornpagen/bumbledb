/**
 * ChangeDraft/ChangeSet laziness, reruns and canonical ownership
 * (API-01/API-10; chapter 35 "Laziness, reruns and stable intent"):
 *
 * - construction is INERT: a built insert effect reads its iterable only
 *   at execution, and each sequential rerun reads the then-current input
 *   and charges its work again — no memoization, no automatic retry, no
 *   iterator replay;
 * - one-shot iterators exhaust: the second run of the same effect ingests
 *   nothing (the input was consumed), never a silent replay;
 * - after successful ingestion the accepted native bytes are independent:
 *   later mutation of the caller's array cannot change the accepted facts;
 * - getter/iterator throws become typed input failures that SPEND the
 *   draft and start tracked drain — never an untracked partial draft;
 * - ingestion charges the draft's CUMULATIVE aggregate budget (chunks and
 *   calls never reset it);
 * - finish consumes the draft: later ingestion and a second finish refuse
 *   through the spent capability state.
 */
import assert from "node:assert/strict"
import { test } from "node:test"
import { Cause, Effect, ManagedRuntime, Option } from "effect"
import { ChangeSet } from "#changes.ts"
import { Db } from "#db.ts"
import { bytes, str } from "#fields.ts"
import type { Fact } from "#relation.ts"
import { assertHostCellFits, cellOf, hostCellCharge } from "#rows.ts"
import { NativeRuntime } from "#runtime.ts"
import { DbError } from "#runtime-errors.ts"
import { Attempt, Learning, runtimeOptions, Student, storeDir, work } from "#test/fixtures/learning.ts"
import type { Uuid } from "#uuid.ts"

function runtime() {
	return ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
}

const newId = () => Effect.runPromise(Effect.sync(() => crypto.randomUUID()))

function studentRow(id: Uuid, name: string): Fact<typeof Student> {
	return { id, name, budget: 10n }
}

test("insert effects are lazy and rerunnable: each run reads the THEN-CURRENT array", async function lazyRerun() {
	const rt = runtime()
	try {
		const first = await newId()
		const second = await newId()
		await rt.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const draft = yield* ChangeSet.builder(Learning, work)
					const rows: Array<Fact<typeof Student>> = []
					const insert = draft.insert(Student, rows)
					// CONSTRUCTION read nothing: the array was empty then and
					// filling it before the first run is fully observed.
					rows.push(studentRow(first, "Ada"))
					yield* insert
					// A sequential RERUN of the same effect value reads the
					// mutated array again — ordinary execution, charged again.
					rows[0] = studentRow(second, "Bo")
					yield* insert
					const changes = yield* draft.finish()

					const db = yield* Db.create(storeDir("lazy-rerun"), Learning, work)
					const outcome = yield* db.apply(changes, { ...work, expected: { kind: "any" } })
					assert.equal(outcome.kind, "accepted")
					const snapshot = yield* db.snapshot(work)
					const ada = yield* snapshot.get(Student, { id: first }, work)
					const bo = yield* snapshot.get(Student, { id: second }, work)
					assert.ok(Option.isSome(ada), "the first run's fact is in the final set")
					assert.ok(Option.isSome(bo), "the rerun's fact is in the final set")
				})
			)
		)
	} finally {
		await Effect.runPromise(rt.disposeEffect)
	}
})

test("a one-shot iterator is consumed, never replayed: the second run ingests nothing", async function exhaustedIterator() {
	const rt = runtime()
	try {
		const only = await newId()
		await rt.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const draft = yield* ChangeSet.builder(Learning, work)
					function* once() {
						yield studentRow(only, "Once")
					}
					const iterator = once()
					const insert = draft.insert(Student, iterator)
					yield* insert
					// The generator is exhausted; rerunning the effect reads
					// zero rows — the SDK never rewinds or replays user input.
					yield* insert
					const changes = yield* draft.finish()
					const db = yield* Db.create(storeDir("exhausted-iterator"), Learning, work)
					const outcome = yield* db.apply(changes, { ...work, expected: { kind: "any" } })
					assert.equal(outcome.kind, "accepted")
				})
			)
		)
	} finally {
		await Effect.runPromise(rt.disposeEffect)
	}
})

test("mutation AFTER successful ingestion cannot change the accepted native facts", async function acceptedIsIndependent() {
	const rt = runtime()
	try {
		const kept = await newId()
		const impostor = await newId()
		await rt.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const draft = yield* ChangeSet.builder(Learning, work)
					const row = { id: kept, name: "Kept", budget: 10n }
					const rows = [row]
					yield* draft.insert(Student, rows)
					// The acceptance boundary passed: these mutations are
					// invisible to the draft's owned native bytes.
					rows[0] = studentRow(impostor, "Impostor")
					;(row as { name: string }).name = "Mutated"
					const changes = yield* draft.finish()
					const db = yield* Db.create(storeDir("accepted-independent"), Learning, work)
					yield* db.apply(changes, { ...work, expected: { kind: "any" } })
					const snapshot = yield* db.snapshot(work)
					const stored = yield* snapshot.get(Student, { id: kept }, work)
					assert.ok(Option.isSome(stored))
					assert.equal(stored.value.name, "Kept")
					const forged = yield* snapshot.get(Student, { id: impostor }, work)
					assert.ok(Option.isNone(forged))
				})
			)
		)
	} finally {
		await Effect.runPromise(rt.disposeEffect)
	}
})

test("a throwing getter is a typed input failure that spends and drains the draft", async function getterThrows() {
	const rt = runtime()
	try {
		const good = await newId()
		const exit = await rt.runPromiseExit(
			Effect.scoped(
				Effect.gen(function* () {
					const draft = yield* ChangeSet.builder(Learning, work)
					const hostile = {
						id: good,
						get name(): string {
							throw new Error("hostile getter")
						},
						budget: 10n
					}
					const failed = yield* Effect.exit(draft.insert(Student, [hostile]))
					assert.equal(failed._tag, "Failure", "the getter throw is a typed input failure")
					// The failure SPENT the draft: every later use refuses.
					const late = yield* Effect.exit(draft.insert(Student, [studentRow(good, "Late")]))
					assert.equal(late._tag, "Failure")
					const finish = yield* Effect.exit(draft.finish())
					assert.equal(finish._tag, "Failure")
					return "done"
				})
			)
		)
		assert.equal(exit._tag, "Success")
	} finally {
		await Effect.runPromise(rt.disposeEffect)
	}
})

test("ingestion charges input once per operation and cumulatively across calls", async function cumulativeBudget() {
	const rt = runtime()
	try {
		// Native charge: UUID 24 + string (8 + UTF-8 length) + i64 16.
		const row = studentRow(await newId(), "x".repeat(2048))
		const tight = { ...work, inputBytes: 2096n }
		await rt.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const draft = yield* ChangeSet.builder(Learning, tight)
					// Exactly one row fits. A duplicate native charge between
					// JS extraction and worker ingestion would refuse this call.
					yield* draft.insert(Student, [row])
					const exit = yield* Effect.exit(draft.insert(Student, [row]))
					assert.equal(exit._tag, "Failure")
					if (exit._tag === "Failure") {
						const failure = exit.cause.reasons.find(Cause.isFailReason)
						assert.ok(failure?.error instanceof DbError)
						assert.deepEqual(failure.error.reason, {
							_tag: "ResourceLimit",
							dimension: "inputBytes",
							used: 2096n,
							requested: 2096n,
							limit: 2096n
						})
					}
					assert.equal((yield* Effect.exit(draft.finish()))._tag, "Failure", "a refused draft is spent")
				})
			)
		)
	} finally {
		await Effect.runPromise(rt.disposeEffect)
	}
})

test("finish consumes the draft: use-after-finish and a second finish refuse as SpentHandle", async function finishSpends() {
	const rt = runtime()
	try {
		const only = await newId()
		await rt.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const draft = yield* ChangeSet.builder(Learning, work)
					yield* draft.insert(Student, [studentRow(only, "Only")])
					const changes = yield* draft.finish()
					assert.ok(typeof changes.schemaId === "string")

					const lateInsert = yield* Effect.exit(draft.insert(Student, [studentRow(only, "Late")]))
					assert.equal(lateInsert._tag, "Failure")
					const secondFinish = yield* Effect.exit(draft.finish())
					assert.equal(secondFinish._tag, "Failure")
					for (const failed of [lateInsert, secondFinish]) {
						if (failed._tag !== "Failure") {
							continue
						}
						const reason = failed.cause.reasons.find(Cause.isFailReason)
						assert.ok(reason?.error instanceof DbError)
						assert.equal(reason.error.code, "SpentHandle")
					}
				})
			)
		)
	} finally {
		await Effect.runPromise(rt.disposeEffect)
	}
})

test("same-command normalization: exact same-fact add wins over remove, independent of call order", async function addWins() {
	const rt = runtime()
	try {
		const studentId = await newId()
		const attemptId = await newId()
		await rt.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.create(storeDir("add-wins"), Learning, work)
					const seed = yield* ChangeSet.builder(Learning, work)
					yield* seed.insert(Student, [studentRow(studentId, "Ada")])
					const seeded = yield* seed.finish()
					yield* db.apply(seeded, { ...work, expected: { kind: "any" } })

					// One command that both deletes and inserts the identical
					// attempt fact: add wins WITHIN the command.
					const fact: Fact<typeof Attempt> = {
						id: attemptId,
						student: studentId,
						score: 0.5,
						units: 1n,
						active: { start: 0n, end: 1n }
					}
					const draft = yield* ChangeSet.builder(Learning, work)
					yield* draft.delete(Attempt, [fact])
					yield* draft.insert(Attempt, [fact])
					const changes = yield* draft.finish()
					const outcome = yield* db.apply(changes, { ...work, expected: { kind: "any" } })
					assert.equal(outcome.kind, "accepted")
					const snapshot = yield* db.snapshot(work)
					const stored = yield* snapshot.get(Attempt, { id: attemptId }, work)
					assert.ok(Option.isSome(stored), "the identical fact's add won the one-command normalization")
				})
			)
		)
	} finally {
		await Effect.runPromise(rt.disposeEffect)
	}
})

test("SharedArrayBuffer-backed cells refuse before any copy (pure projector wall)", function sharedBackingRefuses() {
	const shared = new Uint8Array(new SharedArrayBuffer(4))
	assert.throws(() => cellOf("test", bytes(4), shared), /SharedArrayBuffer-backed views are refused/)
})

test("host string length is judged before isWellFormed / copy (TS-004)", function hostLengthBeforeScan() {
	const oversize = "x".repeat(40_000)
	assert.ok(hostCellCharge(oversize) > 65536n)
	assert.throws(() => cellOf("test", str, oversize), /exceeds the 65536-byte converter bound/)
	assert.throws(() => assertHostCellFits("test", oversize, 65536n), /refuse before copy or scan/)
})
