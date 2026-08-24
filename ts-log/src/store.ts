/**
 * The object-store capability: exactly five verbs, outcomes as sums,
 * infrastructure failures on the ErrStore channel. `fsStore` is tier-1,
 * not a dev double — deployment case 5's production backend.
 *
 * The one on-disk protocol, shared with the Rust driver: create-only
 * publishes an exclusive synced temp with link(2), where EEXIST is the
 * honest exists; the etag is the blake3 of the content, lowercase hex,
 * computed on every read and never stored; `putSwap` serializes under a
 * pid-lockfile beside the key, published with the same exclusive
 * temp-plus-link discipline so it can never exist without its body (the
 * owner pid) — a contender breaks the lock iff the owner is dead, which
 * is sound on one machine only, fsStore's load-bearing deployment law.
 * `created` and `swapped` resolve only after fsync of the object file
 * and its parent directory.
 */

import * as fs from "node:fs/promises"
import * as path from "node:path"
import { internalBlake3 } from "@bjornpagen/bumbledb"
import * as errors from "@superbuilders/errors"
import { toHex } from "#bytes.ts"
import { wrapStore } from "#errors.ts"
import type { StoreKey } from "#keys.ts"
import { LOCK_SUFFIX } from "#keys.ts"

declare const etagBrand: unique symbol
type Etag = string & { readonly [etagBrand]: typeof etagBrand }

function etag(raw: string): Etag {
	return raw as Etag
}

interface Fetched {
	readonly bytes: Uint8Array
	readonly etag: Etag
}

type Poll = { readonly tag: "unchanged" } | { readonly tag: "changed"; readonly fetched: Fetched }

type Create = { readonly tag: "created"; readonly etag: Etag } | { readonly tag: "exists" }

type Swap = { readonly tag: "swapped"; readonly etag: Etag } | { readonly tag: "moved" }

interface ObjectStore {
	/** GET; null on 404. */
	get(key: StoreKey): Promise<Fetched | null>
	/** GET with If-None-Match — the cheap manifest poll. */
	getIfChanged(key: StoreKey, etag: Etag): Promise<Poll>
	/** PUT with If-None-Match: * — the log-slot arbitration primitive. */
	putCreate(key: StoreKey, bytes: Uint8Array): Promise<Create>
	/** PUT with If-Match — the manifest CAS primitive. */
	putSwap(key: StoreKey, bytes: Uint8Array, etag: Etag): Promise<Swap>
	/** DELETE, unconditional — the gc verb's tool. */
	delete(key: StoreKey): Promise<void>
}

/** Ceiling of the jittered wait between probes of a live-held lock. */
const LOCK_RETRY_MS = 10

function contentEtag(bytes: Uint8Array): Etag {
	return etag(toHex(new Uint8Array(internalBlake3(bytes))))
}

function codeOf(error: Error): string | undefined {
	return (error as NodeJS.ErrnoException).code
}

async function fsyncDir(dir: string): Promise<void> {
	const handle = await fs.open(dir, "r")
	const synced = await errors.try(handle.sync())
	await handle.close()
	if (synced.error) {
		throw errors.wrap(synced.error, `fsync directory ${dir}`)
	}
}

let tempSeq = 0

/** Write `bytes` to a fresh `wx` temp file beside `target` and fsync it.
 *  The caller publishes the synced temp atomically. */
async function syncedTemp(target: string, bytes: Uint8Array): Promise<string> {
	const dir = path.dirname(target)
	await fs.mkdir(dir, { recursive: true })
	tempSeq += 1
	const temp = path.join(dir, `.${path.basename(target)}.tmp.${process.pid}.${tempSeq}`)
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
	return temp
}

/** link(2) is the exclusivity primitive: rename replaces an existing
 *  destination, so it cannot arbitrate create-only, while link fails
 *  atomically with EEXIST across processes — exactly the
 *  If-None-Match: * contract. */
async function publishLink(temp: string, dest: string): Promise<"linked" | "occupied"> {
	const linked = await errors.try(fs.link(temp, dest))
	if (linked.error === undefined) {
		return "linked"
	}
	if (codeOf(linked.error) === "EEXIST") {
		return "occupied"
	}
	throw linked.error
}

function pidAlive(pid: number): boolean {
	const probed = errors.trySync(function probe() {
		process.kill(pid, 0)
	})
	if (probed.error === undefined) {
		return true
	}
	return codeOf(probed.error) !== "ESRCH"
}

async function acquireLock(lockPath: string): Promise<void> {
	const body = new TextEncoder().encode(String(process.pid))
	for (;;) {
		const temp = await syncedTemp(lockPath, body)
		const published = await errors.try(publishLink(temp, lockPath))
		await fs.rm(temp, { force: true })
		if (published.error) {
			throw published.error
		}
		if (published.data === "linked") {
			return
		}
		const read = await errors.try(fs.readFile(lockPath, "utf8"))
		if (read.error) {
			if (codeOf(read.error) === "ENOENT") {
				continue
			}
			throw read.error
		}
		const owner = read.data.trim()
		if (!/^\d+$/.test(owner)) {
			throw errors.new(`lockfile body is not a pid: ${lockPath}`)
		}
		if (pidAlive(Number.parseInt(owner, 10))) {
			await new Promise(function later(resolve) {
				setTimeout(resolve, Math.random() * LOCK_RETRY_MS)
			})
		} else {
			await fs.rm(lockPath, { force: true })
		}
	}
}

async function releaseLock(lockPath: string): Promise<void> {
	await fs.rm(lockPath, { force: true })
}

/** The five verbs over one local directory. One machine is load-bearing. */
function fsStore(root: string): ObjectStore {
	const rootPath = path.resolve(root)

	function objectPath(key: StoreKey): string {
		return path.join(rootPath, ...key.split("/"))
	}

	async function readFetched(target: string): Promise<Fetched | null> {
		const read = await errors.try(fs.readFile(target))
		if (read.error) {
			if (codeOf(read.error) === "ENOENT") {
				return null
			}
			throw read.error
		}
		const bytes = new Uint8Array(read.data)
		return { bytes, etag: contentEtag(bytes) }
	}

	return {
		async get(key) {
			const target = objectPath(key)
			const read = await errors.try(readFetched(target))
			if (read.error) {
				throw wrapStore(read.error, `get ${key}`)
			}
			return read.data
		},

		async getIfChanged(key, etag) {
			const target = objectPath(key)
			const read = await errors.try(readFetched(target))
			if (read.error) {
				throw wrapStore(read.error, `getIfChanged ${key}`)
			}
			if (read.data === null) {
				throw wrapStore(errors.new("poll target absent"), `getIfChanged ${key}`)
			}
			if (read.data.etag === etag) {
				return { tag: "unchanged" }
			}
			return { tag: "changed", fetched: read.data }
		},

		async putCreate(key, bytes) {
			const target = objectPath(key)
			const ran = await errors.try(
				(async function createBody(): Promise<Create> {
					const temp = await syncedTemp(target, bytes)
					const published = await errors.try(publishLink(temp, target))
					await fs.rm(temp, { force: true })
					if (published.error) {
						throw published.error
					}
					if (published.data === "occupied") {
						return { tag: "exists" }
					}
					await fsyncDir(path.dirname(target))
					return { tag: "created", etag: contentEtag(bytes) }
				})()
			)
			if (ran.error) {
				throw wrapStore(ran.error, `putCreate ${key}`)
			}
			return ran.data
		},

		async putSwap(key, bytes, etag) {
			const target = objectPath(key)
			const lock = `${target}${LOCK_SUFFIX}`
			const ran = await errors.try(
				(async function swapBody(): Promise<Swap> {
					await acquireLock(lock)
					const swapped = await errors.try(
						(async function underLock(): Promise<Swap> {
							const current = await readFetched(target)
							if (current === null || current.etag !== etag) {
								return { tag: "moved" }
							}
							const temp = await syncedTemp(target, bytes)
							const renamed = await errors.try(fs.rename(temp, target))
							if (renamed.error) {
								await fs.rm(temp, { force: true })
								throw renamed.error
							}
							await fsyncDir(path.dirname(target))
							return { tag: "swapped", etag: contentEtag(bytes) }
						})()
					)
					await releaseLock(lock)
					if (swapped.error) {
						throw swapped.error
					}
					return swapped.data
				})()
			)
			if (ran.error) {
				throw wrapStore(ran.error, `putSwap ${key}`)
			}
			return ran.data
		},

		async delete(key) {
			const target = objectPath(key)
			const ran = await errors.try(
				(async function deleteBody() {
					await fs.rm(target, { force: true })
					await fs.rm(`${target}${LOCK_SUFFIX}`, { force: true })
				})()
			)
			if (ran.error) {
				throw wrapStore(ran.error, `delete ${key}`)
			}
		}
	}
}

export type { Create, Etag, Fetched, ObjectStore, Poll, Swap }
export { etag, fsStore }
