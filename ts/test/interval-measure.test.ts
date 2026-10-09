import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, ManagedRuntime, Result } from "effect"
import { duration, ref, weigh, within } from "../src/capacity.ts"
import { ChangeSet } from "../src/changes.ts"
import { Db } from "../src/db.ts"
import { on } from "../src/face.ts"
import { i64, interval, u64 } from "../src/fields.ts"
import { relation } from "../src/relation.ts"
import { Bumble } from "../src/runtime.ts"
import { schema } from "../src/schema.ts"
import { capacity, key } from "../src/statements.ts"
import { runtimeOptions, storeDir } from "./fixtures/learning.ts"

for (const element of [u64, i64]) {
	test(`paired ${element.kind} interval/scalar capacities prove the finite measure`, async () => {
		const Slice = relation("Slice", { id: u64, span: interval(element), cents: u64 })
		const Theory = schema("Measured", { Slice }, [
			key(Slice, ["id"]),
			capacity(on(Slice, "id"), {
				from: on(Slice, "id"),
				weight: weigh("cents"),
				within: within(0n, duration("span"))
			}),
			capacity(on(Slice, "id"), {
				from: on(Slice, "id"),
				weight: weigh(duration("span")),
				within: within(0n, ref("cents"))
			})
		])
		const runtime = ManagedRuntime.make(Bumble.layer(runtimeOptions))
		try {
			await runtime.runPromise(
				Effect.scoped(
					Effect.gen(function* () {
						const db = yield* Db.create(storeDir(`measure-${element.kind}`), Theory)
						const span = element.kind === "i64" ? { start: -10n, end: 0n } : { start: 100n, end: 110n }
						for (const cents of [0n, 9n, 11n]) {
							const draft = yield* ChangeSet.builder(Theory)
							yield* draft.insert(Slice, [{ id: 1n, span, cents }])
							const before = yield* db.snapshot()
							const refused = yield* db.apply(yield* draft.finish(), { expected: { kind: "any" } })
							assert.equal(refused.kind, "invariant-rejected", `false measure ${cents} refuses`)
							assert.deepEqual((yield* db.snapshot()).witness, before.witness)
						}
						const draft = yield* ChangeSet.builder(Theory)
						yield* draft.insert(Slice, [{ id: 1n, span, cents: 10n }])
						assert.equal((yield* db.apply(yield* draft.finish(), { expected: { kind: "any" } })).kind, "accepted")
						const ray = yield* ChangeSet.builder(Theory)
						yield* ray.insert(Slice, [
							{ id: 2n, span: { start: 0n, end: (1n << (element.kind === "i64" ? 63n : 64n)) - 1n }, cents: 10n }
						])
						const refusedRay = yield* Effect.result(db.apply(yield* ray.finish(), { expected: { kind: "any" } }))
						assert.ok(Result.isFailure(refusedRay), "an infinite measure remains an operational refusal")
					})
				)
			)
		} finally {
			await Effect.runPromise(runtime.disposeEffect)
		}
	})
}
