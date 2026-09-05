import assert from "node:assert/strict"
import { describe, test } from "node:test"
import type { QueryIr } from "#native.ts"
import type { SessionHandle } from "#db-native.ts"
import { dbNative } from "#db-native.ts"
import { policyWire } from "#runtime.ts"
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

	test("the session execute verb rejects an unbranded QueryIr object literal", function unbranded() {
		const typePin: (session: SessionHandle) => void = function expectUnbranded(session) {
			const wire = policyWire(
				{
					inputBytes: 0n,
					workingBytes: 0n,
					scratchBytes: 0n,
					resultBytes: 0n,
					rows: 0n,
					workUnits: 1n,
					timeout: "1 second"
				},
				"typePin"
			)
			// @ts-expect-error — the bridge demands a branded ParsedQuery
			dbNative.runtimeSessionExecute(session, wire, plainIr(), [], () => {})
		}
		void typePin
	})
})
