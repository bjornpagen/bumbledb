import { execFile } from "node:child_process"
import { promisify } from "node:util"
import { key, relation, schema, str, u64 } from "@bjornpagen/bumbledb"
import { openWriter, s3Store } from "@bjornpagen/bumbledb-log"
import { holdReplica } from "./handle.ts"
import { parseRequest } from "./request.ts"

const exec = promisify(execFile)

const Note = relation("note", { id: u64, body: str })
const Notes = schema("Notes", { Note }, [key(Note, ["id"])])

const bucket = process.env.BUCKET ?? ""
const prefix = process.env.PREFIX ?? "log"
const region = process.env.AWS_REGION ?? "us-east-1"

const store = s3Store({
	region,
	bucket,
	prefix,
	credentials: {
		accessKeyId: process.env.AWS_ACCESS_KEY_ID ?? "",
		secretAccessKey: process.env.AWS_SECRET_ACCESS_KEY ?? "",
		...(process.env.AWS_SESSION_TOKEN === undefined || process.env.AWS_SESSION_TOKEN.length === 0
			? {}
			: { sessionToken: process.env.AWS_SESSION_TOKEN })
	}
})

const acquire = holdReplica(async function open() {
	const openedAt = performance.now()
	const writer = await openWriter({ store, prefix: "", dir: "/tmp/store", theory: Notes })
	console.log(`open ${Math.round(performance.now() - openedAt)}`)
	return { replica: writer.replica, writer }
})

function json(value: unknown): string {
	return JSON.stringify(value, function replacer(_key, item: unknown) {
		return typeof item === "bigint" ? String(item) : item
	})
}

export default async function handler(event: unknown): Promise<{ statusCode: number; body: string }> {
	const request = parseRequest(event)
	if (request.tag === "refused") {
		return { statusCode: request.status, body: json({ error: request.reason }) }
	}
	if (request.tag === "duty") {
		const ran = await exec("/opt/bin/bumbledb-log-duty", [
			"--once",
			"--store",
			"s3",
			"--bucket",
			bucket,
			"--dir",
			"/tmp/duty",
			"--theory",
			"/opt/bin/theory.json",
			"--region",
			region,
			"--s3-prefix",
			prefix
		])
		return { statusCode: 200, body: json({ stdout: ran.stdout, stderr: ran.stderr }) }
	}

	const held = await acquire()
	if (held.tag === "unavailable") {
		return { statusCode: held.status, body: json({ error: held.reason }) }
	}
	const { replica, writer } = held.value
	await replica.refresh()
	if (request.tag === "write") {
		const started = performance.now()
		const out = await writer.commit(function record(batch) {
			batch.insert(Note, [{ id: request.id, body: request.body }])
			return request.id
		})
		console.log(`commit ${Math.round(performance.now() - started)}`)
		return { statusCode: 200, body: json(out) }
	}

	return {
		statusCode: 200,
		body: json(
			replica.db.read(function scan(instance) {
				return instance.scan(Note)
			})
		)
	}
}
