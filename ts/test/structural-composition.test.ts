import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, ManagedRuntime } from "effect"
import { Db } from "../src/engine.ts"
import {
	alternatives,
	Bumble,
	ChangeSet,
	Compute,
	closed,
	closedId,
	interval,
	key,
	query,
	relation,
	schema,
	u64,
	v
} from "../src/index.ts"
import { runtimeOptions, storeDir } from "./fixtures/learning.ts"

const Kind = closed("Kind", ["Measured", "Quoted"])
const Parent = relation("Parent", { id: u64, kind: closedId(Kind) })
const Measured = relation("Measured", { parent: u64, span: interval(u64), excluded: interval(u64) })
const Quoted = relation("Quoted", { parent: u64, amount: u64 })
const parentKey = key(Parent, ["id"])
const arms = { Measured: key(Measured, ["parent"]), Quoted: key(Quoted, ["parent"]) }
const Theory = schema("Composed", { Kind, Parent, Measured, Quoted }, [
	parentKey,
	arms.Measured,
	arms.Quoted,
	...alternatives(parentKey, "kind", Kind, arms)
])
const pieces = query(Theory).rule((r) => {
	const row = v(Measured)
	return r
		.match(Measured, row)
		.match(Parent, { id: row.parent, kind: "Measured" })
		.where(r.not(Quoted, { parent: row.parent }))
		.find({ id: row.parent, span: r.difference(row.span, row.excluded) })
})
const packed = query(Theory).rule((r) => {
	const row = v(pieces)
	return r.match(pieces, row).find({ id: row.id, span: r.pack(row.span) })
})
const measured = query(Theory).rule((r) => {
	const row = v(packed)
	return r.match(packed, row).find({ id: row.id, span: row.span, width: Compute.measure(row.span) })
})
const totals = query(Theory).rule((r) => {
	const row = v(measured)
	return r.match(measured, row).find({ id: row.id, total: r.sum(row.width) })
})
const amounts = query(Theory).rule((r) => {
	const row = v(totals)
	return r
		.match(totals, row)
		.find({ id: row.id, amount: Compute.mulDiv(row.total, Compute.u64(3n), Compute.u64(2n), "nearestTiesToEven") })
})

function importedTypeChecks() {
	const row = v(pieces)
	// @ts-expect-error Importing a description does not turn scalar ids into intervals.
	Compute.measure(row.id)
	// @ts-expect-error Integer measurement keeps its unsigned result kind after import.
	Compute.add(Compute.measure(row.span), Compute.i64(1n))
}
void importedTypeChecks

test("alternatives and imported interval stages compose through atomic arm switches", async () => {
	const runtime = ManagedRuntime.make(Bumble.layer(runtimeOptions))
	try {
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.create(storeDir("structural-composition"), Theory)
					const measuredRow = {
						parent: 1n,
						span: { start: 0n, end: 10n },
						excluded: { start: 3n, end: 7n }
					}
					const quotedRow = { parent: 1n, amount: 5n }
					let prior: "Measured" | "Quoted" | undefined
					const read = () =>
						Effect.scoped(
							Effect.gen(function* () {
								const snapshot = yield* db.snapshot()
								const direct = yield* (yield* snapshot.execute(amounts, {})).collect()
								return direct
							})
						)
					for (let step = 0; step < 24; step++) {
						yield* Effect.scoped(
							Effect.gen(function* () {
								const kind = step % 2 === 0 ? "Measured" : "Quoted"
								const before = yield* read()
								for (const invalid of [true, false]) {
									const build = (reverse: boolean) =>
										Effect.gen(function* () {
											const draft = yield* ChangeSet.builder(Theory)
											const insert = Effect.gen(function* () {
												yield* draft.insert(Parent, [{ id: 1n, kind }])
												if (!invalid) {
													if (kind === "Measured") yield* draft.insert(Measured, [measuredRow])
													else yield* draft.insert(Quoted, [quotedRow])
												}
											})
											if (reverse) yield* insert
											if (prior !== undefined) {
												yield* draft.delete(Parent, [{ id: 1n, kind: prior }])
												if (prior === "Measured") yield* draft.delete(Measured, [measuredRow])
												else yield* draft.delete(Quoted, [quotedRow])
											}
											if (!reverse) yield* insert
											return yield* draft.finish()
										})
									const forward = yield* build(false)
									const backward = yield* build(true)
									const judged = yield* db.judge(forward)
									assert.deepEqual(yield* db.judge(backward), judged)
									const result = yield* db.apply(forward)
									assert.equal(result._tag, invalid ? "Rejected" : "Committed")
									if (invalid) {
										assert.equal(judged._tag, "Rejected")
										if (result._tag === "Rejected" && judged._tag === "Rejected") {
											assert.deepEqual(result.violations, judged.violations)
											assert.equal(result.violations.length, 1)
											assert.equal(result.violations[0]?._tag, "Containment")
										}
										assert.deepEqual(yield* read(), before)
									}
								}
								prior = kind
								assert.deepEqual(yield* read(), kind === "Measured" ? [{ id: 1n, amount: 9n }] : [])
							})
						)
					}
				})
			)
		)
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
	}
})
