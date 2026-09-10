#!/usr/bin/env node
/** The `bumbledb-log` executable: the one process boundary over the Effect CLI. */
import { NativeRuntime } from "@bjornpagen/bumbledb"
import * as NodeRuntime from "@effect/platform-node/NodeRuntime"
import { Effect, Exit } from "effect"
import { cliProgram, loadAuthoring, parseCliArguments } from "#migrations/cli.ts"

const main = Effect.gen(function* () {
	const parsed = parseCliArguments(process.argv.slice(2))
	if (typeof parsed === "string") {
		process.stderr.write(`${parsed}\n`)
		return 2
	}
	const program = loadAuthoring(parsed).pipe(Effect.flatMap((authored) => cliProgram(parsed, authored)))
	const exit = yield* Effect.exit(program.pipe(Effect.provide(NativeRuntime.layer())))
	if (Exit.isSuccess(exit)) {
		process.stdout.write(`${JSON.stringify(exit.value.report, null, 2)}\n`)
		return exit.value.code
	}
	const failure = Exit.findErrorOption(exit)
	if (failure._tag === "Some" && typeof failure.value === "string") {
		process.stderr.write(`${failure.value}\n`)
		return 2
	}
	process.stderr.write(`${JSON.stringify(exit.cause, null, 2)}\n`)
	return 1
})

NodeRuntime.runMain(main, {
	teardown: (exit, onExit) => onExit(Exit.isSuccess(exit) && typeof exit.value === "number" ? exit.value : 1)
})
