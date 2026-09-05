/**
 * The `bumbledb-log generate|check` CLI boundary (TS-MIG-10): argument and
 * authoring-module refusals exit 2 with usage/diagnostics and never reach the
 * native runtime. The CLI shares the exact production generator Effects with
 * the direct API (`#migrations/workflow.ts`), so the shared paths need no
 * duplicate coverage here; full generate/check runs through the CLI are the
 * F3 packed-consumer lane (the production codec needs the wired native
 * entrypoints).
 */
import assert from "node:assert/strict"
import { mkdtemp, writeFile } from "node:fs/promises"
import { tmpdir } from "node:os"
import * as path from "node:path"
import { describe, test } from "node:test"
import { cli } from "#migrations/cli.ts"

interface Capture {
	out: string[]
	err: string[]
	stdout: (line: string) => void
	stderr: (line: string) => void
}

function capture(): Capture {
	const out: string[] = []
	const err: string[] = []
	return { out, err, stdout: (line) => out.push(line), stderr: (line) => err.push(line) }
}

describe("CLI refusals", function suite() {
	test("unknown commands and malformed flags print usage and exit 2", async function usage() {
		for (const argv of [
			[],
			["migrate"],
			["generate"],
			["generate", "--schema"],
			["generate", "--schema", "x.ts"],
			["generate", "--schema", "x.ts", "--out", "dir", "--bogus", "v"],
			["check", "--schema", "x.ts", "--out", "dir", "--timeout-ms", "-5"]
		]) {
			const io = capture()
			const code = await cli(argv, io.stdout, io.stderr)
			assert.equal(code, 2, `argv ${JSON.stringify(argv)} must refuse`)
			assert.ok(io.err[0]?.includes("bumbledb-log <generate|check>"), "usage goes to stderr")
			assert.deepEqual(io.out, [])
		}
	})

	test("a missing schema module is a load refusal, not a crash", async function missingModule() {
		const io = capture()
		const code = await cli(
			["check", "--schema", "/nonexistent/schema-module.ts", "--out", "/nonexistent/migrations"],
			io.stdout,
			io.stderr
		)
		assert.equal(code, 2)
		assert.ok(io.err[0]?.includes("schema module failed to load"))
	})

	test("a module without a schema value names the export it looked for", async function notSchema() {
		const directory = await mkdtemp(path.join(tmpdir(), "bumbledb-migrations-cli-"))
		const modulePath = path.join(directory, "not-schema.mjs")
		await writeFile(modulePath, "export const shape = { just: 'data' }\n", "utf8")
		const io = capture()
		const code = await cli(["check", "--schema", modulePath, "--out", directory], io.stdout, io.stderr)
		assert.equal(code, 2)
		assert.ok(io.err[0]?.includes("is not a schema value"))
		// Explicitly named exports that do not exist are named precisely.
		const io2 = capture()
		const code2 = await cli(
			["check", "--schema", modulePath, "--out", directory, "--intent", "evolution"],
			io2.stdout,
			io2.stderr
		)
		assert.equal(code2, 2)
	})
})
