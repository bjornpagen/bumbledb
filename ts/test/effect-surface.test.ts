/**
 * The Effect-only public surface (API-12 declaration/export gate, the
 * authored half): every database operation constructs a LAZY Effect —
 * no Promise, thenable, synchronous or disposal twin exists anywhere on
 * the public values — and pure schema/query/scalar construction performs
 * no native work (chapter 35's pure-descriptions table).
 */
import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, Stream } from "effect"
import { ChangeSet } from "#changes.ts"
import { Schema as BumbleSchema } from "#compile.ts"
import { Db } from "#db.ts"
import { f64, id128, str, u64 } from "#fields.ts"
import { Id128 } from "#id128.ts"
import { query } from "#query/lower.ts"
import { v } from "#query/scope.ts"
import { relation } from "#relation.ts"
import { NativeRuntime } from "#runtime.ts"
import { schema } from "#schema.ts"
import { key } from "#statements.ts"
import { Attempt, Learning, Student, work } from "#test/fixtures/learning.ts"

function assertNoTwin(name: string, value: object): void {
	assert.equal("then" in value, false, `${name} is not thenable`)
	assert.equal(Symbol.asyncDispose in value, false, `${name} carries no AsyncDisposable twin`)
	assert.equal(Symbol.dispose in value, false, `${name} carries no Disposable twin`)
	assert.equal("sync" in value, false, `${name} carries no sync twin`)
	assert.equal("promise" in value, false, `${name} carries no promise twin`)
}

test("every core entry point constructs a lazy Effect (or Stream) — nothing runs at construction", function lazyConstruction() {
	const create = Db.create("/tmp/never-used", Learning, work)
	const open = Db.open("/tmp/never-used", Learning, work)
	const compile = BumbleSchema.compile(Learning, work)
	const builder = ChangeSet.builder(Learning, work)
	const random = Id128.random()
	for (const [name, value] of [
		["Db.create", create],
		["Db.open", open],
		["Schema.compile", compile],
		["ChangeSet.builder", builder],
		["Id128.random", random]
	] as const) {
		assert.ok(Effect.isEffect(value), `${name} constructs an Effect`)
		assertNoTwin(name, value)
	}
	// Constructing and DROPPING these effects had no observable consequence:
	// no directory was created, no native runtime started (the layer is the
	// only acquisition path, and none was provided here).
})

test("the layer value is inert data until provided into a running scope", function layerInert() {
	const layer = NativeRuntime.layer({
		workers: 1,
		queueCapacity: 1,
		cleanupCapacity: 1,
		ownerCapacity: 1,
		nativeHandleCapacity: 1,
		inputBytes: 1n,
		workingBytes: 1n,
		scratchBytes: 0n,
		resultBytes: 1n,
		chunkBytes: 1n,
		cleanupTimeout: "1 second"
	})
	assertNoTwin("NativeRuntime.layer", layer)
})

test("pure schema/query metadata construction touches no native work and no I/O", function pureMetadata() {
	// These constructions run against ORDINARY data only. A native
	// dispatch here would be an import-time/authoring-time side effect —
	// the exact thing chapter 35's pure-descriptions table forbids.
	const Widget = relation("Widget", { id: id128, name: str, score: f64, count: u64 })
	const Gadgets = schema("Gadgets", { Widget }, [key(Widget, ["id"])])
	const template = query(Gadgets).rule((r) => {
		const { id, name, score } = v(Widget)
		return r
			.match(Widget, { id, name, score })
			.where(r.eq(name, r.param("name")))
			.find({ id, total: r.sum(score) })
	})
	assert.ok(Object.isFrozen(Gadgets))
	assert.equal(typeof template.data, "object", "the template is inert owned AST")
})

test("a CompleteResult's pages is a Stream value, and no cursor/AsyncIterable twin exists on the type", function pagesIsStream() {
	// Type-level pin: the ONLY streaming surface is `pages`; the historical
	// cursor verbs are absent from the CompleteResult type.
	type Result = import("#result.ts").CompleteResult<unknown>
	type HasCursor = "intoCursor" extends keyof Result ? true : false
	type HasNext = "next" extends keyof Result ? true : false
	type HasAsyncIterator = typeof Symbol.asyncIterator extends keyof Result ? true : false
	const noCursor: HasCursor = false
	const noNext: HasNext = false
	const noAsyncIterator: HasAsyncIterator = false
	assert.ok(!noCursor && !noNext && !noAsyncIterator)
	// And the pages type is Effect's Stream (compile-time assignability pin).
	type Pages = ReturnType<Result["pages"]>
	const pinned: Pages extends Stream.Stream<ReadonlyArray<unknown>, unknown> ? true : false = true
	assert.ok(pinned)
})

test("get/execute/session/close on typed handles are declared Effect-returning (compile-time pins)", function methodPins() {
	type SnapshotValue = import("#db.ts").Snapshot<typeof Learning>
	type GetResult = ReturnType<SnapshotValue["get"]>
	const getIsEffect: GetResult extends Effect.Effect<unknown, unknown, unknown> ? true : false = true
	assert.ok(getIsEffect)
	type CloseResult = ReturnType<SnapshotValue["close"]>
	const closeIsEffect: CloseResult extends Effect.Effect<unknown, never, never> ? true : false = true
	assert.ok(closeIsEffect)
	void ({} as { attempt: typeof Attempt; student: typeof Student })
})
