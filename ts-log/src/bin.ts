#!/usr/bin/env node
import { NativeRuntime } from "@bjornpagen/bumbledb"
import * as NodeRuntime from "@effect/platform-node/NodeRuntime"
import { Effect } from "effect"
import { parseArguments, usage, writeSchemaFile } from "#schema-files.ts"

const main = Effect.gen(function* () {
	const args = parseArguments(process.argv.slice(2))
	if (args === undefined) return yield* Effect.fail(new Error(usage))
	yield* writeSchemaFile(args)
}).pipe(Effect.provide(NativeRuntime.layer()))

NodeRuntime.runMain(main)
