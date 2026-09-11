/** Provision current-schema tenants and exercise real request handlers. */
import assert from "node:assert/strict"
import { randomUUID } from "node:crypto"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { before, test } from "node:test"
import { inspect } from "node:util"

const scratch = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-notes-test-"))
process.env.SESSION_SECRET = "test-secret-test-secret-test-secret!"
process.env.BUMBLEDB_TENANT_BINDINGS_FILE = path.join(scratch, "tenants.json")
process.env.BUMBLEDB_REQUEST_RECORDS = path.join(scratch, "requests")
process.env.BUMBLEDB_MATERIALIZATION_DIR = path.join(scratch, "cache")

const TENANT_A = "student-a"
const TENANT_B = "student-b"

function hex(): string {
	return randomUUID()
}

async function jsonObject(response: Response): Promise<Record<string, unknown>> {
	const body: unknown = await response.json()
	assert.ok(typeof body === "object" && body !== null && !Array.isArray(body))
	return body as Record<string, unknown>
}

async function provision(tenantId: string): Promise<void> {
	const { Effect, Result } = await import("effect")
	const { NativeRuntime, Schema } = await import("@bjornpagen/bumbledb")
	const log = await import("@bjornpagen/bumbledb-log")
	const { App } = await import("../src/db/schema.ts")
	const { initializeTenant } = await import("../src/db/initialize.ts")
	const { saveTenantBinding } = await import("../src/db/bindings.ts")
	const { runtimePolicy } = await import("../src/db/runtime-policy.ts")
	const databaseId = log.DatabaseId.parse(hex())
	const incarnationId = log.IncarnationId.parse(hex())
	const operationId = log.OperationId.parse(hex())
	assert.ok(Result.isSuccess(databaseId) && Result.isSuccess(incarnationId) && Result.isSuccess(operationId))
	const binding = await Effect.runPromise(
		Effect.gen(function* () {
			const identity = {
				databaseId: databaseId.success,
				incarnationId: incarnationId.success,
				schemaId: (yield* Schema.compile(App)).schemaId
			}
			const binding = { kind: "local", directory: path.join(scratch, "tenants", tenantId), identity } as const
			yield* initializeTenant(binding, operationId.success)
			return yield* initializeTenant(binding, operationId.success)
		}).pipe(Effect.provide(NativeRuntime.layer(runtimePolicy.native)))
	)
	assert.ok(binding.kind === "local")
	saveTenantBinding(tenantId, {
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
	const { Effect, Exit } = await import("effect")
	const { appRuntime, Databases } = await import("../src/db/server.ts")
	const { bindingFor } = await import("../src/db/bindings.ts")
	const opened = await appRuntime.runPromiseExit(
		Effect.scoped(
			Effect.gen(function* () {
				const binding = yield* bindingFor(TENANT_A)
				const databases = yield* Databases
				yield* databases.acquire(binding)
			})
		)
	)
	assert.ok(Exit.isSuccess(opened), inspect(opened, { depth: 10 }))
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
	assert.equal(first.status, 200, await first.clone().text())
	const firstBody = await jsonObject(first)
	assert.equal(firstBody.outcome, "committed")
	assert.ok(typeof firstBody.command === "string")
	assert.ok(firstBody.command.length > 0, "the durable command ref is returned")
	const retry = await routes.POST(request("POST", "/api/notes", token, { id: noteId, text: "hello" }))
	assert.equal(retry.status, 200)
	const retryBody = await jsonObject(retry)
	assert.ok(retryBody.outcome === "committed" || retryBody.outcome === "no-change")
	assert.equal(retryBody.command, firstBody.command, "same-ID retry keeps the original command identity")
})

test("reads see committed notes; tenants are isolated", async () => {
	const routes = await import("../app/api/notes/route.ts")
	const noteId = hex()
	const created = await routes.POST(request("POST", "/api/notes", token, { id: noteId, text: "mine" }))
	assert.equal(created.status, 200, await created.clone().text())
	const list = await routes.GET(request("GET", "/api/notes", token))
	assert.equal(list.status, 200)
	const rows = await list.json()
	assert.ok(Array.isArray(rows))
	assert.ok(rows.some((row) => typeof row === "object" && row !== null && "id" in row && row.id === noteId))
	// Tenant B never observes tenant A's facts (same schema, distinct
	// identity — the cache cannot cross bindings).
	const other = await routes.GET(request("GET", "/api/notes", tokenB))
	assert.equal(other.status, 200)
	const otherRows = await other.json()
	assert.ok(Array.isArray(otherRows))
	assert.ok(!otherRows.some((row) => typeof row === "object" && row !== null && "id" in row && row.id === noteId))
})

test("witnessed pin toggles and a missing note is 404", async () => {
	const collection = await import("../app/api/notes/route.ts")
	const item = await import("../app/api/notes/[id]/route.ts")
	const noteId = hex()
	const created = await collection.POST(request("POST", "/api/notes", token, { id: noteId, text: "pin me" }))
	assert.equal(created.status, 200, await created.clone().text())
	const patched = await item.PATCH(
		request("PATCH", `/api/notes/${noteId}`, token, { requestKey: hex(), pinned: true }),
		{ params: Promise.resolve({ id: noteId }) }
	)
	assert.equal(patched.status, 200)
	const read = await item.GET(request("GET", `/api/notes/${noteId}`, token), {
		params: Promise.resolve({ id: noteId })
	})
	assert.equal(read.status, 200)
	const row = await jsonObject(read)
	assert.equal(row.pinned, true)
	assert.equal(row.text, "pin me")
	const missing = await item.GET(request("GET", `/api/notes/${hex()}`, token), {
		params: Promise.resolve({ id: hex() })
	})
	assert.equal(missing.status, 404)
	const missingBody = await jsonObject(missing)
	assert.equal(missingBody.error, "NotFound")
})
