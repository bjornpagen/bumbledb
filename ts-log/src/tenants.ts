/**
 * Per-tenant replicas (50, case 4): an LRU of replicas keyed by tenant
 * id — a tenant is a prefix (`<root>/t/<tenant>`), eviction closes and
 * deletes the local dir (the disposable law), and `t/_shared` is
 * pinned. Braids shard within a tenant; tenants shard the world. A
 * tenant id is one StoreKey segment. `get` returns a live handle whose
 * refcount pins it against eviction for the duration of the borrow
 * (findings 38, 71) — the pin outlives `get` and drops only on
 * `release`; a disposed handle is a distinct type. One fenced lease
 * owns each replica directory and is CAS-renewed while the slot is
 * open, so a second live replica on that dir is unrepresentable
 * (findings 39, 48).
 */

import * as fs from "node:fs/promises"
import * as path from "node:path"
import type { Schema, SchemaRelations } from "@bjornpagen/bumbledb"
import * as errors from "@superbuilders/errors"
import { tenantPrefix } from "#keys.ts"
import type { Replica } from "#replica.ts"
import { openReplica } from "#replica.ts"
import type { FsLease, ObjectStore } from "#store.ts"
import { acquireFsLease, releaseFsLease } from "#store.ts"

const PINNED_TENANT = "_shared"

/** Directory-lease lifetime: one owner per replica directory. The pool
 *  CAS-extends `expires` at one-third this TTL while the slot is open,
 *  so expiry cannot mint a second owner under a live replica. */
const DIR_LEASE_MS = 300_000

interface OpenTenantsOptions<Rels extends SchemaRelations> {
	readonly store: ObjectStore
	readonly root: string
	readonly dir: string
	readonly theory: Schema<Rels>
	/** 50's 400 MB gate: checkpoint + working set per instance. Advisory,
	 *  measured once at each tenant's open — a replica that grows after
	 *  admission is not re-weighed until it is evicted and re-opened. */
	readonly budgetBytes?: number
	readonly maxOpen?: number
	/** Directory-lease TTL. Renewed at one-third this value while the
	 *  replica is open. Default 300_000. */
	readonly dirLeaseMs?: number
}

const liveHandleBrand: unique symbol = Symbol("liveHandle")
const disposedHandleBrand: unique symbol = Symbol("disposedHandle")

type LiveHandle<Rels extends SchemaRelations> = Replica<Rels> & {
	readonly [liveHandleBrand]: typeof liveHandleBrand
	/** Drop one borrow. LRU and `evict` become legal only when `refs`
	 *  reaches 0 — not when `get` returns. */
	release(): void
}

/** A handle whose replica is gone. Every verb is a compile-time
 *  refusal — there is no replica field to call, so `waitFor` into a
 *  disposed replica is a type error. */
type DisposedHandle = {
	readonly [disposedHandleBrand]: typeof disposedHandleBrand
}

interface Tenants<Rels extends SchemaRelations> extends AsyncDisposable {
	get(tenant: string): Promise<LiveHandle<Rels>>
	/** Evicts one tenant: closes the replica and deletes its directory.
	 *  `_shared` and a still-pinned borrow refuse by doing nothing. */
	evict(tenant: string): Promise<DisposedHandle | null>
	/** Drop one `get` borrow for `tenant`. */
	release(tenant: string): void
}

interface TenantSlot<Rels extends SchemaRelations> {
	readonly replica: LiveHandle<Rels>
	readonly dir: string
	readonly lease: FsLease
	bytes: number
	lastUsed: number
	refs: number
	leaseLost?: Error
}

function codeOf(error: Error): string | undefined {
	return (error as NodeJS.ErrnoException).code
}

function encodeLease(holder: bigint, token: bigint, expires: bigint): Uint8Array {
	return new TextEncoder().encode(`${holder}\n${token}\n${expires}\n`)
}

function parseLease(raw: string): { holder: bigint; token: bigint; expires: bigint } | null {
	const lines = raw.trim().split("\n")
	if (lines.length !== 3) {
		return null
	}
	const [holderLine, tokenLine, expiresLine] = lines
	if (holderLine === undefined || tokenLine === undefined || expiresLine === undefined) {
		return null
	}
	if (!/^-?\d+$/.test(holderLine) || !/^\d+$/.test(tokenLine) || !/^-?\d+$/.test(expiresLine)) {
		return null
	}
	return { holder: BigInt(holderLine), token: BigInt(tokenLine), expires: BigInt(expiresLine) }
}

async function fsyncDir(dir: string): Promise<void> {
	const handle = await fs.open(dir, "r")
	const synced = await errors.try(handle.sync())
	await handle.close()
	if (synced.error) {
		throw errors.wrap(synced.error, `fsync directory ${dir}`)
	}
}

/** CAS-extend `expires` of a token we already hold. A new token would
 *  be a second owner; we rewrite the same path so identity does not
 *  change. */
async function renewDirLease(held: FsLease, ttlMs: number): Promise<void> {
	const read = await errors.try(fs.readFile(held.path, "utf8"))
	if (read.error) {
		if (codeOf(read.error) === "ENOENT") {
			return
		}
		throw read.error
	}
	const lease = parseLease(read.data)
	if (lease === null || lease.holder !== held.holder || lease.token !== held.token) {
		throw errors.new("directory lease is no longer ours")
	}
	const body = encodeLease(held.holder, held.token, BigInt(Date.now() + ttlMs))
	const tmp = `${held.path}.renew.${process.pid}`
	const file = await fs.open(tmp, "w")
	const written = await errors.try(
		(async function writeAll() {
			await file.writeFile(body)
			await file.sync()
		})()
	)
	await file.close()
	if (written.error) {
		await fs.rm(tmp, { force: true })
		throw written.error
	}
	const replaced = await errors.try(fs.rename(tmp, held.path))
	if (replaced.error) {
		await fs.rm(tmp, { force: true })
		if (codeOf(replaced.error) === "ENOENT") {
			return
		}
		throw replaced.error
	}
	await fsyncDir(held.dir)
}

async function directoryBytes(dir: string): Promise<number> {
	let total = 0
	const listed = await errors.try(fs.readdir(dir, { withFileTypes: true, recursive: true }))
	if (listed.error) {
		return 0
	}
	for (const entry of listed.data) {
		if (entry.isFile()) {
			const stat = await errors.try(fs.stat(path.join(entry.parentPath, entry.name)))
			if (stat.error === undefined) {
				total += stat.data.size
			}
		}
	}
	return total
}

function checkTenantId(tenant: string): void {
	if (!/^[A-Za-z0-9._-]+$/.test(tenant)) {
		throw errors.new(`tenant id is not a single path segment: ${tenant}`)
	}
}

function asDisposed(): DisposedHandle {
	return { [disposedHandleBrand]: disposedHandleBrand }
}

async function disposeSlot<Rels extends SchemaRelations>(slot: TenantSlot<Rels>): Promise<void> {
	await slot.replica[Symbol.asyncDispose]()
	await fs.rm(slot.dir, { recursive: true, force: true })
	await releaseFsLease(slot.lease)
}

function openTenants<Rels extends SchemaRelations>(options: OpenTenantsOptions<Rels>): Tenants<Rels> {
	const budgetBytes = options.budgetBytes ?? 400_000_000
	const maxOpen = options.maxOpen ?? 32
	const dirLeaseMs = options.dirLeaseMs ?? DIR_LEASE_MS
	if (dirLeaseMs <= 0) {
		throw errors.new("dirLeaseMs must be positive")
	}
	const renewEveryMs = Math.max(1, Math.floor(dirLeaseMs / 3))
	const open = new Map<string, TenantSlot<Rels>>()
	const opening = new Map<string, Promise<LiveHandle<Rels>>>()
	let tick = 0
	let closed = false
	let renewTimer: ReturnType<typeof setInterval> | undefined
	let leaseGate = Promise.resolve()

	function withLeaseGate<T>(body: () => Promise<T>): Promise<T> {
		const run = leaseGate.then(body, body)
		leaseGate = run.then(
			function absorb() {
				return undefined
			},
			function absorbFailure() {
				return undefined
			}
		)
		return run
	}

	function dropBorrow(tenant: string): void {
		if (closed) {
			throw errors.new("tenants pool is disposed")
		}
		const slot = open.get(tenant)
		if (slot === undefined || slot.refs <= 0) {
			throw errors.new(`no live borrow for tenant ${tenant}`)
		}
		slot.refs -= 1
	}

	function brandLive(replica: Replica<Rels>, tenant: string): LiveHandle<Rels> {
		const handle = replica as LiveHandle<Rels>
		Object.defineProperty(handle, liveHandleBrand, { value: liveHandleBrand })
		Object.defineProperty(handle, "release", {
			value: function release() {
				dropBorrow(tenant)
			},
			enumerable: false
		})
		return handle
	}

	function armRenew(): void {
		if (renewTimer !== undefined) {
			return
		}
		renewTimer = setInterval(function tickRenew() {
			void errors.try(renewAllOpen())
		}, renewEveryMs)
	}

	function disarmRenew(): void {
		if (open.size > 0 || renewTimer === undefined) {
			return
		}
		clearInterval(renewTimer)
		renewTimer = undefined
	}

	async function renewAllOpen(): Promise<void> {
		await withLeaseGate(async function renewHeld() {
			for (const [tenant, slot] of open) {
				const result = await errors.try(renewDirLease(slot.lease, dirLeaseMs))
				if (result.error !== undefined && open.get(tenant) === slot) {
					slot.leaseLost = result.error
				}
			}
		})
	}

	async function evictUntilWithin(keep: string): Promise<void> {
		for (;;) {
			let bytes = 0
			for (const slot of open.values()) {
				bytes += slot.bytes
			}
			if (open.size <= maxOpen && bytes <= budgetBytes) {
				return
			}
			let victim: string | null = null
			let oldest = Number.POSITIVE_INFINITY
			for (const [tenant, slot] of open) {
				if (tenant === PINNED_TENANT || tenant === keep || slot.refs > 0) {
					continue
				}
				if (slot.lastUsed < oldest) {
					oldest = slot.lastUsed
					victim = tenant
				}
			}
			if (victim === null) {
				return
			}
			const slot = open.get(victim)
			open.delete(victim)
			if (slot !== undefined) {
				await withLeaseGate(function disposeVictim() {
					return disposeSlot(slot)
				})
			}
		}
	}

	/** Increment the borrow. The pin outlives `get` — a `finally`
	 *  decrement here would make LRU/`evict` legal while the caller
	 *  still holds the LiveHandle (finding 38). Decrement only if this
	 *  `get` fails before the handle is in the caller's hands. */
	async function pin(slot: TenantSlot<Rels>, tenant: string): Promise<LiveHandle<Rels>> {
		if (slot.leaseLost !== undefined) {
			throw slot.leaseLost
		}
		slot.refs += 1
		slot.lastUsed = tick
		try {
			await evictUntilWithin(tenant)
			const kept = open.get(tenant)
			if (kept === undefined) {
				throw errors.new("tenants pool evicted the handle it is returning")
			}
			await withLeaseGate(function renewOnPin() {
				return renewDirLease(kept.lease, dirLeaseMs)
			})
			return kept.replica
		} catch (error) {
			slot.refs -= 1
			throw error
		}
	}

	async function openOne(tenant: string): Promise<LiveHandle<Rels>> {
		const dir = path.join(options.dir, tenant)
		const lease = await acquireFsLease(dir, "dir", dirLeaseMs, "refuse")
		let replica: LiveHandle<Rels>
		try {
			replica = brandLive(
				await openReplica({
					store: options.store,
					prefix: tenantPrefix(options.root, tenant),
					dir,
					theory: options.theory
				}),
				tenant
			)
		} catch (error) {
			await releaseFsLease(lease)
			throw error
		}
		const slot: TenantSlot<Rels> = {
			replica,
			dir,
			lease,
			bytes: await directoryBytes(dir),
			lastUsed: tick,
			refs: 0
		}
		open.set(tenant, slot)
		armRenew()
		return await pin(slot, tenant)
	}

	return {
		async get(tenant) {
			if (closed) {
				throw errors.new("tenants pool is disposed")
			}
			checkTenantId(tenant)
			tick += 1
			const hit = open.get(tenant)
			if (hit !== undefined) {
				return await pin(hit, tenant)
			}
			const inflight = opening.get(tenant)
			if (inflight !== undefined) {
				const ran = await errors.try(inflight)
				if (ran.error) {
					throw ran.error
				}
				const slot = open.get(tenant)
				if (slot === undefined) {
					throw errors.new("tenants pool dropped the tenant during open")
				}
				return await pin(slot, tenant)
			}
			const pending = openOne(tenant)
			opening.set(tenant, pending)
			const ran = await errors.try(pending)
			opening.delete(tenant)
			if (ran.error) {
				throw ran.error
			}
			return ran.data
		},

		async evict(tenant) {
			if (closed) {
				throw errors.new("tenants pool is disposed")
			}
			checkTenantId(tenant)
			if (tenant === PINNED_TENANT) {
				return null
			}
			const slot = open.get(tenant)
			if (slot === undefined || slot.refs > 0) {
				return null
			}
			open.delete(tenant)
			await withLeaseGate(function disposeEvicted() {
				return disposeSlot(slot)
			})
			disarmRenew()
			return asDisposed()
		},

		release(tenant) {
			dropBorrow(tenant)
		},

		async [Symbol.asyncDispose]() {
			closed = true
			if (renewTimer !== undefined) {
				clearInterval(renewTimer)
				renewTimer = undefined
			}
			await withLeaseGate(async function disposePool() {
				for (const slot of open.values()) {
					await releaseFsLease(slot.lease)
					await slot.replica[Symbol.asyncDispose]()
				}
				open.clear()
			})
		}
	}
}

export type { DisposedHandle, LiveHandle, OpenTenantsOptions, Tenants }
export { openTenants }
