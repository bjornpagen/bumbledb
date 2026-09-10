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
})
