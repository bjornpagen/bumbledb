import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"
import { relation, schema, str, u64 } from "@bjornpagen/bumbledb"
import { Result } from "effect"
import { digest32, digest32FromHex } from "#bytes.ts"
import type { Chain } from "#chain.ts"
import { CHAIN_FILE, renderSidecar } from "#chain.ts"
import { braid, descriptorOf } from "#descriptor.ts"
import { ErrManifestMissing } from "#errors.ts"
import { generation, manifestKey } from "#keys.ts"
import { renderManifest } from "#manifest.ts"
import { coreOf, openReplica } from "#replica.ts"
import { memStore } from "#store.ts"
import { Ledger } from "#test/fixtures.ts"

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-log-replica-open-"))
after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})
describe("replica open refusals", function suite() {
	test("a replica refuses ManifestMissing when the store has no manifest", async function missing() {
		const caught = await Promise.resolve(
			openReplica({
				store: memStore(),
				prefix: "prod/main",
				dir: path.join(tmpRoot, "missing"),
				theory: Ledger
			})
		).then(Result.succeed, (cause: unknown) => Result.fail(cause))
		assert.ok(Result.isFailure(caught))
		assert.ok(caught.failure instanceof ErrManifestMissing)
	})
	test("a sidecar naming a foreign braid is corrupt cache: open reseeds at zero", async function foreignBraid() {
		const store = memStore()
		const descriptor = descriptorOf(Ledger)
		const created = await store.putCreate(
			manifestKey("prod/main"),
			renderManifest({ fingerprint: digest32(descriptor.fingerprintBytes), checkpoint: null })
		)
		assert.equal(created.tag, "created")
		// A two-component theory whose codec spells braid c00000001 — a
		// braid the Ledger's own decomposition lacks.
		const A = relation("A", { id: u64.fresh, body: str })
		const B = relation("B", { id: u64.fresh, body: str })
		const Pair = schema("Pair", { A, B }, [])
		const foreign = descriptorOf(Pair)
		const alien = braid("c00000001")
		assert.ok(foreign.braidMembers.has(alien), "Pair decomposes with braid c00000001")
		assert.ok(!descriptor.braidMembers.has(alien), "the Ledger does not")
		const planted: Chain = {
			tag: "settled",
			entries: new Map(
				[...foreign.braidMembers.keys()].map(function seed(id) {
					return [id, { g: generation(3n), prev: digest32(new Uint8Array(32)), ts: 0n }] as const
				})
			)
		}
		const dir = path.join(tmpRoot, "foreign-braid")
		fs.mkdirSync(dir, { recursive: true })
		fs.writeFileSync(path.join(dir, CHAIN_FILE), renderSidecar(foreign.codec, planted))
		// The codec-backed readSidecar refuses the foreign braid at parse,
		// so the sidecar is corrupt cache and open reseeds fresh.
		const replica = await openReplica({ store, prefix: "prod/main", dir, theory: Ledger })
		assert.ok(replica.vector.size > 0)
		for (const g of replica.vector.values()) {
			assert.equal(g, 0n, "the reseeded replica stands at the zero vector")
		}
		await replica[Symbol.asyncDispose]()
	})
})
describe("adoptManifest is one transition", function suite() {
	test("the etag is assigned only after the checkpoint document is in hand", function order() {
		const source = fs.readFileSync(path.resolve(import.meta.dirname, "../src/replica.ts"), "utf8")
		const start = source.indexOf("async function adoptManifest")
		const end = source.indexOf("async function refreshManifest")
		assert.ok(start !== -1 && end > start)
		const body = source.slice(start, end)
		const facts = body.indexOf("await core.store.get(ckptDocKey")
		const checkpoint = body.indexOf("core.checkpoint = parseCheckpoint")
		const etag = body.indexOf("core.manifestEtag = etag")
		assert.ok(facts !== -1, "checkpoint bytes are fetched")
		assert.ok(checkpoint !== -1, "checkpoint facts are adopted")
		assert.ok(etag !== -1, "etag is adopted")
		assert.equal(body.indexOf("core.manifestEtag = etag", etag + 1), -1, "etag is assigned once")
		assert.ok(facts < checkpoint, "checkpoint bytes precede checkpoint facts")
		assert.ok(checkpoint < etag, "checkpoint facts precede etag")
	})
	test("a failed checkpoint fetch leaves the old etag, so the floor cannot freeze", async function failedFetch() {
		const store = memStore()
		const descriptor = descriptorOf(Ledger)
		const created = await store.putCreate(
			manifestKey("prod/main"),
			renderManifest({ fingerprint: digest32(descriptor.fingerprintBytes), checkpoint: null })
		)
		assert.equal(created.tag, "created")
		const replica = await openReplica({
			store,
			prefix: "prod/main",
			dir: path.join(tmpRoot, "adopt-fail"),
			theory: Ledger
		})
		const core = coreOf(replica)
		const genesis = core.manifestEtag
		assert.ok(genesis !== null)
		const swapped = await store.putSwap(
			manifestKey("prod/main"),
			renderManifest({
				fingerprint: digest32(descriptor.fingerprintBytes),
				checkpoint: digest32FromHex("ab".repeat(32))
			}),
			genesis
		)
		assert.equal(swapped.tag, "swapped")
		core.passes = 15
		const caught = await Promise.resolve(replica.refresh()).then(Result.succeed, (cause: unknown) => Result.fail(cause))
		assert.ok(Result.isFailure(caught))
		assert.equal(core.manifestEtag, genesis, "etag stays the old pointer when the checkpoint is absent")
		assert.equal(core.checkpoint, null)
		await replica[Symbol.asyncDispose]()
	})
})
describe("waitFor surfaces the full Waited sum", function suite() {
	test("a wedged braid the target needs returns Wedged promptly", async function wedged() {
		const store = memStore()
		const descriptor = descriptorOf(Ledger)
		const created = await store.putCreate(
			manifestKey("prod/main"),
			renderManifest({ fingerprint: digest32(descriptor.fingerprintBytes), checkpoint: null })
		)
		assert.equal(created.tag, "created")
		const replica = await openReplica({
			store,
			prefix: "prod/main",
			dir: path.join(tmpRoot, "wedged-wait"),
			theory: Ledger
		})
		const core = coreOf(replica)
		const [home] = descriptor.braidMembers.keys()
		assert.ok(home !== undefined)
		core.wedged.set(home, "planted corruption")
		const waited = await replica.waitFor(new Map([[home, generation(1n)]]))
		assert.equal(waited.tag, "wedged")
		assert.ok(waited.tag === "wedged")
		assert.equal(waited.braid, home)
		assert.equal(waited.cause, "planted corruption")
		await replica[Symbol.asyncDispose]()
	})
	test("a dominated target returns Reached carrying the vector", async function reached() {
		const store = memStore()
		const descriptor = descriptorOf(Ledger)
		const created = await store.putCreate(
			manifestKey("prod/main"),
			renderManifest({ fingerprint: digest32(descriptor.fingerprintBytes), checkpoint: null })
		)
		assert.equal(created.tag, "created")
		const replica = await openReplica({
			store,
			prefix: "prod/main",
			dir: path.join(tmpRoot, "reached-wait"),
			theory: Ledger
		})
		const [home] = descriptor.braidMembers.keys()
		assert.ok(home !== undefined)
		const waited = await replica.waitFor(new Map([[home, generation(0n)]]))
		assert.equal(waited.tag, "reached")
		assert.ok(waited.tag === "reached")
		assert.equal(waited.vector.get(home), 0n)
		await replica[Symbol.asyncDispose]()
	})
})
