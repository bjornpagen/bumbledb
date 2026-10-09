#!/usr/bin/env node
/**
 * `bumbledb generate --schema <file>#<export> --name <name> [--migrations <dir>]`
 * `bumbledb check --schema <file>#<export> [--migrations <dir>]`
 * `bumbledb migrate --config <file>`: the file's default export is the app's `Database` options;
 * the database is opened with `onOpen: "migrate"` and closed, running every pending migration.
 */
import * as path from "node:path"
import { pathToFileURL } from "node:url"
import { parseArgs } from "node:util"
import { Cause, Effect, Exit, Layer } from "effect"
import type { DatabaseOptions } from "./database/database.ts"
import { Database } from "./database/database.ts"
import { check, generate } from "./database/generate.ts"
import type { Migrations } from "./database/migration.ts"
import { Bumble } from "./runtime.ts"
import type { AnySchema } from "./schema.ts"

async function load(specifier: string): Promise<Record<string, unknown>> {
	return (await import(pathToFileURL(path.resolve(specifier)).href)) as Record<string, unknown>
}

async function schemaOf(reference: string): Promise<AnySchema> {
	const [file, name = "default"] = reference.split("#")
	if (file === undefined || file === "") throw new Error("--schema is <file>#<export>")
	const value = (await load(file))[name]
	if (value === undefined) throw new Error(`${file} has no export ${name}`)
	return value as AnySchema
}

async function main(argv: readonly string[]): Promise<number> {
	const [command, ...rest] = argv
	const { values } = parseArgs({
		args: rest,
		options: {
			schema: { type: "string" },
			name: { type: "string" },
			migrations: { type: "string", default: "migrations" },
			config: { type: "string" }
		}
	})
	const migrations = values.migrations ?? "migrations"
	switch (command) {
		case "generate": {
			if (values.schema === undefined || values.name === undefined)
				throw new Error("generate needs --schema and --name")
			const generated = generate(migrations, await schemaOf(values.schema), values.name)
			console.log(generated._tag === "Created" ? `created ${migrations}/${generated.id}` : "schema unchanged")
			return 0
		}
		case "check": {
			if (values.schema === undefined) throw new Error("check needs --schema")
			const bundled = (await load(path.join(migrations, "index.ts"))).migrations as Migrations
			const problems = check(migrations, await schemaOf(values.schema), bundled)
			for (const problem of problems) console.error(problem)
			return problems.length === 0 ? 0 : 1
		}
		case "migrate": {
			if (values.config === undefined) throw new Error("migrate needs --config")
			const options = (await load(values.config)).default as DatabaseOptions<AnySchema>
			const exit = await Effect.runPromiseExit(
				Effect.scoped(Database.make({ ...options, onOpen: "migrate" })).pipe(
					Effect.provide(Layer.fresh(Bumble.layer()))
				)
			)
			if (Exit.isFailure(exit)) {
				console.error(Cause.pretty(exit.cause))
				return 1
			}
			console.log("migrated")
			return 0
		}
		default:
			console.error("usage: bumbledb generate | check | migrate")
			return 2
	}
}

process.exitCode = await main(process.argv.slice(2)).catch((error: unknown) => {
	console.error(error instanceof Error ? error.message : error)
	return 2
})
