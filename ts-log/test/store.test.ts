import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"
import { internalBlake3 } from "@bjornpagen/bumbledb"
import { toHex } from "#bytes.ts"
import { ErrStore } from "#errors.ts"
import { storeKey } from "#keys.ts"
import {
	acquireFsLease,
	encodeLease,
	etag,
	fsStore,
	MUTATION_TTL_MS,
	memStore,
	parseLease,
	releaseFsLease,
	resolveAmbiguousCreate,
	resolveAmbiguousSwap
} from "#store.ts"
import { joinPrefix, s3Store } from "#store-s3.ts"

const SLOT = storeKey("log/c00000000/0000000000000001")
const MANIFEST = storeKey("manifest")
const leaseCorpus = path.resolve(import.meta.dirname, "../../crates/bumbledb-log/conformance/v3/lease")
const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-log-store-"))
after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})
const encoder = new TextEncoder()
function leaseFixture(name: string): Uint8Array {
	return new Uint8Array(fs.readFileSync(path.join(leaseCorpus, `${name}.bin`)))
}
interface LeaseSidecar {
	readonly kind: string
	readonly expect: "ok" | "refusal"
	readonly value?: {
		readonly holder: string
		readonly token: string
		readonly expires: string
	}
	readonly hex: string
}
function leaseSidecar(name: string): LeaseSidecar {
	return JSON.parse(fs.readFileSync(path.join(leaseCorpus, `${name}.json`), "utf8"))
}
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
describe("the LEASE/1 corpus goldens", function suite() {
	const okCases = ["ok_mutation", "ok_released", "ok_generation_sidecar", "ok_unexpired", "ok_expired"]
	const refusalCases = [
		"r_no_magic",
		"r_version_2",
		"r_fifth_line",
		"r_negative_holder",
		"r_expires_overflow",
		"r_missing_expires"
	]
	test("every ok body parses to its sidecar value and re-encodes byte-identically", function round() {
		for (const name of okCases) {
			const bytes = leaseFixture(name)
			const sidecar = leaseSidecar(name)
			assert.equal(sidecar.expect, "ok", name)
			assert.equal(toHex(bytes), sidecar.hex, name)
			const lease = parseLease(new TextDecoder().decode(bytes))
			assert.ok(lease !== null, name)
			assert.ok(sidecar.value !== undefined, name)
			assert.equal(lease.holder, BigInt(sidecar.value.holder), name)
			assert.equal(lease.token, BigInt(sidecar.value.token), name)
			assert.equal(lease.expires, BigInt(sidecar.value.expires), name)
			assert.deepEqual(encodeLease(lease), bytes, name)
		}
	})
	test("every refusal body parses to null: not a lease, never breakable", function refuse() {
		for (const name of refusalCases) {
			const bytes = leaseFixture(name)
			const sidecar = leaseSidecar(name)
			assert.equal(sidecar.expect, "refusal", name)
			assert.equal(toHex(bytes), sidecar.hex, name)
			assert.equal(parseLease(new TextDecoder().decode(bytes)), null, name)
		}
	})
	test("the placement table names the constants this driver runs", function placement() {
		const table = JSON.parse(fs.readFileSync(path.join(leaseCorpus, "placement.json"), "utf8"))
		assert.equal(table.body_magic, "LEASE/1")
		assert.equal(table.namespace, "~lease")
		assert.equal(table.head_file, "~head")
		assert.equal(table.first_token, "1")
		assert.equal(table.constants.mutation_ttl_ms, String(MUTATION_TTL_MS))
		assert.equal(table.constants.lock_retry_ms, "10")
	})
	test("a Rust-spelled unexpired lease refuses to break; its expiry mints the next token", async function crossPin() {
		const root = path.join(tmpRoot, "rust-lease")
		const dir = path.join(root, "~lease", "manifest")
		fs.mkdirSync(dir, { recursive: true })
		fs.writeFileSync(path.join(dir, "1"), leaseFixture("ok_unexpired"))
		fs.writeFileSync(path.join(dir, "~head"), "1")
		await assert.rejects(function unexpired() {
			return acquireFsLease(root, "manifest", MUTATION_TTL_MS, "refuse")
		})
		fs.writeFileSync(path.join(dir, "1"), leaseFixture("ok_expired"))
		const held = await acquireFsLease(root, "manifest", MUTATION_TTL_MS, "refuse")
		assert.equal(held.token, 2n)
		assert.equal(fs.readFileSync(path.join(dir, "~head"), "utf8").trim(), "2")
		assert.equal(fs.existsSync(path.join(dir, "1")), false, "the superseded token is forgotten")
		const minted = parseLease(fs.readFileSync(path.join(dir, "2"), "utf8"))
		assert.ok(minted !== null)
		assert.equal(minted.holder, BigInt(process.pid))
		assert.equal(minted.token, 2n)
		await releaseFsLease(held)
		const released = parseLease(fs.readFileSync(path.join(dir, "2"), "utf8"))
		assert.ok(released !== null)
		assert.equal(released.expires, 0n, "release rewrites the held token with an already-expired body")
	})
})
describe("the five verbs over a directory", function suite() {
	test("an expired lease under ~lease/{key} is broken by putSwap", async function leases() {
		const root = path.join(tmpRoot, "s6")
		const store = fsStore(root)
		const created = await store.putCreate(MANIFEST, encoder.encode("v1"))
		assert.ok(created.tag === "created")
		const dir = path.join(root, "~lease", "manifest")
		fs.writeFileSync(path.join(dir, "2"), encodeLease({ holder: 999n, token: 2n, expires: 0n }))
		fs.writeFileSync(path.join(dir, "~head"), "2")
		const swapped = await store.putSwap(MANIFEST, encoder.encode("v2"), created.etag)
		assert.equal(swapped.tag, "swapped")
		const current = parseLease(fs.readFileSync(path.join(dir, "3"), "utf8"))
		assert.ok(current !== null)
		assert.equal(current.expires, 0n, "the verb's own lease is released after the swap")
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
		const visible = fs.readdirSync(root).filter((name) => !name.startsWith("~"))
		assert.deepEqual(visible, [MANIFEST], "no sidecar and no lease residue beside the object")
	})
	test("open sweeps stale temps under ~tmp and spares fresh ones", async function sweep() {
		const root = path.join(tmpRoot, "sweep")
		const temps = path.join(root, "~tmp")
		fs.mkdirSync(temps, { recursive: true })
		const stale = path.join(temps, "999.1")
		const fresh = path.join(temps, "999.2")
		fs.writeFileSync(stale, "litter")
		fs.writeFileSync(fresh, "mid-link")
		const aged = new Date(Date.now() - 60000)
		fs.utimesSync(stale, aged, aged)
		const store = fsStore(root)
		assert.equal(await store.get(MANIFEST), null)
		assert.equal(fs.existsSync(stale), false)
		assert.equal(fs.existsSync(fresh), true)
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
				return error instanceof Error && error instanceof ErrStore
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
