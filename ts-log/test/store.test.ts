import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"
import { internalBlake3 } from "@bjornpagen/bumbledb"
import * as errors from "@superbuilders/errors"
import { toHex } from "#bytes.ts"
import { ErrStore } from "#errors.ts"
import { storeKey } from "#keys.ts"
import { etag, fsStore, memStore, resolveAmbiguousCreate, resolveAmbiguousSwap } from "#store.ts"
import { joinPrefix, s3Store } from "#store-s3.ts"

const SLOT = storeKey("log/c00000000/0000000000000001")
const MANIFEST = storeKey("manifest")

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-log-store-"))

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

const encoder = new TextEncoder()

describe("the five verbs over a process map", function suite() {
	test("get on an absent key is null, not an error", async function absent() {
		assert.equal(await memStore().get(SLOT), null)
	})

	test("putCreate is create-only: the second writer sees exists", async function createOnly() {
		const store = memStore()
		const first = await store.putCreate(SLOT, encoder.encode("one"))
		assert.equal(first.tag, "created")
		const second = await store.putCreate(SLOT, encoder.encode("two"))
		assert.equal(second.tag, "exists")
		const fetched = await store.get(SLOT)
		assert.equal(new TextDecoder().decode(fetched?.bytes), "one")
	})

	test("putSwap is CAS: a stale etag is moved, a fresh one swaps", async function cas() {
		const store = memStore()
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
		const store = memStore()
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

	test("delete is unconditional and a later create succeeds", async function remove() {
		const store = memStore()
		await store.putCreate(SLOT, encoder.encode("one"))
		await store.delete(SLOT)
		assert.equal(await store.get(SLOT), null)
		const again = await store.putCreate(SLOT, encoder.encode("two"))
		assert.equal(again.tag, "created")
	})

	test("the etag is the blake3 of the content", async function etags() {
		const store = memStore()
		const body = encoder.encode("the judged content")
		const created = await store.putCreate(MANIFEST, body)
		assert.ok(created.tag === "created")
		assert.equal(created.etag, toHex(new Uint8Array(internalBlake3(body))))
		const fetched = await store.get(MANIFEST)
		assert.equal(fetched?.etag, created.etag)
	})

	test("contending writers on one slot: exactly one creates", async function contended() {
		const store = memStore()
		const outcomes = await Promise.all(
			Array.from({ length: 8 }, function racer(_value, index) {
				return store.putCreate(SLOT, encoder.encode(`racer-${index}`))
			})
		)
		assert.equal(outcomes.filter((outcome) => outcome.tag === "created").length, 1)
		assert.equal(outcomes.filter((outcome) => outcome.tag === "exists").length, 7)
	})

	test("get returns a fresh buffer: mutating the fetch leaves the store intact", async function fresh() {
		const store = memStore()
		const created = await store.putCreate(MANIFEST, encoder.encode("keep"))
		assert.ok(created.tag === "created")
		const fetched = await store.get(MANIFEST)
		assert.ok(fetched !== null)
		fetched.bytes[0] = 0
		const again = await store.get(MANIFEST)
		assert.equal(new TextDecoder().decode(again?.bytes), "keep")
		const changed = await store.getIfChanged(MANIFEST, created.etag)
		assert.equal(changed.tag, "unchanged")
		const poll = await store.getIfChanged(MANIFEST, etag("0".repeat(64)))
		assert.ok(poll.tag === "changed")
		poll.fetched.bytes[0] = 0
		const third = await store.get(MANIFEST)
		assert.equal(new TextDecoder().decode(third?.bytes), "keep")
	})

	test("GET-verify names landed, lost, and absent", async function verify() {
		const store = memStore()
		const body = encoder.encode("ours")
		const created = await store.putCreate(SLOT, body)
		assert.ok(created.tag === "created")
		const landed = await resolveAmbiguousCreate(store, SLOT, body)
		assert.equal(landed.tag, "landed")
		const lost = await resolveAmbiguousCreate(store, SLOT, encoder.encode("theirs"))
		assert.equal(lost.tag, "lost")
		await store.delete(SLOT)
		const absent = await resolveAmbiguousCreate(store, SLOT, body)
		assert.equal(absent.tag, "absent")
		await store.putCreate(MANIFEST, encoder.encode("v1"))
		const swapped = await resolveAmbiguousSwap(store, MANIFEST, encoder.encode("v1"))
		assert.equal(swapped.tag, "landed")
	})
})

describe("the five verbs over a directory", function suite() {
	test("an expired lease beside the key is broken by putSwap", async function leases() {
		const root = path.join(tmpRoot, "s6")
		const store = fsStore(root)
		const created = await store.putCreate(MANIFEST, encoder.encode("v1"))
		assert.ok(created.tag === "created")
		fs.writeFileSync(path.join(root, ".manifest.lease.1"), `${process.pid}\n1\n0\n`)
		const swapped = await store.putSwap(MANIFEST, encoder.encode("v2"), created.etag)
		assert.equal(swapped.tag, "swapped")
		assert.equal(fs.readdirSync(root).filter((name) => name.startsWith(".manifest.lease.")).length, 0)
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
		assert.deepEqual(fs.readdirSync(root), [MANIFEST], "no sidecar and no lease residue beside the object")
	})

	test("open sweeps leftover temps and expired leases", async function sweep() {
		const root = path.join(tmpRoot, "sweep")
		fs.mkdirSync(root, { recursive: true })
		fs.writeFileSync(path.join(root, ".manifest.tmp.1.1"), "litter")
		fs.writeFileSync(path.join(root, ".manifest.lease.9"), "1\n9\n0\n")
		const store = fsStore(root)
		assert.equal(await store.get(MANIFEST), null)
		assert.equal(fs.existsSync(path.join(root, ".manifest.tmp.1.1")), false)
		assert.equal(fs.existsSync(path.join(root, ".manifest.lease.9")), false)
	})

	test("putCreate against a directory is a key-shape fault, not exists", async function directory() {
		const root = path.join(tmpRoot, "isdir")
		fs.mkdirSync(path.join(root, "manifest"), { recursive: true })
		const store = fsStore(root)
		await assert.rejects(
			function againstDir() {
				return store.putCreate(MANIFEST, encoder.encode("no"))
			},
			function isStore(error: unknown) {
				return error instanceof Error && errors.is(error, ErrStore)
			}
		)
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
		const fetched = await store.get(SLOT)
		assert.ok(fetched !== null)
		const winner = outcomes.find((outcome) => outcome.tag === "created")
		assert.ok(winner !== undefined && winner.tag === "created")
		assert.equal(fetched.etag, winner.etag)
	})
})

describe("s3Store construction", function suite() {
	test("an empty prefix joins as the key alone", function emptyPrefix() {
		assert.equal(joinPrefix("", "manifest"), "manifest")
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

	test("the constructor accepts a refresh without calling it", function refreshArm() {
		let called = false
		const store = s3Store({
			region: "us-east-1",
			bucket: "example",
			credentials() {
				called = true
				return { accessKeyId: "AKIAEXAMPLE", secretAccessKey: "secret" }
			}
		})
		assert.equal(typeof store.get, "function")
		assert.equal(called, false)
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
