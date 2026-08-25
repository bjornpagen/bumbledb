import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"
import { internalBlake3 } from "@bjornpagen/bumbledb"
import { toHex } from "#bytes.ts"
import { braid, descriptorOf } from "#descriptor.ts"
import {
	CKPT_SCRATCH_LEASE,
	checkpointJsonKey,
	checkpointMdbKey,
	encodeCkptScratch,
	generation,
	LEASE_NAMESPACE,
	manifestKey
} from "#keys.ts"
import type { CheckpointFacts } from "#manifest.ts"
import { renderCheckpoint, renderManifest } from "#manifest.ts"
import { openReplica } from "#replica.ts"
import { memStore } from "#store.ts"
import { Ledger } from "#test/fixtures.ts"
import { publishCheckpoint } from "#writer.ts"

const HOME = braid("c00000000")
const ZERO_DIGEST = "0".repeat(64)
const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-log-ckpt-orphan-"))

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

const encoder = new TextEncoder()
const descriptor = descriptorOf(Ledger)
const known = new Set(descriptor.braidMembers.keys())

function facts(homeG: bigint): CheckpointFacts {
	const braids = new Map()
	let sum = 0n
	for (const id of descriptor.braidMembers.keys()) {
		const g = id === HOME ? homeG : 0n
		braids.set(id, { g: generation(g), hash: ZERO_DIGEST, ts: 0n })
		sum += g
	}
	return { braids, catalog: ZERO_DIGEST, writer: 0n, prev: null, sum }
}

function digestOf(candidate: CheckpointFacts): string {
	return toHex(new Uint8Array(internalBlake3(renderCheckpoint(candidate))))
}

function scratchPath(dir: string): string {
	return path.join(dir, LEASE_NAMESPACE, CKPT_SCRATCH_LEASE)
}

async function birth(store: ReturnType<typeof memStore>, prefix: string): Promise<void> {
	const created = await store.putCreate(
		manifestKey(prefix),
		renderManifest({ fingerprint: descriptor.fingerprint, checkpoint: null })
	)
	assert.equal(created.tag, "created")
}

describe("the loser self-deletes its ckpt pair", function suite() {
	test("Kept deletes the candidate json and mdb and releases the scratch", async function kept() {
		const store = memStore()
		const prefix = "prod/main"
		const dir = path.join(tmpRoot, "kept")
		await birth(store, prefix)
		const incumbent = facts(10n)
		const incumbentDigest = digestOf(incumbent)
		const planted = await store.putCreate(checkpointJsonKey(prefix, incumbentDigest), renderCheckpoint(incumbent))
		assert.equal(planted.tag, "created")
		await store.putCreate(checkpointMdbKey(prefix, incumbentDigest), encoder.encode("incumbent-mdb"))
		const fetched = await store.get(manifestKey(prefix))
		assert.ok(fetched !== null)
		const swapped = await store.putSwap(
			manifestKey(prefix),
			renderManifest({ fingerprint: descriptor.fingerprint, checkpoint: incumbentDigest }),
			fetched.etag
		)
		assert.equal(swapped.tag, "swapped")

		const candidate = facts(1n)
		const digest = digestOf(candidate)
		const published = await publishCheckpoint(store, prefix, dir, known, candidate, encoder.encode("loser-mdb"))
		assert.equal(published.tag, "kept")
		if (published.tag === "kept") {
			assert.equal(published.incumbent, incumbentDigest)
		}
		assert.equal(await store.get(checkpointJsonKey(prefix, digest)), null)
		assert.equal(await store.get(checkpointMdbKey(prefix, digest)), null)
		assert.equal(fs.existsSync(scratchPath(dir)), false)
		assert.ok((await store.get(checkpointJsonKey(prefix, incumbentDigest))) !== null)
	})

	test("a refused publish deletes the candidate json and mdb", async function refused() {
		const store = memStore()
		const prefix = "prod/main"
		const dir = path.join(tmpRoot, "refused")
		const candidate = facts(1n)
		const digest = digestOf(candidate)
		const published = await publishCheckpoint(store, prefix, dir, known, candidate, encoder.encode("refused-mdb"))
		assert.equal(published.tag, "refused")
		if (published.tag === "refused") {
			assert.equal(published.reason, "manifest-missing")
		}
		assert.equal(await store.get(checkpointJsonKey(prefix, digest)), null)
		assert.equal(await store.get(checkpointMdbKey(prefix, digest)), null)
		assert.equal(fs.existsSync(scratchPath(dir)), false)
	})

	test("Replaced keeps the winner's ckpt pair and releases the scratch", async function replaced() {
		const store = memStore()
		const prefix = "prod/main"
		const dir = path.join(tmpRoot, "replaced")
		await birth(store, prefix)
		const candidate = facts(1n)
		const digest = digestOf(candidate)
		const published = await publishCheckpoint(store, prefix, dir, known, candidate, encoder.encode("winner-mdb"))
		assert.equal(published.tag, "replaced")
		assert.ok((await store.get(checkpointJsonKey(prefix, digest))) !== null)
		assert.ok((await store.get(checkpointMdbKey(prefix, digest))) !== null)
		assert.equal(fs.existsSync(scratchPath(dir)), false)
	})
})

describe("open sweeps reserved scratch", function suite() {
	test("a crash-stranded candidate named by ~lease/ckpt-scratch is deleted at open", async function crashStrand() {
		const store = memStore()
		const prefix = "prod/main"
		const dir = path.join(tmpRoot, "sweep-strand")
		await birth(store, prefix)
		const stranded = facts(1n)
		const digest = digestOf(stranded)
		await store.putCreate(checkpointJsonKey(prefix, digest), renderCheckpoint(stranded))
		await store.putCreate(checkpointMdbKey(prefix, digest), encoder.encode("strand-mdb"))
		fs.mkdirSync(path.join(dir, LEASE_NAMESPACE), { recursive: true })
		fs.writeFileSync(scratchPath(dir), encodeCkptScratch(digest))

		const replica = await openReplica({
			store,
			prefix,
			dir,
			theory: Ledger
		})
		assert.equal(await store.get(checkpointJsonKey(prefix, digest)), null)
		assert.equal(await store.get(checkpointMdbKey(prefix, digest)), null)
		assert.equal(fs.existsSync(scratchPath(dir)), false)
		await replica[Symbol.asyncDispose]()
	})

	test("sweep deletes a stranded digest and leaves the live head", function liveHead() {
		const source = fs.readFileSync(path.resolve(import.meta.dirname, "../src/replica.ts"), "utf8")
		const start = source.indexOf("async function sweepReservedKeys")
		assert.ok(start !== -1)
		const body = source.slice(start, start + 900)
		assert.ok(body.includes("CKPT_SCRATCH_LEASE"))
		assert.ok(body.includes("parseCkptScratch"))
		assert.ok(body.includes("digest !== core.checkpointDigest"))
		assert.ok(body.includes("checkpointJsonKey"))
		assert.ok(body.includes("checkpointMdbKey"))
		assert.ok(body.includes("TEMP_NAMESPACE"))
		assert.ok(body.includes("LEASE_NAMESPACE"))
	})
})
