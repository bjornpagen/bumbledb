import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, Exit, Fiber, ManagedRuntime, Option, Result, Scope } from "effect"
import { ChangeSet } from "../src/changes.ts"
import type { Witness } from "../src/db.ts"
import { Db } from "../src/db.ts"
import { str, u64 } from "../src/fields.ts"
import { addon } from "../src/native/addon.ts"
import { relation } from "../src/relation.ts"
import { Bumble } from "../src/runtime.ts"
import { schema } from "../src/schema.ts"
import { key } from "../src/statements.ts"
import { Attempt, Learning, runtimeOptions, Student, storeDir } from "./fixtures/learning.ts"

const Item = relation("Item", { id: u64, text: str })
const ItemById = key(Item, ["id"])
const Theory = schema("Judgment", { Item }, [ItemById])

const delta = (add: readonly { id: bigint; text: string }[], remove: readonly { id: bigint; text: string }[] = []) =>
	Effect.gen(function* () {
		const draft = yield* ChangeSet.builder(Theory)
		yield* draft.insert(Item, add)
		yield* draft.delete(Item, remove)
		return yield* draft.finish()
	})

test("judge and apply share final-state admission, including net counts and add-wins", async () => {
	const runtime = ManagedRuntime.make(Bumble.layer(runtimeOptions))
	try {
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.create(storeDir("judge-counts"), Theory)
					const before = yield* db.snapshot()
					const exact: Witness = before.witness
					const a = { id: 1n, text: "a" }
					const b = { id: 2n, text: "b" }
					const absent = { id: 99n, text: "absent" }
					const changes = yield* delta([a, b, a], [a, absent])
					const original = addon.runtimeDbJudge
					let submitted = 0
					addon.runtimeDbJudge = (...args) => {
						submitted += 1
						return original(...args)
					}
					const operation = db.judge(changes, exact)
					assert.ok(Effect.isEffect(operation))
					assert.equal(submitted, 0, "construction is inert")
					addon.runtimeDbJudge = original
					const judgment = yield* operation
					assert.deepEqual(judgment, { _tag: "Admitted", base: before.witness, changes: { added: 2n, removed: 0n } })
					assert.equal((yield* db.inspect()).generation, before.witness.generation)
					assert.ok(Option.isNone(yield* (yield* db.snapshot()).get(ItemById, { id: 1n })))
					assert.equal((yield* db.apply(changes, exact))._tag, "Committed")
					const landed = yield* db.snapshot()
					assert.deepEqual(Option.getOrThrow(yield* landed.get(ItemById, { id: 1n })), a)
					assert.ok(Option.isNone(yield* before.get(ItemById, { id: 1n })), "the old snapshot is still coherent")
					assert.deepEqual(yield* db.judge(changes), {
						_tag: "Admitted",
						base: landed.witness,
						changes: { added: 0n, removed: 0n }
					})
					const replacement = { id: 1n, text: "replacement" }
					const replace = yield* delta([replacement, replacement], [a, absent])
					assert.deepEqual(yield* db.judge(replace), {
						_tag: "Admitted",
						base: landed.witness,
						changes: { added: 1n, removed: 1n }
					})
					const conflict = yield* delta([replacement, replacement], [b, absent])
					const rejected = yield* db.judge(conflict)
					assert.equal(rejected._tag, "Rejected")
					if (rejected._tag !== "Rejected") return
					assert.deepEqual(rejected.base, landed.witness)
					assert.deepEqual(rejected.changes, { added: 1n, removed: 1n })
					assert.ok(rejected.violations.length > 0)
					const applied = yield* db.apply(conflict)
					assert.equal(applied._tag, "Rejected")
					if (applied._tag === "Rejected") assert.deepEqual(applied.violations, rejected.violations)
					assert.equal((yield* db.inspect()).generation, landed.witness.generation)
					assert.deepEqual(Option.getOrThrow(yield* (yield* db.snapshot()).get(ItemById, { id: 2n })), b)
					assert.equal((yield* db.apply(replace, landed.witness))._tag, "Committed")
					assert.deepEqual(Option.getOrThrow(yield* (yield* db.snapshot()).get(ItemById, { id: 1n })), replacement)
				})
			)
		)
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
	}
})

test("moved is unjudged; foreign witnesses, schemas, and malformed intent are operational errors", async () => {
	const runtime = ManagedRuntime.make(Bumble.layer(runtimeOptions))
	try {
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.create(storeDir("judge-moved"), Theory)
					const before = yield* db.snapshot()
					const changes = yield* delta([{ id: 1n, text: "one" }])
					yield* db.apply(changes)
					const current = yield* db.snapshot()
					const stale = before.witness
					const moved = { _tag: "Moved", witnessed: before.witness, current: current.witness }
					assert.deepEqual(yield* db.judge(changes, stale), moved)
					assert.deepEqual(yield* db.apply(changes, stale), moved)
					const badOptions: readonly unknown[] = [
						{},
						{ store: current.witness.store },
						{ generation: current.witness.generation },
						{ ...current.witness, generation: -1n },
						{ ...current.witness, generation: 2n ** 64n }
					]
					for (const options of badOptions) {
						assert.ok(Result.isFailure(yield* Effect.result(db.judge(changes, options as never))))
						assert.ok(Result.isFailure(yield* Effect.result(db.apply(changes, options as never))))
					}
					const foreignDraft = yield* ChangeSet.builder(Learning)
					const foreignChanges = yield* foreignDraft.finish()
					assert.ok(Result.isFailure(yield* Effect.result(db.judge(foreignChanges as never))))
					assert.ok(Result.isFailure(yield* Effect.result(db.judge({} as never))))
					assert.equal((yield* db.inspect()).generation, current.witness.generation)
				})
			)
		)
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
	}
})

test("containment and capacity rejection agree with apply and preserve the base", async () => {
	const runtime = ManagedRuntime.make(Bumble.layer(runtimeOptions))
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
						const judged = yield* db.judge(changes)
						assert.equal(judged._tag, "Rejected")
						const applied = yield* db.apply(changes)
						assert.equal(applied._tag, "Rejected")
						if (judged._tag !== "Rejected" || applied._tag !== "Rejected") continue
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
	const runtime = ManagedRuntime.make(Bumble.layer(runtimeOptions))
	const original = addon.runtimeDbJudge
	try {
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.create(storeDir("judge-cancel"), Theory)
					const before = yield* db.snapshot()
					const changesScope = yield* Scope.make()
					const changes = yield* delta([{ id: 1n, text: "one" }]).pipe(Scope.provide(changesScope))
					const completed = Promise.withResolvers<() => void>()
					addon.runtimeDbJudge = (db, changes, expected, callback) =>
						original(db, changes, expected, () => completed.resolve(callback))
					const fiber = yield* Effect.forkChild(db.judge(changes))
					const lateCallback = yield* Effect.promise(() => completed.promise)
					yield* Fiber.interrupt(fiber)
					assert.ok(Exit.hasInterrupts(yield* Fiber.await(fiber)))
					lateCallback()
					addon.runtimeDbJudge = original
					assert.equal((yield* db.inspect()).generation, before.witness.generation)
					assert.deepEqual(yield* db.judge(changes), {
						_tag: "Admitted",
						base: before.witness,
						changes: { added: 1n, removed: 0n }
					})
					assert.equal((yield* db.apply(changes, before.witness))._tag, "Committed")
					yield* Scope.close(changesScope, Exit.void)
					assert.ok(Result.isFailure(yield* Effect.result(db.judge(changes))))
				})
			)
		)
	} finally {
		addon.runtimeDbJudge = original
		await Effect.runPromise(runtime.disposeEffect)
	}
})
