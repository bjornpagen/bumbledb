/**
 * The `bumbledb-log generate|check` CLI boundary (TS-MIG-10): argument and
 * authoring-module refusals exit 2 with usage/diagnostics and never reach the
 * native runtime. Framework runners stay at this executable-test boundary.
 */
import assert from "node:assert/strict"
import { mkdtemp, writeFile } from "node:fs/promises"
import { tmpdir } from "node:os"
import * as path from "node:path"
import { describe, test } from "node:test"
import { Effect, Exit } from "effect"
import { CLI_USAGE, loadAuthoring, parseCliArguments } from "#migrations/cli.ts"

describe("CLI refusals", function suite() {
	test("unknown commands and malformed flags print usage and exit 2", function usage() {
		for (const argv of [
			[],
			["migrate"],
			["generate"],
			["generate", "--schema"],
			["generate", "--schema", "x.ts"],
			["generate", "--schema", "x.ts", "--out", "dir", "--bogus", "v"],
			["check", "--schema", "x.ts", "--out", "dir", "--timeout-ms", "-5"]
		]) {
			const parsed = parseCliArguments(argv)
			assert.equal(parsed, CLI_USAGE, `argv ${JSON.stringify(argv)} must refuse`)
		}
	})

	test("a missing schema module is a load refusal, not a crash", async function missingModule() {
		const parsed = parseCliArguments([
			"check",
			"--schema",
			"/nonexistent/schema-module.ts",
			"--out",
			"/nonexistent/migrations"
		])
		assert.notEqual(typeof parsed, "string")
		if (typeof parsed === "string") {
			return
		}
		const exit = await Effect.runPromiseExit(loadAuthoring(parsed))
		assert.ok(Exit.isFailure(exit))
		const failure = Exit.findErrorOption(exit)
		assert.ok(failure._tag === "Some")
		assert.ok(String(failure.value).includes("schema module failed to load"))
	})

	test("a module without a schema value names the export it looked for", async function notSchema() {
		const directory = await mkdtemp(path.join(tmpdir(), "bumbledb-migrations-cli-"))
		const modulePath = path.join(directory, "not-schema.mjs")
		await writeFile(modulePath, "export const shape = { just: 'data' }\n", "utf8")
		const parsed = parseCliArguments(["check", "--schema", modulePath, "--out", directory])
		assert.notEqual(typeof parsed, "string")
		if (typeof parsed === "string") {
			return
		}
		const exit = await Effect.runPromiseExit(loadAuthoring(parsed))
		assert.ok(Exit.isFailure(exit))
		const failure = Exit.findErrorOption(exit)
		assert.ok(failure._tag === "Some")
		assert.ok(String(failure.value).includes("is not a schema value"))
		const parsedIntent = parseCliArguments([
			"check",
			"--schema",
			modulePath,
			"--out",
			directory,
			"--intent",
			"evolution"
		])
		assert.notEqual(typeof parsedIntent, "string")
		if (typeof parsedIntent === "string") {
			return
		}
		const exit2 = await Effect.runPromiseExit(loadAuthoring(parsedIntent))
		assert.ok(Exit.isFailure(exit2))
	})
})
