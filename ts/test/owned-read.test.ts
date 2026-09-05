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
import { native } from "#native.ts"
import { accepted } from "#test/accepted.ts"

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-owned-read-"))
const packageRoot = path.resolve(import.meta.dirname, "..")

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

const Holder = relation("Holder", { id: u64.fresh, name: str })
const Ledger = schema("OwnedRead", { Holder }, [])
type Rels = (typeof Ledger)["relations"]

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

	test("owned reads use direct crossings; hot gets request no new handle or lease", async function hotGet(t) {
		const builder = InstanceBuilder.create(Ledger)
		builder.load(Holder, [{ id: 1n, name: "ada" }])
		const owned = accepted(await builder.admit())
		const originalGet = native.ownedGet
		const handleCrossings = [
			"dbCreate",
			"dbOpen",
			"dbFromInstance",
			"dbRead",
			"dbWrite",
			"dbWriteFrom",
			"instancePrepare",
			"dbPrepare",
			"instanceBuilderNew",
			"instanceBuilderAdmit",
			"ownedPrepare",
			"logCodec"
		] as const
		const handleCalls = handleCrossings.map(function observe(name) {
			return { name, method: t.mock.method(native, name) }
		})
		const scan = t.mock.method(native, "ownedScan")
		const contains = t.mock.method(native, "ownedContains")
		const get = t.mock.method(native, "ownedGet")
		function assertNoHandleCrossings(): void {
			for (const { name, method } of handleCalls) {
				assert.equal(method.mock.callCount(), 0, `owned reads must not call ${name}`)
			}
		}
		try {
			assert.equal("read" in owned, false, "the lease spelling is unrepresentable")
			assert.deepEqual(owned.scan(Holder), [{ id: 1n, name: "ada" }])
			assert.equal(owned.contains(Holder, { id: 1n, name: "ada" }), true)
			assert.equal(scan.mock.callCount(), 1)
			assert.equal(contains.mock.callCount(), 1)
			// Count SDK/native crossings, not uncollected temporary row/key allocations.
			// This is a handle-path regression test, not a heap or throughput benchmark.
			for (let i = 0; i < 4_000; i++) {
				assert.deepEqual(owned.get(Holder, { id: 1n }), { id: 1n, name: "ada" })
			}
			assert.equal(get.mock.callCount(), 4_000, "exactly one direct native get per SDK get")
			assertNoHandleCrossings()

			// Negative control: introduce a real, promptly disposed native owner inside
			// one get. The census must reject this even though no handle is leaked.
			get.mock.mockImplementation(function withExtraHandle(...args: Parameters<typeof native.ownedGet>) {
				const extra = InstanceBuilder.create(Ledger)
				extra[Symbol.dispose]()
				return originalGet(...args)
			})
			assert.deepEqual(owned.get(Holder, { id: 1n }), { id: 1n, name: "ada" })
			assert.equal(get.mock.callCount(), 4_001)
			assert.throws(assertNoHandleCrossings, {
				name: "AssertionError",
				message: /owned reads must not call instanceBuilderNew/
			})
		} finally {
			t.mock.restoreAll()
			owned[Symbol.dispose]()
		}
	})
})
