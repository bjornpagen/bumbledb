#!/usr/bin/env node
/** The `bumbledb-log` executable: the one process boundary over the Effect CLI. */
import { NativeRuntime } from "@bjornpagen/bumbledb"
import { Effect, Exit } from "effect"
import { cliProgram, loadAuthoring, parseCliArguments } from "#migrations/cli.ts"

const parsed = parseCliArguments(process.argv.slice(2))
if (typeof parsed === "string") {
	process.stderr.write(`${parsed}\n`)
	process.exitCode = 2
} else {
	const program = loadAuthoring(parsed).pipe(Effect.flatMap((authored) => cliProgram(parsed, authored)))
	const exit = await Effect.runPromiseExit(program.pipe(Effect.provide(NativeRuntime.layer())))
	if (Exit.isSuccess(exit)) {
		process.stdout.write(`${JSON.stringify(exit.value.report, null, 2)}\n`)
		process.exitCode = exit.value.code
	} else {
		const failure = Exit.findErrorOption(exit)
		if (failure._tag === "Some" && typeof failure.value === "string") {
			process.stderr.write(`${failure.value}\n`)
			process.exitCode = 2
		} else {
			process.stderr.write(`${JSON.stringify(exit.cause, null, 2)}\n`)
			process.exitCode = 1
		}
	}
}
