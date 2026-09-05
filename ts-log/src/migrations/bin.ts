#!/usr/bin/env node
/** The `bumbledb-log` executable: the one process boundary over the Effect CLI. */
import { NativeRuntime } from "@bjornpagen/bumbledb"
import type { NativeRuntimeOptions } from "@bjornpagen/bumbledb"
import { Effect, Exit } from "effect"
import { cliProgram, loadAuthoring, parseCliArguments } from "#migrations/cli.ts"

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

const parsed = parseCliArguments(process.argv.slice(2))
if (typeof parsed === "string") {
	process.stderr.write(`${parsed}\n`)
	process.exitCode = 2
} else {
	const program = loadAuthoring(parsed).pipe(Effect.flatMap((authored) => cliProgram(parsed, authored)))
	const exit = await Effect.runPromiseExit(program.pipe(Effect.provide(NativeRuntime.layer(RUNTIME))))
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
