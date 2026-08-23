import assert from "node:assert/strict"
import { describe, test } from "node:test"
import { toHex } from "#bytes.ts"
import { descriptorOf } from "#descriptor.ts"
import type { BatchOp, FootprintEntry } from "#footprint.ts"
import { capacityIntervalsOf, fkeyOf, footprintOf } from "#footprint.ts"
import { Grid, Ledger, Vocab } from "#test/fixtures.ts"

function bookingRow(
	id: bigint,
	holder: bigint,
	slot: string
): readonly (bigint | string | { start: bigint; end: bigint })[] {
	return [id, holder, slot, { start: 1n, end: 2n }]
}

function classesOf(entries: readonly FootprintEntry[]): string[] {
	return entries.map(function tag(entry) {
		return "mode" in entry ? `${entry.class}:${entry.mode}` : entry.class
	})
}

describe("footprintOf", function suite() {
	test("a booking insert emits F+, both K keys, the C need, and the W child delta", function insertClasses() {
		const ops: BatchOp[] = [{ op: "insert", relation: "Booking", rows: [bookingRow(7n, 1n, "s1")] }]
		const entries = footprintOf(Ledger, ops)
		assert.deepEqual(classesOf(entries), ["F:insert", "K", "K", "C:need", "W:child"])
		const child = entries[4]
		assert.ok(child !== undefined && child.class === "W" && child.mode === "child")
		assert.equal(child.delta, 1n)
	})

	test("a holder insert emits F+, its K key, support+, and parent+", function targetClasses() {
		const ops: BatchOp[] = [{ op: "insert", relation: "Holder", rows: [[1n, "ada"]] }]
		assert.deepEqual(classesOf(footprintOf(Ledger, ops)), ["F:insert", "K", "C:support+", "W:parent+"])
	})

	test("a holder delete emits F-, its K key, support-, and parent-", function deleteClasses() {
		const ops: BatchOp[] = [{ op: "delete", relation: "Holder", rows: [[1n, "ada"]] }]
		assert.deepEqual(classesOf(footprintOf(Ledger, ops)), ["F:delete", "K", "C:support-", "W:parent-"])
	})

	test("a source delete emits no need entry — deleting a source only weakens", function sourceDelete() {
		const ops: BatchOp[] = [{ op: "delete", relation: "Booking", rows: [bookingRow(7n, 1n, "s1")] }]
		const entries = footprintOf(Ledger, ops)
		assert.deepEqual(classesOf(entries), ["F:delete", "K", "K", "W:child"])
		const child = entries[3]
		assert.ok(child !== undefined && child.class === "W" && child.mode === "child")
		assert.equal(child.delta, -1n)
	})

	test("W child deltas at one key merge into one signed sum", function merging() {
		const ops: BatchOp[] = [
			{ op: "insert", relation: "Booking", rows: [bookingRow(7n, 1n, "s1"), bookingRow(8n, 1n, "s2")] },
			{ op: "delete", relation: "Booking", rows: [bookingRow(9n, 1n, "s3")] }
		]
		const entries = footprintOf(Ledger, ops)
		const children = entries.filter(function child(entry) {
			return entry.class === "W" && "mode" in entry && entry.mode === "child"
		})
		assert.equal(children.length, 1)
		const child = children[0]
		assert.ok(child !== undefined && child.class === "W" && child.mode === "child")
		assert.equal(child.delta, 1n)
	})

	test("the evaporation interval widens by the batch's own F entries on weighted children", function intervals() {
		const ops: BatchOp[] = [
			{
				op: "insert",
				relation: "Device",
				rows: [
					[10n, 1n, 5n],
					[11n, 1n, 3n]
				]
			},
			{ op: "delete", relation: "Device", rows: [[12n, 1n, 2n]] }
		]
		const intervals = capacityIntervalsOf(Grid, ops)
		assert.equal(intervals.length, 1)
		const interval = intervals[0]
		assert.ok(interval !== undefined)
		assert.equal(interval.delta, 6n)
		assert.equal(interval.lo, -2n)
		assert.equal(interval.hi, 8n)
	})

	test("net disposition: the batch's last op per fact identity wins", function netting() {
		const row = bookingRow(7n, 1n, "s1")
		const ops: BatchOp[] = [
			{ op: "insert", relation: "Booking", rows: [row] },
			{ op: "delete", relation: "Booking", rows: [row] }
		]
		const entries = footprintOf(Ledger, ops)
		const fact = entries[0]
		assert.ok(fact !== undefined && fact.class === "F")
		assert.equal(fact.mode, "delete")
		assert.equal(entries.filter((entry) => entry.class === "F").length, 1)
	})

	test("closed-target statements emit nothing", function closedTargets() {
		const ops: BatchOp[] = [{ op: "insert", relation: "Account", rows: [[3n, 1n, 0n]] }]
		const entries = footprintOf(Vocab, ops)
		assert.deepEqual(classesOf(entries), ["F:insert", "K"])
	})

	test("equal determinants collide on the same fkey across writers", function stateIndependence() {
		const descriptor = descriptorOf(Ledger)
		const slotKey = descriptor.statements[3]
		assert.ok(slotKey !== undefined && slotKey.kind === "functionality")
		const booking = descriptor.relations[1]
		assert.ok(booking !== undefined)
		const a = fkeyOf(slotKey.id, booking, slotKey.projection, bookingRow(7n, 1n, "s1"))
		const b = fkeyOf(slotKey.id, booking, slotKey.projection, bookingRow(9n, 4n, "s1"))
		assert.equal(toHex(a), toHex(b))
	})

	test("ops on closed relations are refused", function closedWrites() {
		assert.throws(function writeClosed() {
			footprintOf(Vocab, [{ op: "insert", relation: "Status", rows: [[0n]] }])
		})
	})
})
