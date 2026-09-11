/** Notes request and packed-consumer qualification. */
import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { test } from "node:test"
import { DbError } from "@bjornpagen/bumbledb"
import { Effect, Exit } from "effect"
import { knownInvalidMixRefuses, parsedIdentityIsBounded } from "../../consumers/log-ts/consumer.ts"
import { bindingFor } from "../src/db/bindings.ts"
import { exitResponse } from "../src/http.ts"

test("runtime layer acquisition failures become redacted HTTP responses", async () => {
	const response = exitResponse(
		Exit.fail(
			new DbError({
				operation: "TenantCache.make",
				reason: { _tag: "QueueFull" }
			})
		)
	)
	assert.equal(response.status, 429)
	assert.deepEqual(await response.json(), { error: "QueueFull", operation: "TenantCache.make" })
	const alreadyMapped = Response.json({ error: "Denied" }, { status: 403 })
	assert.equal(exitResponse(Exit.fail(alreadyMapped)), alreadyMapped)
	assert.equal(exitResponse(Exit.die(new Error("private details"))).status, 500)
})

test("packed query arithmetic and identity validation reject invalid values", () => {
	assert.equal(knownInvalidMixRefuses, true)
	assert.equal(parsedIdentityIsBounded, true)
})

test("authenticated bindings are not derived from arbitrary user paths", async () => {
	const previous = process.env.BUMBLEDB_TENANT_BINDINGS_FILE
	const scratch = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-notes-bind-"))
	process.env.BUMBLEDB_TENANT_BINDINGS_FILE = path.join(scratch, "tenants.json")
	try {
		const exit = await Effect.runPromiseExit(bindingFor("../../etc/passwd"))
		assert.ok(Exit.isFailure(exit), "a path-shaped tenant id cannot open a store")
	} finally {
		if (previous === undefined) {
			delete process.env.BUMBLEDB_TENANT_BINDINGS_FILE
		} else {
			process.env.BUMBLEDB_TENANT_BINDINGS_FILE = previous
		}
	}
})

test("route modules declare Node runtime, never Edge", async () => {
	const collection = await import("../app/api/notes/route.ts")
	const item = await import("../app/api/notes/[id]/route.ts")
	const attachment = await import("../app/api/notes/[id]/attachment/route.ts")
	assert.equal(collection.runtime, "nodejs")
	assert.equal(item.runtime, "nodejs")
	assert.equal(attachment.runtime, "nodejs")
})
