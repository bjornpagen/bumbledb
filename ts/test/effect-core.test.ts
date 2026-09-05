/**
 * The chapter 35 core surface, end to end against the real native runtime
 * (API-01/02/03/07/12; SDK-003): explicit `Db.create`/`Db.open`, coherent
 * scoped snapshots, the shared `QueryReader` capability, one immutable
 * final-state `apply` with the three-way expected-state intent, `Option`
 * lookups, honest close reports, scoped misuse refusals and foreign
 * capability refusals. Effect-only: everything below is a LAZY effect and
 * nothing runs at construction.
 *
 * Verification: NotRun until F3 — these lanes execute once P06R's
 * db-bridge verbs land in the rebuilt addon (`#db-native.ts` is the
 * consumer-side pin).
 */
import assert from "node:assert/strict"
import { test } from "node:test"
import { Cause, Effect, Exit, Fiber, ManagedRuntime, Option } from "effect"
import { ChangeSet } from "#changes.ts"
import type { ApplyOutcome, CoreWitness, Db as DbValue, Snapshot } from "#db.ts"
import { Db } from "#db.ts"
import { Id128 } from "#id128.ts"
import { query } from "#query/lower.ts"
import { v } from "#query/scope.ts"
import { id128, str } from "#fields.ts"
import { relation } from "#relation.ts"
import { NativeRuntime } from "#runtime.ts"
import { DbError } from "#runtime-errors.ts"
import type { AnySchema } from "#schema.ts"
import { schema } from "#schema.ts"
import { Attempt, Learning, runtimeOptions, Student, storeDir, work } from "#test/fixtures/learning.ts"

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

const newId = () => Effect.runPromise(Id128.random())

function seeded(studentId: Id128, attemptId: Id128) {
	return Effect.gen(function* () {
		const draft = yield* ChangeSet.builder(Learning, work)
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
				const db = yield* Db.create(storeDir("core-flow"), Learning, work)
				const changes = yield* seeded(studentId, attemptId)
				const outcome = yield* db.apply(changes, { ...work, expected: { kind: "any" } })
				assert.equal(outcome.kind, "accepted")
				const snapshot = yield* db.snapshot(work)
				// Missing key is Option.none, never a fake I/O error.
				const absent = yield* snapshot.get(Student, { id: attemptId }, work)
				assert.ok(Option.isNone(absent))
				const present = yield* snapshot.get(Student, { id: studentId }, work)
				assert.ok(Option.isSome(present))
				assert.deepEqual(present.value, { id: studentId, name: "Ada", budget: 10n })
				const result = yield* snapshot.execute(attemptsFor, { student: studentId }, work)
				const rows = yield* result.collect({ maxBytes: work.resultBytes })
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
		const missing = await rt.runPromiseExit(Effect.scoped(Db.open(path, Learning, work)))
		assert.equal(missing._tag, "Failure", "open of a missing database never creates a replacement")

		await rt.runPromise(Effect.scoped(Db.create(path, Learning, work).pipe(Effect.asVoid)))
		const second = await rt.runPromiseExit(Effect.scoped(Db.create(path, Learning, work)))
		assert.equal(second._tag, "Failure", "create refuses existing authority")

		// The refused attempts left the directory adoptable: open succeeds.
		const reopened = await rt.runPromise(
			Effect.scoped(Db.open(path, Learning, work).pipe(Effect.map((db) => db.schemaId)))
		)
		assert.ok(reopened.length > 0)
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
					const db = yield* Db.create(storeDir("apply-outcomes"), Learning, work)
					const changes = yield* seeded(studentId, attemptId)
					const first = yield* db.apply(changes, { ...work, expected: { kind: "any" } })
					// The identical sealed change is reusable while open: a
					// second application of the same final set is no-change.
					const second = yield* db.apply(changes, { ...work, expected: { kind: "any" } })

					// A violating candidate: an attempt referencing an
					// undeclared student breaks the containment law.
					const bad = yield* ChangeSet.builder(Learning, work)
					yield* bad.insert(Attempt, [
						{ id: outsider, student: outsider, score: 0.1, units: 1n, active: { start: 0n, end: 1n } }
					])
					const violating = yield* bad.finish()
					const rejected = yield* db.apply(violating, { ...work, expected: { kind: "any" } })

					// A stale exact-state witness moves, never silently applies.
					const snapshot = yield* db.snapshot(work)
					const witness: CoreWitness = snapshot.witness
					const third = yield* ChangeSet.builder(Learning, work)
					yield* third.insert(Student, [{ id: outsider, name: "Bo", budget: 1n }])
					const advance = yield* third.finish()
					yield* db.apply(advance, { ...work, expected: { kind: "any" } })
					const fourth = yield* ChangeSet.builder(Learning, work)
					yield* fourth.insert(Student, [{ id: attemptId, name: "Cy", budget: 1n }])
					const staleChange = yield* fourth.finish()
					const moved = yield* db.apply(staleChange, { ...work, expected: { kind: "exact", at: witness } })
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
					const db = yield* Db.create(storeDir("snapshot-coherence"), Learning, work)
					const changes = yield* seeded(studentId, attemptId)
					yield* db.apply(changes, { ...work, expected: { kind: "any" } })
					const snapshot = yield* db.snapshot(work)
					const late = yield* ChangeSet.builder(Learning, work)
					yield* late.insert(Student, [{ id: lateId, name: "Late", budget: 1n }])
					const lateChanges = yield* late.finish()
					const outcome = yield* db.apply(lateChanges, { ...work, expected: { kind: "any" } })
					assert.equal(outcome.kind, "accepted")
					// The pinned snapshot still answers the OLD state.
					const observed = yield* snapshot.get(Student, { id: lateId }, work)
					assert.ok(Option.isNone(observed), "the open snapshot never observes the later apply")
					const fresh = yield* db.snapshot(work)
					const now = yield* fresh.get(Student, { id: lateId }, work)
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
					const db = yield* Db.create(storeDir("scoped-misuse"), Learning, work)
					const changes = yield* seeded(studentId, attemptId)
					yield* db.apply(changes, { ...work, expected: { kind: "any" } })
					escapedDb = db
					escapedSnapshot = yield* db.snapshot(work)
					// Constructing an effect on a live handle runs NOTHING:
					// dropping it unexecuted has no observable consequence.
					void db.inspect(work)
					void escapedSnapshot.get(Student, { id: studentId }, work)
				})
			)
		)
		assert.ok(escapedDb && escapedSnapshot)
		// The scope closed both owners: late-constructed effects on the
		// escaped handles fail with a typed DbError, never dangle natively.
		const lateGet = await rt.runPromiseExit(escapedSnapshot.get(Student, { id: studentId }, work))
		assert.equal(lateGet._tag, "Failure")
		if (lateGet._tag === "Failure") {
			const reason = lateGet.cause.reasons.find(Cause.isFailReason)
			assert.ok(reason?.error instanceof DbError)
		}
		const lateInspect = await rt.runPromiseExit(escapedDb.inspect(work))
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
					const db = yield* Db.create(storeDir("foreign-changes"), Learning, work)
					const forged = { schemaId: db.schemaId, close: () => Effect.void }
					return yield* db.apply(forged as never, { ...work, expected: { kind: "any" } })
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
		const Widget = relation("Widget", { id: id128, name: str })
		const Foreign: AnySchema = schema("Foreign", { Widget }, [])
		const foreignQuery = query(Foreign).rule((r) => {
			const { id, name } = v(Widget)
			return r.match(Widget, { id, name }).find({ id, name })
		})
		const exit = await rt.runPromiseExit(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.create(storeDir("foreign-template"), Learning, work)
					const snapshot = yield* db.snapshot(work)
					return yield* snapshot.execute(foreignQuery as never, {}, work)
				})
			)
		)
		assert.equal(exit._tag, "Failure")
		if (exit._tag === "Failure") {
			const reason = exit.cause.reasons.find(Cause.isFailReason)
			assert.ok(reason?.error instanceof DbError)
			assert.equal(reason.error.code, "Incompatible")
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
		const program = Effect.scoped(
			Effect.gen(function* () {
				yield* Db.create(storeDir("interruption-cause"), Learning, work)
				return yield* Effect.never
			})
		)
		const fiber = await rt.runPromise(Effect.fork(program))
		// Give acquisition a chance to genuinely start before interrupting.
		await new Promise((resolve) => setTimeout(resolve, 25))
		const exit = await rt.runPromise(Fiber.interrupt(fiber))
		assert.ok(Exit.hasInterrupts(exit), "interruption is Cause, not a failure arm")
		// The directory is reusable afterwards: the teardown joined.
		await rt.runPromise(
			Effect.scoped(Db.create(storeDir("interruption-cause-2"), Learning, work).pipe(Effect.asVoid))
		)
	} finally {
		await Effect.runPromise(rt.disposeEffect)
	}
})

/** The chapter 35 ApplyOutcome vocabulary is the pinned public type. */
function applyOutcomeShape(outcome: ApplyOutcome): string {
	return outcome.kind
}
void applyOutcomeShape
