import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"
import * as errors from "@superbuilders/errors"
import { CHAIN_FILE, renderSidecar } from "#chain.ts"
import { braid, descriptorOf } from "#descriptor.ts"
import { ErrManifestMissing, ErrRefused, refusalOf } from "#errors.ts"
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
		const caught = await errors.try(
			openReplica({
				store: memStore(),
				prefix: "prod/main",
				dir: path.join(tmpRoot, "missing"),
				theory: Ledger
			})
		)
		assert.ok(caught.error)
		assert.ok(errors.is(caught.error, ErrManifestMissing))
	})

	test("sidecar open refuses an unknown braid through parse", async function unknownBraid() {
		const store = memStore()
		const descriptor = descriptorOf(Ledger)
		const created = await store.putCreate(
			manifestKey("prod/main"),
			renderManifest({ fingerprint: descriptor.fingerprint, checkpoint: null })
		)
		assert.equal(created.tag, "created")
		const dir = path.join(tmpRoot, "unknown-braid")
		fs.mkdirSync(dir, { recursive: true })
		const unknown = braid("c0000ffff")
		fs.writeFileSync(
			path.join(dir, CHAIN_FILE),
			renderSidecar({
				tag: "settled",
				entries: new Map([[unknown, { g: generation(0n), prev: new Uint8Array(32), ts: 0n }]])
			})
		)
		const caught = await errors.try(openReplica({ store, prefix: "prod/main", dir, theory: Ledger }))
		assert.ok(caught.error)
		assert.ok(errors.is(caught.error, ErrRefused))
		assert.equal(refusalOf(caught.error)?.kind, "UnknownBraid")
	})
})

describe("adoptManifest is one transition", function suite() {
	test("the etag is assigned only after the checkpoint document is in hand", function order() {
		const source = fs.readFileSync(path.resolve(import.meta.dirname, "../src/replica.ts"), "utf8")
		const start = source.indexOf("async function adoptManifest")
		const end = source.indexOf("async function refreshManifest")
		assert.ok(start !== -1 && end > start)
		const body = source.slice(start, end)
		const facts = body.indexOf("await core.store.get(checkpointJsonKey")
		const checkpoint = body.indexOf("core.checkpoint = checkpoint")
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
			renderManifest({ fingerprint: descriptor.fingerprint, checkpoint: null })
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
			renderManifest({ fingerprint: descriptor.fingerprint, checkpoint: "ab".repeat(32) }),
			genesis
		)
		assert.equal(swapped.tag, "swapped")
		core.passes = 15
		const caught = await errors.try(replica.refresh())
		assert.ok(caught.error)
		assert.equal(core.manifestEtag, genesis, "etag stays the old pointer when the checkpoint is absent")
		assert.equal(core.checkpoint, null)
		await replica[Symbol.asyncDispose]()
	})
})
