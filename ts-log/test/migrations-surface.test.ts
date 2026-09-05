/**
 * Module-boundary pins (TS-MIG-10, C11): the `/migrations` module re-exports
 * P08's runner operations by REFERENCE — one native executor, no second
 * runtime wrapper — and the `/schema` module exposes only pure intent
 * constructors. Runs against the real modules; needs the wired core barrel
 * (F2 hub patches), like every packed-consumer lane.
 */
import assert from "node:assert/strict"
import { describe, test } from "node:test"
import * as ops from "#migration-ops.ts"
import * as migrations from "#migrations/index.ts"
import * as schemaModule from "#schema.ts"

describe("module boundaries", function suite() {
	test("runner operations are P08's exact values, never a second executor", function reuse() {
		assert.equal(migrations.migrationStatus, ops.migrationStatus)
		assert.equal(migrations.initialize, ops.initialize)
		assert.equal(migrations.migrate, ops.migrate)
		assert.equal(migrations.activateMigration, ops.activateMigration)
		assert.equal(migrations.abortMigration, ops.abortMigration)
	})

	test("the /schema module exports only the pure intent constructors", function pure() {
		assert.deepEqual(
			Object.keys(schemaModule).sort(),
			["backfill", "convert", "dropField", "dropRelation", "migrationIntent", "renameField", "renameRelation", "seed"]
		)
	})

	test("generator entrypoints exist once, bound over the one production codec", function generator() {
		assert.equal(typeof migrations.generateMigrations, "function")
		assert.equal(typeof migrations.checkMigrations, "function")
		assert.equal("cli" in migrations, false, "the public async CLI twin is deleted")
	})
})
