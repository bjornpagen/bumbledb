/**
 * Bounded structural decoding of untrusted generated data (TS-MIG-01/10):
 * the runner's static index import is plain JSON and is never trusted — a
 * well-shaped object is still not a checked plan (native re-judges), but a
 * malformed or manifest-divergent object refuses HERE, before anything
 * crosses the bridge. No unknown operation, codec version or expression node
 * survives decoding.
 */
import assert from "node:assert/strict"
import { describe, test } from "node:test"
import { decodeGeneratedMigrations, decodeManifestData, decodePlanData } from "#migrations/decode.ts"

const DIGEST = "ab".repeat(32)

const manifest = {
	manifestVersion: 1,
	planVersion: 1,
	baseSchemaId: DIGEST,
	basePrefixDigest: DIGEST,
	entries: [
		{
			sequence: "0",
			id: "0000-initialize",
			fromSchemaId: DIGEST,
			toSchemaId: DIGEST,
			planDigest: DIGEST,
			prefixDigest: DIGEST
		}
	]
}

const plan = {
	planVersion: 1,
	sequence: "0",
	id: "0000-initialize",
	fromSchemaId: DIGEST,
	toSchemaId: DIGEST,
	operations: [
		{ kind: "empty-relation", target: "Note" },
		{ kind: "seed", target: "Note", rows: [[{ u64: "1" }, { string: "x" }]] },
		{ kind: "validate-schema", schemaId: DIGEST }
	],
	destructive: []
}

describe("runner-input decoding", function suite() {
	test("well-formed generated data round-trips", function roundTrip() {
		const decoded = decodeGeneratedMigrations({ manifest, plans: [plan] })
		assert.ok(decoded.ok)
		assert.equal(decoded.value.plans.length, 1)
		assert.deepEqual(decoded.value.plans[0]?.operations[1], plan.operations[1])
	})

	test("unknown codec versions refuse", function versions() {
		assert.equal(decodeManifestData({ ...manifest, manifestVersion: 2 }).ok, false)
		assert.equal(decodePlanData({ ...plan, planVersion: 2 }).ok, false)
	})

	test("unknown operation kinds and expression nodes refuse", function unknownOps() {
		const evil = {
			...plan,
			operations: [{ kind: "run-javascript", source: "require('fs')" }]
		}
		assert.equal(decodePlanData(evil).ok, false)
		const evilExpr = {
			...plan,
			operations: [
				{
					kind: "map-relation",
					source: "Note",
					target: "Note",
					fields: [{ target: "id", expression: { kind: "module", path: "./hack.ts" } }]
				}
			]
		}
		assert.equal(decodePlanData(evilExpr).ok, false)
	})

	test("a plan disagreeing with its manifest entry refuses", function divergence() {
		const renamed = { ...plan, id: "0000-other" }
		const decoded = decodeGeneratedMigrations({ manifest, plans: [renamed] })
		assert.equal(decoded.ok, false)
		const miscounted = decodeGeneratedMigrations({ manifest, plans: [] })
		assert.equal(miscounted.ok, false)
	})

	test("destructive entries decode; malformed ones refuse", function losses() {
		const withLoss = { ...plan, destructive: [{ relation: "Note", field: "body" }, { relation: "Old" }] }
		const decoded = decodePlanData(withLoss)
		assert.ok(decoded.ok)
		assert.deepEqual(decoded.value.destructive, [{ relation: "Note", field: "body" }, { relation: "Old" }])
		assert.equal(decodePlanData({ ...plan, destructive: [{ field: "x" }] }).ok, false)
	})
})
