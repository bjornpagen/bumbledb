import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, ManagedRuntime, Result } from "effect"
import {
	ChangeSet,
	contained,
	Db,
	decodeRows,
	Event,
	encodeRows,
	event,
	key,
	NativeRuntime,
	on,
	relation,
	renderStatement,
	rowShape,
	Schema,
	schema,
	select,
	u64
} from "#index.ts"
import { lower } from "#lower.ts"
import { nativeBindingIsLoaded } from "#native.ts"
import { runtimeOptions, storeDir } from "#test/fixtures/learning.ts"

const Child = relation("Child", { group: u64, filter: event })
const Parent = relation("Parent", { group: u64, filter: event })
const theory = (a: Event, b?: Event) =>
	schema("Selected", { Child, Parent }, [
		key(Child, ["group"]),
		key(Parent, ["group"]),
		contained(
			on(select(Child, { filter: b === undefined ? a : [a, b] }), "group"),
			on(select(Parent, { filter: b === undefined ? a : [a, b] }), "group")
		)
	])

// A deliberately incomplete carrier is sufficient for pure authoring; the
// worker must reject its mathematical content when the schema is compiled.
const carrier = () => Result.getOrThrow(Event.fromBytes(Buffer.from("BEVT\x01")))

test("Event schema selections own transport bytes without loading native code", () => {
	assert.equal(nativeBindingIsLoaded(), false)
	const value = carrier()
	const selected = select(Child, { filter: value })
	const law = contained(on(selected, "group"), on(Parent, "group"))
	assert.match(renderStatement(law), /event:4245565401/)
	Event.toBytes(value).fill(0)
	assert.deepEqual(lower(theory(value)).statements[2]?.kind, "containment")
	assert.throws(() => select(Child, { filter: [value, carrier()] }), /twice|duplicate/)
	assert.equal(nativeBindingIsLoaded(), false)
})

test("Event selections enforce exact matches across incremental writes, owner release and reopen", async () => {
	const path = storeDir("event-selections")
	let runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	let saved: Uint8Array | undefined
	try {
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const full = yield* Event.space(new Uint8Array(32).fill(116), 2n)
					const a = yield* Event.coordinate(full, 0n)
					const b = yield* Event.coordinate(full, 1n)
					saved = Event.toBytes(a)
					const copy = Result.getOrThrow(Event.fromBytes(Event.toBytes(a)))
					const compiled = yield* Schema.compile(theory(a, b))
					assert.equal((yield* Schema.compile(theory(b, copy))).schemaId, compiled.schemaId)
					assert.notEqual((yield* Schema.compile(theory(full, b))).schemaId, compiled.schemaId)
					assert.equal((yield* Effect.flip(Schema.compile(theory(carrier())))).reason._tag, "Engine")
					const t = theory(a)
					const db = yield* Db.create(path, t)
					const orphan = yield* ChangeSet.builder(t)
					yield* orphan.insert(Child, [{ group: 1n, filter: copy }])
					assert.equal(
						(yield* db.apply(yield* orphan.finish(), { expected: { kind: "any" } })).kind,
						"invariant-rejected"
					)
					const seed = yield* ChangeSet.builder(t)
					yield* seed.insert(Child, [
						{ group: 1n, filter: copy },
						{ group: 2n, filter: full }
					])
					yield* seed.insert(Parent, [{ group: 1n, filter: a }])
					const sealed = yield* seed.finish()
					const replayed = yield* ChangeSet.fromBytes(t, yield* sealed.toBytes())
					assert.equal((yield* db.apply(replayed, { expected: { kind: "any" } })).kind, "accepted")
					const shape = rowShape(t, Child)
					const rowBytes = yield* encodeRows(shape, [{ group: 9n, filter: a }])
					const decoded = yield* decodeRows(shape, rowBytes)
					assert.equal(decoded[0]?.group, 9n)
					assert.ok(decoded[0])
					assert.deepEqual(Event.toBytes(decoded[0].filter), Event.toBytes(a))
					// Replacing a target with an overlapping Event removes its exact
					// witness. Both scalar-index delta tracking and selected citations
					// must see this, although the determinant stays the same.
					const replace = yield* ChangeSet.builder(t)
					yield* replace.delete(Parent, [{ group: 1n, filter: copy }])
					yield* replace.insert(Parent, [{ group: 1n, filter: full }])
					assert.equal(
						(yield* db.apply(yield* replace.finish(), { expected: { kind: "any" } })).kind,
						"invariant-rejected"
					)
				})
			)
		)
		await runtime.dispose()
		runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
		assert.ok(saved)
		const literal = Result.getOrThrow(Event.fromBytes(saved))
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const t = theory(literal)
					const db = yield* Db.open(path, t)
					const change = yield* ChangeSet.builder(t)
					yield* change.delete(Parent, [{ group: 1n, filter: literal }])
					assert.equal(
						(yield* db.apply(yield* change.finish(), { expected: { kind: "any" } })).kind,
						"invariant-rejected"
					)
					const repair = yield* ChangeSet.builder(t)
					yield* repair.delete(Parent, [{ group: 1n, filter: literal }])
					yield* repair.delete(Child, [{ group: 1n, filter: literal }])
					assert.equal((yield* db.apply(yield* repair.finish(), { expected: { kind: "any" } })).kind, "accepted")
				})
			)
		)
	} finally {
		await runtime.dispose()
	}
})
