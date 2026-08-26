/**
 * The object-store capability: exactly five verbs, outcomes as sums,
 * infrastructure failures on the ErrStore channel. `fsStore` is tier-1,
 * not a dev double — deployment case 5's production backend.
 *
 * The one on-disk protocol, shared with the Rust driver: create-only
 * publishes an exclusive synced temp with link(2), where EEXIST is the
 * honest exists; the etag is the blake3 of the content, lowercase hex,
 * computed on every read and never stored. The mutation lock is a
 * fenced CAS lease `{holder, token, expires}` in the reserved
 * namespace, acquired and broken only through the store's own CAS;
 * a contender breaks a lease iff it is expired. Liveness is
 * Alive | Dead | Unknown; Unknown never breaks a lease. `created` and
 * `swapped` resolve only after fsync of the object file and its parent
 * directory, including newly created ancestors. Temps and leftover
 * expired leases are swept at open.
 */

import * as fs from "node:fs/promises"
import * as path from "node:path"
import { internalBlake3 } from "@bjornpagen/bumbledb"
import * as errors from "@superbuilders/errors"
import { regex } from "arkregex"
import { bytesEqual, toHex } from "#bytes.ts"
import { wrapStore } from "#errors.ts"
import type { StoreKey } from "#keys.ts"

function reservedTemp(basename: string, pid: number, seq: number): string {
	return `.${basename}.tmp.${pid}.${seq}`
}

function reservedLease(basename: string, token: bigint): string {
	return `.${basename}.lease.${token}`
}

function isReservedName(name: string): boolean {
	return name.startsWith(".")
}

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

type Liveness = { readonly tag: "alive" } | { readonly tag: "dead" } | { readonly tag: "unknown" }

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

interface FsLease {
	readonly dir: string
	readonly name: string
	readonly holder: bigint
	readonly token: bigint
	readonly path: string
}

interface LeaseFile {
	readonly path: string
	readonly token: bigint
	readonly lease: Lease | null
	readonly readable: boolean
}

/** Ceiling of the jittered wait between probes of an unexpired lease. */
const LOCK_RETRY_MS = 10

/** Mutation-lease lifetime: long enough for one verb, short enough to expire after a crash. */
const MUTATION_LEASE_MS = 30_000

function ourHolder(): bigint {
	return BigInt(process.pid)
}

function contentEtag(bytes: Uint8Array): Etag {
	return etag(toHex(new Uint8Array(internalBlake3(bytes))))
}

function codeOf(error: Error): string | undefined {
	return (error as NodeJS.ErrnoException).code
}

function encodeLease(lease: Lease): Uint8Array {
	return new TextEncoder().encode(`${lease.holder}\n${lease.token}\n${lease.expires}\n`)
}

const SIGNED_DECIMAL = regex("^-?\\d+$")
const UNSIGNED_DECIMAL = regex("^\\d+$")

function parseLease(raw: string): Lease | null {
	const lines = raw.trim().split("\n")
	if (lines.length !== 3) {
		return null
	}
	const [holderLine, tokenLine, expiresLine] = lines
	if (holderLine === undefined || tokenLine === undefined || expiresLine === undefined) {
		return null
	}
	if (!SIGNED_DECIMAL.test(holderLine) || !UNSIGNED_DECIMAL.test(tokenLine) || !SIGNED_DECIMAL.test(expiresLine)) {
		return null
	}
	return { holder: BigInt(holderLine), token: BigInt(tokenLine), expires: BigInt(expiresLine) }
}

function leaseExpired(lease: Lease, nowMs: number): boolean {
	return lease.expires <= BigInt(nowMs)
}

/** EPERM/unreadable/unparseable ⇒ Unknown. Expiry is a fact of the lease bytes. */
function livenessOf(lease: Lease | null, readable: boolean, nowMs: number): Liveness {
	if (!readable || lease === null) {
		return { tag: "unknown" }
	}
	if (leaseExpired(lease, nowMs)) {
		return { tag: "dead" }
	}
	return { tag: "alive" }
}

/** A lease is breakable only when liveness is Dead. Unknown never breaks. */
function breakable(liveness: Liveness): boolean {
	switch (liveness.tag) {
		case "dead":
			return true
		case "alive":
			return false
		case "unknown":
			return false
	}
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

/** Write `bytes` to a fresh `wx` temp file beside `target` and fsync it.
 *  The caller publishes the synced temp atomically. */
async function syncedTemp(target: string, bytes: Uint8Array, root: string): Promise<string> {
	await ensureParent(target, root)
	tempSeq += 1
	const temp = path.join(path.dirname(target), reservedTemp(path.basename(target), process.pid, tempSeq))
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

const LEASE_NAME_RE = regex("^\\.(.+)\\.lease\\.(\\d+)$")
const TEMP_NAME_RE = regex("^\\.(.+)\\.tmp\\.\\d+\\.\\d+$")

async function listLeaseFiles(dir: string, name: string): Promise<LeaseFile[]> {
	const listed = await errors.try(fs.readdir(dir))
	if (listed.error) {
		if (codeOf(listed.error) === "ENOENT") {
			return []
		}
		throw listed.error
	}
	const found: LeaseFile[] = []
	for (const entry of listed.data) {
		const match = LEASE_NAME_RE.exec(entry)
		if (match === null || match[1] !== name || match[2] === undefined) {
			continue
		}
		const filePath = path.join(dir, entry)
		const read = await errors.try(fs.readFile(filePath, "utf8"))
		if (read.error) {
			if (codeOf(read.error) === "ENOENT") {
				continue
			}
			found.push({ path: filePath, token: BigInt(match[2]), lease: null, readable: false })
			continue
		}
		found.push({
			path: filePath,
			token: BigInt(match[2]),
			lease: parseLease(read.data),
			readable: true
		})
	}
	return found
}

/** Drop a lease file only when its bytes still name this holder and token. */
async function casDrop(leasePath: string, expected: Lease): Promise<void> {
	const read = await errors.try(fs.readFile(leasePath, "utf8"))
	if (read.error) {
		return
	}
	const lease = parseLease(read.data)
	if (lease === null) {
		return
	}
	if (lease.holder !== expected.holder || lease.token !== expected.token) {
		return
	}
	await fs.rm(leasePath, { force: true })
}

async function heldByUs(held: FsLease): Promise<boolean> {
	const read = await errors.try(fs.readFile(held.path, "utf8"))
	if (read.error) {
		return false
	}
	const lease = parseLease(read.data)
	if (lease === null) {
		return false
	}
	return lease.holder === held.holder && lease.token === held.token
}

async function acquireFsLease(
	dir: string,
	name: string,
	ttlMs: number,
	contend: "wait" | "refuse" = "wait"
): Promise<FsLease> {
	await fs.mkdir(dir, { recursive: true })
	for (;;) {
		const now = Date.now()
		const incumbents = await listLeaseFiles(dir, name)
		let highest = 0n
		let blocked = false
		const dead: LeaseFile[] = []
		for (const incumbent of incumbents) {
			if (incumbent.token > highest) {
				highest = incumbent.token
			}
			const liveness = livenessOf(incumbent.lease, incumbent.readable, now)
			switch (liveness.tag) {
				case "dead":
					dead.push(incumbent)
					break
				case "alive": {
					const holder = incumbent.lease === null ? null : incumbent.lease.holder
					if (holder === ourHolder()) {
						blocked = true
					} else {
						blocked = true
					}
					break
				}
				case "unknown":
					blocked = true
					break
			}
		}
		if (blocked) {
			if (contend === "refuse") {
				throw errors.new("replica directory has an owner")
			}
			await new Promise(function later(resolve) {
				setTimeout(resolve, Math.random() * LOCK_RETRY_MS)
			})
			continue
		}
		const holder = ourHolder()
		const token = highest + 1n
		const dest = path.join(dir, reservedLease(name, token))
		const body = encodeLease({
			holder,
			token,
			expires: BigInt(now + ttlMs)
		})
		const wrote = await errors.try(syncedTemp(dest, body, dir))
		if (wrote.error) {
			if (isVanished(wrote.error) || isUnproved(wrote.error)) {
				continue
			}
			throw wrote.error
		}
		const published = await errors.try(publishLink(wrote.data, dest))
		await fs.rm(wrote.data, { force: true })
		if (published.error) {
			if (isVanished(published.error) || isUnproved(published.error)) {
				continue
			}
			throw published.error
		}
		if (published.data === "occupied") {
			continue
		}
		await fsyncDir(dir)
		const held: FsLease = { dir, name, holder, token, path: dest }
		for (const stale of dead) {
			if (stale.lease !== null) {
				await casDrop(stale.path, stale.lease)
			}
		}
		return held
	}
}

async function releaseFsLease(held: FsLease): Promise<void> {
	const read = await errors.try(fs.readFile(held.path, "utf8"))
	if (read.error) {
		return
	}
	const lease = parseLease(read.data)
	if (lease === null) {
		return
	}
	if (lease.holder !== held.holder || lease.token !== held.token) {
		return
	}
	await casDrop(held.path, lease)
	const synced = await errors.try(fsyncDir(held.dir))
	if (synced.error) {
		return
	}
}

async function sweepReserved(root: string): Promise<void> {
	const listed = await errors.try(fs.readdir(root, { withFileTypes: true }))
	if (listed.error) {
		if (codeOf(listed.error) === "ENOENT") {
			return
		}
		throw listed.error
	}
	const now = Date.now()
	for (const entry of listed.data) {
		const full = path.join(root, entry.name)
		if (entry.isDirectory()) {
			if (isReservedName(entry.name)) {
				continue
			}
			await sweepReserved(full)
			continue
		}
		if (!entry.isFile() || !isReservedName(entry.name)) {
			continue
		}
		if (TEMP_NAME_RE.test(entry.name)) {
			await fs.rm(full, { force: true })
			continue
		}
		const match = LEASE_NAME_RE.exec(entry.name)
		if (match === null) {
			continue
		}
		const read = await errors.try(fs.readFile(full, "utf8"))
		if (read.error) {
			continue
		}
		const lease = parseLease(read.data)
		const liveness = livenessOf(lease, true, now)
		if (breakable(liveness) && lease !== null) {
			await casDrop(full, lease)
		}
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
	const swept = sweepReserved(rootPath)

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

	async function withKeyLease<T>(target: string, body: (held: FsLease) => Promise<T>): Promise<T> {
		const held = await acquireFsLease(path.dirname(target), path.basename(target), MUTATION_LEASE_MS)
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
					return await withKeyLease(target, async function underLease(held): Promise<Create> {
						if (!(await heldByUs(held))) {
							return { tag: "ambiguous" }
						}
						const temp = await syncedTemp(target, bytes, rootPath)
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
					return await withKeyLease(target, async function underLease(held): Promise<Swap> {
						if (!(await heldByUs(held))) {
							return { tag: "ambiguous" }
						}
						const current = await readFetched(target)
						if (current === null || current.etag !== etag) {
							return { tag: "moved" }
						}
						const temp = await syncedTemp(target, bytes, rootPath)
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
					await withKeyLease(target, async function underLease(held) {
						if (!(await heldByUs(held))) {
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
export type { Create, CreateProbe, Etag, Fetched, FsLease, Lease, Liveness, ObjectStore, Poll, Swap, SwapProbe }
export { acquireFsLease, etag, fsStore, memStore, releaseFsLease, resolveAmbiguousCreate, resolveAmbiguousSwap }
