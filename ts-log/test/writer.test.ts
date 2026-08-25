import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"
import { internalBlake3 } from "@bjornpagen/bumbledb"
import * as errors from "@superbuilders/errors"
import { bytesEqual, digest32 } from "#bytes.ts"
import { chainSum } from "#chain.ts"
import type { Op } from "#codec.ts"
import { decodeBatch, encodeBatch } from "#codec.ts"
import { braid, descriptorOf } from "#descriptor.ts"
import { ErrManifestMissing, ErrSlotRetired, slotRetiredOf } from "#errors.ts"
import { generation, logKey } from "#keys.ts"
import type { CheckpointFacts } from "#manifest.ts"
import {
	belowFloor,
	clearPending,
	coreOf,
	entriesOf,
	foldPending,
	generationOf,
	holdPending,
	openReplica
} from "#replica.ts"
import { memStore } from "#store.ts"
import { Holder, Ledger } from "#test/fixtures.ts"
import { openWriter } from "#writer.ts"

const HOME = braid("c00000000")
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

describe("writer encode site", function suite() {
	test("the writer births an empty store; a replica alone refuses ManifestMissing", async function exclusiveBirth() {
		const { store, prefix, dir } = lane()
		const missing = await errors.try(openReplica({ store, prefix, dir: dir("reader-cold"), theory: Ledger }))
		assert.ok(missing.error)
		assert.ok(errors.is(missing.error, ErrManifestMissing))

		const writer = await openWriter({ store, prefix, dir: dir("a"), theory: Ledger })
		assert.equal(writer.role, "writer")
		const out = await writer.commit(function record(batch) {
			batch.insert(Holder, [{ id: 1n, name: "ada" }])
			return 0
		})
		assert.equal(out.tag, "accepted")
		assert.ok(out.tag === "accepted")

		const reader = await openReplica({ store, prefix, dir: dir("reader"), theory: Ledger })
		await reader.waitFor(new Map([[HOME, out.generation]]))
		assert.equal(reader.vector.get(HOME), 1n)
		await writer.replica[Symbol.asyncDispose]()
		await reader[Symbol.asyncDispose]()
	})

	test("openWriter on a replica wraps and settles without birthing", async function wrapBorn() {
		const { store, prefix, dir } = lane()
		const born = await openWriter({ store, prefix, dir: dir("a"), theory: Ledger })
		await born.replica[Symbol.asyncDispose]()
		const replica = await openReplica({ store, prefix, dir: dir("b"), theory: Ledger })
		const writer = await openWriter(replica)
		assert.equal(writer.role, "writer")
		assert.equal(writer.replica, replica)
		const out = await writer.commit(function record(batch) {
			batch.insert(Holder, [{ id: 1n, name: "ada" }])
			return 0
		})
		assert.equal(out.tag, "accepted")
		await replica[Symbol.asyncDispose]()
	})

	test("a Digest32 chain prev encodes as 32 predecessor bytes", async function digestPrev() {
		const { store, prefix, dir } = lane()
		const writer = await openWriter({ store, prefix, dir: dir("a"), theory: Ledger })
		const replica = writer.replica
		const core = coreOf(replica)
		entriesOf(core).set(HOME, { g: generation(0n), prev: digest32(new Uint8Array(32)), ts: 0n })

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
		const writerA = await openWriter({ store, prefix, dir: dir("a"), theory: Ledger })
		const a = writerA.replica
		const first = await writerA.commit(function seed(batch) {
			batch.insert(Holder, [{ id: 1n, name: "ada" }])
			return 0
		})
		assert.ok(first.tag === "accepted")

		const writerB = await openWriter({ store, prefix, dir: dir("b"), theory: Ledger })
		const b = writerB.replica
		await b.waitFor(new Map([[HOME, first.generation]]))
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

const ZERO_DIGEST = digest32(new Uint8Array(32))

function floorFacts(homeG: bigint): CheckpointFacts {
	const braids = new Map()
	let sum = 0n
	for (const id of descriptorOf(Ledger).braidMembers.keys()) {
		const g = id === HOME ? homeG : 0n
		braids.set(id, { g: generation(g), hash: ZERO_DIGEST, ts: 0n })
		sum += g
	}
	return { braids, catalog: ZERO_DIGEST, writer: 0n, prev: null, sum }
}

describe("the floor is a write-path invariant", function suite() {
	test("foldPending names BelowFloor before any occupant arm", function foldTable() {
		const ours = new Uint8Array([1, 2, 3])
		const theirs = new Uint8Array([4, 5, 6])
		assert.equal(foldPending(0n, 0n, null, ours, true).tag, "below-floor")
		assert.equal(foldPending(0n, 0n, theirs, ours, true).tag, "below-floor")
		assert.equal(foldPending(3n, 3n, ours, ours, false).tag, "ours")
		assert.equal(foldPending(3n, 3n, theirs, ours, false).tag, "theirs-unapplied")
		assert.equal(foldPending(3n, 4n, theirs, ours, false).tag, "theirs-applied")
		assert.equal(foldPending(3n, 3n, null, ours, false).tag, "absent-unapplied")
		assert.equal(foldPending(3n, 4n, null, ours, false).tag, "absent-applied")
		assert.equal(foldPending(3n, 6n, null, ours, false).tag, "phantom")
	})

	test("putCreate below the floor is refused SlotRetired — a swept slot cannot be recreated", async function retired() {
		const { store, prefix, dir } = lane()
		const writer = await openWriter({ store, prefix, dir: dir("a"), theory: Ledger })
		const core = coreOf(writer.replica)
		core.checkpoint = floorFacts(5n)
		const caught = await errors.try(
			writer.commit(function record(batch) {
				batch.insert(Holder, [{ id: 1n, name: "ada" }])
				return 0
			})
		)
		assert.ok(caught.error)
		assert.ok(errors.is(caught.error, ErrSlotRetired))
		assert.equal(slotRetiredOf(caught.error)?.braid, HOME)
		assert.equal(slotRetiredOf(caught.error)?.slot, 1n)
		assert.equal(await store.get(logKey(prefix, HOME, generation(1n))), null)
		await writer.replica[Symbol.asyncDispose]()
	})

	test("a pending slot the floor already covers is published (Clear), not re-judged", async function belowFloorPublished() {
		const { store, prefix, dir } = lane()
		const writer = await openWriter({ store, prefix, dir: dir("a"), theory: Ledger })
		const seeded = await writer.commit(function seed(batch) {
			batch.insert(Holder, [{ id: 1n, name: "ada" }])
			return 0
		})
		assert.ok(seeded.tag === "accepted")

		const core = coreOf(writer.replica)
		const descriptor = descriptorOf(Ledger)
		const ops: Op[] = [{ op: "insert", relation: "Holder", rows: [[2n, "bob"]] }]
		const zombie = encodeBatch(
			descriptor,
			{
				fingerprint: digest32(descriptor.fingerprintBytes),
				braid: HOME,
				braidGen: generation(1n),
				prev: digest32(new Uint8Array(32)),
				writer: 42n,
				timestamp: 1n
			},
			ops
		)
		holdPending(core, { braid: HOME, gen: generation(1n), bytes: zombie }, ops)
		core.checkpoint = floorFacts(1n)
		assert.equal(
			foldPending(chainSum(core.chain), generationOf(core), null, zombie, belowFloor(core, HOME, generation(1n))).tag,
			"below-floor"
		)
		await clearPending(core)
		assert.equal(core.chain.tag, "settled")
		assert.equal(
			writer.replica.db.read(function count(instance) {
				return instance.count(Holder)
			}),
			1n,
			"bob is not applied — Clear does not re-judge"
		)
		assert.equal(await store.get(logKey(prefix, HOME, generation(2n))), null)
		await writer.replica[Symbol.asyncDispose]()
	})

	test("the resolve sites Clear on BelowFloor and never re-judge", function sites() {
		const replica = fs.readFileSync(path.resolve(import.meta.dirname, "../src/replica.ts"), "utf8")
		const writer = fs.readFileSync(path.resolve(import.meta.dirname, "../src/writer.ts"), "utf8")
		for (const name of ["async function resolvePendingAtOpen", "async function resolveColdPending"]) {
			const start = replica.indexOf(name)
			assert.ok(start !== -1, name)
			const body = replica.slice(start, start + 800)
			assert.ok(body.includes('tag === "below-floor"'), `${name} matches BelowFloor`)
			assert.ok(body.includes("await settle(core)"), `${name} Clears`)
		}
		const inherited = writer.slice(writer.indexOf("async function settleInheritedPending"))
		assert.ok(inherited.includes('tag === "below-floor"'))
		assert.ok(inherited.includes("await clearPending(core)"))
	})
})
