import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"
import { internalBlake3 } from "@bjornpagen/bumbledb"
import { toHex } from "#bytes.ts"
import { storeKey } from "#keys.ts"
import { fsStore } from "#store.ts"
import { joinPrefix, s3Store } from "#store-s3.ts"

const SLOT = storeKey("log/c00000000/0000000000000001")
const MANIFEST = storeKey("manifest.json")

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-log-store-"))

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

const encoder = new TextEncoder()

describe("the five verbs over a directory", function suite() {
	test("get on an absent key is null, not an error", async function absent() {
		const store = fsStore(path.join(tmpRoot, "s1"))
		assert.equal(await store.get(SLOT), null)
	})

	test("putCreate is create-only: the second writer sees exists", async function createOnly() {
		const store = fsStore(path.join(tmpRoot, "s2"))
		const first = await store.putCreate(SLOT, encoder.encode("one"))
		assert.equal(first.tag, "created")
		const second = await store.putCreate(SLOT, encoder.encode("two"))
		assert.equal(second.tag, "exists")
		const fetched = await store.get(SLOT)
		assert.equal(new TextDecoder().decode(fetched?.bytes), "one")
	})

	test("putSwap is CAS: a stale etag is moved, a fresh one swaps", async function cas() {
		const store = fsStore(path.join(tmpRoot, "s3"))
		const created = await store.putCreate(MANIFEST, encoder.encode("v1"))
		assert.ok(created.tag === "created")
		const swapped = await store.putSwap(MANIFEST, encoder.encode("v2"), created.etag)
		assert.ok(swapped.tag === "swapped")
		const stale = await store.putSwap(MANIFEST, encoder.encode("v3"), created.etag)
		assert.equal(stale.tag, "moved")
		const fetched = await store.get(MANIFEST)
		assert.equal(new TextDecoder().decode(fetched?.bytes), "v2")
	})

	test("getIfChanged is the cheap poll: unchanged on the same etag, changed after a swap", async function poll() {
		const store = fsStore(path.join(tmpRoot, "s4"))
		const created = await store.putCreate(MANIFEST, encoder.encode("v1"))
		assert.ok(created.tag === "created")
		const same = await store.getIfChanged(MANIFEST, created.etag)
		assert.equal(same.tag, "unchanged")
		const swapped = await store.putSwap(MANIFEST, encoder.encode("v2"), created.etag)
		assert.ok(swapped.tag === "swapped")
		const changed = await store.getIfChanged(MANIFEST, created.etag)
		assert.ok(changed.tag === "changed")
		assert.equal(new TextDecoder().decode(changed.fetched.bytes), "v2")
	})

	test("delete removes the object and its lockfile; a later create succeeds", async function remove() {
		const store = fsStore(path.join(tmpRoot, "s5"))
		await store.putCreate(SLOT, encoder.encode("one"))
		await store.delete(SLOT)
		assert.equal(await store.get(SLOT), null)
		const again = await store.putCreate(SLOT, encoder.encode("two"))
		assert.equal(again.tag, "created")
	})

	test("a dead owner's pid-lockfile beside the key is broken by putSwap", async function locks() {
		const root = path.join(tmpRoot, "s6")
		const store = fsStore(root)
		const created = await store.putCreate(MANIFEST, encoder.encode("v1"))
		assert.ok(created.tag === "created")
		fs.writeFileSync(path.join(root, "manifest.json.lock"), "999999999")
		const swapped = await store.putSwap(MANIFEST, encoder.encode("v2"), created.etag)
		assert.equal(swapped.tag, "swapped")
		assert.equal(fs.existsSync(path.join(root, "manifest.json.lock")), false)
	})

	test("the etag is the blake3 of the content, computed and never stored", async function etags() {
		const root = path.join(tmpRoot, "s8")
		const store = fsStore(root)
		const body = encoder.encode("the judged content")
		const created = await store.putCreate(MANIFEST, body)
		assert.ok(created.tag === "created")
		assert.equal(created.etag, toHex(new Uint8Array(internalBlake3(body))))
		const fetched = await store.get(MANIFEST)
		assert.equal(fetched?.etag, created.etag)
		const next = encoder.encode("the next content")
		const swapped = await store.putSwap(MANIFEST, next, created.etag)
		assert.ok(swapped.tag === "swapped")
		assert.equal(swapped.etag, toHex(new Uint8Array(internalBlake3(next))))
		assert.deepEqual(fs.readdirSync(root), [MANIFEST], "no sidecar and no lock residue beside the object")
	})

	test("a key wearing the lockfile suffix is refused at the parse boundary", function refused() {
		assert.throws(function lockSuffix() {
			storeKey("manifest.json.lock")
		})
		assert.throws(function lockSegment() {
			storeKey("a.lock/b")
		})
	})

	test("contending writers on one slot: exactly one creates", async function contended() {
		const store = fsStore(path.join(tmpRoot, "s7"))
		const outcomes = await Promise.all(
			Array.from({ length: 8 }, function racer(_value, index) {
				return store.putCreate(SLOT, encoder.encode(`racer-${index}`))
			})
		)
		assert.equal(outcomes.filter((outcome) => outcome.tag === "created").length, 1)
		assert.equal(outcomes.filter((outcome) => outcome.tag === "exists").length, 7)
	})
})

describe("s3Store construction", function suite() {
	test("an empty prefix joins as the key alone", function emptyPrefix() {
		assert.equal(joinPrefix("", "manifest.json"), "manifest.json")
		assert.equal(joinPrefix("smoke/run", "log/c00000000/1"), "smoke/run/log/c00000000/1")
	})

	test("region auto without an endpoint is refused at construction", function autoNeedsEndpoint() {
		assert.throws(function missing() {
			s3Store({
				region: "auto",
				bucket: "example",
				credentials: { accessKeyId: "AKIAEXAMPLE", secretAccessKey: "secret" }
			})
		})
	})

	test("the constructor builds without touching the network", function constructs() {
		const store = s3Store({
			region: "us-east-1",
			bucket: "example",
			credentials: { accessKeyId: "AKIAEXAMPLE", secretAccessKey: "secret" },
			prefix: "/smoke/run/"
		})
		assert.equal(typeof store.get, "function")
		assert.equal(typeof store.getIfChanged, "function")
		assert.equal(typeof store.putCreate, "function")
		assert.equal(typeof store.putSwap, "function")
		assert.equal(typeof store.delete, "function")
	})
})
