/**
 * Requests against a real deployment. Requires DEPLOYED_URL and DEPLOYED_TOKEN (a session token for
 * a tenant that `pnpm migrate` created); a missing variable fails the suite.
 */
import assert from "node:assert/strict"
import { randomUUID } from "node:crypto"
import { test } from "node:test"

const base = process.env.DEPLOYED_URL
const token = process.env.DEPLOYED_TOKEN

function requireEnv(): { base: string; token: string } {
	assert.ok(
		base !== undefined && token !== undefined,
		"DEPLOYED_URL and DEPLOYED_TOKEN are required"
	)
	return { base, token }
}

async function jsonObject(response: Response): Promise<Record<string, unknown>> {
	const body: unknown = await response.json()
	assert.ok(typeof body === "object" && body !== null && !Array.isArray(body))
	return body as Record<string, unknown>
}

async function call(method: string, path: string, body?: unknown, auth?: string | null): Promise<Response> {
	const env = requireEnv()
	return fetch(new URL(path, env.base), {
		method,
		headers: {
			...(auth === null ? {} : { authorization: `Bearer ${auth ?? env.token}` }),
			"content-type": "application/json"
		},
		...(body === undefined ? {} : { body: JSON.stringify(body) })
	})
}

test("anonymous requests refuse at the deployed public boundary", async () => {
	const response = await call("GET", "/api/notes", undefined, null)
	assert.equal(response.status, 401)
})

test("deployed create/read round-trip with idempotent retry", async () => {
	const noteId = randomUUID()
	const first = await call("POST", "/api/notes", { id: noteId, text: "deployed" })
	assert.equal(first.status, 200)
	const retry = await call("POST", "/api/notes", { id: noteId, text: "deployed" })
	assert.equal(retry.status, 200)
	const retryBody = await jsonObject(retry)
	assert.equal(retryBody.outcome, "Committed")
	const read = await call("GET", `/api/notes/${noteId}`)
	assert.equal(read.status, 200)
	const row = await jsonObject(read)
	assert.equal(row.id, noteId)
	assert.equal(row.text, "deployed")
})

test("deployed pins apply in order", async () => {
	const noteId = randomUUID()
	assert.equal((await call("POST", "/api/notes", { id: noteId, text: "pins" })).status, 200)
	assert.equal((await call("PATCH", `/api/notes/${noteId}`, { requestKey: randomUUID(), pinned: true })).status, 200)
	assert.equal((await call("PATCH", `/api/notes/${noteId}`, { requestKey: randomUUID(), pinned: false })).status, 200)
	assert.equal((await jsonObject(await call("GET", `/api/notes/${noteId}`))).pinned, false)
})

test("a reused request id with a different command is refused", async () => {
	const noteId = randomUUID()
	assert.equal((await call("POST", "/api/notes", { id: noteId, text: "original" })).status, 200)
	const conflicting = await call("POST", "/api/notes", { id: noteId, text: "tampered" })
	assert.equal(conflicting.status, 409)
})
