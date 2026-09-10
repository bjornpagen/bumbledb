/**
 * The chapter 35 core surface, end to end against the real native runtime
 * (API-01/02/03/07/12; SDK-003): explicit `Db.create`/`Db.open`, coherent
 * scoped snapshots, the shared `QueryReader` capability, one immutable
 * final-state `apply` with the three-way expected-state intent, `Option`
 * lookups, honest close reports, scoped misuse refusals and foreign
 * capability refusals. Effect-only: everything below is a LAZY effect and
 * nothing runs at construction.
 */
import assert from "node:assert/strict"
import { statSync } from "node:fs"
import { join } from "node:path"
import { test } from "node:test"
import { Cause, Effect, Exit, Fiber, ManagedRuntime, Option } from "effect"
import { ChangeSet } from "#changes.ts"
import { decodeRows, encodeRows, rowShape } from "#codec.ts"
import type { ApplyOutcome, CoreWitness, Db as DbValue, Snapshot } from "#db.ts"
import { Db } from "#db.ts"
import { dbNative } from "#db-native.ts"
import { str, uuid } from "#fields.ts"
import { query } from "#query/lower.ts"
import { v } from "#query/scope.ts"
import { relation } from "#relation.ts"
import { NativeRuntime } from "#runtime.ts"
import { DbError } from "#runtime-errors.ts"
import type { AnySchema } from "#schema.ts"
import { schema } from "#schema.ts"
import { Attempt, Learning, runtimeOptions, Student, StudentById, storeDir } from "#test/fixtures/learning.ts"
import type { Uuid } from "#uuid.ts"

const attemptsFor = query(Learning).rule((r) => {
	const { id, student, score, units, active } = v(Attempt)
	return r
		.match(Attempt, { id, student, score, units, active })
		.where(r.eq(student, r.param("student")))
		.find({ id, student, score, units, active })
})

function runtime() {
	return ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
}

const newId = () => Effect.runPromise(Effect.sync(() => crypto.randomUUID()))

test("inspection separates virtual mapping, file pages and disk blocks", async function inspectStorage() {
	const rt = runtime()
	try {
		await rt.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const directory = storeDir("storage-inspection")
					const db = yield* Db.create(directory, Learning)
					const report = yield* db.inspect()
					const file = statSync(join(directory, "store", "data.mdb"), { bigint: true })
					assert.deepEqual(Object.keys(report.storage).sort(), [
						"allocatedDiskBytes",
						"nonFreePageBytes",
						"populatedFileBytes",
						"virtualMapBytes"
					])
					assert.equal(report.storage.populatedFileBytes, file.size)
					assert.ok(report.storage.virtualMapBytes >= file.size)
					assert.ok(report.storage.nonFreePageBytes > 0n)
					assert.ok(report.storage.nonFreePageBytes <= file.size)
					if (process.platform === "win32") {
						assert.equal(report.storage.allocatedDiskBytes, null)
					} else {
						assert.equal(report.storage.allocatedDiskBytes, file.blocks * 512n)
					}
					assert.equal("residentEstimateBytes" in report, false, "live disk pages do not measure resident RAM")
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
			const encode = dbNative.runtimeEncodeRows
			const decode = dbNative.runtimeDecodeRows
			dbNative.runtimeEncodeRows = (runtime, spec, relation, count, cells, callback) =>
				encode(runtime, spec, relation, count, cells, () => completed.resolve(callback))
			dbNative.runtimeDecodeRows = (runtime, spec, relation, bytes, callback) =>
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
				dbNative.runtimeEncodeRows = encode
				dbNative.runtimeDecodeRows = decode
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

test("create/apply/snapshot/get/execute — the whole chapter 34 core flow, one scope", async function coreFlow() {
	const rt = runtime()
	try {
		const studentId = await newId()
		const attemptId = await newId()
		const program = Effect.scoped(
			Effect.gen(function* () {
				const db = yield* Db.create(storeDir("core-flow"), Learning)
				const changes = yield* seeded(studentId, attemptId)
				const outcome = yield* db.apply(changes, { expected: { kind: "any" } })
				assert.equal(outcome.kind, "accepted")
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
					yield* db.apply(changes, { expected: { kind: "any" } })
					const snapshot = yield* db.snapshot()
					const first = yield* snapshot.prepare(attemptsFor)
					const second = yield* snapshot.prepare(attemptsFor)
					const retained = yield* first.execute({ student: studentId })
					yield* first.releaseMemory()
					yield* first.releaseMemory()
					yield* db.clearCache()
					const reused = yield* first.execute({ student: studentId })
					assert.equal((yield* reused.collect())[0]?.id, attemptId)
					assert.equal((yield* first.close()).kind, "closed")
					assert.equal((yield* first.close()).kind, "closed")
					const closed = yield* Effect.exit(first.execute({ student: studentId }))
					assert.ok(Exit.isFailure(closed))
					assert.ok(Exit.isFailure(yield* Effect.exit(first.releaseMemory())))
					assert.ok(Option.isSome(yield* snapshot.get(StudentById, { id: studentId })))
					const late = yield* seeded(absentId, lateAttemptId)
					assert.equal((yield* db.apply(late, { expected: { kind: "any" } })).kind, "accepted")
					// A preparation shares the pinned version, not the snapshot's close authority.
					assert.equal((yield* snapshot.close()).kind, "closed")
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
					assert.equal((yield* second.close()).kind, "closed")
					const rows = yield* retained.collect()
					assert.equal(rows[0]?.id, attemptId)
					// Ordinary scope release, not just explicit close, spends the plan.
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
					const first = yield* db.apply(changes, { expected: { kind: "any" } })
					// The identical sealed change is reusable while open: a
					// second application of the same final set is no-change.
					const second = yield* db.apply(changes, { expected: { kind: "any" } })

					// A violating candidate: an attempt referencing an
					// undeclared student breaks the containment law.
					const bad = yield* ChangeSet.builder(Learning)
					yield* bad.insert(Attempt, [
						{ id: outsider, student: outsider, score: 0.1, units: 1n, active: { start: 0n, end: 1n } }
					])
					const violating = yield* bad.finish()
					const rejected = yield* db.apply(violating, { expected: { kind: "any" } })

					// A stale exact-state witness moves, never silently applies.
					const snapshot = yield* db.snapshot()
					const witness: CoreWitness = snapshot.witness
					const third = yield* ChangeSet.builder(Learning)
					yield* third.insert(Student, [{ id: outsider, name: "Bo", budget: 1n }])
					const advance = yield* third.finish()
					yield* db.apply(advance, { expected: { kind: "any" } })
					const fourth = yield* ChangeSet.builder(Learning)
					yield* fourth.insert(Student, [{ id: attemptId, name: "Cy", budget: 1n }])
					const staleChange = yield* fourth.finish()
					const moved = yield* db.apply(staleChange, { expected: { kind: "exact", at: witness } })
					return { first, second, rejected, moved }
				})
			)
		)
		assert.equal(outcomes.first.kind, "accepted")
		assert.equal(outcomes.second.kind, "no-change")
		assert.equal(outcomes.rejected.kind, "invariant-rejected")
		if (outcomes.rejected.kind === "invariant-rejected") {
			assert.ok(outcomes.rejected.violations.length > 0, "complete statement diagnostics, never a bare boolean")
		}
		assert.equal(outcomes.moved.kind, "moved")
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
					yield* db.apply(changes, { expected: { kind: "any" } })
					const snapshot = yield* db.snapshot()
					const late = yield* ChangeSet.builder(Learning)
					yield* late.insert(Student, [{ id: lateId, name: "Late", budget: 1n }])
					const lateChanges = yield* late.finish()
					const outcome = yield* db.apply(lateChanges, { expected: { kind: "any" } })
					assert.equal(outcome.kind, "accepted")
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
					yield* db.apply(changes, { expected: { kind: "any" } })
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
		// Early close on an already scope-closed owner is idempotent and honest.
		const report = await rt.runPromise(escapedDb.close())
		assert.ok(report.kind === "closed" || report.kind === "failed")
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
					return yield* db.apply(forged as never, { expected: { kind: "any" } })
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

/** The chapter 35 ApplyOutcome vocabulary is the pinned public type. */
function applyOutcomeShape(outcome: ApplyOutcome): string {
	return outcome.kind
}
void applyOutcomeShape
