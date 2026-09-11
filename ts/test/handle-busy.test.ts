import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, Fiber, ManagedRuntime, Result } from "effect"
import { ChangeSet, Db, NativeRuntime, query, relation, schema, u64, v } from "#index.ts"
import { storeDir } from "#test/fixtures/learning.ts"

const Item = relation("Item", { id: u64 })
const Theory = schema("Contention", { Item }, [])
const cross = query(Theory).rule((r) => {
	const left = v(Item)
	const right = v(Item)
	return r.match(Item, left).match(Item, right).find({ left: left.id, right: right.id })
})
const one = query(Theory).rule((r) => {
	const { id } = v(Item)
	return r.match(Item, { id }).where(r.eq(id, 1n)).find({ value: id })
})

test("a public busy reader becomes reusable after its admitted query finishes", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer({ workers: 2 }))
	try {
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.create(storeDir("handle-busy"), Theory)
					const builder = yield* ChangeSet.builder(Theory)
					yield* builder.insert(
						Item,
						Array.from({ length: 1024 }, (_, i) => ({ id: BigInt(i) }))
					)
					yield* db.apply(yield* builder.finish(), { expected: { kind: "any" } })
					const reader = yield* db.snapshot()
					const pending = yield* Effect.forkChild(reader.execute(cross, {}))
					// The million-row native seal keeps the reader in use while JS
					// returns to dispatch another call. No callback is intercepted.
					yield* Effect.promise(() => new Promise<void>((resolve) => setTimeout(resolve, 1)))
					const busy = yield* Effect.result(reader.execute(one, {}))
					assert.ok(Result.isFailure(busy))
					assert.equal(busy.failure.code, "HandleBusy")
					const result = yield* Fiber.join(pending)
					yield* result.close()
					const reused = yield* reader.execute(one, {})
					assert.deepEqual(yield* reused.collect(), [{ value: 1n }])
				})
			)
		)
		const counts = await runtime.runPromise(Effect.flatMap(NativeRuntime, (native) => native.inspect()))
		assert.equal(counts.natives, 0n)
		assert.equal(counts.owners, 0n)
	} finally {
		await runtime.dispose()
	}
})
