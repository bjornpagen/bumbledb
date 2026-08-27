import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"
import { digest32 } from "#bytes.ts"
import { descriptorOf } from "#descriptor.ts"
import { LEASE_NAMESPACE, manifestKey, tenantPrefix } from "#keys.ts"
import { renderManifest } from "#manifest.ts"
import { acquireFsLease, memStore, parseLease, releaseFsLease } from "#store.ts"
import { openTenants } from "#tenants.ts"
import { Holder, Ledger } from "#test/fixtures.ts"

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-log-tenants-"))

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

async function birthTenant(store: ReturnType<typeof memStore>, root: string, tenant: string): Promise<void> {
	const created = await store.putCreate(
		manifestKey(tenantPrefix(root, tenant)),
		renderManifest({ fingerprint: digest32(descriptorOf(Ledger).fingerprintBytes), checkpoint: null })
	)
	assert.equal(created.tag, "created")
}

async function birthTenants(
	store: ReturnType<typeof memStore>,
	root: string,
	tenants: readonly string[]
): Promise<void> {
	for (const tenant of tenants) {
		await birthTenant(store, root, tenant)
	}
}

describe("per-tenant replicas", function suite() {
	test("tenants are isolated prefixes; a still-held LiveHandle cannot be LRU-replaced", async function isolation() {
		const store = memStore()
		await birthTenants(store, "prod", ["_shared", "acme", "globex"])
		const tenants = openTenants({
			store,
			root: "prod",
			dir: path.join(tmpRoot, "replicas"),
			theory: Ledger,
			maxOpen: 2
		})

		try {
			const shared = await tenants.get("_shared")
			const acme = await tenants.get("acme")
			assert.equal(
				acme.db.read(function count(instance) {
					return instance.count(Holder)
				}),
				0n
			)

			const globex = await tenants.get("globex")
			assert.notEqual(globex, acme)
			assert.equal(
				globex.db.read(function count(instance) {
					return instance.count(Holder)
				}),
				0n
			)

			const acmeAgain = await tenants.get("acme")
			assert.equal(acmeAgain, acme)
			assert.equal(
				acme.db.read(function count(instance) {
					return instance.count(Holder)
				}),
				0n
			)

			const sharedAgain = await tenants.get("_shared")
			assert.equal(sharedAgain, shared)
		} finally {
			await tenants[Symbol.asyncDispose]()
		}
	})

	test("get then evict refuses while the handle is live; after release, evict and LRU may proceed", async function borrowLifetime() {
		const store = memStore()
		await birthTenants(store, "prod", ["_shared", "acme", "globex"])
		const tenants = openTenants({
			store,
			root: "prod",
			dir: path.join(tmpRoot, "replicas-borrow"),
			theory: Ledger,
			maxOpen: 2
		})

		try {
			const shared = await tenants.get("_shared")
			const acme = await tenants.get("acme")
			assert.equal(await tenants.evict("acme"), null)
			assert.equal(
				acme.db.read(function count(instance) {
					return instance.count(Holder)
				}),
				0n
			)

			acme.release()
			const disposed = await tenants.evict("acme")
			assert.ok(disposed)

			const acmeFresh = await tenants.get("acme")
			assert.notEqual(acmeFresh, acme)
			acmeFresh.release()
			const globex = await tenants.get("globex")
			const acmeReopened = await tenants.get("acme")
			assert.notEqual(acmeReopened, acmeFresh)
			assert.notEqual(acmeReopened, acme)

			shared.release()
			globex.release()
			acmeReopened.release()
		} finally {
			await tenants[Symbol.asyncDispose]()
		}
	})

	test("the directory lease renews while the replica is open", async function leaseRenew() {
		const store = memStore()
		await birthTenant(store, "prod", "acme")
		const replicaDir = path.join(tmpRoot, "replicas-lease")
		const tenants = openTenants({
			store,
			root: "prod",
			dir: replicaDir,
			theory: Ledger,
			dirLeaseMs: 90
		})

		try {
			const acme = await tenants.get("acme")
			await new Promise(function later(resolve) {
				setTimeout(resolve, 200)
			})
			await assert.rejects(function secondOwner() {
				return acquireFsLease(replicaDir, "acme", 90, "refuse")
			})

			acme.release()
			const gone = await tenants.evict("acme")
			assert.ok(gone)
			const stolen = await acquireFsLease(replicaDir, "acme", 90, "refuse")
			await releaseFsLease(stolen)
		} finally {
			await tenants[Symbol.asyncDispose]()
		}
	})

	test("a replica open does not delete its tenant's held dir lease", async function leaseSurvivesOpen() {
		const store = memStore()
		await birthTenant(store, "prod", "acme")
		const replicaDir = path.join(tmpRoot, "replicas-lease-survives")
		const tenants = openTenants({
			store,
			root: "prod",
			dir: replicaDir,
			theory: Ledger,
			dirLeaseMs: 300_000
		})

		try {
			const acme = await tenants.get("acme")
			const leaseDir = path.join(replicaDir, LEASE_NAMESPACE, "acme")
			const tokens = fs
				.readdirSync(leaseDir)
				.filter(function decimal(name) {
					return /^\d+$/.test(name)
				})
				.map(BigInt)
			assert.ok(tokens.length >= 1)
			const top = tokens.reduce(function max(a, b) {
				return a > b ? a : b
			})
			const body = parseLease(fs.readFileSync(path.join(leaseDir, String(top)), "utf8"))
			assert.ok(body)
			assert.equal(body.holder, BigInt(process.pid))
			assert.ok(body.expires > BigInt(Date.now()))
			await assert.rejects(function secondOwner() {
				return acquireFsLease(replicaDir, "acme", 90, "refuse")
			})
			acme.release()
		} finally {
			await tenants[Symbol.asyncDispose]()
		}
	})

	test("a tenant id must be a single path segment", async function badId() {
		const store = memStore()
		const tenants = openTenants({ store, root: "prod", dir: path.join(tmpRoot, "replicas2"), theory: Ledger })
		try {
			await assert.rejects(function escapeAttempt() {
				return tenants.get("../prod")
			})
		} finally {
			await tenants[Symbol.asyncDispose]()
		}
	})
})
