import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"
import { fsStore } from "#store.ts"
import { openTenants } from "#tenants.ts"
import { Holder, Ledger } from "#test/fixtures.ts"
import { openWriter } from "#writer.ts"

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-log-tenants-"))

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

describe("per-tenant replicas", function suite() {
	test("tenants are isolated prefixes; eviction is LRU with _shared pinned", async function isolation() {
		const store = fsStore(path.join(tmpRoot, "bucket"))
		const tenants = openTenants({
			store,
			root: "prod",
			dir: path.join(tmpRoot, "replicas"),
			theory: Ledger,
			maxOpen: 2
		})

		const shared = await tenants.get("_shared")
		const acme = await tenants.get("acme")
		const acmeWriter = openWriter(acme)
		await acmeWriter.commit(function record(batch) {
			batch.insert(Holder, [{ id: 1n, name: "acme-holder" }])
			return 0
		})

		const globex = await tenants.get("globex")
		assert.equal(
			globex.db.read(function count(instance) {
				return instance.count(Holder)
			}),
			0n
		)

		const acmeAgain = await tenants.get("acme")
		assert.notEqual(acmeAgain, acme)
		assert.equal(
			acmeAgain.db.read(function count(instance) {
				return instance.count(Holder)
			}),
			1n
		)

		const sharedAgain = await tenants.get("_shared")
		assert.equal(sharedAgain, shared)

		await tenants[Symbol.asyncDispose]()
	})

	test("a tenant id must be a single path segment", async function badId() {
		const store = fsStore(path.join(tmpRoot, "bucket2"))
		const tenants = openTenants({ store, root: "prod", dir: path.join(tmpRoot, "replicas2"), theory: Ledger })
		await assert.rejects(function escapeAttempt() {
			return tenants.get("../prod")
		})
		await tenants[Symbol.asyncDispose]()
	})
})
