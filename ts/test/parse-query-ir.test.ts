import assert from "node:assert/strict"
import { describe, test } from "node:test"
import type { SnapshotHandle } from "#db-native.ts"
import { dbNative } from "#db-native.ts"
import type { QueryIr } from "#native.ts"
import { parseQueryIr } from "#query/parse-ir.ts"

function plainIr(): QueryIr {
	return {
		kind: "cq",
		interiors: [],
		head: [{ kind: "var" }],
		rules: [
			{
				finds: [{ kind: "var", var: 0 }],
				atoms: [{ source: { kind: "edb", relation: 0 }, bindings: [[0, { kind: "var", var: 0 }]] }],
				negated: [],
				conditions: []
			}
		]
	}
}

describe("parseQueryIr", function parseQueryIrSuite() {
	test("brands a shape-legal QueryIr", function brands() {
		const parsed = parseQueryIr(plainIr())
		assert.equal(parsed.rules.length, 1)
		assert.equal(parsed.kind, "cq")
	})

	test("owns complete descriptions without freezing or retaining caller arrays", () => {
		const head = [{ kind: "var" }]
		const bindings = [[0, { kind: "var", var: 0 }]]
		const raw = {
			kind: "cq",
			interiors: [],
			head,
			rules: [
				{
					finds: [{ kind: "var", var: 0 }],
					atoms: [{ source: { kind: "edb", relation: 0 }, bindings }],
					negated: [],
					conditions: []
				}
			]
		}
		const parsed = parseQueryIr(raw)
		assert.equal(Object.isFrozen(head), false)
		assert.equal(Object.isFrozen(bindings), false)
		head[0] = { kind: "count" }
		bindings.length = 0
		assert.deepEqual(parsed.head, [{ kind: "var" }])
		assert.equal(parsed.rules[0]?.atoms[0]?.bindings.length, 1)
		assert.ok(Object.isFrozen(parsed.rules[0]?.atoms[0]?.bindings))
	})

	test("rejects malformed nodes and ordinal overflow at every boundary", () => {
		const atTerm = (term: unknown) => ({
			...plainIr(),
			rules: [
				{
					finds: [{ kind: "count" }],
					atoms: [{ source: { kind: "edb", relation: 0 }, bindings: [[0, term]] }],
					negated: [],
					conditions: []
				}
			],
			head: [{ kind: "aggregate", op: "count" }]
		})
		for (const term of [
			null,
			{},
			{ kind: "unexpected" },
			{ kind: "var" },
			{ kind: "var", var: -1 },
			{ kind: "var", var: 0.5 },
			{ kind: "var", var: 65536 },
			{ kind: "param", param: Number.POSITIVE_INFINITY },
			{ kind: "paramSet", param: Number.NaN },
			{ kind: "var", var: 0, extra: undefined },
			{ kind: "literal", value: { kind: "u64", value: -1n } },
			{ kind: "literal", value: { kind: "intervalI64", start: 2n, end: 1n } },
			{ kind: "literal", value: { kind: "string", value: "\ud800" } },
			{ kind: "literal", value: { kind: "unknown", value: 1n } }
		])
			assert.throws(() => parseQueryIr(atTerm(term)), { name: "AuthoringError" })
		assert.doesNotThrow(() => parseQueryIr(atTerm({ kind: "var", var: 65535 })))
		assert.throws(() => parseQueryIr({ ...plainIr(), extra: true }), /only the declared fields/)
		assert.throws(() => parseQueryIr({ ...plainIr(), head: [{ kind: "unknown" }] }), { name: "AuthoringError" })
		assert.throws(
			() => parseQueryIr({ ...plainIr(), interiors: [{ head: [], rules: [] }] }),
			/interior rules are empty/
		)
		assert.throws(
			() =>
				parseQueryIr({
					...plainIr(),
					rules: [
						{
							finds: [{ kind: "var", var: 0 }],
							atoms: [{ source: { kind: "edb", relation: 0x100000000 }, bindings: [] }],
							negated: [],
							conditions: []
						}
					]
				}),
			/ordinal/
		)
	})

	test("rejects getters, sparse arrays and malformed binding tuples without invoking accessors", () => {
		let reads = 0
		assert.throws(
			() =>
				parseQueryIr({
					...plainIr(),
					get kind() {
						reads += 1
						return "cq"
					}
				}),
			/own data fields/
		)
		const rules = [plainIr().rules[0]]
		Object.defineProperty(rules, "0", {
			enumerable: true,
			get() {
				reads += 1
				return plainIr().rules[0]
			}
		})
		assert.throws(() => parseQueryIr({ ...plainIr(), rules }), /own data elements/)
		assert.equal(reads, 0)
		assert.throws(() => parseQueryIr({ ...plainIr(), rules: new Array(1) }), /present element/)
		for (const bindings of [
			[[0]],
			[[0, { kind: "var", var: 0 }, "extra"]],
			[
				[0, { kind: "var", var: 0 }],
				[0, { kind: "var", var: 1 }]
			]
		]) {
			assert.throws(
				() =>
					parseQueryIr({
						...plainIr(),
						rules: [{ ...plainIr().rules[0], atoms: [{ source: { kind: "edb", relation: 0 }, bindings }] }]
					}),
				/binding/
			)
		}
	})

	test("bounds recursive input before stack exhaustion and checks aggregate operators", () => {
		const computed = (expr: unknown) => ({
			kind: "cq",
			interiors: [],
			head: [{ kind: "compute" }],
			rules: [{ finds: [{ kind: "compute", expr }], atoms: [], negated: [], conditions: [] }]
		})
		let expr: unknown = { kind: "literal", value: { kind: "i64", value: 1n } }
		for (let i = 1; i < 128; i++) expr = { kind: "negate", expr }
		assert.doesNotThrow(() => parseQueryIr(computed(expr)))
		assert.throws(() => parseQueryIr(computed({ kind: "negate", expr })), /deeper than 128/)
		const cycle: { kind: string; expr?: unknown } = { kind: "negate" }
		cycle.expr = cycle
		assert.throws(() => parseQueryIr(computed(cycle)), /deeper than 128/)
		assert.throws(
			() => parseQueryIr(computed({ kind: "cast", cast: "guess", expr: { kind: "var", var: 0 } })),
			/unknown numeric cast/
		)
		assert.throws(
			() =>
				parseQueryIr({
					kind: "cq",
					interiors: [],
					head: [{ kind: "aggregate", op: "sum" }],
					rules: [
						{ finds: [{ kind: "aggregate", op: { kind: "max" }, over: 0 }], atoms: [], negated: [], conditions: [] }
					]
				}),
			/operator does not match/
		)
	})

	test("logical byte literals are detached and scalars use canonical field values", () => {
		const bytes = new Uint8Array([1, 2])
		const raw = {
			...plainIr(),
			rules: [
				{
					...plainIr().rules[0],
					atoms: [
						{
							source: { kind: "edb", relation: 0 },
							bindings: [
								[0, { kind: "literal", value: { kind: "fixedBytes", value: bytes } }],
								[1, { kind: "literal", value: { kind: "f64", value: -0 } }]
							]
						}
					]
				}
			]
		}
		const parsed = parseQueryIr(raw)
		bytes[0] = 9
		assert.deepEqual(parsed.rules[0]?.atoms[0]?.bindings[0]?.[1], {
			kind: "literal",
			value: { kind: "fixedBytes", value: new Uint8Array([1, 2]) }
		})
		assert.deepEqual(parsed.rules[0]?.atoms[0]?.bindings[1]?.[1], { kind: "literal", value: { kind: "f64", value: 0 } })
	})

	test("rejects empty main with populated interiors", function emptyMain() {
		assert.throws(function emptyMainRules() {
			parseQueryIr({
				kind: "cq",
				interiors: [
					{
						head: [{ kind: "var" }],
						rules: [
							{
								finds: [{ kind: "var", var: 0 }],
								atoms: [],
								negated: [],
								conditions: []
							}
						]
					}
				],
				head: [{ kind: "var" }],
				rules: []
			})
		}, /main rules are empty/)
	})

	test("rejects rec with an empty base", function emptyRecBase() {
		assert.throws(function emptyBase() {
			parseQueryIr({
				kind: "reach",
				interiors: [],
				rec: {
					head: [{ kind: "var" }],
					base: [],
					rec: [
						{
							finds: [{ kind: "var", var: 0 }],
							atoms: [],
							negated: [],
							conditions: []
						}
					]
				},
				head: [{ kind: "var" }],
				rules: [
					{
						finds: [{ kind: "var", var: 0 }],
						atoms: [],
						negated: [],
						conditions: []
					}
				]
			})
		}, /rec base is empty/)
	})

	test("a compute find term requires the shared expr grammar arm", function computeRequiresExpr() {
		assert.throws(function missingExpr() {
			parseQueryIr({
				kind: "cq",
				interiors: [],
				head: [{ kind: "compute" }],
				rules: [
					{
						finds: [{ kind: "compute" }],
						atoms: [],
						negated: [],
						conditions: []
					}
				]
			} as unknown as QueryIr)
		}, /compute requires expr/)
	})

	test("rejects Count-with-over", function countWithOver() {
		assert.throws(function countOver() {
			parseQueryIr({
				kind: "cq",
				interiors: [],
				head: [{ kind: "aggregate", op: "count" }],
				rules: [
					{
						finds: [{ kind: "count", over: 0 }],
						atoms: [],
						negated: [],
						conditions: []
					}
				]
			} as unknown as QueryIr)
		}, /Count carries no over/)
	})

	test("preparation rejects an unbranded QueryIr object literal", function unbranded() {
		const typePin: (snapshot: SnapshotHandle) => void = function expectUnbranded(snapshot) {
			// @ts-expect-error — the bridge demands a branded ParsedQuery
			dbNative.runtimeSnapshotPrepare(snapshot, plainIr(), () => {})
		}
		void typePin
	})
	describe("raw Event query programs", () => {
		const output = (find: unknown) => ({
			...plainIr(),
			head: [{ kind: "compute" }],
			rules: [{ ...plainIr().rules[0], finds: [find] }]
		})
		test("owns all structural constructors and test operators", () => {
			const a = { kind: "var", var: 0 }
			const b = { kind: "full", var: 1 }
			const expressions = [
				a,
				b,
				{ kind: "empty", var: 0 },
				{ kind: "not", expr: a },
				{ kind: "ite", condition: a, high: b, low: a },
				{ kind: "cardinality", minimum: 2n, maximum: 2n, events: [a, a, b] },
				...Array.from({ length: 16 }, (_, bits) => ({ kind: "apply", bits, left: a, right: b }))
			]
			for (const expr of expressions) {
				const parsed = parseQueryIr(output({ kind: "event", expr }))
				assert.deepEqual(parsed.rules[0]?.finds[0], { kind: "event", expr })
				const find = parsed.rules[0]?.finds[0]
				assert.ok(find?.kind === "event")
				assert.notEqual(find.expr, expr)
			}
			for (const kind of ["isEmpty", "isFull"])
				assert.doesNotThrow(() => parseQueryIr(output({ kind: "test", expr: { kind, expr: a } })))
			for (const kind of ["subset", "equal", "disjoint", "covers"])
				assert.doesNotThrow(() => parseQueryIr(output({ kind: "test", expr: { kind, left: a, right: b } })))
		})
		test("refuses malformed, cyclic and oversized programs before native execution", () => {
			const a = { kind: "var", var: 0 }
			const cycle: { kind: "not"; expr?: unknown } = { kind: "not" }
			cycle.expr = cycle
			for (const expr of [
				{ kind: "var", var: -1 },
				{ kind: "full", var: 0, ignored: true },
				{ kind: "apply", bits: 16, left: a, right: a },
				{ kind: "ite", condition: a, high: a },
				{ kind: "cardinality", minimum: 0n, maximum: 0n, events: [] },
				{ kind: "cardinality", minimum: -1n, maximum: 0n, events: [a] },
				{ kind: "cardinality", minimum: 0n, maximum: 0n, events: Array(4096).fill(a) },
				cycle
			])
				assert.throws(() => parseQueryIr(output({ kind: "event", expr })), { name: "AuthoringError" })
		})
		test("owns bounded BEDC payloads while native import remains the semantic gate", () => {
			const descriptor = new Uint8Array([66, 69, 68, 67, 1, 0])
			const make = (bytes: Uint8Array, op = "image") => ({
				kind: "map",
				op,
				descriptor: bytes,
				expr: { kind: "var", var: 0 }
			})
			const parsed = parseQueryIr(output({ kind: "event", expr: make(descriptor) }))
			const find = parsed.rules[0]?.finds[0]
			assert.equal(find?.kind, "event")
			if (find?.kind !== "event" || find.expr.kind !== "map") throw new Error("map")
			descriptor.fill(0)
			assert.deepEqual(find.expr.descriptor, new Uint8Array([66, 69, 68, 67, 1, 0]))
			assert.throws(() => parseQueryIr(output({ kind: "event", expr: make(descriptor, "unknown") })))
			assert.throws(() => parseQueryIr(output({ kind: "event", expr: make(new Uint8Array(new SharedArrayBuffer(6))) })))
			const large = new Uint8Array(8 * 1024 * 1024 + 1)
			assert.throws(
				() =>
					parseQueryIr(
						output({
							kind: "test",
							expr: {
								kind: "equal",
								left: make(large),
								right: make(large)
							}
						})
					),
				/at most/
			)
		})

		test("parses owned relation programs under the shared Event tree budget", () => {
			const descriptor = new Uint8Array([66, 69, 68, 67, 1, 3])
			const bind = { kind: "bind", descriptor, expr: { kind: "var", var: 0 } }
			const product = { kind: "product", op: "compose", descriptor, left: bind, right: bind }
			const parsed = parseQueryIr(
				output({ kind: "event", expr: { kind: "relation", op: "region", relation: product } })
			)
			const find = parsed.rules[0]?.finds[0]
			if (find?.kind !== "event" || find.expr.kind !== "relation" || find.expr.relation.kind !== "product")
				throw new Error("relation product")
			descriptor.fill(0)
			assert.deepEqual(find.expr.relation.descriptor, new Uint8Array([66, 69, 68, 67, 1, 3]))
			for (const relation of [
				{ ...product, op: "unknown" },
				{ ...bind, ignored: true },
				{ kind: "star", relation: bind },
				{ kind: "apply", bits: 16, left: bind, right: bind }
			])
				assert.throws(() => parseQueryIr(output({ kind: "event", expr: { kind: "relation", op: "region", relation } })))
			const cycle: { kind: string; relation?: unknown } = { kind: "converse" }
			cycle.relation = cycle
			assert.throws(() =>
				parseQueryIr(output({ kind: "event", expr: { kind: "relation", op: "region", relation: cycle } }))
			)
			const huge = { ...bind, descriptor: new Uint8Array(8 * 1024 * 1024 + 1) }
			assert.throws(
				() =>
					parseQueryIr(
						output({
							kind: "event",
							expr: {
								kind: "relation",
								op: "region",
								relation: { kind: "apply", bits: 8, left: huge, right: huge }
							}
						})
					),
				/at most/
			)
		})
	})
})

test("fixed-point wire parsing owns scopes and shares imported-byte limits", () => {
	const query = (expr: unknown) => ({
		...plainIr(),
		head: [{ kind: "compute" }],
		rules: [{ ...plainIr().rules[0], finds: [{ kind: "event", expr }] }]
	})
	const bound = { kind: "bound", depth: 0 }
	const scope = new Uint8Array([66, 69, 86, 84, 1])
	const fixed = { kind: "fixed", op: "least", scope, expr: bound }
	const parsed = parseQueryIr(query(fixed))
	scope.fill(0)
	const find = parsed.rules[0]?.finds[0]
	assert.ok(find?.kind === "event" && find.expr.kind === "fixed")
	assert.deepEqual([...find.expr.scope], [66, 69, 86, 84, 1])
	for (const invalid of [
		{ ...fixed, op: "approximate" },
		{ ...fixed, extra: 0 },
		{ ...bound, depth: -1 },
		{ ...bound, depth: 65536 },
		{ ...bound, depth: 0.5 },
		{ ...bound, var: 0 },
		{ ...fixed, scope: new Uint8Array(new SharedArrayBuffer(8)) }
	])
		assert.throws(() => parseQueryIr(query(invalid)), { name: "AuthoringError" })
	const large = { ...fixed, scope: new Uint8Array(9 * 1024 * 1024) }
	assert.throws(() => parseQueryIr(query({ ...large, expr: large })), /byte/)
	// Shape parsing deliberately leaves mathematical/lexical admission to the worker.
	assert.doesNotThrow(() => parseQueryIr(query(bound)))
})
