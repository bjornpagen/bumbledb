import * as errors from "@superbuilders/errors"

/**
 * The platforms this release PUBLISHES; each package dir is
 * `npm/<target>` and the build's version-lockstep gate pins every one.
 * Deliberately a hand-written set, never derived from the host:
 * adding a shipped platform is an edit here plus its `npm/<target>`
 * manifest, a decision — building on a linux host must not silently grow
 * the publish set. The loader's `SHIPPED_PLATFORMS` message constant
 * (`src/native.ts` — src cannot import scripts, the packaging boundary)
 * and the `ts/.gitignore` carve-outs spell the same set; the
 * single-source pin in `test/build-platform.test.ts` holds all three in
 * lockstep.
 */
const PUBLISH_PLATFORMS = ["darwin-arm64", "linux-arm64"] as const

function isPublishPlatform(target: string): boolean {
	return (PUBLISH_PLATFORMS as readonly string[]).includes(target)
}

function deriveDevTwinManifest(
	publishManifest: Record<string, unknown>,
	localPlatform: string,
	platform: string,
	arch: string
): Record<string, unknown> {
	return {
		...publishManifest,
		name: `@bjornpagen/bumbledb-${localPlatform}`,
		description: `Locally built ${localPlatform} native binary for @bjornpagen/bumbledb (dev tree only, never published)`,
		os: [platform],
		cpu: [arch]
	}
}

/**
 * The compile allowlist, one table: the key set is the set of platforms
 * the native build compiles on, and the value is cargo's cdylib artifact
 * name there (darwin `.dylib`, linux `.so`). Adding a compile platform is
 * one entry here (the deliberate publish-set edits stay separate, above);
 * the refusal is written once.
 */
const NATIVE_ARTIFACT: Record<"darwin" | "linux", string> = {
	darwin: "libbumbledb_node.dylib",
	linux: "libbumbledb_node.so"
}

function assertSupported(platform: string): asserts platform is keyof typeof NATIVE_ARTIFACT {
	if (!Object.hasOwn(NATIVE_ARTIFACT, platform)) {
		throw errors.new(`unsupported platform for the bumbledb native build: ${platform}`)
	}
}

function localPlatformTarget(platform: string, arch: string): string {
	assertSupported(platform)
	return `${platform}-${arch}`
}

function nativeArtifactName(platform: string): string {
	assertSupported(platform)
	return NATIVE_ARTIFACT[platform]
}

export { deriveDevTwinManifest, isPublishPlatform, localPlatformTarget, nativeArtifactName, PUBLISH_PLATFORMS }
