/**
 * Credential-gated S3 smokes. Same env as the CI gate:
 * BUMBLEDB_S3_SMOKE_BUCKET, AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY
 * required; BUMBLEDB_S3_SMOKE_REGION defaults to us-east-1;
 * BUMBLEDB_S3_SMOKE_ENDPOINT optional. Missing credentials skip
 * loudly and never fail.
 */

import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"
import { storeKey } from "#keys.ts"
import { openReplica } from "#replica.ts"
import type { ObjectStore } from "#store.ts"
import { s3Store } from "#store.ts"
import { Booking, Holder, Ledger } from "#test/fixtures.ts"
import { openWriter } from "#writer.ts"

const REQUIRED = ["BUMBLEDB_S3_SMOKE_BUCKET", "AWS_ACCESS_KEY_ID", "AWS_SECRET_ACCESS_KEY"] as const

const SLOT = storeKey("log/c00000000/1")
const MANIFEST = storeKey("manifest")
const PROBE = storeKey("log/c00000000/probe")

let prefixSeq = 0
const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-log-s3-smoke-"))

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

function missingRequired(): string[] {
	return REQUIRED.filter(function empty(key) {
		const value = process.env[key]
		return value === undefined || value.length === 0
	})
}

const missing = missingRequired()
const smokeOptions = { skip: missing.length > 0 ? `S3 qualification not run: missing ${missing.join(" ")}` : false }

function uniquePrefix(tag: string): string {
	prefixSeq += 1
	return `smoke/${process.pid}/${prefixSeq}/${tag}`
}

function smokeStore(tag: string): ObjectStore {
	const bucket = process.env.BUMBLEDB_S3_SMOKE_BUCKET
	const accessKeyId = process.env.AWS_ACCESS_KEY_ID
	const secretAccessKey = process.env.AWS_SECRET_ACCESS_KEY
	assert.ok(
		bucket !== undefined && accessKeyId !== undefined && secretAccessKey !== undefined,
		"S3 configuration disappeared after test admission"
	)
	const region = process.env.BUMBLEDB_S3_SMOKE_REGION ?? "us-east-1"
	const endpoint = process.env.BUMBLEDB_S3_SMOKE_ENDPOINT
	const sessionToken = process.env.AWS_SESSION_TOKEN
	return s3Store({
		region,
		bucket,
		credentials: {
			accessKeyId,
			secretAccessKey,
			...(sessionToken === undefined ? {} : { sessionToken })
		},
		prefix: uniquePrefix(tag),
		...(endpoint === undefined ? {} : { endpoint })
	})
}

describe("s3 smoke", function suite() {
	test("s3_smoke configuration is present", smokeOptions, function configuration() {
		assert.deepEqual(missingRequired(), [])
	})

	test("s3_smoke create-only race", smokeOptions, async function createOnly() {
		const store = smokeStore("create")
		const [left, right] = await Promise.all([
			store.putCreate(SLOT, new TextEncoder().encode("alpha")),
			store.putCreate(SLOT, new TextEncoder().encode("beta"))
		])
		const tags = [left.tag, right.tag].sort()
		assert.deepEqual(tags, ["created", "exists"])
		const fetched = await store.get(SLOT)
		assert.ok(fetched !== null)
		const body = new TextDecoder().decode(fetched.bytes)
		assert.ok(body === "alpha" || body === "beta")
		await store.delete(SLOT)
	})

	test("s3_smoke CAS linearizes", smokeOptions, async function cas() {
		const store = smokeStore("cas")
		const birth = await store.putCreate(MANIFEST, new TextEncoder().encode("0"))
		assert.equal(birth.tag, "created")
		const workers = [0, 1].map(async function worker() {
			let landed = 0
			while (landed < 4) {
				const current = await store.get(MANIFEST)
				assert.ok(current !== null)
				const value = Number.parseInt(new TextDecoder().decode(current.bytes), 10)
				const next = new TextEncoder().encode(String(value + 1))
				const outcome = await store.putSwap(MANIFEST, next, current.etag)
				if (outcome.tag === "swapped") {
					landed += 1
				}
			}
		})
		await Promise.all(workers)
		const fetched = await store.get(MANIFEST)
		assert.ok(fetched !== null)
		assert.equal(new TextDecoder().decode(fetched.bytes), "8")
		await store.delete(MANIFEST)
	})

	test("s3_smoke 304 poll", smokeOptions, async function poll() {
		const store = smokeStore("poll")
		const created = await store.putCreate(MANIFEST, new TextEncoder().encode('{"v":1}'))
		assert.ok(created.tag === "created")
		const same = await store.getIfChanged(MANIFEST, created.etag)
		assert.equal(same.tag, "unchanged")
		await store.delete(MANIFEST)
	})

	test("s3_smoke GET-before-PUT", smokeOptions, async function negcache() {
		const store = smokeStore("negcache")
		assert.equal(await store.get(PROBE), null)
		const created = await store.putCreate(PROBE, new TextEncoder().encode("after-miss"))
		assert.equal(created.tag, "created")
		const fetched = await store.get(PROBE)
		assert.equal(new TextDecoder().decode(fetched?.bytes), "after-miss")
		await store.delete(PROBE)
	})

	test("s3_smoke replica writer round-trip", smokeOptions, async function roundTrip() {
		const store = smokeStore("roundtrip")
		const dir = path.join(tmpRoot, `roundtrip-${prefixSeq}`)
		const writer = await openWriter({ store, prefix: "", dir: path.join(dir, "a"), theory: Ledger })
		const a = writer.replica
		const out = await writer.commit(function record(batch) {
			batch.insert(Holder, [{ id: 1n, name: "s3-smoke" }])
			const ids = batch.reserve(Booking, "id", 1n)
			const id = ids.at(0n)
			assert.ok(id !== undefined)
			batch.insert(Booking, [{ id, holder: 1n, slot: "s1", at: { start: 1n, end: 2n } }])
		})
		assert.equal(out.tag, "accepted")
		await a[Symbol.asyncDispose]()

		const b = await openReplica({ store, prefix: "", dir: path.join(dir, "b"), theory: Ledger })
		if (out.tag === "accepted") {
			const waited = await b.waitFor(new Map([[out.value.braid, out.value.slot]]))
			assert.ok(waited.tag === "reached", "read-your-writes: waitFor reaches the committed slot")
		}
		const names = b.db.read(function readNames(instance) {
			return instance.scan(Holder).map(function nameOf(fact) {
				return fact.name
			})
		})
		assert.deepEqual(names, ["s3-smoke"])
		await b[Symbol.asyncDispose]()
	})
})
