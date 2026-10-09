import { randomUUID } from "node:crypto"
import * as fs from "node:fs/promises"
import * as path from "node:path"
import { Effect } from "effect"
import { DbError } from "../errors.ts"
import type { Body, Bucket, Created, Deleted, Fetched, Listed, ObjectStore, Reply, Target } from "./io.ts"

function code(cause: unknown): string | undefined {
	return typeof cause === "object" && cause !== null && "code" in cause && typeof cause.code === "string"
		? cause.code
		: undefined
}

function attempt<A>(operation: string, run: () => Promise<A>): Effect.Effect<A, DbError> {
	return Effect.tryPromise({
		try: run,
		catch: (cause) => new DbError({ operation, reason: { _tag: "Io", kind: code(cause) ?? "Unknown" } })
	})
}

async function syncDirectory(directory: string): Promise<void> {
	const handle = await fs.open(directory, "r")
	try {
		await handle.sync()
	} finally {
		await handle.close()
	}
}

async function writeDurably(file: string, body: Body): Promise<void> {
	if (body._tag === "File") await fs.copyFile(body.path, file, fs.constants.COPYFILE_EXCL)
	const handle = await fs.open(file, body._tag === "File" ? "r+" : "wx")
	try {
		if (body._tag === "Bytes") await handle.writeFile(body.bytes)
		await handle.sync()
	} finally {
		await handle.close()
	}
}

/**
 * A store in a local directory, one file per key under `root`. A create writes a temporary file,
 * fsyncs it, hard-links it into place (the link fails if the key exists) and fsyncs the directory,
 * so a created key is durable and never torn. The store clock is the local clock.
 */
function make(root: string): ObjectStore {
	const file = (key: string) => path.join(root, ...key.split("/"))
	const now = () => BigInt(Date.now())
	return {
		get: (_bucket: Bucket, key: string, target: Target) =>
			attempt("FsStore.get", async (): Promise<Reply<Fetched>> => {
				try {
					if (target._tag === "Memory") {
						const [bytes, stat] = await Promise.all([fs.readFile(file(key)), fs.stat(file(key), { bigint: true })])
						return {
							date: now(),
							result: { _tag: "Body", bytes: new Uint8Array(bytes), lastModified: stat.mtimeMs }
						}
					}
					const stat = await fs.stat(file(key), { bigint: true })
					await fs.copyFile(file(key), target.path)
					return { date: now(), result: { _tag: "Saved", lastModified: stat.mtimeMs } }
				} catch (cause) {
					if (code(cause) === "ENOENT") return { date: now(), result: { _tag: "Missing" } }
					throw cause
				}
			}),
		putIfAbsent: (_bucket: Bucket, key: string, body: Body) =>
			attempt("FsStore.putIfAbsent", async (): Promise<Reply<Created>> => {
				const target = file(key)
				const directory = path.dirname(target)
				await fs.mkdir(directory, { recursive: true })
				const temporary = path.join(directory, `.tmp-${randomUUID()}`)
				try {
					await writeDurably(temporary, body)
					try {
						await fs.link(temporary, target)
					} catch (cause) {
						if (code(cause) === "EEXIST") return { date: now(), result: { _tag: "Occupied" } }
						throw cause
					}
				} finally {
					await fs.rm(temporary, { force: true })
				}
				await syncDirectory(directory)
				return { date: now(), result: { _tag: "Created" } }
			}),
		list: (prefix: string, startAfter: string | null, maxKeys: number) =>
			attempt("FsStore.list", async (): Promise<Reply<Listed>> => {
				const directory = path.posix.dirname(`${prefix}_`)
				let names: string[]
				try {
					names = await fs.readdir(path.join(root, ...directory.split("/")))
				} catch (cause) {
					if (code(cause) === "ENOENT") return { date: now(), result: { _tag: "Keys", keys: [] } }
					throw cause
				}
				const keys = names
					.filter((name) => !name.startsWith(".tmp-"))
					.map((name) => (directory === "." ? name : `${directory}/${name}`))
					.filter((key) => key.startsWith(prefix) && (startAfter === null || key > startAfter))
					.sort()
					.slice(0, maxKeys)
				return { date: now(), result: { _tag: "Keys", keys } }
			}),
		delete: (key: string) =>
			attempt("FsStore.delete", async (): Promise<Reply<Deleted>> => {
				await fs.rm(file(key), { force: true })
				await syncDirectory(path.dirname(file(key))).catch((cause) => {
					if (code(cause) !== "ENOENT") throw cause
				})
				return { date: now(), result: { _tag: "Deleted" } }
			})
	}
}

const FsStore = Object.freeze({ make })

export { FsStore }
