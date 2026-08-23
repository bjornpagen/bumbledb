import assert from "node:assert/strict"
import { describe, test } from "node:test"
import { braidsOf, serialAtStatementsOf } from "#braids.ts"
import { Grid, Ledger, Vocab } from "#test/fixtures.ts"

describe("braid derivation", function suite() {
	test("connected relations share a braid named by the smallest RelationId", function components() {
		const braids = braidsOf(Ledger)
		assert.equal(braids.get("Holder"), "c00000000")
		assert.equal(braids.get("Booking"), "c00000000")
		assert.equal(braids.get("Note"), "c00000002")
		assert.equal(braids.size, 3)
	})

	test("a single-component theory degenerates to the serial log", function serial() {
		const braids = braidsOf(Grid)
		assert.deepEqual([...new Set(braids.values())], ["c00000000"])
	})

	test("closed relations and closed-target statements contribute nothing", function closedExcluded() {
		const braids = braidsOf(Vocab)
		assert.equal(braids.has("Status"), false)
		assert.equal(braids.has("Kind"), false)
		assert.deepEqual([...braids.keys()], ["Account"])
	})

	test("no Ledger statement has an empty determinant, so the serial roster is empty", function degenerate() {
		assert.deepEqual(serialAtStatementsOf(Ledger), [])
	})
})
