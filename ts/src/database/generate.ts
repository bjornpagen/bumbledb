/**
 * The migrations directory: one `NNNN_name/` per migration holding the schema it migrates to
 * (`schema.json`, the engine's spec, and `schema.ts`, the same schema as SDK code) and its
 * `migration.ts`; `index.ts` bundles them in order. A migration's hash covers its id and its
 * `schema.json`, so editing either after it ran is caught by `check`.
 */
import { createHash } from "node:crypto"
import * as fs from "node:fs"
import * as path from "node:path"
import { compiledOf } from "../compile.ts"
import { AuthoringError } from "../errors.ts"
import { lower } from "../lower.ts"
import { addon } from "../native/addon.ts"
import type { AnySchema } from "../schema.ts"
import { schemaDescriptor } from "../schema.ts"

const DIRECTORY = /^([0-9]{4})_([A-Za-z0-9_]+)$/

/** The `schema.json` text of a schema: its lowered spec, formatted for review. */
function specText(theory: AnySchema): string {
	return `${JSON.stringify(lower(schemaDescriptor(theory)), null, "\t")}\n`
}

/** A migration's hash: sha256 over its id and its `schema.json` text. */
function migrationHash(id: string, spec: string): string {
	return createHash("sha256").update(`${id}\n${spec}`).digest("hex")
}

/** The migration directories under `root`, in order. */
function migrationIds(root: string): string[] {
	if (!fs.existsSync(root)) return []
	return fs
		.readdirSync(root, { withFileTypes: true })
		.filter((entry) => entry.isDirectory() && DIRECTORY.test(entry.name))
		.map((entry) => entry.name)
		.sort()
}

function indexText(ids: readonly string[]): string {
	const imports = ids.map((id) => `import m${id.slice(0, 4)} from "./${id}/migration.ts"`)
	const list = ids.map((id) => `m${id.slice(0, 4)}`).join(", ")
	return `${imports.join("\n")}\n\nexport const migrations = [${list}] as const\n`
}

function migrationText(id: string, hash: string, previous: string | undefined): string {
	const lines = ['import { Migration } from "@bjornpagen/bumbledb"']
	if (previous !== undefined) lines.push(`import { schema as from } from "../${previous}/schema.ts"`)
	lines.push('import { schema as to } from "./schema.ts"', "")
	lines.push(
		previous === undefined
			? `export default Migration.make({ id: "${id}", hash: "${hash}", to })`
			: `export default Migration.make({ id: "${id}", hash: "${hash}", from, to })`
	)
	return `${lines.join("\n")}\n`
}

type Generated = { readonly _tag: "Unchanged" } | { readonly _tag: "Created"; readonly id: string }

/**
 * Adds a migration to `schema` under `root` when it differs from the last migration's schema, and
 * rewrites `index.ts`.
 */
function generate(root: string, theory: AnySchema, name: string): Generated {
	if (!/^[A-Za-z0-9_]+$/.test(name))
		throw new AuthoringError({ message: `migration name ${name}: letters, digits and _` })
	const ids = migrationIds(root)
	const spec = specText(theory)
	const previous = ids.at(-1)
	if (previous !== undefined && fs.readFileSync(path.join(root, previous, "schema.json"), "utf8") === spec) {
		return { _tag: "Unchanged" }
	}
	const bindings = addon.schemaBindings(compiledOf(schemaDescriptor(theory)).handle)
	if (bindings._tag === "Unrepresentable") {
		throw new AuthoringError({ message: `schema ${theory.name}: ${bindings.coordinate}: ${bindings.reason}` })
	}
	const number = previous === undefined ? 1 : Number(previous.slice(0, 4)) + 1
	const id = `${number.toString().padStart(4, "0")}_${name}`
	const directory = path.join(root, id)
	fs.mkdirSync(directory, { recursive: true })
	fs.writeFileSync(path.join(directory, "schema.json"), spec)
	fs.writeFileSync(path.join(directory, "schema.ts"), bindings.source)
	fs.writeFileSync(path.join(directory, "migration.ts"), migrationText(id, migrationHash(id, spec), previous))
	fs.writeFileSync(path.join(root, "index.ts"), indexText([...ids, id]))
	return { _tag: "Created", id }
}

/** Everything `check` found wrong; empty when the directory is consistent with `schema`. */
function check(
	root: string,
	theory: AnySchema,
	bundled: readonly { readonly id: string; readonly hash: string }[]
): string[] {
	const problems: string[] = []
	const ids = migrationIds(root)
	if (ids.length === 0) return [`${root}: no migrations; run bumbledb generate`]
	if (fs.readFileSync(path.join(root, "index.ts"), "utf8") !== indexText(ids)) {
		problems.push(`${root}/index.ts does not list the migration directories in order; run bumbledb generate`)
	}
	ids.forEach((id, index) => {
		const spec = fs.readFileSync(path.join(root, id, "schema.json"), "utf8")
		const migration = bundled[index]
		if (migration === undefined || migration.id !== id) {
			problems.push(`${id}: not bundled at position ${index + 1}`)
		} else if (migration.hash !== migrationHash(id, spec)) {
			problems.push(`${id}: its hash does not match its schema.json; a migration is never edited after it runs`)
		}
	})
	const last = ids.at(-1)
	if (last !== undefined && fs.readFileSync(path.join(root, last, "schema.json"), "utf8") !== specText(theory)) {
		problems.push(`the schema differs from ${last}; run bumbledb generate`)
	}
	return problems
}

export type { Generated }
export { check, generate, migrationHash, migrationIds, specText }
