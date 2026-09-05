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
import { regex } from "arkregex"
import { Result } from "effect"
import { LogInputError, LogOperationError } from "#errors.ts"
import { TEMP_NAMESPACE, tenantPrefix } from "#keys.ts"
import type { Replica } from "#replica.ts"
import { openReplica } from "#replica.ts"
import type { FsLease, ObjectStore } from "#store.ts"
import { acquireFsLease, encodeLease, parseLease, releaseFsLease, sweepStaleTemps, syncedTemp } from "#store.ts"

const PINNED_TENANT = "_shared"
const TENANT_ID = regex("^[A-Za-z0-9._-]+$")
/** Directory-lease lifetime: one owner per replica directory. The pool
 *  CAS-extends `expires` at one-third this TTL while the slot is open,
 *  so expiry cannot mint a second owner under a live replica. */
const DIR_LEASE_MS = 300000
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
	leaseLost?: LogOperationError
}
function codeOf(error: unknown): string | undefined {
	return typeof error === "object" && error !== null && "code" in error && typeof error.code === "string"
		? error.code
		: undefined
}
async function fsyncDir(dir: string): Promise<void> {
	const handle = await fs.open(dir, "r")
	const synced = await Promise.resolve(handle.sync()).then(Result.succeed, (cause: unknown) => Result.fail(cause))
	await handle.close()
	if (Result.isFailure(synced)) {
		throw new LogOperationError({ message: `fsync directory ${dir}`, cause: synced.failure })
	}
}
/** CAS-extend `expires` of a token we already hold. A new token would
 *  be a second owner; we rewrite the same path so identity does not
 *  change. */
async function renewDirLease(held: FsLease, ttlMs: number): Promise<void> {
	const read = await Promise.resolve(fs.readFile(held.path, "utf8")).then(Result.succeed, (cause: unknown) =>
		Result.fail(cause)
	)
	if (Result.isFailure(read)) {
		if (codeOf(read.failure) === "ENOENT") {
			return
		}
		throw read.failure
	}
	const lease = parseLease(read.success)
	if (lease === null || lease.holder !== held.holder || lease.token !== held.token) {
		throw new LogInputError({ message: "directory lease is no longer ours" })
	}
	const body = encodeLease({ holder: held.holder, token: held.token, expires: BigInt(Date.now() + ttlMs) })
	// The temp lives under the reserved `{root}/~tmp` namespace, where a
	// crash strands sweepable litter — never beside the lease path.
	const temp = await syncedTemp(held.root, body)
	const replaced = await Promise.resolve(fs.rename(temp, held.path)).then(Result.succeed, (cause: unknown) =>
		Result.fail(cause)
	)
	if (Result.isFailure(replaced)) {
		await fs.rm(temp, { force: true })
		if (codeOf(replaced.failure) === "ENOENT") {
			return
		}
		throw replaced.failure
	}
	await fsyncDir(held.dir)
}
async function directoryBytes(dir: string): Promise<number> {
	let total = 0
	const listed = await Promise.resolve(fs.readdir(dir, { withFileTypes: true, recursive: true })).then(
		Result.succeed,
		Result.fail
	)
	if (Result.isFailure(listed)) {
		return 0
	}
	for (const entry of listed.success) {
		if (entry.isFile()) {
			const stat = await Promise.resolve(fs.stat(path.join(entry.parentPath, entry.name))).then(
				Result.succeed,
				Result.fail
			)
			if (Result.isSuccess(stat)) {
				total += stat.success.size
			}
		}
	}
	return total
}
function checkTenantId(tenant: string): void {
	if (!TENANT_ID.test(tenant)) {
		throw new LogInputError({ message: `tenant id is not a single path segment: ${tenant}` })
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
	const budgetBytes = options.budgetBytes ?? 400000000
	const maxOpen = options.maxOpen ?? 32
	const dirLeaseMs = options.dirLeaseMs ?? DIR_LEASE_MS
	if (dirLeaseMs <= 0) {
		throw new LogInputError({ message: "dirLeaseMs must be positive" })
	}
	const renewEveryMs = Math.max(1, Math.floor(dirLeaseMs / 3))
	const open = new Map<string, TenantSlot<Rels>>()
	const opening = new Map<string, Promise<LiveHandle<Rels>>>()
	/** The pool root's `~tmp` holds lease mint/renew/release temps; stale
	 *  ones are crash litter swept once per pool open, best-effort. */
	const swept = Promise.resolve(sweepStaleTemps(path.join(options.dir, TEMP_NAMESPACE))).then(
		Result.succeed,
		Result.fail
	)
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
			throw new LogInputError({ message: "tenants pool is disposed" })
		}
		const slot = open.get(tenant)
		if (slot === undefined || slot.refs <= 0) {
			throw new LogInputError({ message: `no live borrow for tenant ${tenant}` })
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
			void Promise.resolve(renewAllOpen()).then(Result.succeed, (cause: unknown) => Result.fail(cause))
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
				const result = await Promise.resolve(renewDirLease(slot.lease, dirLeaseMs)).then(
					Result.succeed,
					(cause: unknown) => Result.fail(cause)
				)
				if (Result.isFailure(result) && open.get(tenant) === slot) {
					slot.leaseLost = new LogOperationError({ message: "renew tenant directory lease", cause: result.failure })
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
				throw new LogInputError({ message: "tenants pool evicted the handle it is returning" })
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
		await swept
		const dir = path.join(options.dir, tenant)
		/** The dir-lease mirrors Rust `acquire_named`: tokens live at
		 *  `{options.dir}/~lease/{tenant}`, outside the disposable replica
		 *  directory `{options.dir}/{tenant}` — the replica-open sweep
		 *  rm-rfs the replica dir's own `~lease` and must not touch the
		 *  held tenant lease. */
		const lease = await acquireFsLease(options.dir, tenant, dirLeaseMs, "refuse")
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
				throw new LogInputError({ message: "tenants pool is disposed" })
			}
			checkTenantId(tenant)
			tick += 1
			const hit = open.get(tenant)
			if (hit !== undefined) {
				return await pin(hit, tenant)
			}
			const inflight = opening.get(tenant)
			if (inflight !== undefined) {
				const ran = await Promise.resolve(inflight).then(Result.succeed, (cause: unknown) => Result.fail(cause))
				if (Result.isFailure(ran)) {
					throw ran.failure
				}
				const slot = open.get(tenant)
				if (slot === undefined) {
					throw new LogInputError({ message: "tenants pool dropped the tenant during open" })
				}
				return await pin(slot, tenant)
			}
			const pending = openOne(tenant)
			opening.set(tenant, pending)
			const ran = await Promise.resolve(pending).then(Result.succeed, (cause: unknown) => Result.fail(cause))
			opening.delete(tenant)
			if (Result.isFailure(ran)) {
				throw ran.failure
			}
			return ran.success
		},
		async evict(tenant) {
			if (closed) {
				throw new LogInputError({ message: "tenants pool is disposed" })
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

export type { OpenTenantsOptions, Tenants }
export { openTenants }
