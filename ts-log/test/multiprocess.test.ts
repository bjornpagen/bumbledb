import assert from "node:assert/strict"
import { spawn } from "node:child_process"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"
import { braid as asBraid } from "#descriptor.ts"
import { generation, idsKey, logKey } from "#keys.ts"
import { openReplica } from "#replica.ts"
import { fsStore } from "#store.ts"
import { Holder, Ledger, Note } from "#test/fixtures.ts"
import { openWriter } from "#writer.ts"

/**
 * The TS multi-process lane (60): real Node child processes over one
 * FsStore prefix, mirroring lane_b_fs_multiprocess's re-exec pattern —
 * children print structured lines, the parent asserts hard. Every other
 * TS contention test races promises in one process; this one races
 * processes.
 */

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-log-mp-"))
const childScript = path.join(import.meta.dirname, "multiprocess-child.ts")
const PREFIX = "prod/main"

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

async function seedHolder(bucket: string, dir: string): Promise<void> {
	const writer = await openWriter({ store: fsStore(bucket), prefix: PREFIX, dir, theory: Ledger })
	const replica = writer.replica
	const seeded = await writer.commit(function seed(batch) {
		batch.insert(Holder, [{ id: 1n, name: "root" }])
		return 0
	})
	assert.ok(seeded.tag === "accepted")
	await replica[Symbol.asyncDispose]()
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
	test("N writers, disjoint content: every ack exactly once in a gap-free chain", async function disjoint() {
		const { base, bucket, dir } = lane()
		const writers = 4
		const commits = 5
		const children = Array.from({ length: writers }, function spawnOne(_value, id) {
			return spawnChild(["disjoint", bucket, dir(`w${id}`), String(id), String(commits)])
		})
		const settled = collect(children[0] as Spawned)
		const others = children.slice(1).map(collect)
		fs.writeFileSync(path.join(base, "go"), "go")
		const results = [await settled, ...(await Promise.all(others))]

		const acks: { braid: string; generation: bigint; noteId: string }[] = []
		for (const result of results) {
			assert.equal(result.code, 0, `child exits clean: ${result.stderr}`)
			for (const line of result.lines) {
				assert.equal(line.tag, "ack")
				acks.push({
					braid: line.braid as string,
					generation: BigInt(line.generation as string),
					noteId: line.noteId as string
				})
			}
		}
		assert.equal(acks.length, writers * commits, "every commit acked exactly once")
		const braid = acks[0]?.braid
		assert.ok(braid !== undefined)
		const generations = new Set(acks.map((ack) => String(ack.generation)))
		assert.equal(generations.size, acks.length, "no two acks share a slot")
		const noteIds = new Set(acks.map((ack) => ack.noteId))
		assert.equal(noteIds.size, acks.length, "every acked row is distinct")
		for (const ack of acks) {
			assert.equal(ack.braid, braid, "all Notes land in the one Note braid")
			assert.ok(ack.generation >= 1n && ack.generation <= BigInt(acks.length), "acks cover exactly the chain")
		}
		await assertGapFreeChain(bucket, braid, BigInt(acks.length))

		const verifier = await openReplica({ store: fsStore(bucket), prefix: PREFIX, dir: dir("verify"), theory: Ledger })
		assert.equal(
			verifier.db.read(function count(instance) {
				return instance.count(Note)
			}),
			BigInt(acks.length),
			"the converged replica holds every acked row exactly once"
		)
		assert.equal(verifier.vector.get(asBraid(braid)), BigInt(acks.length))
		await verifier[Symbol.asyncDispose]()
	})

	test("a shared determinant: one winner, N-1 typed FD rejections", async function shared() {
		const { base, bucket, dir } = lane()
		await seedHolder(bucket, dir("seed"))
		const writers = 4
		const children = Array.from({ length: writers }, function spawnOne(_value, id) {
			return spawnChild(["fd", bucket, dir(`w${id}`), String(id)])
		})
		const pending = children.map(collect)
		fs.writeFileSync(path.join(base, "go"), "go")
		const results = await Promise.all(pending)

		const verdicts: { result: string; canonical: string | undefined }[] = []
		for (const result of results) {
			assert.equal(result.code, 0, `child exits clean: ${result.stderr}`)
			assert.equal(result.lines.length, 1)
			const line = result.lines[0] as Record<string, unknown>
			assert.equal(line.tag, "verdict")
			verdicts.push({ result: line.result as string, canonical: line.canonical as string | undefined })
		}
		const winners = verdicts.filter((verdict) => verdict.result === "accepted")
		const losers = verdicts.filter((verdict) => verdict.result === "rejected")
		assert.equal(winners.length, 1, "exactly one writer lands the shared determinant")
		assert.equal(losers.length, writers - 1, "every other writer gets the serial rejection")
		const canonical = losers[0]?.canonical
		assert.ok(canonical !== undefined && canonical.length > 0, "the rejection is typed: it names the statement")
		for (const loser of losers) {
			assert.equal(loser.canonical, canonical, "every loser names the same FD")
		}
	})

	test("a child killed mid-commit: the fleet converges and the restart resolves its pending", async function killed() {
		const { bucket, dir } = lane()
		const victimDir = dir("victim")
		const victim = spawnChild(["victim", bucket, victimDir, "7"])
		const seen: bigint[] = []
		let buffered = ""
		await new Promise<void>(function watch(resolve) {
			victim.stdout.on("data", function onOut(chunk: Buffer) {
				buffered += chunk.toString("utf8")
				let newline = buffered.indexOf("\n")
				while (newline !== -1) {
					const line = buffered.slice(0, newline)
					buffered = buffered.slice(newline + 1)
					if (line.startsWith("MP ")) {
						const parsed = JSON.parse(line.slice(3)) as Record<string, unknown>
						seen.push(BigInt(parsed.generation as string))
					}
					newline = buffered.indexOf("\n")
				}
				if (seen.length >= 3) {
					victim.kill("SIGKILL")
					resolve()
				}
			})
		})
		await new Promise(function gone(resolve) {
			victim.on("close", resolve)
		})
		assert.ok(seen.length >= 3, "the victim acked before dying")

		const revived = spawnChild(["revive", bucket, victimDir, "7"])
		const result = await collect(revived)
		assert.equal(result.code, 0, `the restarted process resolves its pending: ${result.stderr}`)
		assert.equal(result.lines.length, 1)
		const line = result.lines[0] as Record<string, unknown>
		assert.equal(line.tag, "revived")
		const braid = line.braid as string
		const tip = BigInt(line.generation as string)
		const notes = BigInt(line.notes as string)
		assert.ok(tip >= BigInt(seen.length) + 1n, "every ack the parent observed is in the chain, plus the revival commit")
		assert.equal(notes, tip, "one Note per slot: the chain replays whole")
		for (const generation of seen) {
			assert.ok(generation < tip, `acked slot ${generation} precedes the revival tip`)
		}
		await assertGapFreeChain(bucket, braid, tip)

		const verifier = await openReplica({ store: fsStore(bucket), prefix: PREFIX, dir: dir("verify"), theory: Ledger })
		assert.equal(
			verifier.db.read(function count(instance) {
				return instance.count(Note)
			}),
			notes,
			"a fresh replica converges to the same state"
		)
		assert.equal(verifier.vector.get(asBraid(braid)), tip)
		const store = fsStore(bucket)
		const leaseTouched = await store.get(idsKey(PREFIX, 0, 0))
		assert.equal(leaseTouched, null, "explicit ids drew no lease: the lane exercised only the commit path")
		await verifier[Symbol.asyncDispose]()
	})
})
