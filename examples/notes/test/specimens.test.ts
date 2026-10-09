import assert from "node:assert/strict"
import { test } from "node:test"
import { DbError } from "@bjornpagen/bumbledb"
import { Exit } from "effect"
import { exitResponse } from "../src/http.ts"

test("typed failures become redacted HTTP responses", async () => {
	const queueFull = exitResponse(Exit.fail(new DbError({ operation: "Database.make", reason: { _tag: "QueueFull" } })))
	assert.equal(queueFull.status, 429)
	assert.deepEqual(await queueFull.json(), { error: "QueueFull", operation: "Database.make" })
	const missing = exitResponse(
		Exit.fail(new DbError({ operation: "Database.open", reason: { _tag: "Engine", kind: "NotFound", message: "NotFound" } }))
	)
	assert.equal(missing.status, 404)
	assert.deepEqual(await missing.json(), { error: "TenantNotProvisioned", operation: "Database.open" })
	const alreadyMapped = Response.json({ error: "Denied" }, { status: 403 })
	assert.equal(exitResponse(Exit.fail(alreadyMapped)), alreadyMapped)
	assert.equal(exitResponse(Exit.die(new Error("private details"))).status, 500)
})

test("route modules run on Node, never Edge", async () => {
	const collection = await import("../app/api/notes/route.ts")
	const item = await import("../app/api/notes/[id]/route.ts")
	const attachment = await import("../app/api/notes/[id]/attachment/route.ts")
	assert.equal(collection.runtime, "nodejs")
	assert.equal(item.runtime, "nodejs")
	assert.equal(attachment.runtime, "nodejs")
})
