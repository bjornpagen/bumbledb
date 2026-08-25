import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"
import { digest32 } from "#bytes.ts"
import { readSidecar, writeSidecar } from "#chain.ts"
import type { Op } from "#codec.ts"
import { encodeBatch } from "#codec.ts"
import { braid, descriptorOf } from "#descriptor.ts"
import { generation, storeKey } from "#keys.ts"
import { openReplica } from "#replica.ts"
import { memStore } from "#store.ts"
import { Holder, Ledger } from "#test/fixtures.ts"
import { openWriter } from "#writer.ts"

const HOME = braid("c00000000")
const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-log-recovery-"))

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

describe("pending recovery (60)", function suite() {
	test("a resurrected pending applies at open, is re-addressed, and publishes on the next commit", async function unpublished() {
		const { store, prefix, dir } = lane()
		{
			const writer = await openWriter({ store, prefix, dir: dir("a"), theory: Ledger })
			const a = writer.replica
			await writer.commit(function seed(batch) {
				batch.insert(Holder, [{ id: 1n, name: "ada" }])
				return 0
			})
			await a[Symbol.asyncDispose]()
		}
		const sidecarFile = path.join(dir("a"), "chain.json")
		const sidecar = await readSidecar(sidecarFile)
		assert.equal(sidecar.tag, "read")
		const descriptor = descriptorOf(Ledger)
		const entry = sidecar.chain.entries.get(HOME)
		assert.ok(entry !== undefined)
		const ops: Op[] = [{ op: "insert", relation: "Holder", rows: [[2n, "bob"]] }]
		const bytes = encodeBatch(
			Ledger,
			{
				fingerprint: digest32(descriptor.fingerprintBytes),
				braid: HOME,
				braidGen: generation(entry.g + 1n),
				prev: entry.prev,
				writer: 42n,
				timestamp: entry.ts + 1n
			},
			ops
		)
		await writeSidecar(sidecarFile, {
			tag: "pending",
			entries: sidecar.chain.entries,
			batch: { braid: HOME, gen: generation(entry.g + 1n), bytes }
		})

		const again = await openReplica({ store, prefix, dir: dir("a"), theory: Ledger })
		assert.equal(
			again.db.read(function count(instance) {
				return instance.count(Holder)
			}),
			2n
		)
		assert.equal(again.vector.get(HOME), 1n)
		await again[Symbol.asyncDispose]()

		const writer = await openWriter({ store, prefix, dir: dir("a"), theory: Ledger })
		const out = await writer.commit(function more(batch) {
			batch.insert(Holder, [{ id: 3n, name: "eve" }])
			return 0
		})
		assert.ok(out.tag === "accepted")
		assert.equal(out.generation, 3n)

		const b = await openReplica({ store, prefix, dir: dir("b"), theory: Ledger })
		assert.equal(
			b.db.read(function count(instance) {
				return instance.count(Holder)
			}),
			3n
		)
		await writer.replica[Symbol.asyncDispose]()
		await b[Symbol.asyncDispose]()
	})

	test("a pending whose slot already holds our bytes is absorbed, never re-published", async function published() {
		const { store, prefix, dir } = lane()
		{
			const writer = await openWriter({ store, prefix, dir: dir("a"), theory: Ledger })
			const a = writer.replica
			await writer.commit(function seed(batch) {
				batch.insert(Holder, [{ id: 1n, name: "ada" }])
				return 0
			})
			await a[Symbol.asyncDispose]()
		}
		const sidecarFile = path.join(dir("a"), "chain.json")
		const sidecar = await readSidecar(sidecarFile)
		assert.equal(sidecar.tag, "read")
		const published = await store.get(storeKey("prod/main/log/c00000000/0000000000000001"))
		assert.ok(published !== null)
		const rewound = new Map(sidecar.chain.entries)
		rewound.set(HOME, { g: generation(0n), prev: digest32(new Uint8Array(32)), ts: 0n })
		await writeSidecar(sidecarFile, {
			tag: "pending",
			entries: rewound,
			batch: { braid: HOME, gen: generation(1n), bytes: published.bytes }
		})

		const again = await openReplica({ store, prefix, dir: dir("a"), theory: Ledger })
		assert.equal(again.vector.get(HOME), 1n)
		assert.equal(
			again.db.read(function count(instance) {
				return instance.count(Holder)
			}),
			1n
		)
		const second = await store.get(storeKey("prod/main/log/c00000000/0000000000000002"))
		assert.equal(second, null)
		await again[Symbol.asyncDispose]()
	})
})
