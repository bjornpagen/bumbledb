/**
 * The multi-process lane's child program: a real Node process opening a
 * replica and writer over one shared FsStore prefix, driven by
 * multiprocess.test.ts. Children print structured `MP {...}` lines; the
 * parent asserts hard.
 *
 * Usage: node multiprocess-child.ts <role> <bucket> <dir> <id> [count]
 *   disjoint — after the go barrier, commit <count> Notes with ids
 *              disjoint per child, reporting every ack
 *   fd       — after the go barrier, commit one Booking on the shared
 *              slot determinant, reporting the serial verdict
 *   recover  — re-open a directory the parent planted as Pending at a
 *              known slot; report Settled, generation, and the slot arm
 */

import * as fs from "node:fs"
import * as path from "node:path"
import * as errors from "@superbuilders/errors"
import { braid as asBraid } from "#descriptor.ts"
import { generation, logKey } from "#keys.ts"
import { fsStore } from "#store.ts"
import { Booking, Ledger, Note } from "#test/fixtures.ts"
import { openWriter } from "#writer.ts"

const PREFIX = "prod/main"

function report(line: Record<string, unknown>): void {
	process.stdout.write(`MP ${JSON.stringify(line)}\n`)
}

async function waitForGo(bucket: string): Promise<void> {
	const go = path.join(bucket, "..", "go")
	const deadline = Date.now() + 30_000
	while (!fs.existsSync(go)) {
		if (Date.now() > deadline) {
			throw errors.new("start barrier never appeared")
		}
		await new Promise(function later(resolve) {
			setTimeout(resolve, 2)
		})
	}
}

async function main(): Promise<void> {
	const [role, bucket, dir, idRaw, countRaw] = process.argv.slice(2)
	if (role === undefined || bucket === undefined || dir === undefined || idRaw === undefined) {
		throw errors.new("usage: multiprocess-child.ts <role> <bucket> <dir> <id> [count]")
	}
	const id = Number.parseInt(idRaw, 10)
	const store = fsStore(bucket)
	const writer = await openWriter({ store, prefix: PREFIX, dir, theory: Ledger })
	const replica = writer.replica

	if (role === "disjoint") {
		const count = Number.parseInt(countRaw ?? "1", 10)
		await waitForGo(bucket)
		for (let i = 0; i < count; i++) {
			const noteId = BigInt(id * 1000 + i)
			const outcome = await writer.commit(function record(batch) {
				batch.insert(Note, [{ id: noteId, body: `note from child ${id} commit ${i}` }])
				return 0
			})
			if (outcome.tag !== "accepted") {
				throw errors.new(`a disjoint commit was rejected: child ${id} commit ${i}`)
			}
			report({
				tag: "ack",
				id,
				noteId: String(noteId),
				braid: outcome.braid,
				generation: String(outcome.generation)
			})
		}
		await replica[Symbol.asyncDispose]()
		return
	}

	if (role === "fd") {
		await waitForGo(bucket)
		const outcome = await writer.commit(function record(batch) {
			batch.insert(Booking, [{ id: BigInt(100 + id), holder: 1n, slot: "hot", at: { start: 1n, end: 2n } }])
			return 0
		})
		if (outcome.tag === "accepted") {
			report({ tag: "verdict", id, result: "accepted", braid: outcome.braid, generation: String(outcome.generation) })
		} else {
			const canonical = outcome.violations[0]?.canonical
			if (canonical === undefined) {
				throw errors.new("a rejection carried no violation")
			}
			report({ tag: "verdict", id, result: "rejected", canonical })
		}
		await replica[Symbol.asyncDispose]()
		return
	}

	if (role === "recover") {
		const count = replica.db.read(function countNotes(instance) {
			return instance.count(Note)
		})
		const tip = replica.vector.get(asBraid("c00000002")) ?? 0n
		const slot = await store.get(logKey(PREFIX, asBraid("c00000002"), generation(1n)))
		report({
			tag: "recovered",
			id,
			arm: "Settled",
			braid: "c00000002",
			generation: String(tip),
			notes: String(count),
			slot: slot === null ? "absent" : "present"
		})
		await replica[Symbol.asyncDispose]()
		return
	}

	throw errors.new(`unknown role: ${role}`)
}

await main()
