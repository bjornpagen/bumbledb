/**
 * Packed core-TypeScript consumer (D07/D22/D27): the shared `Learning`
 * schema, reusable typed queries, one scoped ChangeSet, the shared
 * QueryReader helper, direct local admission, a witnessed correction,
 * field-arithmetic backfill metadata, scoped collect/pages, and joined
 * close. Importing this module performs no native work.
 *
 * Verification: NotRun until packed-consumer qualification.
 */
import {
	capacity,
	ChangeSet,
	contained,
	Db,
	describeQuery,
	f64,
	i64,
	Uuid,
	uuid,
	interval,
	key,
	NativeRuntime,
	type NativeRuntimeOptions,
	on,
	query,
	queryFromDescription,
	type QueryReader,
	ref,
	relation,
	Scalar,
	schema,
	str,
	u64,
	v,
	weigh,
	within
} from "@bjornpagen/bumbledb"
import { Effect, ManagedRuntime, Option, Stream } from "effect"
import { randomUUID } from "node:crypto"

export const Student = relation("Student", { id: uuid, name: str, budget: u64 })
export const Attempt = relation("Attempt", {
	id: uuid,
	student: uuid,
	score: f64,
	units: u64,
	active: interval(i64)
})

export const StudentById = key(Student, ["id"])
export const AttemptById = key(Attempt, ["id"])
export const Learning = schema("Learning", { Student, Attempt }, [
	StudentById,
	AttemptById,
	contained(on(Attempt, "student"), on(Student, "id")),
	capacity(on(Student, "id"), {
		from: on(Attempt, "student"),
		weight: weigh("units"),
		within: within(0n, ref("budget"))
	})
])

/** D27: unresolved field arithmetic authors synchronously. No native load. */
export const incrementUnits = Scalar.add(Scalar.field("units"), Scalar.u64(1n))
export const incrementUnitsAsF64 = Scalar.toF64(Scalar.add(Scalar.field("units"), Scalar.u64(1n)))

export const attemptsFor = query(Learning).rule((r) => {
	const { id, student, score, units, active } = v(Attempt)
	return r
		.match(Attempt, { id, student, score, units, active })
		.where(r.eq(student, r.param("student")))
		.find({ id, student, score, units, active })
})

/** Generated logical descriptions use the same checked schema and typed result fields. */
export const describedAttempts = queryFromDescription(Learning, describeQuery(attemptsFor), Attempt.fields)

export const attemptStats = query(Learning)
	.rule((r) => {
		const { id, student, score } = v(Attempt)
		return r.match(Attempt, { id, student, score }).find({ student, total: r.sum(score), mean: r.mean(score) })
	})
	.named("attemptStats")

export const studentSummary = query(Learning).rule((r) => {
	const { student, total, mean } = v(attemptStats)
	const { name } = v(Student)
	return r
		.match(attemptStats, { student, total, mean })
		.match(Student, { id: student, name })
		.find({ student, name, total, mean })
})

export const newAttempt = Effect.fn("newAttempt")(function* (
	studentId: Uuid,
	attemptId: Uuid
) {
	const draft = yield* ChangeSet.builder(Learning)
	const active = { start: 0n, end: 60n }
	yield* draft.insert(Student, [{ id: studentId, name: "Ada", budget: 10n }])
	yield* draft.insert(Attempt, [
		{ id: attemptId, student: studentId, score: 0.9, units: 1n, active }
	])
	return yield* draft.finish()
})

/** Same helper on a core snapshot and a published log snapshot — no adapter. */
export const readAttempts = Effect.fn("readAttempts")(
	function* (reader: QueryReader<typeof Learning>, student: Uuid) {
		const result = yield* reader.execute(attemptsFor, { student })
		return yield* result.collect()
	},
	Effect.scoped
)

export const runtimePolicy: NativeRuntimeOptions = {
	workers: 2,
	queueCapacity: 16,
	cleanupCapacity: 16,
	ownerCapacity: 16,
	nativeHandleCapacity: 64,
	cleanupTimeout: "2 seconds"
}



/** One process-lifetime runtime. Request code must not construct another. */
export const makeConsumerRuntime = () => ManagedRuntime.make(NativeRuntime.layer(runtimePolicy))

export const coreProgram = (localPath: string) =>
	Effect.scoped(
		Effect.gen(function* () {
			const db = yield* Db.create(localPath, Learning)
			const studentId = yield* Effect.sync(() => randomUUID())
			const attemptId = yield* Effect.sync(() => randomUUID())
			const changes = yield* newAttempt(studentId, attemptId)
			const outcome = yield* db.apply(changes, { expected: { kind: "any" } })
			if (outcome.kind !== "accepted" && outcome.kind !== "no-change") {
				const closed = yield* db.close()
				return { outcome, rows: [] as const, closed }
			}
			const snapshot = yield* db.snapshot()
			const rows = yield* readAttempts(snapshot, studentId)
			const closed = yield* db.close()
			return { outcome, rows, closed }
		})
	)

export const correctScore = (localPath: string, attemptId: Uuid) =>
	Effect.scoped(
		Effect.gen(function* () {
			const db = yield* Db.open(localPath, Learning)
			const observed = yield* Effect.scoped(
				Effect.gen(function* () {
					const snapshot = yield* db.snapshot()
					const previous = yield* snapshot.get(AttemptById, { id: attemptId })
					if (Option.isNone(previous)) {
						return yield* Effect.fail({ missing: attemptId })
					}
					return { previous: previous.value, at: snapshot.witness }
				})
			)
			const draft = yield* ChangeSet.builder(Learning)
			yield* draft.delete(Attempt, [observed.previous])
			yield* draft.insert(Attempt, [{ ...observed.previous, score: 0.95 }])
			const changes = yield* draft.finish()
			const outcome = yield* db.apply(changes, { expected: { kind: "exact", at: observed.at } })
			const closed = yield* db.close()
			return { outcome, closed }
		})
	)

export const drainPages = (reader: QueryReader<typeof Learning>, student: Uuid) =>
	Effect.scoped(
		Effect.gen(function* () {
			const result = yield* reader.execute(attemptsFor, { student })
			return yield* result.pages().pipe(
				Stream.runFold(() => 0, (rows, page) => rows + page.length)
			)
		})
	)
