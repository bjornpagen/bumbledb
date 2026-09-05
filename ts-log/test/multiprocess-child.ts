import { braid as asBraid } from "#descriptor.ts"
import { LogInputError } from "#errors.ts"
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
		throw new LogInputError({ message: "usage: multiprocess-child.ts recover <bucket> <dir> <id>" })
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
	throw new LogInputError({ message: `unknown role: ${role}` })
}
await main()
