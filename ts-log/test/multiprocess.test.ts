import assert from "node:assert/strict"
import { spawn } from "node:child_process"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"
import { readSidecar, writeSidecar } from "#chain.ts"
import { encodeBatch } from "#codec.ts"
import { braid as asBraid, descriptorOf } from "#descriptor.ts"
import { generation, idsKey, logKey } from "#keys.ts"
import { openReplica, ZERO_HASH } from "#replica.ts"
import { fsStore } from "#store.ts"
import { Ledger, Note } from "#test/fixtures.ts"
import { openWriter } from "#writer.ts"

/**
 * A second Node process recovers a scripted Pending over one FsStore
 * prefix. Children print structured lines; the parent asserts hard.
 */

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-log-mp-"))
const childScript = path.join(import.meta.dirname, "multiprocess-child.ts")
const PREFIX = "prod/main"
const CODEC = descriptorOf(Ledger).codec

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

let laneCounter = 0
function lane(): { base: string; bucket: string; dir: (name: string) => string } {
	laneCounter += 1
	const base = path.join(tmpRoot, `lane-${laneCounter}`)
	fs.mkdirSync(base, { recursive: true })
	return {
		base,
		bucket: path.join(base, "bucket"),
		dir(name: string) {
			return path.join(base, name)
		}
	}
}

function spawnChild(args: readonly string[]) {
	return spawn(process.execPath, [childScript, ...args], { stdio: ["ignore", "pipe", "pipe"] })
}

type Spawned = ReturnType<typeof spawnChild>

interface Exited {
	readonly code: number | null
	readonly lines: readonly Record<string, unknown>[]
	readonly stderr: string
}

function collect(child: Spawned): Promise<Exited> {
	let stdout = ""
	let stderr = ""
	child.stdout.on("data", function onOut(chunk: Buffer) {
		stdout += chunk.toString("utf8")
	})
	child.stderr.on("data", function onErr(chunk: Buffer) {
		stderr += chunk.toString("utf8")
	})
	return new Promise(function done(resolve) {
		child.on("close", function closed(code) {
			const lines: Record<string, unknown>[] = []
			for (const line of stdout.split("\n")) {
				if (line.startsWith("MP ")) {
					lines.push(JSON.parse(line.slice(3)) as Record<string, unknown>)
				}
			}
			resolve({ code, lines, stderr })
		})
	})
}

async function assertGapFreeChain(bucket: string, id: string, tip: bigint): Promise<void> {
	const store = fsStore(bucket)
	const home = asBraid(id)
	for (let g = 1n; g <= tip; g++) {
		const slot = await store.get(logKey(PREFIX, home, generation(g)))
		assert.ok(slot !== null, `slot ${g} of ${tip} is present: the chain is gap-free`)
	}
	const beyond = await store.get(logKey(PREFIX, home, generation(tip + 1n)))
	assert.equal(beyond, null, "nothing is published beyond the tip")
}

describe("the TS multi-process lane", function suite() {
	test("a scripted Pending at a known slot recovers in a second process", async function scriptedPending() {
		const { bucket, dir } = lane()
		const victimDir = dir("victim")
		const notes = asBraid("c00000002")
		{
			const writer = await openWriter({ store: fsStore(bucket), prefix: PREFIX, dir: victimDir, theory: Ledger })
			await writer.replica[Symbol.asyncDispose]()
		}
		const sidecarFile = path.join(victimDir, "chain")
		const sidecar = await readSidecar(CODEC, sidecarFile)
		assert.ok(sidecar.tag === "read")
		assert.equal(sidecar.chain.tag, "settled", "birth is Settled before the script plants Pending")
		const bytes = encodeBatch(
			Ledger,
			{
				braid: notes,
				braidGen: generation(1n),
				prev: ZERO_HASH,
				writer: 7n,
				timestamp: 1n
			},
			[{ op: "insert", relation: "Note", rows: [[7n, "scripted"]] }]
		)
		await writeSidecar(CODEC, sidecarFile, {
			tag: "pending",
			entries: sidecar.chain.entries,
			batch: { braid: notes, slot: generation(1n), bytes }
		})
		const planted = await readSidecar(CODEC, sidecarFile)
		assert.ok(planted.tag === "read")
		assert.equal(planted.chain.tag, "pending", "the test script wrote Pending at slot 1")

		const recovered = spawnChild(["recover", bucket, victimDir, "7"])
		const result = await collect(recovered)
		assert.equal(result.code, 0, `the second process recovers the scripted pending: ${result.stderr}`)
		assert.equal(result.lines.length, 1)
		const line = result.lines[0] as Record<string, unknown>
		assert.equal(line.tag, "recovered")
		assert.equal(line.arm, "Settled")
		assert.equal(line.generation, "1")
		assert.equal(line.notes, "1")
		assert.equal(line.slot, "present")
		await assertGapFreeChain(bucket, notes, 1n)

		const after = await readSidecar(CODEC, sidecarFile)
		assert.ok(after.tag === "read")
		assert.equal(after.chain.tag, "settled", "open published the Pending arm to Settled")

		const verifier = await openReplica({ store: fsStore(bucket), prefix: PREFIX, dir: dir("verify"), theory: Ledger })
		assert.equal(
			verifier.db.read(function count(instance) {
				return instance.count(Note)
			}),
			1n,
			"a fresh replica converges to the recovered slot"
		)
		assert.equal(verifier.vector.get(notes), 1n)
		const store = fsStore(bucket)
		const leaseTouched = await store.get(idsKey(PREFIX, 0, 0))
		assert.equal(leaseTouched, null, "explicit ids drew no lease: the lane exercised only the commit path")
		await verifier[Symbol.asyncDispose]()
	})
})
