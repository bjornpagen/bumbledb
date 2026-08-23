/**
 * The object-store capability (40): exactly five verbs, outcomes as
 * sums, infrastructure failures on the ErrStore channel. `fsStore` is
 * tier-1, not a dev double — deployment case 5's production backend.
 *
 * The fs discipline, stated: every verb on a key serializes under an
 * O_EXCL lockfile in `<root>/.locks` (the lock body is the owner's pid;
 * a lock whose pid is dead is broken and retaken — sound on one machine
 * only, which is FsStore's load-bearing deployment law). Create-only is
 * a `wx`-flag temp plus rename; `Created` and `Swapped` resolve only
 * after fsync of the object file and its parent directory. Etags are
 * fresh random tokens in a `<key>.etag` sidecar written through the
 * same temp-rename-fsync path.
 */

import * as crypto from "node:crypto"
import * as fs from "node:fs/promises"
import * as path from "node:path"
import * as errors from "@superbuilders/errors"
import { wrapStore } from "#errors.ts"

interface Fetched {
	readonly bytes: Uint8Array
	readonly etag: string
}

type Poll = { readonly tag: "unchanged" } | { readonly tag: "changed"; readonly fetched: Fetched }

type Create = { readonly tag: "created"; readonly etag: string } | { readonly tag: "exists" }

type Swap = { readonly tag: "swapped"; readonly etag: string } | { readonly tag: "moved" }

interface ObjectStore {
	/** GET; null on 404. */
	get(key: string): Promise<Fetched | null>
	/** GET with If-None-Match — the cheap manifest poll. */
	getIfChanged(key: string, etag: string): Promise<Poll>
	/** PUT with If-None-Match: * — the log-slot arbitration primitive. */
	putCreate(key: string, bytes: Uint8Array): Promise<Create>
	/** PUT with If-Match — the manifest CAS primitive. */
	putSwap(key: string, bytes: Uint8Array, etag: string): Promise<Swap>
	/** DELETE, unconditional — the gc verb's tool. */
	delete(key: string): Promise<void>
}

function checkKey(key: string): void {
	if (key.length === 0 || key.startsWith("/") || key.endsWith("/")) {
		throw errors.new(`store key is not a slash path: ${key}`)
	}
	for (const segment of key.split("/")) {
		if (segment.length === 0 || segment === "." || segment === "..") {
			throw errors.new(`store key segment is illegal: ${key}`)
		}
	}
	if (key.endsWith(".etag")) {
		throw errors.new(`store key collides with the etag sidecar suffix: ${key}`)
	}
}

function freshEtag(): string {
	return `${process.hrtime.bigint().toString(16)}-${crypto.randomBytes(8).toString("hex")}`
}

async function fsyncFile(file: string): Promise<void> {
	const handle = await fs.open(file, "r")
	const synced = await errors.try(handle.sync())
	await handle.close()
	if (synced.error) {
		throw errors.wrap(synced.error, `fsync ${file}`)
	}
}

async function fsyncDir(dir: string): Promise<void> {
	const handle = await fs.open(dir, "r")
	const synced = await errors.try(handle.sync())
	await handle.close()
	if (synced.error) {
		throw errors.wrap(synced.error, `fsync directory ${dir}`)
	}
}

/** Temp under `wx`, write, fsync, rename into place, fsync the directory. */
async function atomicWrite(target: string, bytes: Uint8Array): Promise<void> {
	const dir = path.dirname(target)
	await fs.mkdir(dir, { recursive: true })
	const temp = path.join(dir, `.tmp-${process.pid}-${crypto.randomBytes(6).toString("hex")}`)
	const handle = await fs.open(temp, "wx")
	const written = await errors.try(
		(async function writeAll() {
			await handle.writeFile(bytes)
			await handle.sync()
		})()
	)
	await handle.close()
	if (written.error) {
		await fs.rm(temp, { force: true })
		throw errors.wrap(written.error, `write ${target}`)
	}
	await fs.rename(temp, target)
	await fsyncDir(dir)
}

function pidAlive(pid: number): boolean {
	const probed = errors.trySync(function probe() {
		process.kill(pid, 0)
	})
	if (probed.error === undefined) {
		return true
	}
	const code = (probed.error as NodeJS.ErrnoException).code
	return code !== "ESRCH"
}

async function acquireLock(lockPath: string): Promise<void> {
	await fs.mkdir(path.dirname(lockPath), { recursive: true })
	const deadline = Date.now() + 10_000
	for (;;) {
		const opened = await errors.try(fs.open(lockPath, "wx"))
		if (opened.error === undefined) {
			await opened.data.writeFile(String(process.pid))
			await opened.data.close()
			return
		}
		const body = await errors.try(fs.readFile(lockPath, "utf8"))
		if (body.error === undefined) {
			const pid = Number.parseInt(body.data, 10)
			if (Number.isInteger(pid) && pid > 0 && !pidAlive(pid)) {
				await fs.rm(lockPath, { force: true })
				continue
			}
		}
		if (Date.now() > deadline) {
			throw errors.new(`store lock is held past the deadline: ${lockPath}`)
		}
		await new Promise(function later(resolve) {
			setTimeout(resolve, 4 + Math.floor(Math.random() * 6))
		})
	}
}

async function releaseLock(lockPath: string): Promise<void> {
	await fs.rm(lockPath, { force: true })
}

/** The five verbs over one local directory. One machine is load-bearing. */
function fsStore(root: string): ObjectStore {
	const rootPath = path.resolve(root)

	function objectPath(key: string): string {
		checkKey(key)
		return path.join(rootPath, ...key.split("/"))
	}

	function lockPath(key: string): string {
		return path.join(rootPath, ".locks", encodeURIComponent(key))
	}

	async function locked<R>(key: string, verb: string, body: () => Promise<R>): Promise<R> {
		const lock = lockPath(key)
		const acquired = await errors.try(acquireLock(lock))
		if (acquired.error) {
			throw wrapStore(acquired.error, `${verb} ${key}`)
		}
		const ran = await errors.try(body())
		await releaseLock(lock)
		if (ran.error) {
			throw wrapStore(ran.error, `${verb} ${key}`)
		}
		return ran.data
	}

	async function readEtag(target: string): Promise<string | null> {
		const read = await errors.try(fs.readFile(`${target}.etag`, "utf8"))
		if (read.error) {
			return null
		}
		return read.data
	}

	/** A crash window can leave an object without its etag sidecar; the
	 *  next locked reader repairs it with a fresh token. */
	async function etagOf(target: string): Promise<string> {
		const existing = await readEtag(target)
		if (existing !== null) {
			return existing
		}
		const minted = freshEtag()
		await atomicWrite(`${target}.etag`, new TextEncoder().encode(minted))
		return minted
	}

	async function exists(target: string): Promise<boolean> {
		const stat = await errors.try(fs.stat(target))
		return stat.error === undefined
	}

	return {
		async get(key) {
			return locked(key, "get", async function getBody() {
				const target = objectPath(key)
				const read = await errors.try(fs.readFile(target))
				if (read.error) {
					return null
				}
				return { bytes: new Uint8Array(read.data), etag: await etagOf(target) }
			})
		},

		async getIfChanged(key, etag) {
			return locked(key, "getIfChanged", async function pollBody(): Promise<Poll> {
				const target = objectPath(key)
				const read = await errors.try(fs.readFile(target))
				if (read.error) {
					throw errors.wrap(read.error, "poll target absent")
				}
				const current = await etagOf(target)
				if (current === etag) {
					return { tag: "unchanged" }
				}
				return { tag: "changed", fetched: { bytes: new Uint8Array(read.data), etag: current } }
			})
		},

		async putCreate(key, bytes) {
			return locked(key, "putCreate", async function createBody(): Promise<Create> {
				const target = objectPath(key)
				if (await exists(target)) {
					return { tag: "exists" }
				}
				await atomicWrite(target, bytes)
				const etag = freshEtag()
				await atomicWrite(`${target}.etag`, new TextEncoder().encode(etag))
				await fsyncFile(target)
				return { tag: "created", etag }
			})
		},

		async putSwap(key, bytes, etag) {
			return locked(key, "putSwap", async function swapBody(): Promise<Swap> {
				const target = objectPath(key)
				if (!(await exists(target))) {
					return { tag: "moved" }
				}
				const current = await etagOf(target)
				if (current !== etag) {
					return { tag: "moved" }
				}
				await atomicWrite(target, bytes)
				const next = freshEtag()
				await atomicWrite(`${target}.etag`, new TextEncoder().encode(next))
				await fsyncFile(target)
				return { tag: "swapped", etag: next }
			})
		},

		async delete(key) {
			await locked(key, "delete", async function deleteBody() {
				const target = objectPath(key)
				await fs.rm(target, { force: true })
				await fs.rm(`${target}.etag`, { force: true })
				const dirSynced = await errors.try(fsyncDir(path.dirname(target)))
				if (dirSynced.error) {
					return
				}
			})
		}
	}
}

export type { Create, Fetched, ObjectStore, Poll, Swap }
export { fsStore }
