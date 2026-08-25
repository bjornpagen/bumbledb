/**
 * Per-tenant replicas (50, case 4): an LRU of replicas keyed by tenant
 * id — a tenant is a prefix (`<root>/t/<tenant>`), eviction closes and
 * deletes the local dir (the disposable law), and `t/_shared` is
 * pinned. Braids shard within a tenant; tenants shard the world. A
 * tenant id is one StoreKey segment. `get` returns a live handle whose
 * refcount pins it against eviction for the duration of the borrow; a
 * disposed handle is a distinct type. One fenced lease owns each
 * replica directory.
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

/** Directory-lease lifetime: one owner per replica directory. */
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
}

const liveHandleBrand: unique symbol = Symbol("liveHandle")
const disposedHandleBrand: unique symbol = Symbol("disposedHandle")

type LiveHandle<Rels extends SchemaRelations> = Replica<Rels> & {
	readonly [liveHandleBrand]: typeof liveHandleBrand
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
}

interface TenantSlot<Rels extends SchemaRelations> {
	readonly replica: LiveHandle<Rels>
	readonly dir: string
	readonly lease: FsLease
	bytes: number
	lastUsed: number
	refs: number
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

function asLive<Rels extends SchemaRelations>(replica: Replica<Rels>): LiveHandle<Rels> {
	return replica as LiveHandle<Rels>
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
	const open = new Map<string, TenantSlot<Rels>>()
	const opening = new Map<string, Promise<LiveHandle<Rels>>>()
	let tick = 0
	let closed = false

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
				await disposeSlot(slot)
			}
		}
	}

	async function pin(slot: TenantSlot<Rels>, tenant: string): Promise<LiveHandle<Rels>> {
		slot.refs += 1
		try {
			await evictUntilWithin(tenant)
			const kept = open.get(tenant)
			if (kept === undefined) {
				throw errors.new("tenants pool evicted the handle it is returning")
			}
			return kept.replica
		} finally {
			slot.refs -= 1
		}
	}

	async function openOne(tenant: string): Promise<LiveHandle<Rels>> {
		const dir = path.join(options.dir, tenant)
		const lease = await acquireFsLease(dir, "dir", DIR_LEASE_MS, "refuse")
		let replica: LiveHandle<Rels>
		try {
			replica = asLive(
				await openReplica({
					store: options.store,
					prefix: tenantPrefix(options.root, tenant),
					dir,
					theory: options.theory
				})
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
				hit.lastUsed = tick
				return await pin(hit, tenant)
			}
			const inflight = opening.get(tenant)
			if (inflight !== undefined) {
				return await inflight
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
			await disposeSlot(slot)
			return asDisposed()
		},

		async [Symbol.asyncDispose]() {
			closed = true
			for (const slot of open.values()) {
				await releaseFsLease(slot.lease)
				await slot.replica[Symbol.asyncDispose]()
			}
			open.clear()
		}
	}
}

export type { DisposedHandle, LiveHandle, OpenTenantsOptions, Tenants }
export { openTenants }
