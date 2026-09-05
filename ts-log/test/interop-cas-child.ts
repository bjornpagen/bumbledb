/** One-shot cross-language CAS fixture. No production hooks or timing races. */
import assert from "node:assert/strict"
import fs from "node:fs/promises"
import { syncBuiltinESMExports } from "node:module"
import * as path from "node:path"
import { createInterface } from "node:readline"

function report(event: Record<string, unknown>): void {
	process.stdout.write(`INTEROP_CAS ${JSON.stringify(event)}\n`)
}

async function release(): Promise<void> {
	const lines = createInterface({ input: process.stdin })
	try {
		for await (const line of lines) {
			assert.equal(line, "continue")
			return
		}
		assert.fail("parent closed the pause gate without releasing it")
	} finally {
		lines.close()
		process.stdin.pause()
	}
}

const [mode, root, expected] = process.argv.slice(2)
assert.ok(mode === "pause-read" || mode === "poison-lock")
assert.ok(root !== undefined && expected !== undefined)
const targetPath = path.join(root, "race/counter")
const originalRead = fs.readFile
let paused = false

if (mode === "pause-read") {
	// The old JS adapter can pause after its real compare-read while Rust
	// makes a competing CAS. The native delegate performs no JS read: its
	// completion then becomes the first event, and Rust must lose its CAS.
	// Both branches assert exactly one winner; neither retries or times out
	// as a successful outcome. This delay is confined to this child process.
	fs.readFile = new Proxy(originalRead, {
		async apply(target, thisArg, argumentsList) {
			const bytes = await Reflect.apply(target, thisArg, argumentsList)
			if (argumentsList[0] === targetPath && !paused) {
				paused = true
				assert.ok(Buffer.isBuffer(bytes))
				report({ event: "read-paused", bytes: bytes.toString("utf8") })
				await release()
			}
			return bytes
		}
	})
	syncBuiltinESMExports()
}

try {
	// Default package resolution intentionally exercises the emitted adapter,
	// not a source-condition override. Load after installing the isolated gate.
	const { etag, fsStore } = await import("#store.ts")
	const { storeKey } = await import("#keys.ts")
	const store = fsStore(root)
	const key = storeKey("race/counter")
	if (mode === "poison-lock") {
		const current = await store.get(key)
		assert.ok(current !== null)
		assert.equal(new TextDecoder().decode(current.bytes), "before")
		assert.equal(current.etag, expected)
	}
	try {
		const outcome = await store.putSwap(key, new TextEncoder().encode("node-after"), etag(expected))
		report({ event: "completed", outcome: outcome.tag })
	} catch (cause) {
		if (mode !== "poison-lock") throw cause
		report({ event: "refused", error: String(cause) })
	}
} finally {
	fs.readFile = originalRead
	syncBuiltinESMExports()
}
