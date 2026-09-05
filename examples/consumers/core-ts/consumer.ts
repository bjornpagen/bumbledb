/**
 * Chapter 34 core-TypeScript consumer fixture (API-12 / PKG-03): the
 * shared `Learning` schema, reusable typed queries including composition,
 * one scoped ChangeSet helper, the shared QueryReader read helper, direct
 * local admission and a witnessed correction — compiled by a strict
 * downstream tsc against the STAGED `@bjornpagen/bumbledb` tarball and
 * the exact Effect 4.0.0-rc.112 peer. Everything here is a lazy Effect
 * description; importing this module performs no native work.
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
	schema,
	span,
	str,
	u64,
	v,
	weigh,
	within
} from "@bjornpagen/bumbledb"
import { Effect, Option, Stream } from "effect"

// ── The same schema in both languages (chapter 34) ──────────────────────────

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

// ── Queries are reusable typed values, including their intermediate results ─

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

// ── One TypeScript change, usable by either product ─────────────────────────

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

// ── One Effect read helper for core and log ─────────────────────────────────

export const readAttempts = Effect.fn("readAttempts")(
	function* (reader: QueryReader<typeof Learning>, student: Id128, work: ExecutionPolicy) {
		const result = yield* reader.execute(attemptsFor, { student }, work)
		return yield* result.collect({ maxBytes: work.resultBytes })
	},
	Effect.scoped
)

// ── Core: direct local admission, no receipt ceremony ───────────────────────

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

export const coreProgram = (localPath: string) =>
	Effect.scoped(
		Effect.gen(function* () {
			const db = yield* Db.create(localPath, Learning, work)
			const studentId = yield* Id128.random()
			const attemptId = yield* Id128.random()
			const changes = yield* newAttempt(studentId, attemptId, work)
			const outcome = yield* db.apply(changes, { ...work, expected: { kind: "any" } })
			if (outcome.kind !== "accepted" && outcome.kind !== "no-change") {
				return { outcome, rows: [] as const }
			}
			const snapshot = yield* db.snapshot(work)
			return { outcome, rows: yield* readAttempts(snapshot, studentId, work) }
		})
	).pipe(Effect.provide(NativeRuntime.layer(runtimePolicy)))

// ── Witnessed correction: keep the read scope short ─────────────────────────

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
			return yield* db.apply(changes, { ...work, expected: { kind: "exact", at: observed.at } })
		})
	).pipe(Effect.provide(NativeRuntime.layer(runtimePolicy)))

// ── Large completed answers stream owned pages after complete evaluation ────

export const drainPages = (reader: QueryReader<typeof Learning>, student: Id128) =>
	Effect.scoped(
		Effect.gen(function* () {
			const result = yield* reader.execute(attemptsFor, { student }, work)
			return yield* result.pages({ pageBytes: 65_536n }).pipe(
				Stream.runFold(0, (rows, page) => rows + page.length)
			)
		})
	)
