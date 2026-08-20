/**
 * Issue 04: the TS builder is the engine verb set — load (objects and
 * columns), delete, reserve, contains, get, admit. A staged fact can be
 * retracted and a fresh range minted before admit. Column load allocates
 * no per-row JS array.
 */

import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"
import * as errors from "@superbuilders/errors"
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
		assert.match(lib, /\bfn instance_builder_delete\b/)
		assert.match(lib, /\bfn instance_builder_reserve\b/)
		assert.match(lib, /\bfn instance_builder_contains\b/)
		assert.match(lib, /\bfn instance_builder_get\b/)
		assert.match(lib, /\bfn instance_builder_load_columns\b/)
		assert.match(lib, /\bfn tx_insert_columns\b/)
		const marshal = fs.readFileSync(path.join(packageRoot, "crate/src/marshal.rs"), "utf8")
		assert.match(marshal, /\bfn fact_columns\b/, "column transport is marshal parse-all-first")
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
		assert.equal(next, start! + 1n)
		builder.load(Holder, [
			{ id: start!, name: "ada" },
			{ id: next!, name: "grace" }
		])
		const owned = accepted(await builder.admit())
		assert.equal(owned.get(Holder, { id: start! })?.name, "ada")
		assert.equal(owned.get(Holder, { id: next! })?.name, "grace")
		owned[Symbol.dispose]()
	})

	test("bulk load via columns allocates no per-row JS array", async function columnLoad() {
		const db = fs.readFileSync(path.join(packageRoot, "src/db.ts"), "utf8")
		assert.match(db, /never a JS array per row/)
		const columnsFn = db.slice(db.indexOf("function columnsOf"), db.indexOf("function mutateCollection"))
		assert.doesNotMatch(columnsFn, /\browOf\b/, "the column path never materializes a row array")
		const count = 4_000
		const ids = Array.from({ length: count }, function idAt(_value, index) {
			return BigInt(index + 1)
		})
		const names = ids.map(function nameAt(id) {
			return `n${id}`
		})
		const builder = InstanceBuilder.create(Ledger)
		const report = builder.load(Holder, { id: ids, name: names })
		assert.equal(report.submitted, BigInt(count))
		assert.equal(report.changed, BigInt(count))
		assert.equal(builder.contains(Holder, { id: 1n, name: "n1" }), true)
		const owned = accepted(await builder.admit())
		assert.equal(owned.get(Holder, { id: 1n })?.name, "n1")
		assert.equal(owned.get(Holder, { id: BigInt(count) })?.name, `n${count}`)
		owned[Symbol.dispose]()
	})

	test("WriteTx.insert accepts the same column transport", async function txInsertColumns() {
		const builder = InstanceBuilder.create(Ledger)
		const owned = accepted(await builder.admit())
		const db = await Db.fromInstance(path.join(tmpRoot, "tx-cols"), owned)
		const minted = { first: 0n }
		db.write(function insertColumns(tx) {
			const range = tx.reserve(Holder, "id", 2n)
			const first = range.at(0n)
			const second = range.at(1n)
			assert.ok(first !== undefined && second !== undefined)
			minted.first = first
			const report = tx.insert(Holder, { id: [first, second], name: ["ada", "grace"] })
			assert.equal(report.changed, 2n)
			assert.equal(tx.contains(Holder, { id: first, name: "ada" }), true)
		})
		assert.equal(db.get(Holder, { id: minted.first })?.name, "ada")
		owned[Symbol.dispose]()
	})

	test("a spent builder refuses every verb before the native call", async function spentRefuses() {
		const builder = InstanceBuilder.create(Ledger)
		builder.load(Holder, [{ id: 1n, name: "ada" }])
		builder[Symbol.dispose]()
		function isSpent(error: unknown): boolean {
			return error instanceof Error && errors.is(error, ErrSpentHandle)
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
