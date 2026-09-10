import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, Exit, Fiber, ManagedRuntime, Option, Result } from "effect"
import { ChangeSet } from "#changes.ts"
import type { WriteOptions } from "#db.ts"
import { Db } from "#db.ts"
import { dbNative } from "#db-native.ts"
import { str, u64 } from "#fields.ts"
import { relation } from "#relation.ts"
import { NativeRuntime } from "#runtime.ts"
import { schema } from "#schema.ts"
import { key } from "#statements.ts"
import { Attempt, Learning, runtimeOptions, Student, storeDir } from "#test/fixtures/learning.ts"

const Item = relation("Item", { id: u64, text: str })
const ItemById = key(Item, ["id"])
const Theory = schema("Judgment", { Item }, [ItemById])
const any: WriteOptions = { expected: { kind: "any" } }

const delta = (add: readonly { id: bigint; text: string }[], remove: readonly { id: bigint; text: string }[] = []) =>
	Effect.gen(function* () {
		const draft = yield* ChangeSet.builder(Theory)
		yield* draft.insert(Item, add)
		yield* draft.delete(Item, remove)
		return yield* draft.finish()
	})

test("judge and apply share final-state admission, including net counts and add-wins", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.create(storeDir("judge-counts"), Theory)
					const before = yield* db.snapshot()
					const exact: WriteOptions = { expected: { kind: "exact", at: before.witness } }
					const a = { id: 1n, text: "a" }
					const b = { id: 2n, text: "b" }
					const absent = { id: 99n, text: "absent" }
					const changes = yield* delta([a, b, a], [a, absent])
					const original = dbNative.runtimeDbJudge
					let submitted = 0
					dbNative.runtimeDbJudge = (...args) => {
						submitted += 1
						return original(...args)
					}
					const operation = db.judge(changes, exact)
					assert.ok(Effect.isEffect(operation))
					assert.equal(submitted, 0, "construction is inert")
					dbNative.runtimeDbJudge = original
					const judgment = yield* operation
					assert.deepEqual(judgment, { kind: "admitted", base: before.witness, changes: { added: 2n, removed: 0n } })
					assert.equal((yield* db.inspect()).generation, before.witness.generation)
					assert.ok(Option.isNone(yield* (yield* db.snapshot()).get(ItemById, { id: 1n })))
					assert.equal((yield* db.apply(changes, exact)).kind, "accepted")
					const landed = yield* db.snapshot()
					assert.deepEqual(Option.getOrThrow(yield* landed.get(ItemById, { id: 1n })), a)
					assert.ok(Option.isNone(yield* before.get(ItemById, { id: 1n })), "the old snapshot is still coherent")
					assert.deepEqual(yield* db.judge(changes, any), {
						kind: "admitted",
						base: landed.witness,
						changes: { added: 0n, removed: 0n }
					})
					const replacement = { id: 1n, text: "replacement" }
					const replace = yield* delta([replacement, replacement], [a, absent])
					assert.deepEqual(yield* db.judge(replace, any), {
						kind: "admitted",
						base: landed.witness,
						changes: { added: 1n, removed: 1n }
					})
					const conflict = yield* delta([replacement, replacement], [b, absent])
					const rejected = yield* db.judge(conflict, any)
					assert.equal(rejected.kind, "invariant-rejected")
					if (rejected.kind !== "invariant-rejected") return
					assert.deepEqual(rejected.base, landed.witness)
					assert.deepEqual(rejected.changes, { added: 1n, removed: 1n })
					assert.ok(rejected.violations.length > 0)
					const applied = yield* db.apply(conflict, any)
					assert.equal(applied.kind, "invariant-rejected")
					if (applied.kind === "invariant-rejected") assert.deepEqual(applied.violations, rejected.violations)
					assert.equal((yield* db.inspect()).generation, landed.witness.generation)
					assert.deepEqual(Option.getOrThrow(yield* (yield* db.snapshot()).get(ItemById, { id: 2n })), b)
					assert.equal((yield* db.apply(replace, { expected: { kind: "exact", at: landed.witness } })).kind, "accepted")
					assert.deepEqual(Option.getOrThrow(yield* (yield* db.snapshot()).get(ItemById, { id: 1n })), replacement)
				})
			)
		)
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
	}
})

test("moved is unjudged; foreign witnesses, schemas, and malformed intent are operational errors", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.create(storeDir("judge-moved"), Theory)
					const before = yield* db.snapshot()
					const changes = yield* delta([{ id: 1n, text: "one" }])
					yield* db.apply(changes, any)
					const current = yield* db.snapshot()
					const stale = { expected: { kind: "exact", at: before.witness } } satisfies WriteOptions
					const moved = { kind: "moved", witnessed: before.witness, current: current.witness }
					assert.deepEqual(yield* db.judge(changes, stale), moved)
					assert.deepEqual(yield* db.apply(changes, stale), moved)
					const other = yield* Db.create(storeDir("judge-foreign"), Theory)
					const foreign = yield* other.snapshot()
					const badOptions: readonly unknown[] = [
						{},
						null,
						{ expected: null },
						{ expected: { kind: "latest" } },
						{ expected: { kind: "any", extra: true } },
						{ ...any, extra: true },
						{ expected: { kind: "exact", at: { ...current.witness, generation: -1n } } },
						{ expected: { kind: "exact", at: { ...current.witness, generation: 2n ** 64n } } },
						{ expected: { kind: "exact", at: { ...current.witness, extra: true } } },
						{ expected: { kind: "exact", at: Object.create(current.witness) } },
						{ expected: { kind: "exact", at: foreign.witness } }
					]
					for (const options of badOptions) {
						assert.ok(Result.isFailure(yield* Effect.result(db.judge(changes, options as never))))
						assert.ok(Result.isFailure(yield* Effect.result(db.apply(changes, options as never))))
					}
					const foreignDraft = yield* ChangeSet.builder(Learning)
					const foreignChanges = yield* foreignDraft.finish()
					assert.ok(Result.isFailure(yield* Effect.result(db.judge(foreignChanges as never, any))))
					assert.ok(Result.isFailure(yield* Effect.result(db.judge({} as never, any))))
					assert.equal((yield* db.inspect()).generation, current.witness.generation)
				})
			)
		)
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
	}
})

test("containment and capacity rejection agree with apply and preserve the base", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.create(storeDir("judge-constraints"), Learning)
					const snapshot = yield* db.snapshot()
					const id = "00000000-0000-0000-0000-000000000001"
					for (const includeStudent of [false, true]) {
						const draft = yield* ChangeSet.builder(Learning)
						if (includeStudent) yield* draft.insert(Student, [{ id, name: "synthetic", budget: 1n }])
						yield* draft.insert(Attempt, [{ id, student: id, score: 0.5, units: 2n, active: { start: 0n, end: 1n } }])
						const changes = yield* draft.finish()
						const judged = yield* db.judge(changes, any)
						assert.equal(judged.kind, "invariant-rejected")
						const applied = yield* db.apply(changes, any)
						assert.equal(applied.kind, "invariant-rejected")
						if (judged.kind !== "invariant-rejected" || applied.kind !== "invariant-rejected") continue
						assert.deepEqual(judged.violations, applied.violations)
						assert.deepEqual(judged.changes, { added: includeStudent ? 2n : 1n, removed: 0n })
						assert.deepEqual(judged.base, snapshot.witness)
						assert.equal((yield* db.inspect()).generation, snapshot.witness.generation)
					}
				})
			)
		)
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
	}
})

test("cancelled judgment delivery releases the writer and retains no mutation", { timeout: 10000 }, async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	const original = dbNative.runtimeDbJudge
	try {
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.create(storeDir("judge-cancel"), Theory)
					const before = yield* db.snapshot()
					const changes = yield* delta([{ id: 1n, text: "one" }])
					const completed = Promise.withResolvers<() => void>()
					dbNative.runtimeDbJudge = (db, changes, expected, callback) =>
						original(db, changes, expected, () => completed.resolve(callback))
					const fiber = yield* Effect.forkChild(db.judge(changes, any))
					const lateCallback = yield* Effect.promise(() => completed.promise)
					yield* Fiber.interrupt(fiber)
					assert.ok(Exit.hasInterrupts(yield* Fiber.await(fiber)))
					lateCallback()
					dbNative.runtimeDbJudge = original
					assert.equal((yield* db.inspect()).generation, before.witness.generation)
					assert.deepEqual(yield* db.judge(changes, any), {
						kind: "admitted",
						base: before.witness,
						changes: { added: 1n, removed: 0n }
					})
					assert.equal((yield* db.apply(changes, { expected: { kind: "exact", at: before.witness } })).kind, "accepted")
					yield* changes.close()
					assert.ok(Result.isFailure(yield* Effect.result(db.judge(changes, any))))
				})
			)
		)
	} finally {
		dbNative.runtimeDbJudge = original
		await Effect.runPromise(runtime.disposeEffect)
	}
})
