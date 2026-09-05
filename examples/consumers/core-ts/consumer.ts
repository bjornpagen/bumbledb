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
	type ExecutionPolicy,
	f64,
	i64,
	Id128,
	id128,
	interval,
	key,
	NativeRuntime,
	type NativeRuntimeOptions,
	on,
	query,
	type QueryReader,
	ref,
	relation,
	Scalar,
	schema,
	span,
	str,
	u64,
	v,
	weigh,
	within
} from "@bjornpagen/bumbledb"
import { Effect, ManagedRuntime, Option, Stream } from "effect"

export const Student = relation("Student", { id: id128, name: str, budget: u64 })
export const Attempt = relation("Attempt", {
	id: id128,
	student: id128,
	score: f64,
	units: u64,
	active: interval(i64)
})

export const Learning = schema("Learning", { Student, Attempt }, [
	key(Student, ["id"]),
	key(Attempt, ["id"]),
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
	studentId: Id128,
	attemptId: Id128,
	work: ExecutionPolicy
) {
	const draft = yield* ChangeSet.builder(Learning, work)
	const active = yield* Effect.fromResult(span(0n, 60n))
	yield* draft.insert(Student, [{ id: studentId, name: "Ada", budget: 10n }])
	yield* draft.insert(Attempt, [
		{ id: attemptId, student: studentId, score: 0.9, units: 1n, active }
	])
	return yield* draft.finish()
})

/** Same helper on a core snapshot and a published log snapshot — no adapter. */
export const readAttempts = Effect.fn("readAttempts")(
	function* (reader: QueryReader<typeof Learning>, student: Id128, work: ExecutionPolicy) {
		const result = yield* reader.execute(attemptsFor, { student }, work)
		return yield* result.collect({ maxBytes: work.resultBytes }, work)
	},
	Effect.scoped
)

export const runtimePolicy: NativeRuntimeOptions = {
	workers: 2,
	queueCapacity: 16,
	cleanupCapacity: 16,
	ownerCapacity: 16,
	nativeHandleCapacity: 64,
	inputBytes: 16_000_000n,
	workingBytes: 64_000_000n,
	scratchBytes: 64_000_000n,
	resultBytes: 16_000_000n,
	chunkBytes: 1_000_000n,
	cleanupTimeout: "2 seconds"
}

export const work: ExecutionPolicy = {
	inputBytes: 4_000_000n,
	workingBytes: 16_000_000n,
	scratchBytes: 16_000_000n,
	resultBytes: 4_000_000n,
	rows: 100_000n,
	workUnits: 10_000_000n,
	timeout: "10 seconds"
}

/** D07: a delivery budget that cannot pay for a completed attempt row. */
export const tinyDelivery: ExecutionPolicy = {
	...work,
	resultBytes: 8n,
	timeout: "2 seconds"
}

/** One process-lifetime runtime. Request code must not construct another. */
export const makeConsumerRuntime = () => ManagedRuntime.make(NativeRuntime.layer(runtimePolicy))

export const coreProgram = (localPath: string) =>
	Effect.scoped(
		Effect.gen(function* () {
			const db = yield* Db.create(localPath, Learning, work)
			const studentId = yield* Id128.random()
			const attemptId = yield* Id128.random()
			const changes = yield* newAttempt(studentId, attemptId, work)
			const outcome = yield* db.apply(changes, { ...work, expected: { kind: "any" } })
			if (outcome.kind !== "accepted" && outcome.kind !== "no-change") {
				const closed = yield* db.close()
				return { outcome, rows: [] as const, closed }
			}
			const snapshot = yield* db.snapshot(work)
			const rows = yield* readAttempts(snapshot, studentId, work)
			const closed = yield* db.close()
			return { outcome, rows, closed }
		})
	)

export const correctScore = (localPath: string, attemptId: Id128) =>
	Effect.scoped(
		Effect.gen(function* () {
			const db = yield* Db.open(localPath, Learning, work)
			const observed = yield* Effect.scoped(
				Effect.gen(function* () {
					const snapshot = yield* db.snapshot(work)
					const previous = yield* snapshot.get(Attempt, { id: attemptId }, work)
					if (Option.isNone(previous)) {
						return yield* Effect.fail({ missing: attemptId })
					}
					return { previous: previous.value, at: snapshot.witness }
				})
			)
			const draft = yield* ChangeSet.builder(Learning, work)
			yield* draft.delete(Attempt, [observed.previous])
			yield* draft.insert(Attempt, [{ ...observed.previous, score: 0.95 }])
			const changes = yield* draft.finish()
			const outcome = yield* db.apply(changes, { ...work, expected: { kind: "exact", at: observed.at } })
			const closed = yield* db.close()
			return { outcome, closed }
		})
	)

export const drainPages = (reader: QueryReader<typeof Learning>, student: Id128, delivery: ExecutionPolicy) =>
	Effect.scoped(
		Effect.gen(function* () {
			const result = yield* reader.execute(attemptsFor, { student }, work)
			return yield* result.pages({ pageBytes: 65_536n }, delivery).pipe(
				Stream.runFold(0, (rows, page) => rows + page.length)
			)
		})
	)

/** D07: collect under a result-bytes cap that a real row cannot fit. */
export const collectUnderTinyBudget = (
	reader: QueryReader<typeof Learning>,
	student: Id128
) =>
	Effect.scoped(
		Effect.gen(function* () {
			const result = yield* reader.execute(attemptsFor, { student }, work)
			return yield* result.collect({ maxBytes: tinyDelivery.resultBytes }, tinyDelivery)
		})
	)
