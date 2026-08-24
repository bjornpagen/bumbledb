import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"
import * as errors from "@superbuilders/errors"
import { readSidecar, writeSidecar } from "#chain.ts"
import { braid } from "#descriptor.ts"
import { ErrContention } from "#errors.ts"
import { generation, storeKey } from "#keys.ts"
import { openReplica } from "#replica.ts"
import { memStore } from "#store.ts"

const HOME = braid("c00000000")
const NOTES = braid("c00000002")

import { Booking, Holder, Ledger } from "#test/fixtures.ts"
import { openWriter } from "#writer.ts"

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-log-e2e-"))

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

function booking(id: bigint, holder: bigint, slot: string) {
	return { id, holder, slot, at: { start: 1n, end: 2n } }
}

describe("replica and writer over the mem store", function suite() {
	test("commit publishes, a second replica replays it, waitFor delivers read-your-writes", async function commitReplay() {
		const { store, prefix, dir } = lane()
		const a = await openReplica({ store, prefix, dir: dir("a"), theory: Ledger })
		const writerA = openWriter(a)

		const out = await writerA.commit(function record(batch) {
			batch.insert(Holder, [{ id: 1n, name: "ada" }])
			const ids = batch.reserve(Booking, "id", 1n)
			const id = ids[0]
			assert.ok(id !== undefined)
			batch.insert(Booking, [booking(id, 1n, "s1")])
			return ids
		})
		assert.equal(out.tag, "accepted")
		assert.ok(out.tag === "accepted")
		assert.equal(out.braid, "c00000000")
		assert.equal(out.generation, 1n)
		assert.equal(out.durability, "published")

		const b = await openReplica({ store, prefix, dir: dir("b"), theory: Ledger })
		await b.waitFor(new Map([[out.braid, out.generation]]))
		const names = b.db.read(function readNames(instance) {
			return instance.scan(Holder).map(function nameOf(fact) {
				return fact.name
			})
		})
		assert.deepEqual(names, ["ada"])
		assert.equal(b.vector.get(HOME), 1n)
		await a[Symbol.asyncDispose]()
		await b[Symbol.asyncDispose]()
	})

	test("a rejected commit never reaches the network and surfaces typed violations", async function rejectedLocal() {
		const { store, prefix, dir } = lane()
		const a = await openReplica({ store, prefix, dir: dir("a"), theory: Ledger })
		const writer = openWriter(a)
		await writer.commit(function seed(batch) {
			batch.insert(Holder, [{ id: 1n, name: "ada" }])
			batch.insert(Booking, [booking(10n, 1n, "s1")])
			return 0
		})
		const out = await writer.commit(function collide(batch) {
			batch.insert(Booking, [booking(11n, 1n, "s1")])
			return 0
		})
		assert.equal(out.tag, "rejected")
		assert.ok(out.tag === "rejected")
		assert.equal(out.violations[0]?.kind, "functionality")
		assert.equal(a.vector.get(HOME), 1n)
		await a[Symbol.asyncDispose]()
	})

	test("reopen from the same directory serves without replay drift", async function reopen() {
		const { store, prefix, dir } = lane()
		{
			const a = await openReplica({ store, prefix, dir: dir("a"), theory: Ledger })
			const writer = openWriter(a)
			await writer.commit(function seed(batch) {
				batch.insert(Holder, [{ id: 1n, name: "ada" }])
				return 0
			})
			await a[Symbol.asyncDispose]()
		}
		const again = await openReplica({ store, prefix, dir: dir("a"), theory: Ledger })
		assert.equal(again.vector.get(HOME), 1n)
		assert.equal(
			again.db.read(function count(instance) {
				return instance.count(Holder)
			}),
			1n
		)
		await again[Symbol.asyncDispose]()
	})

	test("the crash window heals by idempotent re-apply: a rewound sidecar catches up as engine no-ops", async function crashWindow() {
		const { store, prefix, dir } = lane()
		{
			const a = await openReplica({ store, prefix, dir: dir("a"), theory: Ledger })
			const writer = openWriter(a)
			await writer.commit(function seed(batch) {
				batch.insert(Holder, [{ id: 1n, name: "ada" }])
				return 0
			})
			await writer.commit(function more(batch) {
				batch.insert(Holder, [{ id: 2n, name: "bob" }])
				return 0
			})
			await a[Symbol.asyncDispose]()
		}
		const sidecarFile = path.join(dir("a"), "chain.json")
		const sidecar = await readSidecar(sidecarFile)
		assert.ok(sidecar !== null)
		const rewound = new Map(sidecar.chain)
		rewound.set(HOME, { g: generation(0n), prev: "0".repeat(64), ts: 0n })
		await writeSidecar(sidecarFile, { chain: rewound, pending: null })

		const again = await openReplica({ store, prefix, dir: dir("a"), theory: Ledger })
		assert.equal(again.vector.get(HOME), 2n)
		assert.equal(
			again.db.read(function count(instance) {
				return instance.count(Holder)
			}),
			2n
		)
		await again[Symbol.asyncDispose]()
	})

	test("a torn sidecar fails the wholeness identity and the directory is discarded and re-pulled", async function wholenessDiscard() {
		const { store, prefix, dir } = lane()
		{
			const a = await openReplica({ store, prefix, dir: dir("a"), theory: Ledger })
			const writer = openWriter(a)
			await writer.commit(function seed(batch) {
				batch.insert(Holder, [{ id: 1n, name: "ada" }])
				return 0
			})
			await a[Symbol.asyncDispose]()
		}
		const sidecarFile = path.join(dir("a"), "chain.json")
		const sidecar = await readSidecar(sidecarFile)
		assert.ok(sidecar !== null)
		const torn = new Map(sidecar.chain)
		torn.set(NOTES, { g: generation(5n), prev: "1".repeat(64), ts: 0n })
		await writeSidecar(sidecarFile, { chain: torn, pending: null })

		const again = await openReplica({ store, prefix, dir: dir("a"), theory: Ledger })
		assert.equal(again.vector.get(HOME), 1n)
		assert.equal(again.vector.get(NOTES), 0n)
		assert.equal(
			again.db.read(function count(instance) {
				return instance.count(Holder)
			}),
			1n
		)
		await again[Symbol.asyncDispose]()
	})

	test("the double-mint K-conflict re-judges into the serial rejection — the case-5 story", async function doubleMint() {
		const { store, prefix, dir } = lane()
		const a = await openReplica({ store, prefix, dir: dir("a"), theory: Ledger })
		const b = await openReplica({ store, prefix, dir: dir("b"), theory: Ledger })
		const writerA = openWriter(a)
		const writerB = openWriter(b)

		await writerA.commit(function seed(batch) {
			batch.insert(Holder, [{ id: 1n, name: "ada" }])
			return 0
		})
		await b.refresh()

		const winner = await writerA.commit(function mint(batch) {
			batch.insert(Booking, [booking(100n, 1n, "s1")])
			return 0
		})
		assert.ok(winner.tag === "accepted")

		const loser = await writerB.commit(function mint(batch) {
			batch.insert(Booking, [booking(200n, 1n, "s1")])
			return 0
		})
		assert.equal(loser.tag, "rejected")
		assert.ok(loser.tag === "rejected")
		assert.equal(loser.violations[0]?.kind, "functionality")

		const slots = b.db.read(function readSlots(instance) {
			return instance.scan(Booking).map(function slotOf(fact) {
				return [fact.id, fact.slot] as const
			})
		})
		assert.deepEqual(slots, [[100n, "s1"]])
		await a[Symbol.asyncDispose]()
		await b[Symbol.asyncDispose]()
	})

	test("a disjoint-shaped loss re-judges at the tip and lands a fresh header at tip+1", async function disjointLoss() {
		const { store, prefix, dir } = lane()
		const a = await openReplica({ store, prefix, dir: dir("a"), theory: Ledger })
		const b = await openReplica({ store, prefix, dir: dir("b"), theory: Ledger })
		const writerA = openWriter(a)
		const writerB = openWriter(b)

		await writerA.commit(function seed(batch) {
			batch.insert(Holder, [
				{ id: 1n, name: "ada" },
				{ id: 2n, name: "bob" }
			])
			return 0
		})
		await b.refresh()

		const winner = await writerA.commit(function mint(batch) {
			batch.insert(Booking, [booking(100n, 1n, "s1")])
			return 0
		})
		assert.ok(winner.tag === "accepted" && winner.generation === 2n)

		const published = await writerB.commit(function mint(batch) {
			batch.insert(Booking, [booking(200n, 2n, "s2")])
			return 0
		})
		assert.ok(published.tag === "accepted")
		assert.equal(published.generation, 3n)
		assert.equal(published.durability, "published")

		await a.refresh()
		const slots = a.db.read(function readSlots(instance) {
			return instance
				.scan(Booking)
				.map(function slotOf(fact) {
					return fact.slot
				})
				.sort()
		})
		assert.deepEqual(slots, ["s1", "s2"])
		await a[Symbol.asyncDispose]()
		await b[Symbol.asyncDispose]()
	})

	test("a net-noop-shaped loss re-judges to a net no-op: Accepted at the current generation, nothing published", async function netNoopLoss() {
		const { store, prefix, dir } = lane()
		const a = await openReplica({ store, prefix, dir: dir("a"), theory: Ledger })
		const b = await openReplica({ store, prefix, dir: dir("b"), theory: Ledger })
		const writerA = openWriter(a)
		const writerB = openWriter(b)

		const winner = await writerA.commit(function mint(batch) {
			batch.insert(Holder, [{ id: 7n, name: "ada" }])
			return 0
		})
		assert.ok(winner.tag === "accepted" && winner.generation === 1n)

		const absorbed = await writerB.commit(function mint(batch) {
			batch.insert(Holder, [{ id: 7n, name: "ada" }])
			return 0
		})
		assert.ok(absorbed.tag === "accepted")
		assert.equal(absorbed.generation, 1n)
		assert.equal(absorbed.durability, "published")

		const second = await store.get(storeKey("prod/main/log/c00000000/0000000000000002"))
		assert.equal(second, null)
		assert.equal(
			b.db.read(function count(instance) {
				return instance.count(Holder)
			}),
			1n
		)
		await a[Symbol.asyncDispose]()
		await b[Symbol.asyncDispose]()
	})

	test("commitSplit returns the per-braid outcome vector", async function split() {
		const { store, prefix, dir } = lane()
		const a = await openReplica({ store, prefix, dir: dir("a"), theory: Ledger })
		const writer = openWriter(a)
		const out = await writer.commitSplit(function record(batch) {
			batch.insert(Holder, [{ id: 1n, name: "ada" }])
			batch.insert(Ledger.relations.Note, [{ id: 1n, body: "memo" }])
			return "both"
		})
		assert.equal(out.value, "both")
		assert.equal(out.outcomes.length, 2)
		assert.deepEqual(
			out.outcomes.map(function braidOf(outcome) {
				return [outcome.braid, outcome.tag]
			}),
			[
				["c00000000", "accepted"],
				["c00000002", "accepted"]
			]
		)
		await a[Symbol.asyncDispose]()
	})

	test("a stale writer forty slots behind resolves through one re-open and one race at the tip", async function staleForty() {
		const { store, prefix, dir } = lane()
		const a = await openReplica({ store, prefix, dir: dir("a"), theory: Ledger })
		const b = await openReplica({ store, prefix, dir: dir("b"), theory: Ledger })
		const writerA = openWriter(a)
		const writerB = openWriter(b)

		await writerA.commit(function seed(batch) {
			batch.insert(Holder, [{ id: 1n, name: "ada" }])
			return 0
		})
		await b.refresh()

		for (let round = 0; round < 40; round++) {
			const out = await writerA.commit(function mint(batch) {
				batch.insert(Holder, [{ id: 100n + BigInt(round), name: `h${round}` }])
				return 0
			})
			assert.ok(out.tag === "accepted")
		}
		assert.equal(a.vector.get(HOME), 41n)

		const stale = await writerB.commit(function mint(batch) {
			batch.insert(Holder, [{ id: 999n, name: "zed" }])
			return 0
		})
		assert.ok(stale.tag === "accepted", "forty historical losses count nothing against the live bound")
		assert.equal(stale.generation, 42n, "exactly one race at the tip: the re-judged batch lands at tip+1")
		assert.equal(stale.durability, "published")
		assert.equal(b.vector.get(HOME), 42n)
		const beyond = await store.get(storeKey("prod/main/log/c00000000/000000000000002b"))
		assert.equal(beyond, null, "no slot beyond the single republication exists")
		assert.equal(
			b.db.read(function count(instance) {
				return instance.count(Holder)
			}),
			42n
		)
		await a[Symbol.asyncDispose]()
		await b[Symbol.asyncDispose]()
	})

	test("open sweeps dead rotated store dirs, keeping only the adopted one", async function sweep() {
		const { store, prefix, dir } = lane()
		{
			const a = await openReplica({ store, prefix, dir: dir("a"), theory: Ledger })
			const writer = openWriter(a)
			await writer.commit(function seed(batch) {
				batch.insert(Holder, [{ id: 1n, name: "ada" }])
				return 0
			})
			await a[Symbol.asyncDispose]()
		}
		fs.mkdirSync(path.join(dir("a"), "store-corpse-1"), { recursive: true })
		fs.writeFileSync(path.join(dir("a"), "store-corpse-1", "data.mdb"), "dead")
		fs.utimesSync(path.join(dir("a"), "store-corpse-1"), 0, 0)

		const again = await openReplica({ store, prefix, dir: dir("a"), theory: Ledger })
		const rotations = fs.readdirSync(dir("a")).filter(function stores(name) {
			return name.startsWith("store-")
		})
		assert.equal(rotations.length, 1)
		assert.notEqual(rotations[0], "store-corpse-1")
		assert.equal(again.vector.get(HOME), 1n)
		await again[Symbol.asyncDispose]()
	})

	test("a spanning commit is a typed refusal naming the verb boundary", async function spanning() {
		const { store, prefix, dir } = lane()
		const a = await openReplica({ store, prefix, dir: dir("a"), theory: Ledger })
		const writer = openWriter(a)
		const caught = await errors.try(
			writer.commit(function record(batch) {
				batch.insert(Holder, [{ id: 1n, name: "ada" }])
				batch.insert(Ledger.relations.Note, [{ id: 1n, body: "memo" }])
				return 0
			})
		)
		assert.ok(caught.error)
		assert.ok(!errors.is(caught.error, ErrContention))
		await a[Symbol.asyncDispose]()
	})
})
