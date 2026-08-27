import assert from "node:assert/strict"
import * as fs from "node:fs"
import { describe, test } from "node:test"
import { SHIPPED_PLATFORMS } from "#native.ts"
import {
	deriveDevTwinManifest,
	localPlatformTarget,
	nativeArtifactName,
	PUBLISH_PLATFORMS
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
	test("the loader's SHIPPED_PLATFORMS is the build's PUBLISH_PLATFORMS", function shippedSetLockstep() {
		assert.deepEqual([...SHIPPED_PLATFORMS], [...PUBLISH_PLATFORMS])
	})

	test("the .gitignore carve-outs name every publish platform", function gitignoreCarveOut() {
		const gitignore = fs.readFileSync(new URL("../.gitignore", import.meta.url), "utf8")
		for (const platform of PUBLISH_PLATFORMS) {
			assert.ok(
				gitignore.includes(`!npm/${platform}/`),
				`the committed platform-manifest carve-out must track ${platform}`
			)
			assert.ok(gitignore.includes(`npm/${platform}/bumbledb.node`), `the binary re-ignore must track ${platform}`)
		}
	})

	test("every shipped platform has a committed publish manifest", function publishManifests() {
		for (const platform of PUBLISH_PLATFORMS) {
			const manifest = JSON.parse(
				fs.readFileSync(new URL(`../npm/${platform}/package.json`, import.meta.url), "utf8")
			) as { name: string; os: string[]; cpu: string[] }
			const [os, cpu] = platform.split("-")
			assert.equal(manifest.name, `@bjornpagen/bumbledb-${platform}`)
			assert.deepEqual(manifest.os, [os])
			assert.deepEqual(manifest.cpu, [cpu])
		}
	})

	test("every linux tarball is packable from the darwin publish host", function linuxPackHost() {
		for (const platform of PUBLISH_PLATFORMS) {
			if (!platform.startsWith("linux-")) {
				continue
			}
			const yaml = fs.readFileSync(new URL(`../npm/${platform}/pnpm-workspace.yaml`, import.meta.url), "utf8")
			assert.match(yaml, /supportedArchitectures:/)
			assert.match(yaml, /^\s+-\s+current$/m)
			assert.match(yaml, /^\s+-\s+linux$/m)
			const cpu = platform.split("-")[1]
			assert.ok(new RegExp(`^\\s+-\\s+${cpu}$`, "m").test(yaml), `the cpu set must carry ${cpu}`)
		}
	})
})

describe("the dev-twin manifest derives from the publish manifest", function suite() {
	test("every field except name/description/os/cpu is inherited by construction", function fieldInheritance() {
		const publish = JSON.parse(
			fs.readFileSync(new URL(`../npm/${PUBLISH_PLATFORMS[0]}/package.json`, import.meta.url), "utf8")
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

		for (const key of ["version", "engines", "repository", "publishConfig", "main", "files"]) {
			assert.ok(Object.hasOwn(twin, key), `field ${key} must ride into the dev twin`)
		}
	})
})
