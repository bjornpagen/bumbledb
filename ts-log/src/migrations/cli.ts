/**
 * The generator CLI surface: argument parsing and the Effect program.
 * Framework runners (`Effect.runPromiseExit`) live only at the executable
 * boundary (`#migrations/bin.ts`) and in authored tests. There is no public
 * Promise/async twin of generate/check.
 */
import * as path from "node:path"
import { pathToFileURL } from "node:url"
import type { AnySchema } from "@bjornpagen/bumbledb"
import { Effect } from "effect"
import type { MigrationIntent } from "#migrations/intent.ts"
import type { CheckReport, GenerationReport } from "#migrations/types.ts"
import { checkMigrations, generateMigrations } from "#migrations/workflow.ts"

export const CLI_USAGE = `bumbledb-log <generate|check> --schema <module.ts> --out <migrations-dir>
	[--export <name>]    schema export (default: "default", then "schema")
	[--intent <name>]    intent export (default: "evolution" when present)
	[--label <label>]    stable human label for a new plan ([a-z0-9-])
	[--contract <path>]  runtime contract path (default: <out>/runtime-contract.json)
`

export interface CliArguments {
	readonly command: "generate" | "check"
	readonly schemaPath: string
	readonly directory: string
	readonly exportName: string | null
	readonly intentName: string | null
	readonly label: string | null
	readonly contract: string | null
}

export function parseCliArguments(argv: readonly string[]): CliArguments | string {
	const command = argv[0]
	if (command !== "generate" && command !== "check") {
		return CLI_USAGE
	}
	let schemaPath: string | null = null
	let directory: string | null = null
	let exportName: string | null = null
	let intentName: string | null = null
	let label: string | null = null
	let contract: string | null = null
	for (let index = 1; index < argv.length; index += 2) {
		const flag = argv[index]
		const value = argv[index + 1]
		if (flag === undefined || value === undefined) {
			return CLI_USAGE
		}
		switch (flag) {
			case "--schema":
				schemaPath = value
				break
			case "--out":
				directory = value
				break
			case "--export":
				exportName = value
				break
			case "--intent":
				intentName = value
				break
			case "--label":
				label = value
				break
			case "--contract":
				contract = value
				break
			default:
				return CLI_USAGE
		}
	}
	if (schemaPath === null || directory === null) {
		return CLI_USAGE
	}
	return { command, schemaPath, directory, exportName, intentName, label, contract }
}

function isSchemaValue(value: unknown): value is AnySchema {
	return (
		typeof value === "object" &&
		value !== null &&
		"relations" in value &&
		"statements" in value &&
		"classes" in value &&
		"name" in value
	)
}

function isIntentValue(value: unknown): value is MigrationIntent<AnySchema["relations"]> {
	return typeof value === "object" && value !== null && "schema" in value && "entries" in value
}

export interface AuthoredModule {
	readonly schema: AnySchema
	readonly intent: MigrationIntent<AnySchema["relations"]> | undefined
}

/** Load the authoring module. Dynamic import stays inside this Effect. */
export const loadAuthoring = Effect.fn("bumbledb-log.cli.loadAuthoring")(function* (cli: CliArguments) {
	const loaded = yield* Effect.tryPromise({
		try: () => import(pathToFileURL(path.resolve(cli.schemaPath)).href) as Promise<Record<string, unknown>>,
		catch: (cause) => `schema module failed to load: ${cause instanceof Error ? cause.message : String(cause)}`
	})
	const exportName = cli.exportName ?? (loaded.default !== undefined ? "default" : "schema")
	const candidate = loaded[exportName]
	if (!isSchemaValue(candidate)) {
		return yield* Effect.fail(
			`export ${exportName} of ${cli.schemaPath} is not a schema value (declare it with schema(...))`
		)
	}
	const intentName = cli.intentName ?? "evolution"
	const intentCandidate = loaded[intentName]
	if (cli.intentName !== null && intentCandidate === undefined) {
		return yield* Effect.fail(`export ${intentName} of ${cli.schemaPath} does not exist`)
	}
	if (intentCandidate !== undefined && !isIntentValue(intentCandidate)) {
		return yield* Effect.fail(`export ${intentName} of ${cli.schemaPath} is not a migrationIntent(...) value`)
	}
	const authored: AuthoredModule = { schema: candidate, intent: intentCandidate }
	return authored
})

export const cliProgram = Effect.fn("bumbledb-log.cli.program")(function* (
	cli: CliArguments,
	authored: AuthoredModule
) {
	const repository = {
		directory: cli.directory,
		...(cli.contract === null ? {} : { contract: cli.contract })
	}
	if (cli.command === "generate") {
		const report: GenerationReport = yield* generateMigrations({
			schema: authored.schema,
			...(authored.intent === undefined ? {} : { intent: authored.intent }),
			...(cli.label === null ? {} : { label: cli.label }),
			repository
		})
		return { report, code: 0 as const }
	}
	const report: CheckReport = yield* checkMigrations({
		schema: authored.schema,
		...(authored.intent === undefined ? {} : { intent: authored.intent }),
		repository
	})
	return { report, code: report.status === "clean" ? (0 as const) : (1 as const) }
})
