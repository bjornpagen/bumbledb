import assert from "node:assert/strict"
import { test } from "node:test"
import { Cause, Effect, Exit, Fiber, ManagedRuntime, Option, Scope } from "effect"
import { ChangeSet } from "../src/changes.ts"
import { decodeRows, encodeRows, rowShape } from "../src/codec.ts"
import type { Db as DbValue, Snapshot, Witness } from "../src/db.ts"
import { Db } from "../src/db.ts"
import { DbError } from "../src/errors.ts"
import { str, uuid } from "../src/fields.ts"
import { addon } from "../src/native/addon.ts"
import { query } from "../src/query/lower.ts"
import { v } from "../src/query/scope.ts"
import { relation } from "../src/relation.ts"
import { Bumble } from "../src/runtime.ts"
import type { AnySchema } from "../src/schema.ts"
import { schema } from "../src/schema.ts"
import type { Uuid } from "../src/uuid.ts"
import { Attempt, Learning, runtimeOptions, Student, StudentById, storeDir } from "./fixtures/learning.ts"

const attemptsFor = query(Learning).rule((r) => {
	const { id, student, score, units, active } = v(Attempt)
	return r
		.match(Attempt, { id, student, score, units, active })
		.where(r.eq(student, r.param("student")))
		.find({ id, student, score, units, active })
})

function runtime() {
	return ManagedRuntime.make(Bumble.layer(runtimeOptions))
}

const newId = () => Effect.runPromise(Effect.sync(() => crypto.randomUUID()))

test("inspection reports measurements of a fresh database", async function inspectStorage() {
	const rt = runtime()
	try {
		await rt.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.create(storeDir("storage-inspection"), Learning)
					const report = yield* db.inspect()
					assert.equal(report.schemaId, db.schemaId)
					assert.ok(report.diskBytes > 0n)
				})
			)
		)
	} finally {
		await Effect.runPromise(rt.disposeEffect)
	}
})

test("native row codecs own their values and remain reusable after cancelled delivery", async function canonicalRows() {
	const rt = runtime()
	const shape = rowShape(Learning, Student)
	const first: {
		id: Uuid
		name: string
		budget: bigint
	} = { id: "00000000-0000-0000-0000-000000000001", name: "\u{1f41d}".repeat(4097), budget: 10n }
	const second = { ...first, id: "00000000-0000-0000-0000-000000000002" } satisfies typeof first
	try {
		const encoded = await rt.runPromise(encodeRows(shape, [second, first, first]))
		assert.ok(encoded instanceof Uint8Array)
		const decoded = await rt.runPromise(decodeRows(shape, encoded))
		assert.deepEqual(decoded, [first, second])
		encoded.fill(0)
		assert.deepEqual(decoded, [first, second], "decoded values do not alias the input buffer")
		const fresh = await rt.runPromise(encodeRows(shape, [first]))
		for (const kind of ["encode", "decode"] as const) {
			const completed = Promise.withResolvers<() => void>()
			const encode = addon.runtimeEncodeRows
			const decode = addon.runtimeDecodeRows
			addon.runtimeEncodeRows = (runtime, spec, relation, count, cells, callback) =>
				encode(runtime, spec, relation, count, cells, () => completed.resolve(callback))
			addon.runtimeDecodeRows = (runtime, spec, relation, bytes, callback) =>
				decode(runtime, spec, relation, bytes, () => completed.resolve(callback))
			try {
				const operation =
					kind === "encode"
						? encodeRows(shape, [first]).pipe(Effect.asVoid)
						: decodeRows(shape, fresh).pipe(Effect.asVoid)
				const fiber = rt.runFork(operation)
				const lateCallback = await completed.promise
				await Effect.runPromise(Fiber.interrupt(fiber))
				assert.ok(Exit.hasInterrupts(await Effect.runPromise(Fiber.await(fiber))))
				lateCallback()
			} finally {
				addon.runtimeEncodeRows = encode
				addon.runtimeDecodeRows = decode
			}
			assert.deepEqual(await rt.runPromise(decodeRows(shape, fresh)), [first])
		}
		assert.deepEqual(await rt.runPromise(decodeRows(shape, await rt.runPromise(encodeRows(shape, [])))), [])
		const Flag = relation("Flag", {})
		const flagShape = rowShape(schema("Flags", { Flag }, []), Flag)
		for (const input of [[], [{}], [{}, {}, {}]]) {
			const bytes = await rt.runPromise(encodeRows(flagShape, input))
			assert.deepEqual(await rt.runPromise(decodeRows(flagShape, bytes)), input.length === 0 ? [] : [{}])
		}
	} finally {
		await Effect.runPromise(rt.disposeEffect)
	}
})

function seeded(studentId: Uuid, attemptId: Uuid) {
	return Effect.gen(function* () {
		const draft = yield* ChangeSet.builder(Learning)
		yield* draft.insert(Student, [{ id: studentId, name: "Ada", budget: 10n }])
		yield* draft.insert(Attempt, [
			{ id: attemptId, student: studentId, score: 0.9, units: 1n, active: { start: 0n, end: 60n } }
		])
		return yield* draft.finish()
	})
}

test("create/apply/snapshot/get/execute — the whole core flow, one scope", async function coreFlow() {
	const rt = runtime()
	try {
		const studentId = await newId()
		const attemptId = await newId()
		const program = Effect.scoped(
			Effect.gen(function* () {
				const db = yield* Db.create(storeDir("core-flow"), Learning)
				const changes = yield* seeded(studentId, attemptId)
				const outcome = yield* db.apply(changes)
				assert.equal(outcome._tag, "Committed")
				const snapshot = yield* db.snapshot()
				// Missing key is Option.none, never a fake I/O error.
				const absent = yield* snapshot.get(StudentById, { id: attemptId })
				assert.ok(Option.isNone(absent))
				const present = yield* snapshot.get(StudentById, { id: studentId })
				assert.ok(Option.isSome(present))
				assert.deepEqual(present.value, { id: studentId, name: "Ada", budget: 10n })
				const result = yield* snapshot.execute(attemptsFor, { student: studentId })
				const rows = yield* result.collect()
				assert.equal(rows.length, 1)
				assert.deepEqual(rows[0], {
					id: attemptId,
					student: studentId,
					score: 0.9,
					units: 1n,
					active: { start: 0n, end: 60n }
				})
				return db.schemaId
			})
		)
		const schemaId = await rt.runPromise(program)
		assert.ok(typeof schemaId === "string" && schemaId.length > 0)
	} finally {
		await Effect.runPromise(rt.disposeEffect)
	}
})

test("open never creates; create refuses existing authority", async function createOpenSplit() {
	const rt = runtime()
	try {
		const path = storeDir("create-open")
		const missing = await rt.runPromiseExit(Effect.scoped(Db.open(path, Learning)))
		assert.equal(missing._tag, "Failure", "open of a missing database never creates a replacement")

		await rt.runPromise(Effect.scoped(Db.create(path, Learning).pipe(Effect.asVoid)))
		const second = await rt.runPromiseExit(Effect.scoped(Db.create(path, Learning)))
		assert.equal(second._tag, "Failure", "create refuses existing authority")

		// The refused attempts left the directory adoptable: open succeeds.
		const reopened = await rt.runPromise(Effect.scoped(Db.open(path, Learning).pipe(Effect.map((db) => db.schemaId))))
		assert.ok(reopened.length > 0)
	} finally {
		await Effect.runPromise(rt.disposeEffect)
	}
})

test("scoped preparation reuses parameters and closes independently of snapshots and results", async function preparedOwnership() {
	const rt = runtime()
	try {
		const studentId = await newId()
		const attemptId = await newId()
		const absentId = await newId()
		const lateAttemptId = await newId()
		await rt.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.create(storeDir("prepared-ownership"), Learning)
					const changes = yield* seeded(studentId, attemptId)
					yield* db.apply(changes)
					const snapshotScope = yield* Scope.make()
					const firstScope = yield* Scope.make()
					const secondScope = yield* Scope.make()
					const snapshot = yield* db.snapshot().pipe(Scope.provide(snapshotScope))
					const first = yield* snapshot.prepare(attemptsFor).pipe(Scope.provide(firstScope))
					const second = yield* snapshot.prepare(attemptsFor).pipe(Scope.provide(secondScope))
					const retained = yield* first.execute({ student: studentId })
					yield* first.releaseMemory()
					yield* first.releaseMemory()
					yield* db.clearCache()
					const reused = yield* first.execute({ student: studentId })
					assert.equal((yield* reused.collect())[0]?.id, attemptId)
					yield* Scope.close(firstScope, Exit.void)
					const closed = yield* Effect.exit(first.execute({ student: studentId }))
					assert.ok(Exit.isFailure(closed))
					assert.ok(Exit.isFailure(yield* Effect.exit(first.releaseMemory())))
					assert.ok(Option.isSome(yield* snapshot.get(StudentById, { id: studentId })))
					const late = yield* seeded(absentId, lateAttemptId)
					assert.equal((yield* db.apply(late))._tag, "Committed")
					// A preparation pins the snapshot's version on its own; closing the snapshot leaves it usable.
					yield* Scope.close(snapshotScope, Exit.void)
					for (let i = 0; i < 16; i += 1) {
						yield* second.releaseMemory()
						yield* Effect.scoped(
							Effect.gen(function* () {
								const result = yield* second.execute({ student: i % 2 === 0 ? studentId : absentId })
								const rows = yield* result.collect()
								assert.equal(rows.length, i % 2 === 0 ? 1 : 0)
								if (rows.length > 0) assert.equal(rows[0]?.id, attemptId)
							})
						)
					}
					yield* Scope.close(secondScope, Exit.void)
					const rows = yield* retained.collect()
					assert.equal(rows[0]?.id, attemptId)
					const fresh = yield* db.snapshot()
					assert.ok(Option.isSome(yield* fresh.get(StudentById, { id: absentId })))
					const escaped = yield* Effect.scoped(fresh.prepare(attemptsFor))
					const misuse = yield* Effect.exit(escaped.execute({ student: studentId }))
					assert.ok(Exit.isFailure(misuse))
				})
			)
		)
	} finally {
		await Effect.runPromise(rt.disposeEffect)
	}
})

test("apply is the three-coordinate judgment: accepted, no-change, invariant-rejected, moved", async function applyOutcomes() {
	const rt = runtime()
	try {
		const studentId = await newId()
		const attemptId = await newId()
		const outsider = await newId()
		const outcomes = await rt.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.create(storeDir("apply-outcomes"), Learning)
					const changes = yield* seeded(studentId, attemptId)
					const first = yield* db.apply(changes)
					// Applying the same sealed change again commits without changing the state.
					const second = yield* db.apply(changes)

					// A violating candidate: an attempt referencing an
					// undeclared student breaks the containment law.
					const bad = yield* ChangeSet.builder(Learning)
					yield* bad.insert(Attempt, [
						{ id: outsider, student: outsider, score: 0.1, units: 1n, active: { start: 0n, end: 1n } }
					])
					const violating = yield* bad.finish()
					const rejected = yield* db.apply(violating)

					// A stale exact-state witness moves, never silently applies.
					const snapshot = yield* db.snapshot()
					const witness: Witness = snapshot.witness
					const third = yield* ChangeSet.builder(Learning)
					yield* third.insert(Student, [{ id: outsider, name: "Bo", budget: 1n }])
					const advance = yield* third.finish()
					yield* db.apply(advance)
					const fourth = yield* ChangeSet.builder(Learning)
					yield* fourth.insert(Student, [{ id: attemptId, name: "Cy", budget: 1n }])
					const staleChange = yield* fourth.finish()
					const moved = yield* db.apply(staleChange, witness)
					return { first, second, rejected, moved }
				})
			)
		)
		assert.equal(outcomes.first._tag, "Committed")
		assert.equal(outcomes.second._tag, "Committed")
		if (outcomes.first._tag === "Committed" && outcomes.second._tag === "Committed") {
			assert.equal(outcomes.first.changed, true)
			assert.deepEqual(outcomes.second, { _tag: "Committed", generation: outcomes.first.generation, changed: false })
		}
		assert.equal(outcomes.rejected._tag, "Rejected")
		if (outcomes.rejected._tag === "Rejected") {
			assert.ok(outcomes.rejected.violations.length > 0, "complete statement diagnostics, never a bare boolean")
		}
		assert.equal(outcomes.moved._tag, "Moved")
	} finally {
		await Effect.runPromise(rt.disposeEffect)
	}
})

test("a snapshot is coherent: a later apply cannot move an open snapshot's facts", async function snapshotCoherence() {
	const rt = runtime()
	try {
		const studentId = await newId()
		const attemptId = await newId()
		const lateId = await newId()
		await rt.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.create(storeDir("snapshot-coherence"), Learning)
					const changes = yield* seeded(studentId, attemptId)
					yield* db.apply(changes)
					const snapshot = yield* db.snapshot()
					const late = yield* ChangeSet.builder(Learning)
					yield* late.insert(Student, [{ id: lateId, name: "Late", budget: 1n }])
					const lateChanges = yield* late.finish()
					const outcome = yield* db.apply(lateChanges)
					assert.equal(outcome._tag, "Committed")
					// The pinned snapshot still answers the OLD state.
					const observed = yield* snapshot.get(StudentById, { id: lateId })
					assert.ok(Option.isNone(observed), "the open snapshot never observes the later apply")
					const fresh = yield* db.snapshot()
					const now = yield* fresh.get(StudentById, { id: lateId })
					assert.ok(Option.isSome(now))
				})
			)
		)
	} finally {
		await Effect.runPromise(rt.disposeEffect)
	}
})

test("methods are lazy: construction dispatches nothing, and a scope-escaped handle fails typed", async function scopedMisuse() {
	const rt = runtime()
	try {
		const studentId = await newId()
		const attemptId = await newId()
		let escapedDb: DbValue<typeof Learning> | undefined
		let escapedSnapshot: Snapshot<typeof Learning> | undefined
		await rt.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.create(storeDir("scoped-misuse"), Learning)
					const changes = yield* seeded(studentId, attemptId)
					yield* db.apply(changes)
					escapedDb = db
					escapedSnapshot = yield* db.snapshot()
					// Constructing an effect on a live handle runs NOTHING:
					// dropping it unexecuted has no observable consequence.
					void db.inspect()
					void escapedSnapshot.get(StudentById, { id: studentId })
				})
			)
		)
		assert.ok(escapedDb && escapedSnapshot)
		// The scope closed both owners: late-constructed effects on the
		// escaped handles fail with a typed DbError, never dangle natively.
		const lateGet = await rt.runPromiseExit(escapedSnapshot.get(StudentById, { id: studentId }))
		assert.equal(lateGet._tag, "Failure")
		if (lateGet._tag === "Failure") {
			const reason = lateGet.cause.reasons.find(Cause.isFailReason)
			assert.ok(reason?.error instanceof DbError)
		}
		const lateInspect = await rt.runPromiseExit(escapedDb.inspect())
		assert.equal(lateInspect._tag, "Failure")
	} finally {
		await Effect.runPromise(rt.disposeEffect)
	}
})

test("a foreign object where a ChangeSet is expected refuses BEFORE any native dispatch", async function foreignChanges() {
	const rt = runtime()
	try {
		const exit = await rt.runPromiseExit(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.create(storeDir("foreign-changes"), Learning)
					const forged = { schemaId: db.schemaId, close: () => Effect.void }
					return yield* db.apply(forged as never)
				})
			)
		)
		assert.equal(exit._tag, "Failure")
		if (exit._tag === "Failure") {
			const reason = exit.cause.reasons.find(Cause.isFailReason)
			assert.ok(reason?.error instanceof DbError)
			assert.equal(reason.error.code, "InvalidArgument")
		}
	} finally {
		await Effect.runPromise(rt.disposeEffect)
	}
})

test("a foreign-schema query template refuses typed at execute", async function foreignTemplate() {
	const rt = runtime()
	try {
		const Widget = relation("Widget", { id: uuid, name: str })
		const Foreign: AnySchema = schema("Foreign", { Widget }, [])
		const foreignQuery = query(Foreign).rule((r) => {
			const { id, name } = v(Widget)
			return r.match(Widget, { id, name }).find({ id, name })
		})
		const exit = await rt.runPromiseExit(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.create(storeDir("foreign-template"), Learning)
					const snapshot = yield* db.snapshot()
					return yield* snapshot.execute(foreignQuery as never, {})
				})
			)
		)
		assert.equal(exit._tag, "Failure")
		if (exit._tag === "Failure") {
			const reason = exit.cause.reasons.find(Cause.isFailReason)
			assert.ok(reason?.error instanceof DbError)
			assert.equal(reason.error.code, "InvalidArgument")
		}
	} finally {
		await Effect.runPromise(rt.disposeEffect)
	}
})

test("interruption surfaces in Cause, never as a manufactured outcome arm", async function interruptionIsCause() {
	const rt = runtime()
	try {
		// A forever-suspended program holding a real database: interruption
		// tears the scope down (drain joins natively) and the Exit carries
		// interruption in Cause — no DbError is invented for it.
		const path = storeDir("interruption-cause")
		const acquired = Promise.withResolvers<void>()
		const program = Effect.scoped(
			Effect.gen(function* () {
				yield* Db.create(path, Learning)
				acquired.resolve()
				return yield* Effect.never
			})
		)
		const fiber = rt.runFork(program)
		await acquired.promise
		await rt.runPromise(Fiber.interrupt(fiber))
		const exit = await rt.runPromise(Fiber.await(fiber))
		assert.ok(Exit.hasInterrupts(exit), "interruption is Cause, not a failure arm")
		// The directory is reusable afterwards: the teardown joined.
		await rt.runPromise(Effect.scoped(Db.open(path, Learning).pipe(Effect.asVoid)))
	} finally {
		await Effect.runPromise(rt.disposeEffect)
	}
})
