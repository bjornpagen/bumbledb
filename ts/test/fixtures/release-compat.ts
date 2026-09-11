import { createHash } from "node:crypto"
import { Effect } from "effect"

/** Execute unchanged public algebra with either the released or current SDK. */
export function releaseCompatibility(sdk: typeof import("#index.ts"), path: string, existing = false) {
	const {
		capacity,
		ChangeSet,
		Compute,
		Db,
		duration,
		i64,
		interval,
		key,
		on,
		query,
		ref,
		relation,
		schema,
		Schema,
		u64,
		v,
		weigh,
		within
	} = sdk
	const Row = relation("Row", { id: u64, group: u64, span: interval(i64), observed: u64 })
	const Theory = schema("Compatibility", { Row }, [
		key(Row, ["id"]),
		capacity(on(Row, "id"), { from: on(Row, "id"), weight: weigh("observed"), within: within(0n, duration("span")) }),
		capacity(on(Row, "id"), { from: on(Row, "id"), weight: duration("span"), within: within(0n, ref("observed")) })
	])
	const arithmetic = query(Theory).rule((r) => {
		const row = v(Row)
		return r
			.match(Row, row)
			.find({ id: row.id, value: Compute.divide(Compute.multiply(row.id, Compute.u64(2n)), Compute.u64(3n)) })
	})
	const packed = query(Theory).rule((r) => {
		const row = v(Row)
		return r.match(Row, row).find({ group: row.group, span: r.pack(row.span) })
	})
	const summed = query(Theory).rule((r) => {
		const row = v(Row)
		return r.match(Row, row).find({ group: row.group, total: r.sum(row.observed) })
	})
	return Effect.scoped(
		Effect.gen(function* () {
			const compiled = yield* Schema.compile(Theory)
			const db = yield* existing ? Db.open(path, Theory) : Db.create(path, Theory)
			if (!existing) {
				const change = yield* ChangeSet.builder(Theory)
				yield* change.insert(
					Row,
					Array.from({ length: 128 }, (_, id) => ({
						id: BigInt(id),
						group: BigInt(id % 8),
						span: { start: BigInt(id * 2), end: BigInt(id * 2 + 3) },
						observed: 3n
					}))
				)
				yield* db.apply(yield* change.finish(), { expected: { kind: "any" } })
			}
			const snapshot = yield* db.snapshot()
			const digest = (value: unknown) =>
				createHash("sha256")
					.update(JSON.stringify(value, (_, x: unknown) => (typeof x === "bigint" ? x.toString() : x)))
					.digest("hex")
			const arithmeticRows = yield* (yield* snapshot.execute(arithmetic, {})).collect()
			const packRows = yield* (yield* snapshot.execute(packed, {})).collect()
			const sumRows = yield* (yield* snapshot.execute(summed, {})).collect()
			const sort = (rows: readonly unknown[]) =>
				rows.map((row) => JSON.stringify(row, (_, x: unknown) => (typeof x === "bigint" ? x.toString() : x))).sort()
			return {
				schemaId: compiled.schemaId,
				descriptor: digest(compiled.descriptor),
				arithmetic: digest(sort(arithmeticRows)),
				pack: digest(sort(packRows)),
				sum: digest(sort(sumRows)),
				counts: [arithmeticRows.length, packRows.length, sumRows.length]
			}
		})
	)
}
