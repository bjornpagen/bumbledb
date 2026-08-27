/**
 * The object-store capability: exactly five verbs, outcomes as sums,
 * infrastructure failures on the ErrStore channel. `fsStore` is tier-1,
 * not a dev double — deployment case 5's production backend.
 *
 * The one on-disk protocol, shared with the Rust driver and pinned by
 * the `lease/` corpus goldens: create-only publishes an exclusive
 * synced temp (under the reserved `~tmp` namespace) with link(2), where
 * EEXIST is the honest exists; the etag is the blake3 of the content,
 * lowercase hex, computed on every read and never stored. The mutation
 * lock is a fenced CAS lease: a versioned `LEASE/1` body whose identity
 * is a monotonic token file `{root}/~lease/{key}/{n}`, with `~head`
 * naming the current token. A contender mints the next token iff the
 * current lease's own bytes are expired — expiry is the only break.
 * Release rewrites the held token with an already-expired body so the
 * next acquirer does not wait us out. `created` and `swapped` resolve
 * only after fsync of the object file and its parent directory,
 * including newly created ancestors. Stale temps are swept at open.
 */

import * as fs from "node:fs/promises"
import * as path from "node:path"
import { internalBlake3 } from "@bjornpagen/bumbledb"
import * as errors from "@superbuilders/errors"
import { regex } from "arkregex"
import { bytesEqual, toHex, U64_MAX } from "#bytes.ts"
import { wrapStore } from "#errors.ts"
import type { StoreKey } from "#keys.ts"
import { LEASE_NAMESPACE, reservedLease, reservedTemp, TEMP_NAMESPACE } from "#keys.ts"

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

type Create =
	| { readonly tag: "created"; readonly etag: Etag }
	| { readonly tag: "exists" }
	| { readonly tag: "ambiguous" }

type Swap = { readonly tag: "swapped"; readonly etag: Etag } | { readonly tag: "moved" } | { readonly tag: "ambiguous" }

interface Lease {
	readonly holder: bigint
	readonly token: bigint
	readonly expires: bigint
}

type CreateProbe =
	| { readonly tag: "landed"; readonly etag: Etag }
	| { readonly tag: "lost"; readonly fetched: Fetched }
	| { readonly tag: "absent" }

type SwapProbe =
	| { readonly tag: "landed"; readonly etag: Etag }
	| { readonly tag: "lost"; readonly fetched: Fetched }
	| { readonly tag: "absent" }

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

/** A held fenced lease: identity is the token file `{dir}/{token}`. */
interface FsLease {
	readonly root: string
	readonly dir: string
	readonly holder: bigint
	readonly token: bigint
	readonly path: string
}

/** Ceiling of the jittered wait between probes of an unexpired lease. */
const LOCK_RETRY_MS = 10

/** How long a mutation lease stays current, in milliseconds. */
const MUTATION_TTL_MS = 5_000

/** A live temp under `~tmp` exists only for write-then-link. Anything
 *  older than this is crash litter the open sweep deletes. */
const TEMP_STALE_MS = 30_000

/** `~head` is not a StoreKey. It names the current token so a
 *  successor reads `{dir}/{n}` without listing. */
const HEAD = "~head"

function ourHolder(): bigint {
	return BigInt(process.pid)
}

function contentEtag(bytes: Uint8Array): Etag {
	return etag(toHex(new Uint8Array(internalBlake3(bytes))))
}

function codeOf(error: Error): string | undefined {
	return (error as NodeJS.ErrnoException).code
}

/** The lease body's magic first line. Version 1 of the one lock protocol. */
const LEASE_MAGIC = "LEASE/1"

function encodeLease(lease: Lease): Uint8Array {
	return new TextEncoder().encode(`${LEASE_MAGIC}\n${lease.holder}\n${lease.token}\n${lease.expires}\n`)
}

const U64_DECIMAL = regex("^\\d+$")

function u64Line(line: string): bigint | null {
	if (!U64_DECIMAL.test(line)) {
		return null
	}
	const value = BigInt(line)
	if (value > U64_MAX) {
		return null
	}
	return value
}

/** Lines as Rust `str::lines`: split on `\n`, one trailing terminator
 *  unyielded, a trailing `\r` stripped per line. */
function leaseLines(raw: string): string[] {
	const parts = raw.split("\n")
	if (parts.length > 0 && parts[parts.length - 1] === "") {
		parts.pop()
	}
	return parts.map((line) => (line.endsWith("\r") ? line.slice(0, -1) : line))
}

/** The `LEASE/1` body: magic line, then holder, token, expires as
 *  decimal u64 lines, and nothing after. Anything else is not a lease
 *  and never breakable. */
function parseLease(raw: string): Lease | null {
	const lines = leaseLines(raw)
	if (lines.length !== 4) {
		return null
	}
	const [magic, holderLine, tokenLine, expiresLine] = lines
	if (magic !== LEASE_MAGIC || holderLine === undefined || tokenLine === undefined || expiresLine === undefined) {
		return null
	}
	const holder = u64Line(holderLine)
	const token = u64Line(tokenLine)
	const expires = u64Line(expiresLine)
	if (holder === null || token === null || expires === null) {
		return null
	}
	return { holder, token, expires }
}

/** Expiry of the lease's own bytes: the only break. */
function leaseExpired(lease: Lease, nowMs: number): boolean {
	return lease.expires <= BigInt(nowMs)
}

function isUnproved(error: Error): boolean {
	const code = codeOf(error)
	return code === "EIO" || code === "EINTR" || code === "ETIMEDOUT" || code === "EAGAIN" || code === "EBUSY"
}

/** A temp or parent that vanished mid-link is not Exists and not Created. */
function isVanished(error: Error): boolean {
	return codeOf(error) === "ENOENT"
}

async function fsyncDir(dir: string): Promise<void> {
	const handle = await fs.open(dir, "r")
	const synced = await errors.try(handle.sync())
	await handle.close()
	if (synced.error) {
		throw errors.wrap(synced.error, `fsync directory ${dir}`)
	}
}

async function ensureParent(target: string, root: string): Promise<void> {
	const dir = path.dirname(target)
	const resolvedRoot = path.resolve(root)
	const ancestors: string[] = []
	let cursor = dir
	while (cursor.startsWith(resolvedRoot) && cursor !== resolvedRoot) {
		ancestors.push(cursor)
		cursor = path.dirname(cursor)
	}
	const missing: string[] = []
	for (const ancestor of ancestors.reverse()) {
		const st = await errors.try(fs.stat(ancestor))
		if (st.error) {
			if (codeOf(st.error) === "ENOENT") {
				missing.push(ancestor)
				continue
			}
			throw st.error
		}
	}
	await fs.mkdir(dir, { recursive: true })
	for (const created of missing) {
		await fsyncDir(path.dirname(created))
	}
}

let tempSeq = 0

/** Write `bytes` to a fresh `wx` temp under `{root}/~tmp` and fsync it.
 *  The caller publishes the synced temp atomically. */
async function syncedTemp(root: string, bytes: Uint8Array): Promise<string> {
	tempSeq += 1
	const temp = path.join(root, ...reservedTemp(process.pid, tempSeq).split("/"))
	await fs.mkdir(path.dirname(temp), { recursive: true })
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
		throw errors.wrap(written.error, `write temp for ${root}`)
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

/** The lease directory for `key`: `{root}/~lease/{key}`. */
function leaseDir(root: string, key: string): string {
	return path.join(root, LEASE_NAMESPACE, ...key.split("/"))
}

function tokenPath(dir: string, token: bigint): string {
	return path.join(dir, String(token))
}

function headPath(dir: string): string {
	return path.join(dir, HEAD)
}

async function readHead(dir: string): Promise<bigint | null> {
	const read = await errors.try(fs.readFile(headPath(dir), "utf8"))
	if (read.error) {
		if (codeOf(read.error) === "ENOENT") {
			return null
		}
		throw read.error
	}
	const token = u64Line(read.data.trim())
	if (token === null || token < 1n) {
		return null
	}
	return token
}

async function writeHead(root: string, dir: string, token: bigint): Promise<void> {
	const dest = headPath(dir)
	const temp = await syncedTemp(root, new TextEncoder().encode(String(token)))
	const replaced = await errors.try(fs.rename(temp, dest))
	if (replaced.error) {
		await fs.rm(temp, { force: true })
		throw replaced.error
	}
	await fsyncDir(dir)
}

/** Removes `{dir}/{1..=current-1}` after `~head` names `current`. */
async function forgetPredecessors(dir: string, current: bigint): Promise<void> {
	for (let token = current - 1n; token >= 1n; token -= 1n) {
		await fs.rm(tokenPath(dir, token), { force: true })
	}
}

interface CurrentLease {
	readonly token: bigint
	readonly lease: Lease
}

function probeFrom(dir: string, start: bigint): Promise<CurrentLease | null> {
	return (async function probe(): Promise<CurrentLease | null> {
		let best: CurrentLease | null = null
		let token = start
		while (token <= U64_MAX) {
			const read = await errors.try(fs.readFile(tokenPath(dir, token), "utf8"))
			if (read.error) {
				if (codeOf(read.error) === "ENOENT") {
					break
				}
				token += 1n
				continue
			}
			const lease = parseLease(read.data)
			if (lease !== null) {
				best = { token, lease }
			}
			token += 1n
		}
		return best
	})()
}

/** The current lease is `{dir}/{n}` for the token `~head` names, or
 *  the highest `{dir}/{n}` at or after that hint. A mint past a stale
 *  head is still visible: the probe opens `n`, `n+1`, … until a gap. */
async function currentLease(dir: string): Promise<CurrentLease | null> {
	const start = (await readHead(dir)) ?? 1n
	const found = await probeFrom(dir, start)
	if (found === null && start > 1n) {
		return await probeFrom(dir, 1n)
	}
	return found
}

async function tryMint(
	root: string,
	dir: string,
	key: string,
	token: bigint,
	holder: bigint,
	ttlMs: number
): Promise<boolean> {
	await fs.mkdir(dir, { recursive: true })
	const body = encodeLease({ holder, token, expires: BigInt(Date.now() + ttlMs) })
	const dest = path.join(root, ...reservedLease(key, token).split("/"))
	const temp = await syncedTemp(root, body)
	const published = await errors.try(publishLink(temp, dest))
	await fs.rm(temp, { force: true })
	if (published.error) {
		throw published.error
	}
	if (published.data === "occupied") {
		return false
	}
	await fsyncDir(dir)
	const headed = await errors.try(writeHead(root, dir, token))
	if (headed.error === undefined) {
		await forgetPredecessors(dir, token)
	}
	return true
}

/** Acquire the fenced lease on `{root}/~lease/{key}`: mint the next
 *  monotonic token iff the current lease's own bytes are expired.
 *  `wait` sleeps out a live holder; `refuse` throws. */
async function acquireFsLease(
	root: string,
	key: string,
	ttlMs: number,
	contend: "wait" | "refuse" = "wait"
): Promise<FsLease> {
	const dir = leaseDir(root, key)
	const holder = ourHolder()
	for (;;) {
		const current = await currentLease(dir)
		if (current !== null && !leaseExpired(current.lease, Date.now())) {
			if (contend === "refuse") {
				throw errors.new("replica directory has an owner")
			}
			await new Promise(function later(resolve) {
				setTimeout(resolve, Math.random() * LOCK_RETRY_MS)
			})
			continue
		}
		const token = current === null ? 1n : current.token + 1n
		const minted = await tryMint(root, dir, key, token, holder, ttlMs)
		if (!minted) {
			continue
		}
		return { root, dir, holder, token, path: tokenPath(dir, token) }
	}
}

/** True iff this token is still the max — a stale holder lost the CAS
 *  and must not publish. */
async function stillCurrent(held: FsLease): Promise<boolean> {
	const current = await currentLease(held.dir)
	return current !== null && current.token === held.token
}

/** Release by rewriting the held token with an already-expired body so
 *  the next acquirer does not wait us out. */
async function releaseFsLease(held: FsLease): Promise<void> {
	const body = encodeLease({ holder: held.holder, token: held.token, expires: 0n })
	const wrote = await errors.try(syncedTemp(held.root, body))
	if (wrote.error) {
		return
	}
	const replaced = await errors.try(fs.rename(wrote.data, held.path))
	if (replaced.error) {
		await fs.rm(wrote.data, { force: true })
		return
	}
	const synced = await errors.try(fsyncDir(held.dir))
	if (synced.error) {
		return
	}
}

async function sweepStaleTemps(dir: string): Promise<void> {
	const listed = await errors.try(fs.readdir(dir, { withFileTypes: true }))
	if (listed.error) {
		if (codeOf(listed.error) === "ENOENT") {
			return
		}
		throw listed.error
	}
	const now = Date.now()
	for (const entry of listed.data) {
		if (!entry.isFile()) {
			continue
		}
		const full = path.join(dir, entry.name)
		const st = await errors.try(fs.stat(full))
		if (st.error) {
			continue
		}
		if (now - st.data.mtimeMs > TEMP_STALE_MS) {
			await fs.rm(full, { force: true })
		}
	}
}

/** Sweep crash litter: stale temps under `~tmp`, and superseded tokens
 *  directly under `~lease` once `~head` names the current one. */
async function sweepReserved(root: string): Promise<void> {
	await sweepStaleTemps(path.join(root, TEMP_NAMESPACE))
	const dir = path.join(root, LEASE_NAMESPACE)
	const current = await currentLease(dir)
	if (current !== null) {
		await forgetPredecessors(dir, current.token)
	}
}

async function resolveAmbiguousCreate(store: ObjectStore, key: StoreKey, attempted: Uint8Array): Promise<CreateProbe> {
	const fetched = await store.get(key)
	if (fetched === null) {
		return { tag: "absent" }
	}
	if (bytesEqual(fetched.bytes, attempted)) {
		return { tag: "landed", etag: fetched.etag }
	}
	return { tag: "lost", fetched }
}

async function resolveAmbiguousSwap(store: ObjectStore, key: StoreKey, attempted: Uint8Array): Promise<SwapProbe> {
	const fetched = await store.get(key)
	if (fetched === null) {
		return { tag: "absent" }
	}
	if (bytesEqual(fetched.bytes, attempted)) {
		return { tag: "landed", etag: fetched.etag }
	}
	return { tag: "lost", fetched }
}

function cloneFetched(fetched: Fetched): Fetched {
	return { bytes: new Uint8Array(fetched.bytes), etag: fetched.etag }
}

/** The five verbs over one local directory. One machine is load-bearing. */
function fsStore(root: string): ObjectStore {
	const rootPath = path.resolve(root)
	const swept = (async function open() {
		await sweepReserved(rootPath)
		await fs.mkdir(path.join(rootPath, TEMP_NAMESPACE), { recursive: true })
		await fs.mkdir(path.join(rootPath, LEASE_NAMESPACE), { recursive: true })
	})()

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

	async function withKeyLease<T>(key: StoreKey, body: (held: FsLease) => Promise<T>): Promise<T> {
		const held = await acquireFsLease(rootPath, key, MUTATION_TTL_MS)
		const ran = await errors.try(body(held))
		await releaseFsLease(held)
		if (ran.error) {
			throw ran.error
		}
		return ran.data
	}

	return {
		async get(key) {
			await swept
			const target = objectPath(key)
			const read = await errors.try(readFetched(target))
			if (read.error) {
				throw wrapStore(read.error, `get ${key}`)
			}
			return read.data
		},

		async getIfChanged(key, etag) {
			await swept
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
			await swept
			const target = objectPath(key)
			const ran = await errors.try(
				(async function createBody(): Promise<Create> {
					return await withKeyLease(key, async function underLease(held): Promise<Create> {
						if (!(await stillCurrent(held))) {
							return { tag: "ambiguous" }
						}
						await ensureParent(target, rootPath)
						const temp = await syncedTemp(rootPath, bytes)
						const published = await errors.try(publishLink(temp, target))
						await fs.rm(temp, { force: true })
						if (published.error) {
							if (isUnproved(published.error) || isVanished(published.error)) {
								return { tag: "ambiguous" }
							}
							throw published.error
						}
						if (published.data === "occupied") {
							const st = await errors.try(fs.stat(target))
							if (st.error) {
								if (codeOf(st.error) === "ENOENT" || isUnproved(st.error)) {
									return { tag: "ambiguous" }
								}
								throw st.error
							}
							if (st.data.isDirectory()) {
								throw errors.new("key path is a directory")
							}
							return { tag: "exists" }
						}
						const synced = await errors.try(fsyncDir(path.dirname(target)))
						if (synced.error) {
							return { tag: "ambiguous" }
						}
						return { tag: "created", etag: contentEtag(bytes) }
					})
				})()
			)
			if (ran.error) {
				if (isUnproved(ran.error) || isVanished(ran.error)) {
					return { tag: "ambiguous" }
				}
				throw wrapStore(ran.error, `putCreate ${key}`)
			}
			return ran.data
		},

		async putSwap(key, bytes, etag) {
			await swept
			const target = objectPath(key)
			const ran = await errors.try(
				(async function swapBody(): Promise<Swap> {
					return await withKeyLease(key, async function underLease(held): Promise<Swap> {
						if (!(await stillCurrent(held))) {
							return { tag: "ambiguous" }
						}
						const current = await readFetched(target)
						if (current === null || current.etag !== etag) {
							return { tag: "moved" }
						}
						const temp = await syncedTemp(rootPath, bytes)
						const renamed = await errors.try(fs.rename(temp, target))
						if (renamed.error) {
							await fs.rm(temp, { force: true })
							if (isUnproved(renamed.error)) {
								return { tag: "ambiguous" }
							}
							throw renamed.error
						}
						const synced = await errors.try(fsyncDir(path.dirname(target)))
						if (synced.error) {
							return { tag: "ambiguous" }
						}
						return { tag: "swapped", etag: contentEtag(bytes) }
					})
				})()
			)
			if (ran.error) {
				if (isUnproved(ran.error)) {
					return { tag: "ambiguous" }
				}
				throw wrapStore(ran.error, `putSwap ${key}`)
			}
			return ran.data
		},

		async delete(key) {
			await swept
			const target = objectPath(key)
			const ran = await errors.try(
				(async function deleteBody() {
					await withKeyLease(key, async function underLease(held) {
						if (!(await stillCurrent(held))) {
							return
						}
						await fs.rm(target, { force: true })
						await fsyncDir(path.dirname(target))
					})
				})()
			)
			if (ran.error) {
				throw wrapStore(ran.error, `delete ${key}`)
			}
		}
	}
}

/**
 * The five verbs over one in-process Map. Single-process only: tests
 * and ephemeral dev inside this process, no persistence, no
 * cross-process claim, no configuration. Third `Etag` producer beside
 * `fsStore` and `s3Store`: blake3 of the content, `fsStore`'s mint,
 * carried as the same opaque brand. Every read returns a fresh buffer.
 */
function memStore(): ObjectStore {
	const objects = new Map<StoreKey, Fetched>()
	return {
		async get(key) {
			const current = objects.get(key)
			return current === undefined ? null : cloneFetched(current)
		},

		async getIfChanged(key, etag) {
			const current = objects.get(key)
			if (current === undefined) {
				throw wrapStore(errors.new("poll target absent"), `getIfChanged ${key}`)
			}
			if (current.etag === etag) {
				return { tag: "unchanged" }
			}
			return { tag: "changed", fetched: cloneFetched(current) }
		},

		async putCreate(key, bytes) {
			if (objects.has(key)) {
				return { tag: "exists" }
			}
			const copy = new Uint8Array(bytes)
			const tag = contentEtag(copy)
			objects.set(key, { bytes: copy, etag: tag })
			return { tag: "created", etag: tag }
		},

		async putSwap(key, bytes, etag) {
			const current = objects.get(key)
			if (current === undefined || current.etag !== etag) {
				return { tag: "moved" }
			}
			const copy = new Uint8Array(bytes)
			const tag = contentEtag(copy)
			objects.set(key, { bytes: copy, etag: tag })
			return { tag: "swapped", etag: tag }
		},

		async delete(key) {
			objects.delete(key)
		}
	}
}

export type { S3Config, S3Credentials } from "#store-s3.ts"
export { s3Store } from "#store-s3.ts"
export type { Create, CreateProbe, Etag, Fetched, FsLease, Lease, ObjectStore, Poll, Swap, SwapProbe }
export {
	acquireFsLease,
	encodeLease,
	etag,
	fsStore,
	MUTATION_TTL_MS,
	memStore,
	parseLease,
	releaseFsLease,
	resolveAmbiguousCreate,
	resolveAmbiguousSwap
}
