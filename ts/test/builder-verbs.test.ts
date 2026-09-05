import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"
import { Db, ErrSpentHandle, InstanceBuilder, relation, schema, str, u64 } from "#index.ts"
import { accepted } from "#test/accepted.ts"

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-builder-verbs-"))
const packageRoot = path.resolve(import.meta.dirname, "..")

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

const Holder = relation("Holder", { id: u64.fresh, name: str })
const Ledger = schema("BuilderVerbs", { Holder }, [])

describe("TS builder verb set", function suite() {
	test("the surface is load, delete, reserve, contains, get, admit, dispose", function surface() {
		const db = fs.readFileSync(path.join(packageRoot, "src/db.ts"), "utf8")
		const lib = fs.readFileSync(path.join(packageRoot, "crate/src/lib.rs"), "utf8")
		assert.match(db, /interface InstanceBuilder/)
		assert.match(db, /load<R extends MemberRelation<Rels>>/)
		assert.match(db, /delete<R extends MemberRelation<Rels>>/)
		assert.match(db, /reserve<R extends MemberRelation<Rels>>/)
		assert.match(db, /contains<R extends MemberRelation<Rels>>/)
		assert.match(db, /get<R extends MemberRelation<Rels>>\(relation: R, key: KeyFact<R>\)/)
		assert.match(db, /admit\(\): Promise<Admission/)
		assert.match(lib, /\bfn instance_builder_load\b/)
		assert.match(lib, /\bfn instance_builder_delete\b/)
		assert.match(lib, /\bfn instance_builder_reserve\b/)
		assert.match(lib, /\bfn instance_builder_contains\b/)
		assert.match(lib, /\bfn instance_builder_get\b/)
	})

	test("the column transport symbols are gone from the tree (70-deletions D1/D2)", function columnSymbolsDead() {
		const db = fs.readFileSync(path.join(packageRoot, "src/db.ts"), "utf8")
		const index = fs.readFileSync(path.join(packageRoot, "src/index.ts"), "utf8")
		const nativeSurface = fs.readFileSync(path.join(packageRoot, "src/native.ts"), "utf8")
		const lib = fs.readFileSync(path.join(packageRoot, "crate/src/lib.rs"), "utf8")
		const marshal = fs.readFileSync(path.join(packageRoot, "crate/src/marshal.rs"), "utf8")
		assert.doesNotMatch(
			db,
			/ColumnBatch<|type ColumnBatch|isColumnBatch|columnsOf/,
			"D1: the public column arm is deleted"
		)
		assert.doesNotMatch(index, /ColumnBatch/, "D1: no ColumnBatch export exists")
		assert.doesNotMatch(
			nativeSurface,
			/txInsertColumns|instanceBuilderLoadColumns/,
			"D2: the paired native crossings are deleted"
		)
		assert.doesNotMatch(
			lib,
			/tx_insert_columns|instance_builder_load_columns/,
			"D2: the bridge column verbs are deleted"
		)
		assert.doesNotMatch(
			marshal,
			/\bfn fact_columns\b|\bfn fact_rows\b/,
			"D2/D6: the column parse and the nested Vec<Vec<Value>> product are deleted"
		)
		assert.match(marshal, /\bfn accepted_collection\b/, "the one replacement: the flat accepted-collection crossing")
		assert.match(
			marshal,
			/\bfn fact_row\b/,
			"the single-fact point lane survives (20-accepted-collection pins its scope)"
		)
	})

	test("the column spelling is unrepresentable — @ts-expect-error walls (70-deletions D1)", async function columnWall() {
		const builder = InstanceBuilder.create(Ledger)
		assert.throws(function loadBatch() {
			// @ts-expect-error — ColumnBatch is deleted: Iterable<Fact<R>> is the ONE collection spelling (20-accepted-collection)
			builder.load(Holder, { id: [1n], name: ["ada"] })
		})
		const owned = accepted(await builder.admit())
		const db = await Db.fromInstance(path.join(tmpRoot, "column-wall"), owned)
		db.write(function insertColumns(tx) {
			assert.throws(function insertBatch() {
				// @ts-expect-error — the CollectionWrite union lost its ColumnBatch arm (70-deletions D1)
				tx.insert(Holder, { id: [1n], name: ["ada"] })
			})
		})
		owned[Symbol.dispose]()
	})

	test("a staged fact can be retracted before admit", async function retractThenAdmit() {
		const builder = InstanceBuilder.create(Ledger)
		assert.equal(builder.load(Holder, [{ id: 1n, name: "ada" }]).changed, 1n)
		assert.equal(builder.contains(Holder, { id: 1n, name: "ada" }), true)
		assert.deepEqual(builder.get(Holder, { id: 1n }), { id: 1n, name: "ada" })
		assert.equal(builder.delete(Holder, [{ id: 1n, name: "ada" }]).changed, 1n)
		assert.equal(builder.contains(Holder, { id: 1n, name: "ada" }), false)
		assert.equal(builder.get(Holder, { id: 1n }), undefined)
		const owned = accepted(await builder.admit())
		assert.deepEqual(owned.scan(Holder), [])
		owned[Symbol.dispose]()
	})

	test("a fresh range can be minted from TypeScript before admit", async function reserveFromTs() {
		const builder = InstanceBuilder.create(Ledger)
		const range = builder.reserve(Holder, "id", 2n)
		assert.equal(range.empty, false)
		assert.equal(range.count, 2n)
		const start = range.at(0n)
		const next = range.at(1n)
		assert.equal(typeof start, "bigint")
		assert.equal(typeof next, "bigint")
		if (typeof start !== "bigint" || typeof next !== "bigint") {
			throw new Error("reserve minted two ids")
		}
		assert.equal(next, start + 1n)
		builder.load(Holder, [
			{ id: start, name: "ada" },
			{ id: next, name: "grace" }
		])
		const owned = accepted(await builder.admit())
		assert.equal(owned.get(Holder, { id: start })?.name, "ada")
		assert.equal(owned.get(Holder, { id: next })?.name, "grace")
		owned[Symbol.dispose]()
	})

	test("bulk load crosses as one flat row-major array — no JS array per fact", async function bulkLoad() {
		const db = fs.readFileSync(path.join(packageRoot, "src/db.ts"), "utf8")
		assert.match(db, /no JS array per fact/)
		const count = 4_000
		const facts = Array.from({ length: count }, function factAt(_value, index) {
			return { id: BigInt(index + 1), name: `n${index + 1}` }
		})
		const builder = InstanceBuilder.create(Ledger)
		const report = builder.load(Holder, facts)
		assert.equal(report.submitted, BigInt(count))
		assert.equal(report.changed, BigInt(count))
		assert.equal(builder.contains(Holder, { id: 1n, name: "n1" }), true)
		const owned = accepted(await builder.admit())
		assert.equal(owned.get(Holder, { id: 1n })?.name, "n1")
		assert.equal(owned.get(Holder, { id: BigInt(count) })?.name, `n${count}`)
		owned[Symbol.dispose]()
	})

	test("WriteTx.insert rides the same flat crossing", async function txInsertFlat() {
		const builder = InstanceBuilder.create(Ledger)
		const owned = accepted(await builder.admit())
		const db = await Db.fromInstance(path.join(tmpRoot, "tx-flat"), owned)
		const minted = { first: 0n }
		db.write(function insertFacts(tx) {
			const range = tx.reserve(Holder, "id", 2n)
			const first = range.at(0n)
			const second = range.at(1n)
			assert.ok(first !== undefined && second !== undefined)
			minted.first = first
			const report = tx.insert(Holder, [
				{ id: first, name: "ada" },
				{ id: second, name: "grace" }
			])
			assert.equal(report.changed, 2n)
			assert.equal(tx.contains(Holder, { id: first, name: "ada" }), true)
		})
		assert.equal(db.read((i) => i.get(Holder, { id: minted.first }))?.name, "ada")
		owned[Symbol.dispose]()
	})

	test("nullary rows are representable on the crossing — N facts, 0 cells, exact reports", async function nullaryRows() {
		const Marker = relation("Marker", {})
		const Flags = schema("NullaryFlags", { Marker }, [])
		const builder = InstanceBuilder.create(Flags)
		const owned = accepted(await builder.admit())
		const db = await Db.fromInstance(path.join(tmpRoot, "nullary-rows"), owned)
		db.write(function insertMarkers(tx) {
			const report = tx.insert(Marker, [{}, {}, {}])
			assert.equal(report.submitted, 3n)
			assert.equal(report.changed, 1n)
			assert.equal(tx.contains(Marker, {}), true)
		})
		assert.equal(
			db.read((i) => i.contains(Marker, {})),
			true
		)
		assert.equal(
			db.read((i) => i.count(Marker)),
			1n
		)
		db.write(function deleteMarker(tx) {
			const report = tx.delete(Marker, [{}])
			assert.equal(report.submitted, 1n)
			assert.equal(report.changed, 1n)
		})
		assert.equal(
			db.read((i) => i.count(Marker)),
			0n
		)
		owned[Symbol.dispose]()
	})

	test("a spent builder refuses every verb before the native call", async function spentRefuses() {
		const builder = InstanceBuilder.create(Ledger)
		builder.load(Holder, [{ id: 1n, name: "ada" }])
		builder[Symbol.dispose]()
		function isSpent(error: unknown): boolean {
			return error instanceof Error && error instanceof ErrSpentHandle
		}
		assert.throws(function loadSpent() {
			builder.load(Holder, [{ id: 2n, name: "grace" }])
		}, isSpent)
		assert.throws(function deleteSpent() {
			builder.delete(Holder, [{ id: 1n, name: "ada" }])
		}, isSpent)
		assert.throws(function reserveSpent() {
			builder.reserve(Holder, "id", 1n)
		}, isSpent)
		assert.throws(function containsSpent() {
			builder.contains(Holder, { id: 1n, name: "ada" })
		}, isSpent)
		assert.throws(function getSpent() {
			builder.get(Holder, { id: 1n })
		}, isSpent)
		await assert.rejects(function admitSpent() {
			return builder.admit()
		}, isSpent)
	})
})
