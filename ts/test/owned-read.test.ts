/**
 * Issue 03: one way to read an owned instance — direct native methods,
 * no lease, no `read(fn)`. A hot get loop mints no per-call handle.
 * A generic host typed against the shared read surface compiles against
 * both `ReadInstance` and `OwnedInstance`.
 */

import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"
import type {
	Fact,
	KeyFact,
	MemberRelation,
	OwnedInstance,
	ParamsRecord,
	Prepared,
	ReadInstance,
	SchemaRelations
} from "#index.ts"
import { Db, InstanceBuilder, relation, schema, str, u64 } from "#index.ts"
import { accepted } from "#test/accepted.ts"

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-owned-read-"))
const packageRoot = path.resolve(import.meta.dirname, "..")

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

const Holder = relation("Holder", { id: u64.fresh, name: str })
const Ledger = schema("OwnedRead", { Holder }, [])
type Rels = (typeof Ledger)["relations"]

/**
 * The shared read surface both arms structurally satisfy. Assigning a
 * `ReadInstance` and an `OwnedInstance` here is the type-level gate.
 */
function againstSharedRead<Rels extends SchemaRelations>(host: {
	scan<R extends MemberRelation<Rels>>(relation: R): Fact<R>[]
	contains<R extends MemberRelation<Rels>>(relation: R, fact: Fact<R>): boolean
	get<R extends MemberRelation<Rels>>(relation: R, key: KeyFact<R>): Fact<R> | undefined
	execute<Row, Params extends ParamsRecord>(prepared: Prepared<Rels, Row, Params>, params: Params): Row[]
}): void {
	void host
}

describe("one way to read an owned instance", function suite() {
	test("owned_read and OwnedInstance.read are gone; the five direct entries exist", function gone() {
		const lib = fs.readFileSync(path.join(packageRoot, "crate/src/lib.rs"), "utf8")
		assert.doesNotMatch(lib, /\bfn owned_read\b/, "owned_read is deleted")
		assert.match(lib, /\bfn owned_scan\b/)
		assert.match(lib, /\bfn owned_contains\b/)
		assert.match(lib, /\bfn owned_get\b/)
		assert.match(lib, /\bfn owned_execute\b/)
		assert.match(lib, /\bfn owned_prepare\b/)
		const db = fs.readFileSync(path.join(packageRoot, "src/db.ts"), "utf8")
		const owned = db.slice(db.indexOf("interface OwnedInstance"), db.indexOf("interface InstanceBuilder"))
		assert.doesNotMatch(owned, /\bread\s*<R>\s*\(/, "OwnedInstance.read is deleted")
		assert.doesNotMatch(db, /ownedRead/, "the SDK does not call ownedRead")
	})

	test("a generic host function compiles against ReadInstance and OwnedInstance", async function sharedSurface() {
		const builder = InstanceBuilder.create(Ledger)
		builder.load(Holder, [{ id: 1n, name: "ada" }])
		const owned: OwnedInstance<Rels> = accepted(await builder.admit())
		againstSharedRead<Rels>(owned)
		const store = path.join(tmpRoot, "shared")
		const db = await Db.fromInstance(store, owned)
		db.read(function inScope(instance: ReadInstance<Rels>) {
			againstSharedRead<Rels>(instance)
		})
		owned[Symbol.dispose]()
	})

	test("owned get/scan/contains are plain methods; a hot get loop mints no per-call handle", async function hotGet() {
		const builder = InstanceBuilder.create(Ledger)
		builder.load(Holder, [{ id: 1n, name: "ada" }])
		const owned = accepted(await builder.admit())
		assert.equal("read" in owned, false, "the lease spelling is unrepresentable")
		assert.deepEqual(owned.scan(Holder), [{ id: 1n, name: "ada" }])
		assert.equal(owned.contains(Holder, { id: 1n, name: "ada" }), true)
		assert.deepEqual(owned.get(Holder, { id: 1n }), { id: 1n, name: "ada" })
		owned.get(Holder, { id: 1n })
		const before = process.memoryUsage().heapUsed
		for (let i = 0; i < 4_000; i++) {
			owned.get(Holder, { id: 1n })
		}
		const after = process.memoryUsage().heapUsed
		assert.ok(
			after - before < 4_000_000,
			`hot get grew the heap by ${after - before} bytes — a per-call handle lease would allocate far more`
		)
		owned[Symbol.dispose]()
	})
})
