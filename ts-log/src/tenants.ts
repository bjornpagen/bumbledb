/**
 * Per-tenant replicas (50, case 4): an LRU of replicas keyed by tenant
 * id — a tenant is a prefix (`<root>/t/<tenant>`), eviction closes and
 * deletes the local dir (the disposable law), and `t/_shared` is
 * pinned. Braids shard within a tenant; tenants shard the world.
 */

import * as fs from "node:fs/promises"
import * as path from "node:path"
import type { Schema, SchemaRelations } from "@bjornpagen/bumbledb"
import * as errors from "@superbuilders/errors"
import { tenantPrefix } from "#keys.ts"
import type { Replica } from "#replica.ts"
import { openReplica } from "#replica.ts"
import type { ObjectStore } from "#store.ts"

const PINNED_TENANT = "_shared"

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

interface Tenants<Rels extends SchemaRelations> extends AsyncDisposable {
	get(tenant: string): Promise<Replica<Rels>>
}

interface TenantSlot<Rels extends SchemaRelations> {
	readonly replica: Replica<Rels>
	readonly dir: string
	bytes: number
	lastUsed: number
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

function openTenants<Rels extends SchemaRelations>(options: OpenTenantsOptions<Rels>): Tenants<Rels> {
	const budgetBytes = options.budgetBytes ?? 400_000_000
	const maxOpen = options.maxOpen ?? 32
	const open = new Map<string, TenantSlot<Rels>>()
	let tick = 0
	let closed = false

	async function evictUntilWithin(): Promise<void> {
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
				if (tenant === PINNED_TENANT) {
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
				await slot.replica[Symbol.asyncDispose]()
				await fs.rm(slot.dir, { recursive: true, force: true })
			}
		}
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
				return hit.replica
			}
			const dir = path.join(options.dir, tenant)
			const replica = await openReplica({
				store: options.store,
				prefix: tenantPrefix(options.root, tenant),
				dir,
				theory: options.theory
			})
			const slot: TenantSlot<Rels> = { replica, dir, bytes: await directoryBytes(dir), lastUsed: tick }
			open.set(tenant, slot)
			await evictUntilWithin()
			return replica
		},

		async [Symbol.asyncDispose]() {
			closed = true
			for (const slot of open.values()) {
				await slot.replica[Symbol.asyncDispose]()
			}
			open.clear()
		}
	}
}

export type { OpenTenantsOptions, Tenants }
export { openTenants }
