import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"
import * as errors from "@superbuilders/errors"
import { descriptorOf } from "#descriptor.ts"
import { DOC_VERSION } from "#document.ts"
import { ErrManifestMissing, ErrRefused, refusalOf } from "#errors.ts"
import { manifestKey } from "#keys.ts"
import { renderManifest } from "#manifest.ts"
import { openReplica } from "#replica.ts"
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
		const zero = "0".repeat(64)
		fs.writeFileSync(
			path.join(dir, "chain.json"),
			`{"v":${DOC_VERSION},"chain":{"c00000000":{"g":"0","prev":"${zero}","ts":"0"},"c0000ffff":{"g":"0","prev":"${zero}","ts":"0"}},"pending":null}`
		)
		const caught = await errors.try(openReplica({ store, prefix: "prod/main", dir, theory: Ledger }))
		assert.ok(caught.error)
		assert.ok(errors.is(caught.error, ErrRefused))
		assert.equal(refusalOf(caught.error)?.kind, "UnknownBraid")
	})
})
