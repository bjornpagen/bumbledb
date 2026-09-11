import assert from "node:assert/strict"
import { spawnSync } from "node:child_process"
import * as fs from "node:fs"
import * as path from "node:path"
import { test } from "node:test"
import { fileURLToPath } from "node:url"
import { NativeRuntime } from "@bjornpagen/bumbledb"
import { Effect, ManagedRuntime } from "effect"
import * as schemaModule from "#schema.ts"
import { parseArguments, writeSchemaFile } from "#schema-files.ts"

const root = fileURLToPath(new URL("..", import.meta.url))

test("schema tooling exports only snapshot emission and independent bindings", () => {
	assert.deepEqual(Object.keys(schemaModule).sort(), ["schemaBindings", "schemaSnapshot"])
	assert.equal(parseArguments(["generate", "--schema", "app.ts", "--out", "old"]), undefined)
	assert.equal(parseArguments(["bindings", "--snapshot", "a", "--out", "a"]), undefined)
	assert.equal(parseArguments(["bindings", "--snapshot", "a", "--out", "b", "--out", "c"]), undefined)
	assert.equal(parseArguments(["snapshot", "--schema", "a", "--out", "b", "--intent", "old"]), undefined)
})

test("the CLI emits each schema independently and never replaces output on admission failure", async () => {
	const directory = fs.mkdtempSync(path.join(root, "node_modules/.schema-files-"))
	const runtime = ManagedRuntime.make(NativeRuntime.layer())
	try {
		const authored = path.join(directory, "authored.mjs")
		const snapshot = path.join(directory, "schema.json")
		const bindings = path.join(directory, "bindings.ts")
		fs.writeFileSync(
			authored,
			`import { relation, schema, u64, key } from "@bjornpagen/bumbledb"
const Row = relation("Row", { id: u64 })
export const schemaValue = schema("Rows", { Row }, [key(Row, ["id"])])
`
		)
		await runtime.runPromise(
			writeSchemaFile({ command: "snapshot", input: authored, output: snapshot, exportName: "schemaValue" })
		)
		const cli = spawnSync(
			process.execPath,
			[path.join(root, "dist/bin.js"), "bindings", "--snapshot", snapshot, "--out", bindings],
			{ encoding: "utf8" }
		)
		assert.equal(cli.status, 0, cli.stderr)
		const initial = fs.readFileSync(bindings, "utf8")
		assert.match(initial, /db\.relation\("Row"/)
		assert.match(initial, /db\.key\(r0, \["id"\]\)/)
		assert.equal(initial, await runtime.runPromise(schemaModule.schemaBindings(fs.readFileSync(snapshot, "utf8"))))
		for (const rejected of [
			"not JSON",
			'{"relations":[],"relations":[],"statements":[]}',
			JSON.stringify({ relations: [{ name: "1", fields: [{ name: "value", type: "u64" }] }], statements: [] })
		]) {
			fs.writeFileSync(snapshot, rejected)
			const result = await runtime.runPromise(
				Effect.result(writeSchemaFile({ command: "bindings", input: snapshot, output: bindings }))
			)
			assert.equal(result._tag, "Failure")
			assert.equal(fs.readFileSync(bindings, "utf8"), initial)
		}
		assert.equal(
			fs.readdirSync(directory).some((file) => file.startsWith(".bumbledb-schema-")),
			false
		)
	} finally {
		await runtime.dispose()
		fs.rmSync(directory, { recursive: true, force: true })
	}
})
