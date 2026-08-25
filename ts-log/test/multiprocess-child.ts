/**
 * The multi-process lane's child program: a real Node process opening a
 * replica and writer over one shared FsStore prefix, driven by
 * multiprocess.test.ts. Children print structured `MP {...}` lines; the
 * parent asserts hard.
 *
 * Usage: node multiprocess-child.ts recover <bucket> <dir> <id>
 *   recover — re-open a directory the parent planted as Pending at a
 *             known slot; report Settled, generation, and the slot arm
 */

import * as errors from "@superbuilders/errors"
import { braid as asBraid } from "#descriptor.ts"
import { generation, logKey } from "#keys.ts"
import { fsStore } from "#store.ts"
import { Ledger, Note } from "#test/fixtures.ts"
import { openWriter } from "#writer.ts"

const PREFIX = "prod/main"

function report(line: Record<string, unknown>): void {
	process.stdout.write(`MP ${JSON.stringify(line)}\n`)
}

async function main(): Promise<void> {
	const [role, bucket, dir, idRaw] = process.argv.slice(2)
	if (role === undefined || bucket === undefined || dir === undefined || idRaw === undefined) {
		throw errors.new("usage: multiprocess-child.ts recover <bucket> <dir> <id>")
	}
	const id = Number.parseInt(idRaw, 10)
	const store = fsStore(bucket)
	const writer = await openWriter({ store, prefix: PREFIX, dir, theory: Ledger })
	const replica = writer.replica

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
