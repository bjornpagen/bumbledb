import assert from "node:assert/strict"
import { test } from "node:test"
import { bytes, str, u64 } from "#fields.ts"
import { relation } from "#relation.ts"
import { flatRowsOf } from "#rows.ts"

test("flat projection keeps only row count and owned cells, without obsolete quota accounting", () => {
	const Item = relation("Item", { id: u64, name: str, payload: bytes(2) })
	const payload = new Uint8Array([3, 4])
	let pulls = 0
	function* facts() {
		pulls += 1
		yield { id: 1n, name: "first", payload }
		pulls += 1
		yield { id: 2n, name: "second", payload }
	}
	const flat = flatRowsOf(Item.data, facts())
	payload.fill(0)
	assert.equal(pulls, 2)
	assert.deepEqual(flat, {
		rows: 2n,
		cells: [1n, "first", new Uint8Array([3, 4]), 2n, "second", new Uint8Array([3, 4])]
	})
})

test("flat projection closes the source iterator when a row is invalid", () => {
	const Item = relation("Item", { id: u64 })
	let closed = 0
	function* facts() {
		try {
			yield { id: 1n }
			yield { id: "invalid" }
			assert.fail("must not pull past the invalid row")
		} finally {
			closed += 1
		}
	}
	assert.throws(() => flatRowsOf(Item.data, facts()))
	assert.equal(closed, 1)
})
