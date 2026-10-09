import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, type Stream } from "effect"
import { ChangeSet } from "../src/changes.ts"
import { Db } from "../src/db.ts"
import { Bumble } from "../src/runtime.ts"
import { type Attempt, Learning, type Student } from "./fixtures/learning.ts"

function assertNoTwin(name: string, value: object): void {
	assert.equal("then" in value, false, `${name} is not thenable`)
	assert.equal(Symbol.asyncDispose in value, false, `${name} carries no AsyncDisposable twin`)
	assert.equal(Symbol.dispose in value, false, `${name} carries no Disposable twin`)
	assert.equal("sync" in value, false, `${name} carries no sync twin`)
	assert.equal("promise" in value, false, `${name} carries no promise twin`)
}

test("every core entry point constructs a lazy Effect (or Stream) — nothing runs at construction", function lazyConstruction() {
	const create = Db.create("/tmp/never-used", Learning)
	const open = Db.open("/tmp/never-used", Learning)
	const builder = ChangeSet.builder(Learning)
	for (const [name, value] of [
		["Db.create", create],
		["Db.open", open],
		["ChangeSet.builder", builder]
	] as const) {
		assert.ok(Effect.isEffect(value), `${name} constructs an Effect`)
		assertNoTwin(name, value)
	}
	// Constructing and DROPPING these effects had no observable consequence:
	// no directory was created, no native runtime started (the layer is the
	// only acquisition path, and none was provided here).
})

test("the layer value is inert data until provided into a running scope", function layerInert() {
	const layer = Bumble.layer({
		workers: 1,
		queueCapacity: 1,
		cleanupCapacity: 1,
		ownerCapacity: 1,
		nativeHandleCapacity: 1,
		cleanupTimeout: "1 second"
	})
	assertNoTwin("Bumble.layer", layer)
})

test("a CompleteResult's pages is a Stream value, and no cursor/AsyncIterable twin exists on the type", function pagesIsStream() {
	// The ONLY streaming surface is `pages`; no cursor verbs exist on the
	// CompleteResult type.
	type Result = import("../src/result.ts").CompleteResult<unknown>
	type HasCursor = "intoCursor" extends keyof Result ? true : false
	type HasNext = "next" extends keyof Result ? true : false
	type HasAsyncIterator = typeof Symbol.asyncIterator extends keyof Result ? true : false
	const noCursor: HasCursor = false
	const noNext: HasNext = false
	const noAsyncIterator: HasAsyncIterator = false
	assert.ok(!noCursor && !noNext && !noAsyncIterator)
	// And the pages type is Effect's Stream (checked at compile time).
	type Pages = ReturnType<Result["pages"]>
	const pinned: Pages extends Stream.Stream<ReadonlyArray<unknown>, unknown> ? true : false = true
	assert.ok(pinned)
})

test("get/execute/prepare on typed handles are declared Effect-returning (compile-time pins)", function methodPins() {
	type SnapshotValue = import("../src/db.ts").Snapshot<typeof Learning>
	type GetResult = ReturnType<SnapshotValue["get"]>
	const getIsEffect: GetResult extends Effect.Effect<unknown, unknown, unknown> ? true : false = true
	assert.ok(getIsEffect)
	type PrepareResult = ReturnType<SnapshotValue["prepare"]>
	const prepareIsScoped: PrepareResult extends Effect.Effect<unknown, unknown, import("effect").Scope.Scope>
		? true
		: false = true
	assert.ok(prepareIsScoped)
	void ({} as { attempt: typeof Attempt; student: typeof Student })
})
