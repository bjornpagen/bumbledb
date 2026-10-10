import assert from "node:assert/strict"
import { spawnSync } from "node:child_process"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, test } from "node:test"
import { pathToFileURL } from "node:url"
import { compiledOf } from "../src/compile.ts"
import { check, generate, migrationHash, migrationIds, specText } from "../src/database/generate.ts"
import { str, u64 } from "../src/fields.ts"
import { relation } from "../src/relation.ts"
import type { AnySchema } from "../src/schema.ts"
import { schema } from "../src/schema.ts"
import { key } from "../src/statements.ts"

const root = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-migrations-"))
after(() => fs.rmSync(root, { recursive: true, force: true }))

const Note = relation("Note", { id: u64, text: str })
const V1 = schema("Notes", { Note }, [key(Note, ["id"])])
const Tag = relation("Tag", { note: u64, label: str })
const V2 = schema("Notes", { Note, Tag }, [key(Note, ["id"]), key(Tag, ["note", "label"])])

const bundleOf = (dir: string) =>
	migrationIds(dir).map((id) => ({
		id,
		hash: migrationHash(id, fs.readFileSync(path.join(dir, id, "schema.json"), "utf8"))
	}))

test("generate adds a numbered migration only when the schema changed, and check agrees", () => {
	const dir = path.join(root, "flow")
	assert.deepEqual(generate(dir, V1, "init"), { _tag: "Created", id: "0001_init" })
	assert.deepEqual(generate(dir, V1, "again"), { _tag: "Unchanged" })
	assert.deepEqual(generate(dir, V2, "tags"), { _tag: "Created", id: "0002_tags" })
	assert.deepEqual(migrationIds(dir), ["0001_init", "0002_tags"])
	assert.equal(fs.readFileSync(path.join(dir, "0002_tags", "schema.json"), "utf8"), specText(V2))
	const migration = fs.readFileSync(path.join(dir, "0002_tags", "migration.ts"), "utf8")
	assert.match(migration, /import \{ schema as from \} from "\.\.\/0001_init\/schema\.ts"/)
	assert.match(migration, new RegExp(`hash: "${migrationHash("0002_tags", specText(V2))}"`))
	assert.equal(
		fs.readFileSync(path.join(dir, "index.ts"), "utf8"),
		'import m0001 from "./0001_init/migration.ts"\nimport m0002 from "./0002_tags/migration.ts"\n\nexport const migrations = [m0001, m0002] as const\n'
	)
	assert.deepEqual(check(dir, V2, bundleOf(dir)), [])
})

test("check reports an edited migration, a stale index and an ungenerated schema change", () => {
	const dir = path.join(root, "drift")
	generate(dir, V1, "init")
	const bundled = bundleOf(dir)
	assert.equal(check(dir, V2, bundled).length, 1, "the schema moved past the last migration")
	fs.appendFileSync(path.join(dir, "0001_init", "schema.json"), " ")
	assert.ok(check(dir, V1, bundled).some((problem) => /hash does not match/.test(problem)))
	fs.writeFileSync(path.join(dir, "index.ts"), "export const migrations = [] as const\n")
	assert.ok(check(dir, V1, bundled).some((problem) => /index\.ts/.test(problem)))
})

test("a generated schema.ts rebuilds the same schema", async () => {
	const dir = path.join(root, "bindings")
	generate(dir, V2, "init")
	const source = fs
		.readFileSync(path.join(dir, "0001_init", "schema.ts"), "utf8")
		.replace('"@bjornpagen/bumbledb"', JSON.stringify(pathToFileURL(path.resolve("src/index.ts")).href))
	const file = path.join(dir, "rebuilt.ts")
	fs.writeFileSync(file, source)
	const rebuilt = (await import(pathToFileURL(file).href)) as { readonly schema: AnySchema }
	assert.equal(compiledOf(rebuilt.schema).schemaId, compiledOf(V2).schemaId)
})

test("the bumbledb command generates a migration from a schema module", () => {
	const dir = path.join(root, "cli")
	const run = (...args: string[]) =>
		spawnSync(process.execPath, ["src/bin.ts", ...args], { encoding: "utf8", cwd: path.resolve(".") })
	const created = run("generate", "--schema", "test/fixtures/cli-schema.ts#App", "--name", "init", "--migrations", dir)
	assert.equal(created.status, 0, created.stderr)
	assert.match(created.stdout, /created .*0001_init/)
	const unchanged = run("generate", "--schema", "test/fixtures/cli-schema.ts#App", "--name", "x", "--migrations", dir)
	assert.equal(unchanged.status, 0, unchanged.stderr)
	assert.match(unchanged.stdout, /schema unchanged/)
	assert.equal(run("nonsense").status, 2)
})

test("bumbledb migrate creates the database its config describes", () => {
	const dir = path.join(root, "migrate")
	fs.mkdirSync(dir)
	const migrate = spawnSync(
		process.execPath,
		[path.resolve("src/bin.ts"), "migrate", "--config", path.resolve("test/fixtures/cli-config.ts")],
		{ encoding: "utf8", cwd: dir }
	)
	assert.equal(migrate.status, 0, migrate.stderr)
	assert.match(migrate.stdout, /migrated/)
	assert.ok(fs.readdirSync(path.join(dir, "log", "log")).length > 0, "the log has its genesis entry")
})
