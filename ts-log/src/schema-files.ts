import * as fs from "node:fs/promises"
import * as path from "node:path"
import { pathToFileURL } from "node:url"
import type { AnySchema } from "@bjornpagen/bumbledb"
import { Effect } from "effect"
import { schemaBindings, schemaSnapshot } from "#schema.ts"

export const usage =
	"bumbledb-log snapshot --schema <module.ts> --out <snapshot.json> [--export <name>]\nbumbledb-log bindings --snapshot <snapshot.json> --out <bindings.ts>"

export type SchemaFileArguments =
	| { readonly command: "snapshot"; readonly input: string; readonly output: string; readonly exportName: string }
	| { readonly command: "bindings"; readonly input: string; readonly output: string }

export function parseArguments(args: readonly string[]): SchemaFileArguments | undefined {
	const command = args[0]
	if (command !== "snapshot" && command !== "bindings") return undefined
	const flags = new Map<string, string>()
	for (let i = 1; i < args.length; i += 2) {
		const flag = args[i]
		const value = args[i + 1]
		if (flag === undefined || value === undefined || flags.has(flag)) return undefined
		if (
			flag !== "--out" &&
			flag !== (command === "snapshot" ? "--schema" : "--snapshot") &&
			!(command === "snapshot" && flag === "--export")
		)
			return undefined
		flags.set(flag, value)
	}
	const input = flags.get(command === "snapshot" ? "--schema" : "--snapshot")
	const output = flags.get("--out")
	if (!input || !output || path.resolve(input) === path.resolve(output)) return undefined
	return command === "snapshot"
		? { command, input, output, exportName: flags.get("--export") ?? "schema" }
		: { command, input, output }
}

function isSchema(input: unknown): input is AnySchema {
	return (
		typeof input === "object" &&
		input !== null &&
		"relations" in input &&
		"statements" in input &&
		"classes" in input &&
		"name" in input
	)
}

const io = <A>(operation: () => Promise<A>) => Effect.tryPromise({ try: operation, catch: (cause) => cause })

/** The output is replaced only after native admission and emission succeed. */
export const writeSchemaFile = Effect.fn("bumbledb-log.writeSchemaFile")(function* (args: SchemaFileArguments) {
	const source = yield* Effect.gen(function* () {
		if (args.command === "bindings") {
			return yield* schemaBindings(yield* io(() => fs.readFile(args.input, "utf8")))
		}
		const loaded: unknown = yield* io(() => import(pathToFileURL(path.resolve(args.input)).href))
		if (typeof loaded !== "object" || loaded === null || !Object.hasOwn(loaded, args.exportName)) {
			return yield* Effect.fail(new Error(`missing schema export ${args.exportName}`))
		}
		const candidate: unknown = Reflect.get(loaded, args.exportName)
		if (!isSchema(candidate)) return yield* Effect.fail(new Error(`export ${args.exportName} is not a schema`))
		return yield* schemaSnapshot(candidate)
	})
	// Mask interruption over the short replace/cleanup sequence. Releasing
	// a temporary path before an outstanding write settles would race it.
	yield* Effect.uninterruptible(
		io(async () => {
			const directory = await fs.mkdtemp(path.join(path.dirname(path.resolve(args.output)), ".bumbledb-schema-"))
			try {
				const temporary = path.join(directory, "output")
				await fs.writeFile(temporary, source, { flag: "wx" })
				await fs.rename(temporary, args.output)
			} finally {
				await fs.rm(directory, { recursive: true, force: true })
			}
		})
	)
})
