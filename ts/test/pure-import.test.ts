/**
 * TS-010 / D22 discriminator: pure schema/scalar authoring must not load
 * the native addon. Native resolution is made unavailable — importing
 * successfully while the platform package is still installed is not this
 * gate. Verification: NotRun
 */
import assert from "node:assert/strict"
import { spawnSync } from "node:child_process"
import path from "node:path"
import { describe, it } from "node:test"

const packageRoot = path.resolve(import.meta.dirname, "..")
const preload = path.join(packageRoot, "test/fixtures/no-addon-preload.mjs")
const child = path.join(packageRoot, "test/fixtures/no-addon-import.ts")

describe("pure schema import", () => {
	it("constructs metadata with the platform addon unresolvable", () => {
		const result = spawnSync(process.execPath, ["--import", preload, child], {
			cwd: packageRoot,
			encoding: "utf8",
			env: { ...process.env }
		})
		assert.equal(
			result.status,
			0,
			`no-addon child failed:\nstdout:\n${result.stdout}\nstderr:\n${result.stderr}`
		)
	})
})
