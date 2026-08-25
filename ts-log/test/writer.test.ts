import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"
import { internalBlake3 } from "@bjornpagen/bumbledb"
import { bytesEqual, type Digest32, digest32 } from "#bytes.ts"
import { decodeBatch } from "#codec.ts"
import { braid, descriptorOf } from "#descriptor.ts"
import { generation, logKey, manifestKey } from "#keys.ts"
import { renderManifest } from "#manifest.ts"
import { coreOf, openReplica } from "#replica.ts"
import { memStore } from "#store.ts"
import { Holder, Ledger } from "#test/fixtures.ts"
import { openWriter } from "#writer.ts"

const HOME = braid("c00000000")
const ZERO_HEX = "0".repeat(64)
const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-log-writer-"))

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

let laneCounter = 0
function lane(): { store: ReturnType<typeof memStore>; prefix: string; dir: (name: string) => string } {
	laneCounter += 1
	const base = path.join(tmpRoot, `lane-${laneCounter}`)
	return {
		store: memStore(),
		prefix: "prod/main",
		dir(name: string) {
			return path.join(base, name)
		}
	}
}

async function birthManifest(store: ReturnType<typeof memStore>, prefix: string): Promise<void> {
	const created = await store.putCreate(
		manifestKey(prefix),
		renderManifest({ fingerprint: descriptorOf(Ledger).fingerprint, checkpoint: null })
	)
	assert.equal(created.tag, "created")
}

describe("writer encode site", function suite() {
	test("a hex-string chain prev is branded Digest32 before encode", async function hexPrev() {
		const { store, prefix, dir } = lane()
		await birthManifest(store, prefix)
		const replica = await openReplica({ store, prefix, dir: dir("a"), theory: Ledger })
		const writer = openWriter(replica)
		const core = coreOf(replica)
		core.chain.set(HOME, { g: generation(0n), prev: ZERO_HEX as unknown as Digest32, ts: 0n })

		const out = await writer.commit(function record(batch) {
			batch.insert(Holder, [{ id: 1n, name: "ada" }])
			return 0
		})
		assert.equal(out.tag, "accepted")
		assert.ok(out.tag === "accepted")
		assert.equal(out.generation, 1n)

		const published = await store.get(logKey(prefix, HOME, generation(1n)))
		assert.ok(published !== null)
		const decoded = decodeBatch(descriptorOf(Ledger), published.bytes)
		assert.equal(decoded.header.prev.length, 32)
		assert.ok(bytesEqual(decoded.header.prev, new Uint8Array(32)))

		await replica[Symbol.asyncDispose]()
	})

	test("the first publish after a replica-built chain cites the predecessor", async function replicaBuilt() {
		const { store, prefix, dir } = lane()
		await birthManifest(store, prefix)
		const a = await openReplica({ store, prefix, dir: dir("a"), theory: Ledger })
		const writerA = openWriter(a)
		const first = await writerA.commit(function seed(batch) {
			batch.insert(Holder, [{ id: 1n, name: "ada" }])
			return 0
		})
		assert.ok(first.tag === "accepted")

		const b = await openReplica({ store, prefix, dir: dir("b"), theory: Ledger })
		await b.waitFor(new Map([[HOME, first.generation]]))
		const writerB = openWriter(b)
		const second = await writerB.commit(function more(batch) {
			batch.insert(Holder, [{ id: 2n, name: "bob" }])
			return 0
		})
		assert.ok(second.tag === "accepted")
		assert.equal(second.generation, 2n)

		const published = await store.get(logKey(prefix, HOME, generation(2n)))
		assert.ok(published !== null)
		const decoded = decodeBatch(descriptorOf(Ledger), published.bytes)
		const predecessor = await store.get(logKey(prefix, HOME, generation(1n)))
		assert.ok(predecessor !== null)
		assert.ok(bytesEqual(decoded.header.prev, digest32(new Uint8Array(internalBlake3(predecessor.bytes)))))

		await a[Symbol.asyncDispose]()
		await b[Symbol.asyncDispose]()
	})
})
