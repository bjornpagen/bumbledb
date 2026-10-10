/** Exercises the real request handlers against tenants created on first use. */
import assert from "node:assert/strict"
import { randomUUID } from "node:crypto"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, before, test } from "node:test"

const scratch = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-notes-test-"))
process.env.SESSION_SECRET = "test-secret-test-secret-test-secret!"
process.chdir(scratch)

const TENANT_A = "student-a"
const TENANT_B = "student-b"

let token = ""
let tokenB = ""

before(async () => {
	const { signSession } = await import("../src/auth.ts")
	const expires = Math.floor(Date.now() / 1000) + 3600
	token = signSession(TENANT_A, expires)
	tokenB = signSession(TENANT_B, expires)
})

after(async () => {
	const { appRuntime } = await import("../src/db/server.ts")
	await appRuntime.dispose()
	fs.rmSync(scratch, { recursive: true, force: true })
})

async function jsonObject(response: Response): Promise<Record<string, unknown>> {
	const body: unknown = await response.json()
	assert.ok(typeof body === "object" && body !== null && !Array.isArray(body))
	return body as Record<string, unknown>
}

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

test("anonymous and forged requests are refused before any database opens", async () => {
	const routes = await import("../app/api/notes/route.ts")
	assert.equal((await routes.GET(request("GET", "/api/notes", null))).status, 401)
	const forged = await routes.GET(request("GET", "/api/notes", `${TENANT_A}.9999999999.${"0".repeat(64)}`))
	assert.equal(forged.status, 401)
	assert.equal(fs.existsSync(path.join(scratch, "tenants")), false)
})

test("create is decided once per client-supplied id", async () => {
	const routes = await import("../app/api/notes/route.ts")
	const noteId = randomUUID()
	const first = await routes.POST(request("POST", "/api/notes", token, { id: noteId, text: "hello" }))
	assert.equal(first.status, 200, await first.clone().text())
	const firstBody = await jsonObject(first)
	assert.equal(firstBody.outcome, "Committed")
	const retry = await routes.POST(request("POST", "/api/notes", token, { id: noteId, text: "hello" }))
	assert.equal(retry.status, 200)
	assert.deepEqual(await jsonObject(retry), firstBody, "a retry returns the original receipt")
})

test("a request id reused for a different command is refused", async () => {
	const routes = await import("../app/api/notes/route.ts")
	const noteId = randomUUID()
	assert.equal((await routes.POST(request("POST", "/api/notes", token, { id: noteId, text: "original" }))).status, 200)
	const reused = await routes.POST(request("POST", "/api/notes", token, { id: noteId, text: "tampered" }))
	assert.equal(reused.status, 409)
	assert.deepEqual(await jsonObject(reused), { submitted: false, error: "RequestReused" })
})

test("reads see committed notes, and tenants are isolated", async () => {
	const routes = await import("../app/api/notes/route.ts")
	const noteId = randomUUID()
	const created = await routes.POST(request("POST", "/api/notes", token, { id: noteId, text: "mine" }))
	assert.equal(created.status, 200, await created.clone().text())
	const rows = await (await routes.GET(request("GET", "/api/notes", token))).json()
	assert.ok(Array.isArray(rows) && rows.some((row) => row.id === noteId))
	const other = await (await routes.GET(request("GET", "/api/notes", tokenB))).json()
	assert.ok(Array.isArray(other) && !other.some((row) => row.id === noteId))
})

test("a pin is set at the revision it was read at, and a missing note is 404", async () => {
	const collection = await import("../app/api/notes/route.ts")
	const item = await import("../app/api/notes/[id]/route.ts")
	const noteId = randomUUID()
	const params = { params: Promise.resolve({ id: noteId }) }
	assert.equal((await collection.POST(request("POST", "/api/notes", token, { id: noteId, text: "pin me" }))).status, 200)
	const patched = await item.PATCH(
		request("PATCH", `/api/notes/${noteId}`, token, { requestKey: randomUUID(), pinned: true }),
		params
	)
	assert.equal(patched.status, 200, await patched.clone().text())
	const row = await jsonObject(await item.GET(request("GET", `/api/notes/${noteId}`, token), params))
	assert.equal(row.pinned, true)
	assert.equal(row.text, "pin me")
	const missingId = randomUUID()
	const missing = await item.GET(request("GET", `/api/notes/${missingId}`, token), {
		params: Promise.resolve({ id: missingId })
	})
	assert.equal(missing.status, 404)
})

test("a created tenant is seeded by migration 0001", async () => {
	const { Effect } = await import("effect")
	const { query, v } = await import("@bjornpagen/bumbledb")
	const { App, Tag } = await import("../src/db/schema.ts")
	const { appRuntime, Databases } = await import("../src/db/server.ts")
	const tags = query(App).rule((r) => {
		const { id, name } = v(Tag)
		return r.match(Tag, { id, name }).find({ name })
	})
	const names = await appRuntime.runPromise(
		Effect.scoped(
			Effect.gen(function* () {
				const db = yield* (yield* Databases).get(TENANT_A)
				return yield* (yield* (yield* db.read("latest")).execute(tags, {})).collect()
			})
		)
	)
	assert.deepEqual(names.map((row) => row.name).sort(), ["archive", "inbox"])
})
