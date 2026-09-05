/**
 * Local request tests — the LocalHistory development flow end to end
 * through the REAL route handlers (APP-01/02/03 local halves): provision a
 * tenant from the generated plan chain, then drive Request objects at the
 * handlers. Requires the generated artifacts (run `pnpm run generate`
 * first — an F3 step); a missing chain FAILS loudly instead of skipping
 * green.
 *
 * Verification: NotRun until F3 (campaign phase rule).
 */
import assert from "node:assert/strict"
import { randomBytes } from "node:crypto"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { before, test } from "node:test"

const scratch = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-notes-test-"))
process.env.SESSION_SECRET = "test-secret-test-secret-test-secret!"
process.env.BUMBLEDB_TENANT_BINDINGS_FILE = path.join(scratch, "tenants.json")
process.env.BUMBLEDB_REQUEST_RECORDS = path.join(scratch, "requests")
process.env.BUMBLEDB_MATERIALIZATION_DIR = path.join(scratch, "cache")

const TENANT_A = "student-a"
const TENANT_B = "student-b"

function hex(): string {
	return Buffer.from(randomBytes(16)).toString("hex")
}

async function provision(tenantId: string): Promise<void> {
	const [{ Effect }, { NativeRuntime }, log, migrations, bindings, policy] = await Promise.all([
		import("effect"),
		import("@bjornpagen/bumbledb"),
		import("@bjornpagen/bumbledb-log"),
		import("@bjornpagen/bumbledb-log/migrations"),
		import("../src/db/bindings.ts"),
		import("../src/db/runtime-policy.ts")
	])
	const { Id128 } = await import("@bjornpagen/bumbledb")
	const { Result } = await import("effect")
	const migrationsDir = path.join(process.cwd(), "bumbledb", "migrations")
	assert.ok(
		fs.existsSync(path.join(migrationsDir, "manifest.json")),
		"generated migrations are required: run `pnpm run generate` (F3) before the route tests"
	)
	const manifest = JSON.parse(fs.readFileSync(path.join(migrationsDir, "manifest.json"), "utf8")) as {
		entries: ReadonlyArray<{ id: string }>
	}
	const plans = manifest.entries.map((entry) =>
		JSON.parse(fs.readFileSync(path.join(migrationsDir, `${entry.id}.plan.json`), "utf8"))
	)
	const decoded = migrations.decodeGeneratedMigrations({ manifest, plans })
	assert.ok(decoded.ok, "the committed chain decodes")
	const contract = JSON.parse(fs.readFileSync(path.join(migrationsDir, "runtime-contract.json"), "utf8")) as {
		schemaId: string
	}
	const dbId = Id128.fromHex(hex())
	const incId = Id128.fromHex(hex())
	const opId = Id128.fromHex(hex())
	assert.ok(Result.isSuccess(dbId) && Result.isSuccess(incId) && Result.isSuccess(opId))
	const databaseId = log.DatabaseId.from(dbId.success)
	const incarnationId = log.IncarnationId.from(incId.success)
	const operation = log.OperationId.from(opId.success)
	const schemaId = log.parseSchemaId(contract.schemaId)
	assert.ok(
		Result.isSuccess(databaseId) && Result.isSuccess(incarnationId) && Result.isSuccess(operation) && Result.isSuccess(schemaId)
	)
	const outcome = await Effect.runPromise(
		migrations
			.initialize(
				{
					kind: "local",
					directory: path.join(scratch, "tenants", tenantId),
					identity: {
						databaseId: databaseId.success,
						incarnationId: incarnationId.success,
						schemaId: schemaId.success
					}
				},
				decoded.value,
				{ ...policy.adminWork, operationId: operation.success }
			)
			.pipe(Effect.provide(NativeRuntime.layer(policy.runtimePolicy.native)))
	)
	assert.equal(outcome.kind, "completed", "initialization completes")
	if (outcome.kind !== "completed") {
		return
	}
	const binding = outcome.value.binding
	assert.equal(binding.kind, "local")
	if (binding.kind !== "local") {
		return
	}
	bindings.saveTenantBinding(tenantId, {
		kind: "local",
		identity: log.renderDatabaseIdentity(binding.identity),
		directory: binding.directory
	})
}

let token = ""
let tokenB = ""

before(async () => {
	await provision(TENANT_A)
	await provision(TENANT_B)
	const { signSession } = await import("../src/auth.ts")
	const expires = Math.floor(Date.now() / 1000) + 3600
	token = signSession(TENANT_A, expires)
	tokenB = signSession(TENANT_B, expires)
})

function request(method: string, url: string, auth: string | null, body?: unknown): Request {
	return new Request(`http://localhost${url}`, {
		method,
		headers: {
			...(auth === null ? {} : { authorization: `Bearer ${auth}` }),
			"content-type": "application/json"
		},
		...(body === undefined ? {} : { body: JSON.stringify(body) })
	})
}

test("anonymous and forged requests refuse before any open", async () => {
	const routes = await import("../app/api/notes/route.ts")
	const anonymous = await routes.GET(request("GET", "/api/notes", null))
	assert.equal(anonymous.status, 401)
	const forged = await routes.GET(request("GET", "/api/notes", `${TENANT_A}.9999999999.${"0".repeat(64)}`))
	assert.equal(forged.status, 401)
})

test("create is idempotent under the client-supplied id", async () => {
	const routes = await import("../app/api/notes/route.ts")
	const noteId = hex()
	const first = await routes.POST(request("POST", "/api/notes", token, { id: noteId, text: "hello" }))
	assert.equal(first.status, 200)
	const firstBody = (await first.json()) as { outcome: string }
	assert.equal(firstBody.outcome, "committed")
	// The identical retried request resolves to a decided receipt, never a
	// duplicate business effect or a fresh identity.
	const retry = await routes.POST(request("POST", "/api/notes", token, { id: noteId, text: "hello" }))
	assert.equal(retry.status, 200)
	const retryBody = (await retry.json()) as { outcome: string }
	assert.ok(retryBody.outcome === "committed" || retryBody.outcome === "no-change")
})

test("reads see committed notes; tenants are isolated", async () => {
	const routes = await import("../app/api/notes/route.ts")
	const noteId = hex()
	const created = await routes.POST(request("POST", "/api/notes", token, { id: noteId, text: "mine" }))
	assert.equal(created.status, 200)
	const list = await routes.GET(request("GET", "/api/notes", token))
	assert.equal(list.status, 200)
	const rows = (await list.json()) as ReadonlyArray<{ id: string }>
	assert.ok(rows.some((row) => row.id === noteId))
	// Tenant B never observes tenant A's facts (same schema, distinct
	// identity — the cache cannot cross bindings).
	const other = await routes.GET(request("GET", "/api/notes", tokenB))
	assert.equal(other.status, 200)
	const otherRows = (await other.json()) as ReadonlyArray<{ id: string }>
	assert.ok(!otherRows.some((row) => row.id === noteId))
})

test("witnessed pin toggles and a missing note is 404", async () => {
	const collection = await import("../app/api/notes/route.ts")
	const item = await import("../app/api/notes/[id]/route.ts")
	const noteId = hex()
	const created = await collection.POST(request("POST", "/api/notes", token, { id: noteId, text: "pin me" }))
	assert.equal(created.status, 200)
	const patched = await item.PATCH(
		request("PATCH", `/api/notes/${noteId}`, token, { requestKey: hex(), pinned: true }),
		{ params: Promise.resolve({ id: noteId }) }
	)
	assert.equal(patched.status, 200)
	const read = await item.GET(request("GET", `/api/notes/${noteId}`, token), {
		params: Promise.resolve({ id: noteId })
	})
	assert.equal(read.status, 200)
	const row = (await read.json()) as { pinned: boolean }
	assert.equal(row.pinned, true)
	const missing = await item.GET(request("GET", `/api/notes/${hex()}`, token), {
		params: Promise.resolve({ id: hex() })
	})
	assert.equal(missing.status, 404)
})
