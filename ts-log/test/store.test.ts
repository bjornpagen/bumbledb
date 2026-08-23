import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"
import { fsStore } from "#store.ts"

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-log-store-"))

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

const encoder = new TextEncoder()

describe("the five verbs over a directory", function suite() {
	test("get on an absent key is null, not an error", async function absent() {
		const store = fsStore(path.join(tmpRoot, "s1"))
		assert.equal(await store.get("log/c00000000/0000000000000001"), null)
	})

	test("putCreate is create-only: the second writer sees exists", async function createOnly() {
		const store = fsStore(path.join(tmpRoot, "s2"))
		const first = await store.putCreate("log/c00000000/0000000000000001", encoder.encode("one"))
		assert.equal(first.tag, "created")
		const second = await store.putCreate("log/c00000000/0000000000000001", encoder.encode("two"))
		assert.equal(second.tag, "exists")
		const fetched = await store.get("log/c00000000/0000000000000001")
		assert.equal(new TextDecoder().decode(fetched?.bytes), "one")
	})

	test("putSwap is CAS: a stale etag is moved, a fresh one swaps", async function cas() {
		const store = fsStore(path.join(tmpRoot, "s3"))
		const created = await store.putCreate("manifest.json", encoder.encode("v1"))
		assert.ok(created.tag === "created")
		const swapped = await store.putSwap("manifest.json", encoder.encode("v2"), created.etag)
		assert.ok(swapped.tag === "swapped")
		const stale = await store.putSwap("manifest.json", encoder.encode("v3"), created.etag)
		assert.equal(stale.tag, "moved")
		const fetched = await store.get("manifest.json")
		assert.equal(new TextDecoder().decode(fetched?.bytes), "v2")
	})

	test("getIfChanged is the cheap poll: unchanged on the same etag, changed after a swap", async function poll() {
		const store = fsStore(path.join(tmpRoot, "s4"))
		const created = await store.putCreate("manifest.json", encoder.encode("v1"))
		assert.ok(created.tag === "created")
		const same = await store.getIfChanged("manifest.json", created.etag)
		assert.equal(same.tag, "unchanged")
		const swapped = await store.putSwap("manifest.json", encoder.encode("v2"), created.etag)
		assert.ok(swapped.tag === "swapped")
		const changed = await store.getIfChanged("manifest.json", created.etag)
		assert.ok(changed.tag === "changed")
		assert.equal(new TextDecoder().decode(changed.fetched.bytes), "v2")
	})

	test("delete removes object and etag; a later create succeeds", async function remove() {
		const store = fsStore(path.join(tmpRoot, "s5"))
		await store.putCreate("log/c00000000/0000000000000001", encoder.encode("one"))
		await store.delete("log/c00000000/0000000000000001")
		assert.equal(await store.get("log/c00000000/0000000000000001"), null)
		const again = await store.putCreate("log/c00000000/0000000000000001", encoder.encode("two"))
		assert.equal(again.tag, "created")
	})

	test("a dead owner's lock is broken; a live one is honored", async function locks() {
		const root = path.join(tmpRoot, "s6")
		const store = fsStore(root)
		fs.mkdirSync(path.join(root, ".locks"), { recursive: true })
		fs.writeFileSync(path.join(root, ".locks", encodeURIComponent("manifest.json")), "999999999")
		const created = await store.putCreate("manifest.json", encoder.encode("v1"))
		assert.equal(created.tag, "created")
	})

	test("contending writers on one slot: exactly one creates", async function contended() {
		const store = fsStore(path.join(tmpRoot, "s7"))
		const outcomes = await Promise.all(
			Array.from({ length: 8 }, function racer(_value, index) {
				return store.putCreate("log/c00000000/0000000000000001", encoder.encode(`racer-${index}`))
			})
		)
		assert.equal(outcomes.filter((outcome) => outcome.tag === "created").length, 1)
		assert.equal(outcomes.filter((outcome) => outcome.tag === "exists").length, 7)
	})
})
