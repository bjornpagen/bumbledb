import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"
import type { Db as DbValue, Fact } from "#index.ts"
import { closed, Db, InstanceBuilder, relation, schema, str, u64 } from "#index.ts"
import { accepted } from "#test/accepted.ts"
import { put } from "#test/put.ts"

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-count-"))
const storeDir = path.join(tmpRoot, "store")

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

const Kind = closed("Kind", ["Checking", "Savings"])
const Holder = relation("Holder", { id: u64.fresh, name: str })
const Unwritten = relation("Unwritten", { id: u64.fresh })
const Counted = schema("Counted", { Kind, Holder, Unwritten }, [])

function must<T>(value: T | undefined): T {
	assert.ok(value !== undefined, "expected a present value")
	return value
}

const facts: { ada?: Fact<typeof Holder> } = {}

describe("the exact count against a real store", function suite() {
	let db: DbValue<(typeof Counted)["relations"]>

	test("create admits the Counted theory", async function create() {
		db = accepted(await Db.create(storeDir, Counted))
	})

	test("count equals BigInt(scan().length) in the same lease after mixed commits", function countIsScanLength() {
		const seeded = db.write(function seed(tx) {
			facts.ada = put(tx, Holder, { name: "ada" })
			put(tx, Holder, { name: "grace" })
			put(tx, Holder, { name: "kurt" })
		})
		assert.equal(seeded.tag, "accepted")
		const churned = db.write(function churn(tx) {
			assert.equal(tx.delete(Holder, [must(facts.ada)]).changed, 1n)
			put(tx, Holder, { name: "alan" })
		})
		assert.equal(churned.tag, "accepted")
		db.read(function sameLease(instance) {
			assert.equal(
				instance.count(Holder),
				BigInt(instance.scan(Holder).length),
				"one snapshot, two reads, one cardinality"
			)
			assert.equal(instance.count(Holder), 3n)
		})
	})

	test("a held lease reports the pre-commit count; a fresh lease reports the new one", function snapshotLaw() {
		const counts = db.read(function heldLease(instance) {
			const before = instance.count(Holder)
			const landed = db.write(function interleave(tx) {
				put(tx, Holder, { name: "interleaved" })
			})
			assert.equal(landed.tag, "accepted", "the interleaved commit lands")
			return { before, held: instance.count(Holder) }
		})
		assert.equal(counts.held, counts.before, "the held lease still observes its own snapshot")
		const fresh = db.read(function freshLease(instance) {
			return instance.count(Holder)
		})
		assert.equal(fresh, counts.before + 1n, "a fresh lease observes the committed count")
	})

	test("an empty relation counts 0n — a value, never an empty result to reinterpret", function emptyIsZero() {
		const empty = db.read(function inLease(instance) {
			return instance.count(Unwritten)
		})
		assert.equal(empty, 0n)
	})

	test("a closed relation is a type error at count, and the untyped path hits the runtime wall", function closedWall() {
		assert.throws(function countClosed() {
			db.read(function inLease(instance) {
				// @ts-expect-error — Kind is closed: MemberRelation excludes it from count exactly as from scan
				instance.count(Kind)
			})
		}, /closed/)
	})

	test("OwnedInstance.count agrees with its scan of the one frozen catalog", async function ownedCount() {
		const builder = InstanceBuilder.create(Counted)
		const range = builder.reserve(Holder, "id", 2n)
		assert.equal(range.empty, false)
		builder.load(Holder, [
			{ id: must(range.at(0n)), name: "ada" },
			{ id: must(range.at(1n)), name: "grace" }
		])
		builder.delete(Holder, [{ id: must(range.at(1n)), name: "grace" }])
		const owned = accepted(await builder.admit())
		assert.equal(owned.count(Holder), BigInt(owned.scan(Holder).length), "one catalog, two reads, one cardinality")
		assert.equal(owned.count(Holder), 1n)
		assert.equal(owned.count(Unwritten), 0n)
		owned[Symbol.dispose]()
	})
})
