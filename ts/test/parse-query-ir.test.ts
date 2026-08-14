/**
 * Host `parseQueryIr` pins: rec/main emptiness, Count-with-over, and
 * `dbPrepare` rejecting an unbranded `QueryIr` object literal.
 */

import assert from "node:assert/strict"
import { describe, test } from "node:test"
import type { DbHandle, QueryIr } from "#native.ts"
import { native } from "#native.ts"
import { parseQueryIr } from "#query/parse-ir.ts"

/** A shape-legal one-var query over relation 0. */
function plainIr(): QueryIr {
	return {
		interiors: [],
		rec: null,
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
		assert.equal(parsed.rec, null)
	})

	test("rejects empty main with populated interiors", function emptyMain() {
		assert.throws(function emptyMainRules() {
			parseQueryIr({
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
				rec: null,
				head: [{ kind: "var" }],
				rules: []
			})
		}, /main rules are empty/)
	})

	test("rejects rec with an empty base", function emptyRecBase() {
		assert.throws(function emptyBase() {
			parseQueryIr({
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
				interiors: [],
				rec: null,
				head: [{ kind: "aggregate", op: "count" }],
				rules: [
					{
						finds: [{ kind: "aggregate", op: { kind: "count" }, over: 0 }],
						atoms: [],
						negated: [],
						conditions: []
					}
				]
			} as unknown as QueryIr)
		}, /Count carries no over/)
	})

	test("dbPrepare rejects an unbranded QueryIr object literal", function unbranded() {
		const typePin: (db: DbHandle) => void = function expectUnbranded(db) {
			// @ts-expect-error — dbPrepare demands a branded ParsedQuery
			native.dbPrepare(db, plainIr())
		}
		void typePin
	})
})
