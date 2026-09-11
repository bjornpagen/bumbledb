import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, ManagedRuntime } from "effect"
import { ChangeSet } from "#changes.ts"
import { Db } from "#db.ts"
import { i64, u64 } from "#fields.ts"
import { Compute } from "#query/compute.ts"
import { describeQuery, queryFromDescription } from "#query/description.ts"
import { query } from "#query/lower.ts"
import { v } from "#query/scope.ts"
import { relation } from "#relation.ts"
import { NativeRuntime } from "#runtime.ts"
import { schema } from "#schema.ts"
import { runtimeOptions, storeDir } from "#test/fixtures/learning.ts"

test("native mulDiv keeps exact wide products and explicit signed rounding across the bridge", async () => {
	const Input = relation("Input", { signed: i64, unsigned: u64 })
	const Theory = schema("MulDiv", { Input }, [])
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.create(storeDir("muldiv"), Theory)
					const draft = yield* ChangeSet.builder(Theory)
					yield* draft.insert(Input, [{ signed: -(1n << 63n), unsigned: (1n << 64n) - 1n }])
					yield* db.apply(yield* draft.finish(), { expected: { kind: "any" } })
					const snap = yield* db.snapshot()
					const result = query(Theory).rule((r) => {
						const row = v(Input)
						return r.match(Input, row).find({
							unsigned: Compute.mulDiv(row.unsigned, Compute.u64(2n), Compute.u64(2n), "towardZero"),
							signed: Compute.mulDiv(row.signed, Compute.i64(-1n), Compute.i64(2n), "towardZero"),
							toward: Compute.mulDiv(Compute.i64(-5n), Compute.i64(1n), Compute.i64(2n), "towardZero"),
							away: Compute.mulDiv(Compute.i64(-5n), Compute.i64(1n), Compute.i64(2n), "nearestTiesAwayFromZero"),
							even: Compute.mulDiv(Compute.i64(-5n), Compute.i64(1n), Compute.i64(2n), "nearestTiesToEven"),
							odd: Compute.mulDiv(Compute.i64(-7n), Compute.i64(1n), Compute.i64(2n), "nearestTiesToEven")
						})
					})
					const expected = [
						{ unsigned: (1n << 64n) - 1n, signed: 1n << 62n, toward: -2n, away: -3n, even: -2n, odd: -4n }
					]
					assert.deepEqual(yield* (yield* snap.execute(result, {})).collect(), expected)
					const imported = queryFromDescription(Theory, describeQuery(result), {
						unsigned: u64,
						signed: i64,
						toward: i64,
						away: i64,
						even: i64,
						odd: i64
					})
					assert.deepEqual(yield* (yield* snap.execute(imported, {})).collect(), expected)
					for (const divisor of [0n, -1n, 1n]) {
						const bad = query(Theory).rule((r) => {
							const row = v(Input)
							return r
								.match(Input, row)
								.find({ value: Compute.mulDiv(row.signed, Compute.i64(-1n), Compute.i64(divisor), "towardZero") })
						})
						const failed = yield* Effect.result(
							Effect.gen(function* () {
								return yield* (yield* snap.execute(bad, {})).collect()
							})
						)
						assert.equal(failed._tag, "Failure", `divisor ${divisor} must refuse`)
						if (failed._tag === "Failure") {
							assert.equal(failed.failure.reason._tag, "Engine")
							if (failed.failure.reason._tag === "Engine") {
								assert.equal(failed.failure.reason.kind, "scalar")
								const reasons = new Map([
									[0n, "DivisionByZero"],
									[-1n, "NonPositiveDivisor"],
									[1n, "Overflow"]
								])
								const reason = reasons.get(divisor)
								assert.equal(failed.failure.reason.message, `find 0: scalar computation: ${reason}`)
							}
						}
						const importedBad = queryFromDescription(Theory, describeQuery(bad), { value: i64 })
						const filtered = query(Theory).rule((r) => {
							const row = v(importedBad)
							return r.match(importedBad, row).where(r.eq(row.value, 0n)).find(row)
						})
						const downstream = yield* Effect.result(
							Effect.scoped(
								Effect.gen(function* () {
									return yield* (yield* snap.execute(filtered, {})).collect()
								})
							)
						)
						assert.equal(downstream._tag, "Failure", "import and downstream filters preserve upstream failure")
						if (failed._tag === "Failure" && downstream._tag === "Failure")
							assert.deepEqual(downstream.failure.reason, failed.failure.reason)
					}
					const checked = query(Theory).rule((r) => {
						const row = v(Input)
						return r
							.match(Input, row)
							.find({ value: Compute.divide(Compute.multiply(row.unsigned, Compute.u64(2n)), Compute.u64(2n)) })
					})
					const overflow = yield* Effect.result(
						Effect.gen(function* () {
							return yield* (yield* snap.execute(checked, {})).collect()
						})
					)
					assert.equal(overflow._tag, "Failure", "old checked multiply still overflows before division")
					if (overflow._tag === "Failure" && overflow.failure.reason._tag === "Engine") {
						assert.equal(overflow.failure.reason.message, "find 0: scalar computation: Overflow")
					}
				})
			)
		)
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
	}
})
