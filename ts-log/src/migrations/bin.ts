#!/usr/bin/env node
/** The `bumbledb-log` executable: one thin process boundary over `cli`. */
import { cli } from "#migrations/cli.ts"

const code = await cli(
	process.argv.slice(2),
	(line) => {
		process.stdout.write(`${line}\n`)
	},
	(line) => {
		process.stderr.write(`${line}\n`)
	}
)
process.exitCode = code
