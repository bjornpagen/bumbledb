import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"
import { internalBlake3, type SchemaRelations, type WriteTx } from "@bjornpagen/bumbledb"
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
import { type Batch, openWriter } from "#writer.ts"

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
		await reader.waitFor(new Map([[HOME, out.value.slot]]))
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
		assert.equal(out.value.slot, 1n)

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
		await b.waitFor(new Map([[HOME, first.value.slot]]))
		const second = await writerB.commit(function more(batch) {
			batch.insert(Holder, [{ id: 2n, name: "bob" }])
			return 0
		})
		assert.ok(second.tag === "accepted")
		assert.equal(second.value.slot, 2n)

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

describe("one vocabulary", function suite() {
	test("the recorder is the engine write surface: a WriteTx serves as a Batch", function subtype() {
		function adopt<Rels extends SchemaRelations>(tx: WriteTx<Rels>): Batch<Rels> {
			return tx
		}
		assert.equal(typeof adopt, "function")
	})

	test("the empty commit is not a commit: a distinct outcome, never a slot", async function emptyCommit() {
		const { store, prefix, dir } = lane()
		const writer = await openWriter({ store, prefix, dir: dir("a"), theory: Ledger })
		const out = await writer.commit(function nothing() {
			return 7
		})
		assert.deepEqual(out, { tag: "empty", value: 7 })
		const split = await writer.commitSplit(function quiet() {
			return "still"
		})
		assert.deepEqual(split, { tag: "empty", value: "still" })
		assert.equal(await store.get(logKey(prefix, HOME, generation(1n))), null)
		await writer.replica[Symbol.asyncDispose]()
	})

	test("reserve speaks the engine's FreshRange: zero is Empty, a draw is contiguous", async function freshRange() {
		const { store, prefix, dir } = lane()
		const writer = await openWriter({ store, prefix, dir: dir("a"), theory: Ledger })
		const out = await writer.commit(function record(batch) {
			const none = batch.reserve(Holder, "id", 0n)
			assert.ok(none.empty)
			assert.equal(none.count, 0n)
			assert.deepEqual([...none], [])
			const drawn = batch.reserve(Holder, "id", 3n)
			assert.ok(!drawn.empty)
			assert.equal(drawn.count, 3n)
			assert.equal(drawn.endExclusive - drawn.start, 3n)
			assert.deepEqual([...drawn], [drawn.start, drawn.start + 1n, drawn.start + 2n])
			assert.equal(drawn.at(1n), drawn.start + 1n)
			assert.equal(drawn.at(3n), undefined)
			const report = batch.insert(Holder, [{ id: drawn.start, name: "ada" }])
			assert.equal(report.submitted, 1n)
			assert.equal(report.changed, 0n, "the recorder applies nothing; change is judged at commit")
			return drawn.start
		})
		assert.ok(out.tag === "accepted")
		assert.equal(out.value.value, 0n)
		assert.equal(out.value.slot, 1n)
		assert.equal(out.value.durability, "published")
		await writer.replica[Symbol.asyncDispose]()
	})

	test("a draw the cached pool cannot serve refills the lease — never Exhausted", async function refill() {
		const { store, prefix, dir } = lane()
		const writer = await openWriter({ store, prefix, dir: dir("a"), theory: Ledger })
		const out = await writer.commit(function burst(batch) {
			const first = batch.reserve(Holder, "id", 4096n)
			const second = batch.reserve(Holder, "id", 1n)
			assert.ok(!first.empty)
			assert.ok(!second.empty)
			batch.insert(Holder, [{ id: second.start, name: "ada" }])
			return { first: first.start, second: second.start }
		})
		assert.ok(out.tag === "accepted")
		const ids = out.value.value
		assert.ok(
			ids.second < ids.first || ids.second >= ids.first + 4096n,
			"the refilled draw is outside the abandoned block — unique, never dense"
		)
		await writer.replica[Symbol.asyncDispose]()
	})

	test("commitSplit outcomes are the engine's Admission beside the braid", async function splitAdmission() {
		const { store, prefix, dir } = lane()
		const writer = await openWriter({ store, prefix, dir: dir("a"), theory: Ledger })
		const split = await writer.commitSplit(function record(batch) {
			batch.insert(Holder, [{ id: 1n, name: "ada" }])
			return 0
		})
		assert.ok(split.tag === "split")
		assert.equal(split.value, 0)
		assert.equal(split.outcomes.length, 1)
		const outcome = split.outcomes[0]
		assert.ok(outcome !== undefined)
		assert.equal(outcome.braid, HOME)
		assert.ok(outcome.admission.tag === "accepted")
		assert.equal(outcome.admission.value.slot, 1n)
		assert.equal(outcome.admission.value.durability, "published")
		await writer.replica[Symbol.asyncDispose]()
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
				braid: HOME,
				braidGen: generation(1n),
				prev: digest32(new Uint8Array(32)),
				writer: 42n,
				timestamp: 1n
			},
			ops
		)
		holdPending(core, { braid: HOME, slot: generation(1n), bytes: zombie }, ops, 1n)
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
