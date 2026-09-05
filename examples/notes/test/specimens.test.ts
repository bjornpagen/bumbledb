/**
 * Notes specimen assertions for D07/D22/D27 — calls the packed consumers
 * and Notes helpers, not reconstructed Scalar/JSON claims. Native
 * operations require generated `{ manifest, plans, snapshots }` and a
 * packed install; missing chain FAILS instead of skipping green.
 *
 * Verification: NotRun until F3.
 */
import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { test } from "node:test"
import { Effect, Exit } from "effect"
import { incrementUnits, incrementUnitsAsF64 } from "../../consumers/core-ts/consumer.ts"
import { incrementUnitsIntent, knownInvalidMixRefuses, parsedIdentityIsBounded } from "../../consumers/log-ts/consumer.ts"
import { bindingFor } from "../src/db/bindings.ts"
import { loadGeneratedMigrations } from "../src/db/generated.ts"

test("D27: consumer field-arithmetic convert authors unresolved", () => {
	assert.equal(incrementUnits.kind, "add")
	assert.equal(incrementUnits.result, "unresolved")
	assert.equal(incrementUnitsAsF64.kind, "cast")
	if (incrementUnitsAsF64.kind === "cast") {
		assert.equal(incrementUnitsAsF64.cast, "toF64")
	}
	const entry = incrementUnitsIntent.entries[0]
	assert.ok(entry !== undefined)
	assert.equal(entry.kind, "convert")
	assert.equal(entry.relation, "Attempt")
	assert.equal(entry.field, "units")
	if (entry.kind === "convert") {
		assert.equal(entry.expression, incrementUnits)
	}
	assert.equal(knownInvalidMixRefuses, true, "known I64/U64 mixing refuses at the authoring boundary")
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

test("generated runner input is { manifest, plans, snapshots }", () => {
	const generated = loadGeneratedMigrations()
	assert.ok(generated.manifest.entries.length > 0, "the committed chain has plans")
	assert.equal(generated.plans.length, generated.manifest.entries.length)
	assert.equal(
		generated.snapshots.length,
		generated.manifest.entries.length + 1,
		"snapshots are the empty-base schema plus one target per entry"
	)
	for (const snapshot of generated.snapshots) {
		assert.ok(typeof snapshot === "string" && snapshot.length > 0)
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
