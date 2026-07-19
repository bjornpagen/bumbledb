/**
 * The build's platform vocabulary, pinned as a unit (`scripts/platform.ts`):
 * the local-platform derivation (the build must place, link, and smoke-load
 * this host's artifact under `<platform>-<arch>` — the exact name the
 * runtime loader resolves — on EVERY host it can compile on, so a linux
 * build never misfiles its `.so` under the darwin name); the shipped-set
 * single-source pin (`PUBLISH_PLATFORM` === the loader's
 * `SHIPPED_PLATFORMS` === the `.gitignore` carve-out — src cannot import
 * scripts, so the pin is what holds the spellings in lockstep); and the
 * dev-twin manifest derivation (field inheritance from the committed
 * publish manifest by construction — the old hand-written literal had
 * silently dropped `engines`/`repository`/`publishConfig`).
 */

import assert from "node:assert/strict"
import * as fs from "node:fs"
import { describe, test } from "node:test"
import { SHIPPED_PLATFORMS } from "#native.ts"
import {
	deriveDevTwinManifest,
	localPlatformTarget,
	nativeArtifactName,
	PUBLISH_PLATFORM
} from "../scripts/platform.ts"

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

describe("the shipped set, single-sourced", function suite() {
	test("the loader's SHIPPED_PLATFORMS is the build's PUBLISH_PLATFORM", function shippedSetLockstep() {
		/**
		 * src cannot import scripts (the packaging boundary), so the shipped
		 * set is necessarily spelled on both sides — this pin is what holds
		 * the two spellings in lockstep: adding a platform that edits one and
		 * not the other fails here.
		 */
		assert.equal(SHIPPED_PLATFORMS, PUBLISH_PLATFORM)
	})

	test("the .gitignore carve-out names the publish platform", function gitignoreCarveOut() {
		const gitignore = fs.readFileSync(new URL("../.gitignore", import.meta.url), "utf8")
		assert.ok(
			gitignore.includes(`!npm/${PUBLISH_PLATFORM}/`),
			"the committed platform-manifest carve-out must track PUBLISH_PLATFORM"
		)
		assert.ok(
			gitignore.includes(`npm/${PUBLISH_PLATFORM}/bumbledb.node`),
			"the binary re-ignore must track PUBLISH_PLATFORM"
		)
	})
})

describe("the dev-twin manifest derives from the publish manifest", function suite() {
	test("every field except name/description/os/cpu is inherited by construction", function fieldInheritance() {
		const publish = JSON.parse(
			fs.readFileSync(new URL(`../npm/${PUBLISH_PLATFORM}/package.json`, import.meta.url), "utf8")
		) as Record<string, unknown>
		const twin = deriveDevTwinManifest(publish, "linux-x64", "linux", "x64")
		assert.equal(twin.name, "@bjornpagen/bumbledb-linux-x64")
		assert.deepEqual(twin.os, ["linux"])
		assert.deepEqual(twin.cpu, ["x64"])
		assert.match(String(twin.description), /dev tree only, never published/)
		const rewritten = new Set(["name", "description", "os", "cpu"])
		assert.deepEqual(Object.keys(twin), Object.keys(publish), "no key appears, disappears, or moves")
		for (const key of Object.keys(publish)) {
			if (rewritten.has(key)) {
				continue
			}
			assert.deepEqual(twin[key], publish[key], `field ${key} must be inherited from the publish manifest verbatim`)
		}
		// The exact fields the old hand-written literal silently dropped.
		for (const key of ["version", "engines", "repository", "publishConfig", "main", "files"]) {
			assert.ok(Object.hasOwn(twin, key), `field ${key} must ride into the dev twin`)
		}
	})
})
