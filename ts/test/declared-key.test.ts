import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, ManagedRuntime, Option, Result } from "effect"
import { ChangeSet } from "#changes.ts"
import { closed } from "#closed.ts"
import { declaredKey } from "#compile.ts"
import type { QueryReader } from "#db.ts"
import { Db } from "#db.ts"
import { on } from "#face.ts"
import { i64, interval, str, u64 } from "#fields.ts"
import type { Fact } from "#relation.ts"
import { relation } from "#relation.ts"
import { NativeRuntime } from "#runtime.ts"
import { schema } from "#schema.ts"
import type { Key } from "#shape.ts"
import { key, mirrors } from "#statements.ts"
import { runtimeOptions, storeDir } from "#test/fixtures/learning.ts"

const Flag = closed("Flag", ["On", "Off"])
const Item = relation("Item", { id: u64, tenant: u64, code: str, description: str })
const Detail = relation("Detail", { item: u64 })
const Booking = relation("Booking", { room: u64, during: interval(i64), label: str })
const ItemById = key(Item, ["id"])
// Deliberately not field-declaration order: native lookup uses this projection.
const ItemByCode = key(Item, ["code", "tenant"])
const DetailByItem = key(Detail, ["item"])
const BookingByRoomAndTime = key(Booking, ["room", "during"])
const Theory = schema("DeclaredKeys", { Flag, Item, Detail, Booking }, [
	ItemById,
	DetailByItem,
	mirrors(on(Item, "id"), on(Detail, "item")),
	ItemByCode,
	BookingByRoomAndTime
])

type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2 ? true : false
type Expect<T extends true> = T
type KeyPin = Expect<Equal<Key<typeof ItemByCode>, { code: string; tenant: bigint }>>

function checkTypes(reader: QueryReader<typeof Theory>) {
	const byId = reader.get(ItemById, { id: 1n })
	const byCode = reader.get(ItemByCode, { tenant: 2n, code: "A" })
	const keyPin: KeyPin = true
	const factPin: Expect<Equal<Effect.Success<typeof byCode>, Option.Option<Fact<typeof Item>>>> = true
	// @ts-expect-error Every selected field is required.
	reader.get(ItemByCode, { code: "A" })
	// @ts-expect-error A key's value cannot select some other declared key.
	reader.get(ItemByCode, { id: 1n })
	// @ts-expect-error The key argument cannot widen the declaration's inferred fields.
	reader.get(ItemById, { id: 1n, description: "extra" })
	// @ts-expect-error Field scalar types come from the relation.
	reader.get(ItemById, { id: "1" })
	// @ts-expect-error A relation is not a declared key.
	reader.get(Item, { id: 1n })
	return { byId, byCode, keyPin, factPin }
}

test("key values infer exactly their declared projection", () => {
	assert.equal(typeof checkTypes, "function")
	assert.equal(declaredKey(Theory, ItemById)?.statementId, 1)
	assert.equal(declaredKey(Theory, ItemByCode)?.statementId, 5)
	assert.equal(declaredKey(Theory, BookingByRoomAndTime)?.statementId, 6)
	assert.throws(() => key(Item, [] as never), /nonempty/)
	assert.throws(() => key(Item, ["missing"] as never), /unknown field/)
	assert.throws(() => key(Item, ["id", "id"]), /twice/)
})

test("a key owns its projection without freezing or retaining the caller's array", () => {
	const projection: ["id" | "tenant"] = ["id"]
	const declaration = key(Item, projection)
	assert.equal(Object.isFrozen(projection), false)
	projection[0] = "tenant"
	assert.deepEqual(declaration.projection, ["id"])
	assert.equal(Object.isFrozen(declaration.projection), true)
})

test("every declared key reads the same snapshot with its own projection and native statement id", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	const item = { id: 1n, tenant: 2n, code: "A", description: "first" }
	const booking = { room: 3n, during: { start: -10n, end: 20n }, label: "reserved" }
	try {
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.create(storeDir("declared-keys"), Theory)
					const draft = yield* ChangeSet.builder(Theory)
					yield* draft.insert(Item, [item])
					yield* draft.insert(Detail, [{ item: item.id }])
					yield* draft.insert(Booking, [booking])
					const changes = yield* draft.finish()
					assert.equal((yield* db.apply(changes, { expected: { kind: "any" } })).kind, "accepted")
					const snapshot = yield* db.snapshot()
					for (const read of [
						snapshot.get(ItemById, { id: item.id }),
						snapshot.get(ItemByCode, { tenant: item.tenant, code: item.code })
					]) {
						assert.deepEqual(yield* read, Option.some(item))
					}
					assert.deepEqual(
						yield* snapshot.get(BookingByRoomAndTime, { during: booking.during, room: booking.room }),
						Option.some(booking)
					)
					assert.ok(Option.isNone(yield* snapshot.get(ItemByCode, { code: "absent", tenant: 2n })))
					for (const invalid of [
						{},
						{ id: "1" },
						{ id: 1n, extra: undefined },
						{ id: 1n, code: "A" },
						Object.create({ id: 1n }),
						null
					]) {
						const result = yield* Effect.result(snapshot.get(ItemById, invalid as never))
						assert.ok(Result.isFailure(result))
						assert.equal(result.failure.code, "InvalidArgument")
					}
					const undeclared = key(Item, ["description"])
					const anotherDeclaration = key(Item, ["id"])
					assert.deepEqual(
						yield* snapshot.get(anotherDeclaration, { id: 1n }),
						Option.some(item),
						"equivalent descriptors name the same declared law"
					)
					for (const descriptor of [undeclared]) {
						const result = yield* Effect.result(snapshot.get(descriptor, { id: 1n } as never))
						assert.ok(Result.isFailure(result), "lookup requires the declaration in this schema")
					}
					const equivalent = relation("Item", { id: u64, tenant: u64, code: str, description: str })
					assert.deepEqual(yield* snapshot.get(key(equivalent, ["id"]), { id: 1n }), Option.some(item))
					const conflicting = relation("Item", { tenant: u64, id: u64, code: str, description: str })
					assert.ok(Result.isFailure(yield* Effect.result(snapshot.get(key(conflicting, ["id"]), { id: 1n }))))
				})
			)
		)
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
	}
})
