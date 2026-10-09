/** Runs the core-ts consumer example against the installed package tarballs. */
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { coreProgram, makeConsumerRuntime } from "./consumer.ts"

const dir = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-packed-"))
const runtime = makeConsumerRuntime()
try {
	await runtime.runPromise(coreProgram(path.join(dir, "smoke.bdb")))
} finally {
	await runtime.dispose()
	fs.rmSync(dir, { recursive: true, force: true })
}
