/**
 * PRD-03 native-loader pins: the arch-split loader resolves the per-platform
 * binary package BY NAME and fails LOUDLY and TYPED on any host it does not
 * ship for. Simulating a foreign `platform`/`arch` (no matching optional dep
 * was ever installed) must yield the actionable unsupported-platform error
 * — naming the running platform-arch and the shipped set — never a raw
 * module-not-found leaking through. The RUNNING host resolves and loads the
 * real addon the build just placed (the SDK's single FFI boundary is
 * exercised) — both cases are computed from `process.platform`/`process.arch`
 * so the suite is host-invariant by construction.
 */

import assert from "node:assert/strict"
import { describe, test } from "node:test"
import { loadNativeBinding } from "#native.ts"

/**
 * A platform-arch pair that is NEVER the running host and NEVER installed:
 * no linux package ships at all, so any linux pair differing from the host
 * itself (whose locally built package the build links by name) is a
 * guaranteed-absent foreign target on every host.
 */
const foreign =
	process.platform === "linux" && process.arch === "x64"
		? { platform: "linux", arch: "arm64" }
		: { platform: "linux", arch: "x64" }

describe("the native loader's platform resolution", function suite() {
	test("a foreign platform throws the typed unsupported-platform error", function foreignCase() {
		assert.throws(
			function loadForeign() {
				loadNativeBinding(foreign.platform, foreign.arch)
			},
			function typed(error: unknown) {
				assert.ok(error instanceof Error, "the loader throws a typed Error, not a bare value")
				assert.match(
					error.message,
					new RegExp(`${foreign.platform}-${foreign.arch}`),
					"the message names the requested platform-arch"
				)
				assert.match(error.message, /darwin-arm64/, "the message names the shipped set")
				return true
			}
		)
	})

	test("the running host resolves and loads the real binary", function host() {
		const native = loadNativeBinding(process.platform, process.arch)
		const version = native.engineVersion()
		assert.equal(typeof version, "string")
		assert.notEqual(version, "", "engineVersion() proves the addon linked and loaded")
	})
})
