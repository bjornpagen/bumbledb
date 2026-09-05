/**
 * The generator CLI: `bumbledb-log generate|check` (chapter 33's two
 * authorship commands). Generation loads the app's ordinary schema module
 * with the normal module loader — generation-time authoring evaluation only;
 * NOTHING here is imported at application runtime, and the admin runner
 * never loads authoring code. CLI and direct API share the exact same
 * Effects, so outcomes are identical by construction (TS-MIG-10).
 */
import * as path from "node:path"
import { pathToFileURL } from "node:url"
import { NativeRuntime } from "@bjornpagen/bumbledb"
import type { AnySchema, ExecutionPolicy, NativeRuntimeOptions } from "@bjornpagen/bumbledb"
import { Effect, Exit } from "effect"
import type { MigrationIntent } from "#migrations/intent.ts"
import { checkMigrations, generateMigrations } from "#migrations/workflow.ts"

const USAGE = `bumbledb-log <generate|check> --schema <module.ts> --out <migrations-dir>
	[--export <name>]    schema export (default: "default", then "schema")
	[--intent <name>]    intent export (default: "evolution" when present)
	[--label <label>]    stable human label for a new plan ([a-z0-9-])
	[--contract <path>]  runtime contract path (default: <out>/runtime-contract.json)
	[--timeout-ms <n>]   generation work deadline (default: 600000)
`

/** Generous fixed authoring-tool budgets; native admission still re-judges. */
const WORK: ExecutionPolicy = {
	inputBytes: 256n * 1024n * 1024n,
	workingBytes: 256n * 1024n * 1024n,
	scratchBytes: 256n * 1024n * 1024n,
	resultBytes: 64n * 1024n * 1024n,
	rows: 1_000_000n,
	workUnits: 1_000_000_000n,
	timeout: 600000
}

const RUNTIME: NativeRuntimeOptions = {
	workers: 2,
	queueCapacity: 64,
	cleanupCapacity: 16,
	ownerCapacity: 8,
	nativeHandleCapacity: 64,
	inputBytes: 512n * 1024n * 1024n,
	workingBytes: 512n * 1024n * 1024n,
	scratchBytes: 512n * 1024n * 1024n,
	resultBytes: 128n * 1024n * 1024n,
	chunkBytes: 1024n * 1024n,
	cleanupTimeout: 5000
}

interface CliArguments {
	readonly command: "generate" | "check"
	readonly schemaPath: string
	readonly directory: string
	readonly exportName: string | null
	readonly intentName: string | null
	readonly label: string | null
	readonly contract: string | null
	readonly timeoutMs: number | null
}

function parseArguments(argv: readonly string[]): CliArguments | string {
	const command = argv[0]
	if (command !== "generate" && command !== "check") {
		return USAGE
	}
	let schemaPath: string | null = null
	let directory: string | null = null
	let exportName: string | null = null
	let intentName: string | null = null
	let label: string | null = null
	let contract: string | null = null
	let timeoutMs: number | null = null
	for (let index = 1; index < argv.length; index += 2) {
		const flag = argv[index]
		const value = argv[index + 1]
		if (flag === undefined || value === undefined) {
			return USAGE
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
			case "--timeout-ms": {
				const parsed = Number.parseInt(value, 10)
				if (!Number.isSafeInteger(parsed) || parsed <= 0) {
					return USAGE
				}
				timeoutMs = parsed
				break
			}
			default:
				return USAGE
		}
	}
	if (schemaPath === null || directory === null) {
		return USAGE
	}
	return { command, schemaPath, directory, exportName, intentName, label, contract, timeoutMs }
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

interface AuthoredModule {
	readonly schema: AnySchema
	readonly intent: MigrationIntent<AnySchema["relations"]> | undefined
}

async function loadAuthoring(cli: CliArguments): Promise<AuthoredModule | string> {
	let loaded: Record<string, unknown>
	try {
		loaded = (await import(pathToFileURL(path.resolve(cli.schemaPath)).href)) as Record<string, unknown>
	} catch (cause) {
		return `schema module failed to load: ${cause instanceof Error ? cause.message : String(cause)}`
	}
	const exportName = cli.exportName ?? (loaded.default !== undefined ? "default" : "schema")
	const candidate = loaded[exportName]
	if (!isSchemaValue(candidate)) {
		return `export ${exportName} of ${cli.schemaPath} is not a schema value (declare it with schema(...))`
	}
	const intentName = cli.intentName ?? "evolution"
	const intentCandidate = loaded[intentName]
	if (cli.intentName !== null && intentCandidate === undefined) {
		return `export ${intentName} of ${cli.schemaPath} does not exist`
	}
	if (intentCandidate !== undefined && !isIntentValue(intentCandidate)) {
		return `export ${intentName} of ${cli.schemaPath} is not a migrationIntent(...) value`
	}
	return { schema: candidate, intent: intentCandidate }
}

/**
 * Run one CLI invocation. Returns the process exit code; stdout carries the
 * JSON report, stderr carries refusals. Never calls process.exit itself.
 */
export async function cli(
	argv: readonly string[],
	stdout: (line: string) => void,
	stderr: (line: string) => void
): Promise<number> {
	const parsed = parseArguments(argv)
	if (typeof parsed === "string") {
		stderr(parsed)
		return 2
	}
	const authored = await loadAuthoring(parsed)
	if (typeof authored === "string") {
		stderr(authored)
		return 2
	}
	const work: ExecutionPolicy = parsed.timeoutMs === null ? WORK : { ...WORK, timeout: parsed.timeoutMs }
	const repository = {
		directory: parsed.directory,
		...(parsed.contract === null ? {} : { contract: parsed.contract })
	}
	const program =
		parsed.command === "generate"
			? Effect.map(
					generateMigrations({
						schema: authored.schema,
						...(authored.intent === undefined ? {} : { intent: authored.intent }),
						...(parsed.label === null ? {} : { label: parsed.label }),
						repository,
						work
					}),
					(report) => ({ report, code: 0 })
				)
			: Effect.map(
					checkMigrations({
						schema: authored.schema,
						...(authored.intent === undefined ? {} : { intent: authored.intent }),
						repository,
						work
					}),
					(report) => ({ report, code: report.status === "clean" ? 0 : 1 })
				)
	const exit = await Effect.runPromiseExit(program.pipe(Effect.provide(NativeRuntime.layer(RUNTIME))))
	if (Exit.isSuccess(exit)) {
		stdout(JSON.stringify(exit.value.report, null, 2))
		return exit.value.code
	}
	stderr(JSON.stringify(exit.cause, null, 2))
	return 1
}
