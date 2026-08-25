import assert from "node:assert/strict"
import { describe, test } from "node:test"
import { holdReplica } from "./handle.ts"

describe("the replica as a value", function suite() {
	test("a successful open is held as a value across acquires", async function held() {
		let opens = 0
		const acquire = holdReplica(async function open() {
			opens += 1
			return { n: opens }
		})
		const first = await acquire()
		const second = await acquire()
		assert.deepEqual(first, { tag: "live", value: { n: 1 } })
		assert.deepEqual(second, { tag: "live", value: { n: 1 } })
		assert.equal(opens, 1)
	})

	test("a failed open leaves the value absent so the next acquire retries", async function retries() {
		let opens = 0
		const acquire = holdReplica(async function open() {
			opens += 1
			if (opens === 1) {
				throw new Error("cold open refused")
			}
			return { n: opens }
		})
		const first = await acquire()
		const second = await acquire()
		assert.deepEqual(first, { tag: "unavailable", status: 503, reason: "replica is unavailable" })
		assert.deepEqual(second, { tag: "live", value: { n: 2 } })
		assert.equal(opens, 2)
	})

	test("concurrent acquires share one in-flight open", async function share() {
		let opens = 0
		let release!: (value: { n: number }) => void
		const gate = new Promise<{ n: number }>(function executor(resolve) {
			release = resolve
		})
		const acquire = holdReplica(async function open() {
			opens += 1
			return gate
		})
		const first = acquire()
		const second = acquire()
		release({ n: 1 })
		assert.deepEqual(await first, { tag: "live", value: { n: 1 } })
		assert.deepEqual(await second, { tag: "live", value: { n: 1 } })
		assert.equal(opens, 1)
	})
})
