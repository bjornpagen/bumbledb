import assert from "node:assert/strict"
import { describe, test } from "node:test"
import type { BatchOp } from "#footprint.ts"
import { capacityCommutes, intersectionOf } from "#intersect.ts"
import { Grid, Ledger } from "#test/fixtures.ts"

function booking(id: bigint, holder: bigint, slot: string): BatchOp {
	return { op: "insert", relation: "Booking", rows: [[id, holder, slot, { start: 1n, end: 2n }]] }
}

describe("the loser's intersection", function suite() {
	test("byte-identical effects are subsumed", function subsumed() {
		const ops = [booking(7n, 1n, "s1")]
		assert.deepEqual(intersectionOf(Ledger, ops, ops), { tag: "subsumed" })
	})

	test("the booking race: two writers of one determinant is a K conflict", function doubleBooking() {
		const outcome = intersectionOf(Ledger, [booking(7n, 1n, "s1")], [booking(9n, 4n, "s1")])
		assert.equal(outcome.tag, "conflict")
		assert.ok(outcome.tag === "conflict" && outcome.shared.some((key) => key.class === "K"))
	})

	test("different holders and different slots are fully disjoint", function disjoint() {
		const outcome = intersectionOf(Ledger, [booking(7n, 1n, "s1")], [booking(9n, 4n, "s2")])
		assert.deepEqual(outcome, { tag: "disjoint" })
	})

	test("a shared commute-cell C key still blocks the fast path — strict disjointness", function strict() {
		const outcome = intersectionOf(Ledger, [booking(7n, 1n, "s1")], [booking(9n, 1n, "s2")])
		assert.equal(outcome.tag, "conflict")
		assert.ok(outcome.tag === "conflict" && outcome.shared.some((key) => key.class === "C"))
	})

	test("a shared W parent key alone is the quantitative arm, carrying both intervals", function capacityArm() {
		const loser: BatchOp[] = [{ op: "insert", relation: "Device", rows: [[10n, 1n, 5n]] }]
		const winner: BatchOp[] = [{ op: "insert", relation: "Device", rows: [[11n, 1n, 3n]] }]
		const outcome = intersectionOf(Grid, loser, winner)
		assert.equal(outcome.tag, "capacity")
		assert.ok(outcome.tag === "capacity")
		assert.equal(outcome.parents.length, 1)
		const parent = outcome.parents[0]
		assert.ok(parent !== undefined)
		assert.deepEqual(parent.loser, { lo: 0n, hi: 5n })
		assert.deepEqual(parent.winner, { lo: 0n, hi: 3n })

		assert.equal(
			capacityCommutes(outcome.parents, function roomy() {
				return { plus: 8n, minus: null }
			}),
			true
		)
		assert.equal(
			capacityCommutes(outcome.parents, function tight() {
				return { plus: 7n, minus: null }
			}),
			false
		)
	})

	test("a parent delete against child writes is a conflict", function parentDelete() {
		const loser: BatchOp[] = [{ op: "insert", relation: "Device", rows: [[10n, 1n, 5n]] }]
		const winner: BatchOp[] = [{ op: "delete", relation: "Pool", rows: [[1n, 100n]] }]
		const outcome = intersectionOf(Grid, winner, loser)
		assert.equal(outcome.tag, "conflict")
	})

	test("insert-vs-delete of one fact identity is an F conflict", function factOrder() {
		const row = [7n, 1n, "s1", { start: 1n, end: 2n }]
		const outcome = intersectionOf(
			Ledger,
			[{ op: "insert", relation: "Booking", rows: [row] }],
			[{ op: "delete", relation: "Booking", rows: [row] }]
		)
		assert.equal(outcome.tag, "conflict")
	})
})
