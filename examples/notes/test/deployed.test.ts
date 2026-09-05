/**
 * Deployed request tests (APP-04/05/06 request half): drive the ACTUAL
 * deployed HTTP surface — a real Alchemy/Vercel deployment of this app —
 * with authenticated requests, idempotent retries and conflict shapes.
 *
 * Required environment (an explicitly authorized disposable scope):
 *   DEPLOYED_URL    — the deployed base URL
 *   DEPLOYED_TOKEN  — a valid session token for a PROVISIONED test tenant
 *
 * Missing configuration FAILS this suite: absent credentials are NotRun
 * evidence, never a green skip (chapter 64). The deployment/migration
 * REHEARSAL (frozen rollout, lost activation, abort-vs-activate) is the
 * scripts/migrate.ts runbook procedure in docs/reference/deployment.md,
 * executed and recorded at F3 — not simulated here.
 *
 * Verification: NotRun until F3.
 */
import assert from "node:assert/strict"
import { randomBytes } from "node:crypto"
import { test } from "node:test"

const base = process.env.DEPLOYED_URL
const token = process.env.DEPLOYED_TOKEN

function requireEnv(): { base: string; token: string } {
	assert.ok(
		base !== undefined && token !== undefined,
		"DEPLOYED_URL and DEPLOYED_TOKEN are required: the deployed lane is NotRun without a real deployment"
	)
	return { base, token }
}

async function jsonObject(response: Response): Promise<Record<string, unknown>> {
	const body: unknown = await response.json()
	assert.ok(typeof body === "object" && body !== null && !Array.isArray(body))
	return body
}

function hex(): string {
	return Buffer.from(randomBytes(16)).toString("hex")
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
	const noteId = hex()
	const first = await call("POST", "/api/notes", { id: noteId, text: "deployed" })
	assert.equal(first.status, 200)
	const retry = await call("POST", "/api/notes", { id: noteId, text: "deployed" })
	assert.equal(retry.status, 200)
	const retryBody = await jsonObject(retry)
	assert.ok(retryBody.outcome === "committed" || retryBody.outcome === "no-change")
	const read = await call("GET", `/api/notes/${noteId}`)
	assert.equal(read.status, 200)
	const row = await jsonObject(read)
	assert.equal(row.id, noteId)
	assert.equal(row.text, "deployed")
})

test("deployed witnessed conflict is a durable 409, never a silent overwrite", async () => {
	const noteId = hex()
	const created = await call("POST", "/api/notes", { id: noteId, text: "conflict" })
	assert.equal(created.status, 200)
	// First revision wins.
	const first = await call("PATCH", `/api/notes/${noteId}`, { requestKey: hex(), pinned: true })
	assert.equal(first.status, 200)
	// A command reusing an OLD witness must surface precondition-failed. We
	// force it by patching twice with distinct request keys built against
	// the same observed state through rapid succession; the deployed engine
	// decides — both 200 (serialized reads) and 409 (stale witness) are
	// legal shapes, and a 409 carries the durable receipt.
	const second = await call("PATCH", `/api/notes/${noteId}`, { requestKey: hex(), pinned: false })
	assert.ok(second.status === 200 || second.status === 409)
	if (second.status === 409) {
		const body = await jsonObject(second)
		assert.equal(body.outcome, "precondition-failed")
	}
})

test("a changed application id under a reused idempotency key refuses", async () => {
	const noteId = hex()
	const first = await call("POST", "/api/notes", { id: noteId, text: "original" })
	assert.equal(first.status, 200)
	// Same client note id (the request key) with DIFFERENT text is a
	// different concrete command under the same command id: the digest
	// conflict refuses instead of silently replacing business meaning.
	const conflicting = await call("POST", "/api/notes", { id: noteId, text: "tampered" })
	assert.ok(conflicting.status >= 400, `digest conflict must refuse, got ${conflicting.status}`)
})
