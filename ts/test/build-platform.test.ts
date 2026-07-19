/**
 * The build's local-platform derivation, pinned as a unit
 * (`scripts/platform.ts`): the build must place, link, and smoke-load this
 * host's artifact under `<platform>-<arch>` — the exact name the runtime
 * loader resolves — on EVERY host it can compile on, so a linux build never
 * misfiles its `.so` under the darwin name. The publish set is a deliberate
 * constant in `build.ts` and is out of scope here by design.
 */

import assert from "node:assert/strict"
import { describe, test } from "node:test"
import { localPlatformTarget, nativeArtifactName } from "../scripts/platform.ts"

describe("the build's local-platform derivation", function suite() {
	test("darwin/arm64 derives darwin-arm64", function darwinArm() {
		assert.equal(localPlatformTarget("darwin", "arm64"), "darwin-arm64")
	})

	test("linux/x64 derives linux-x64", function linuxX64() {
		assert.equal(localPlatformTarget("linux", "x64"), "linux-x64")
	})

	test("the running host derives its own loader-resolvable name", function runningHost() {
		assert.equal(
			localPlatformTarget(process.platform, process.arch),
			`${process.platform}-${process.arch}`,
			"placement, link, and smoke-load follow the running host"
		)
	})

	test("a platform the native build cannot compile on fails loudly", function unsupported() {
		assert.throws(
			function deriveForeign() {
				localPlatformTarget("win32", "x64")
			},
			function typed(error: unknown) {
				assert.ok(error instanceof Error)
				assert.match(error.message, /win32/, "the message names the refused platform")
				return true
			}
		)
	})

	test("the cargo artifact name follows the platform's cdylib convention", function artifact() {
		assert.equal(nativeArtifactName("darwin"), "libbumbledb_node.dylib")
		assert.equal(nativeArtifactName("linux"), "libbumbledb_node.so")
		assert.throws(function artifactForeign() {
			nativeArtifactName("win32")
		})
	})
})
