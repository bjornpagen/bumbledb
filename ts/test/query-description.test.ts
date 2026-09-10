import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, ManagedRuntime, Result } from "effect"
import {
	ChangeSet,
	Compute,
	closed,
	closedId,
	contained,
	Db,
	describeQuery,
	i64,
	interval,
	NativeRuntime,
	on,
	query,
	queryFromDescription,
	relation,
	schema,
	u64,
	v
} from "#index.ts"
import { lowerQuery } from "#query/lower.ts"
import { runtimeOptions, storeDir } from "#test/fixtures/learning.ts"

const Edge = relation("Edge", { from: u64, to: u64 })
const Graph = schema("Graph", { Edge }, [])

test("generated descriptions execute, prepare, compose and infer their checked result", async () => {
	const generated = {
		ir: {
			kind: "cq",
			interiors: [],
			head: [{ kind: "var" }],
			rules: [
				{
					atoms: [
						{
							source: { kind: "edb", relation: 0 },
							bindings: [
								[0, { kind: "param", param: 0 }],
								[1, { kind: "var", var: 7 }]
							]
						}
					],
					negated: [],
					conditions: [],
					finds: [{ kind: "var", var: 7 }]
				}
			]
		},
		columns: ["next"],
		interiors: [],
		recursion: undefined,
		parameters: [{ name: "start", members: undefined }]
	}
	const described = queryFromDescription(Graph, generated, { next: u64 })
	const composed = described.rule((r) => {
		const { to } = v(Edge)
		return r.match(Edge, { from: 9n, to }).find({ next: to })
	})
	// Mutating the input description after construction cannot alter execution.
	generated.ir.rules.length = 0
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.create(storeDir("query-description"), Graph)
					const draft = yield* ChangeSet.builder(Graph)
					yield* draft.insert(Edge, [
						{ from: 1n, to: 2n },
						{ from: 2n, to: 3n }
					])
					assert.equal((yield* db.apply(yield* draft.finish(), { expected: { kind: "any" } })).kind, "accepted")
					const snapshot = yield* db.snapshot()
					const prepared = yield* snapshot.prepare(described)
					const rows = yield* (yield* prepared.execute({ start: 1n })).collect()
					const typed: readonly { readonly next: bigint }[] = rows
					assert.deepEqual(typed, [{ next: 2n }])
					assert.deepEqual(yield* (yield* snapshot.execute(composed, { start: 2n })).collect(), [{ next: 3n }])
					assert.ok(Result.isFailure(yield* Effect.result(prepared.execute({ start: -1n }))))
					assert.ok(Result.isFailure(yield* Effect.result(prepared.execute({ start: 1n, extra: true }))))
				})
			)
		)
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
	}
})

test("descriptions preserve joins, negation, aggregates, computations and interiors", () => {
	const built = query(Graph)
		.interior("Degree", (r) => {
			const { from, to } = v(Edge)
			return r.match(Edge, { from, to }).find({ from, count: r.count() })
		})
		.rule((r) => {
			const { from, to } = v(Edge)
			return r
				.interior("Degree", { from, count: to })
				.where(r.not(Edge, { from: to, to: from }))
				.where(r.or(r.ge(to, 1n), r.eq(from, r.param("root"))))
				.find({ from, adjusted: Compute.add(to, Compute.u64(1n)) })
		})
	const described = queryFromDescription(Graph, describeQuery(built), { from: u64, adjusted: u64 })
	assert.deepEqual(lowerQuery(described), lowerQuery(built))
	const imported = query(Graph).rule((r) => {
		const row = v(Edge)
		return r.match(Edge, row).find(row)
	})
	const importing = query(Graph).rule((r) => {
		const row = v(imported)
		return r.match(imported, row).find({ edgeFrom: row.from, edgeTo: row.to })
	})
	assert.deepEqual(
		lowerQuery(queryFromDescription(Graph, describeQuery(importing), { edgeFrom: u64, edgeTo: u64 })),
		lowerQuery(importing)
	)
})

test("descriptions preserve recursion and interval/closed-reference domains", async () => {
	const built = query(Graph)
		.reach("Path", {
			base: [
				(r) => {
					const row = v(Edge)
					return r.match(Edge, row).find(row)
				}
			],
			rec: [
				(r) => {
					const { from, to } = v(Edge)
					const middle = v(Edge).to
					return r.interior("Path", { from, to: middle }).match(Edge, { from: middle, to }).find({ from, to })
				}
			]
		})
		.rule((r) => {
			const row = v(Edge)
			return r.interior("Path", row).find(row)
		})
	const described = queryFromDescription(Graph, describeQuery(built), Edge.fields)
	assert.deepEqual(lowerQuery(described), lowerQuery(built))
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.create(storeDir("description-reach"), Graph)
					const draft = yield* ChangeSet.builder(Graph)
					yield* draft.insert(Edge, [
						{ from: 1n, to: 2n },
						{ from: 2n, to: 3n }
					])
					yield* db.apply(yield* draft.finish(), { expected: { kind: "any" } })
					const snapshot = yield* db.snapshot()
					assert.deepEqual(
						new Set(yield* (yield* snapshot.execute(described, {})).collect()),
						new Set([
							{ from: 1n, to: 2n },
							{ from: 1n, to: 3n },
							{ from: 2n, to: 3n }
						])
					)
				})
			)
		)
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
	}
	const Kind = closed("Kind", ["A", "B", "C"])
	const Span = relation("Span", { kind: closedId(Kind), span: interval(i64) })
	const theory = schema("SpanTheory", { Kind, Span }, [contained(on(Span, "kind"), on(Kind, "id"))])
	const intervals = query(theory).rule((r) => {
		const row = v(Span)
		return r
			.match(Span, { kind: ["A", "B"], span: row.span })
			.where(r.pointIn(2n, row.span))
			.find({ span: r.pack(row.span) })
	})
	assert.deepEqual(
		lowerQuery(queryFromDescription(theory, describeQuery(intervals), { span: interval(i64) })),
		lowerQuery(intervals)
	)
})

test("description checking refuses malformed grammar, wrong result claims, unbound vars and foreign field tags", () => {
	const built = query(Graph).rule((r) => {
		const row = v(Edge)
		return r.match(Edge, row).find(row)
	})
	const description = describeQuery(built)
	assert.throws(() => queryFromDescription(Graph, description, { from: i64, to: u64 }), /field domains/)
	for (const members of [undefined, [{ kind: "u64", value: 1n }]]) {
		assert.throws(
			() => queryFromDescription(Graph, { ...description, parameters: [{ name: "unused", members }] }, Edge.fields),
			/unused/
		)
	}
	assert.throws(() => queryFromDescription(Graph, { ...description, columns: ["from"] }, Edge.fields), /head width/)
	const first = description.ir.rules[0]
	assert.ok(first)
	assert.throws(
		() =>
			queryFromDescription(
				Graph,
				{
					...description,
					ir: {
						...description.ir,
						rules: [
							{
								...first,
								finds: [
									{ kind: "var", var: 99 },
									{ kind: "var", var: 1 }
								]
							}
						]
					}
				},
				Edge.fields
			),
		/positive binding/
	)
	const literal = query(Graph).rule((r) => r.match(Edge, { from: 1n }).find({ count: r.count() }))
	const raw = describeQuery(literal)
	const rule = raw.ir.rules[0]
	assert.ok(rule)
	assert.throws(
		() =>
			queryFromDescription(
				Graph,
				{
					...raw,
					ir: {
						...raw.ir,
						rules: [
							{
								...rule,
								atoms: [
									{
										source: { kind: "edb", relation: 0 },
										bindings: [[0, { kind: "literal", value: { kind: "i64", value: 1n } }]]
									}
								]
							}
						]
					}
				},
				{ count: u64 }
			),
		/literal tag/
	)
	let accessed = 0
	assert.throws(
		() =>
			queryFromDescription(
				Graph,
				{
					...description,
					get columns() {
						accessed++
						return []
					}
				},
				Edge.fields
			),
		/own data fields/
	)
	assert.equal(accessed, 0)
})
