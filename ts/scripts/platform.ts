import * as errors from "@superbuilders/errors"

/**
 * The build-host platform vocabulary, extracted from `build.ts` so each
 * piece is a unit the test suite pins (`test/build-platform.test.ts`)
 * without executing the build. Two distinct concepts meet in the build and
 * BOTH are declared here: the LOCAL platform (where did this host's cargo
 * build land, where must it be placed/linked/smoke-loaded — derived from
 * the running process) is computed; the PUBLISH set ({@link
 * PUBLISH_PLATFORM} — which platform package ships to the registry) is a
 * deliberate hand-written constant and is never derived from a host.
 */

/**
 * The single platform this release PUBLISHES; its package dir is
 * `npm/<target>` and the build's version-lockstep gate pins it.
 * Deliberately a hand-written constant, never derived from the host:
 * adding a shipped platform is an edit here plus its `npm/<target>`
 * manifest, a decision — building on a linux host must not silently grow
 * the publish set. The loader's `SHIPPED_PLATFORMS` message constant
 * (`src/native.ts` — src cannot import scripts, the packaging boundary)
 * and the `ts/.gitignore` carve-out spell the same target; the
 * single-source pin in `test/build-platform.test.ts` holds all three in
 * lockstep.
 */
const PUBLISH_PLATFORM = "darwin-arm64"

/**
 * The dev twin's whole manifest derivation, pure and testable: the
 * committed publish manifest with exactly four host-specific fields
 * rewritten (`name`, `description`, `os`, `cpu`); every other field
 * (`version`, `main`, `files`, `engines`, `repository`, `publishConfig`,
 * …) is inherited BY CONSTRUCTION, so the twin can never drift from the
 * publish shape field by field — the old hand-written literal had already
 * drifted. Key order is preserved (spread keeps the source order; the
 * rewritten keys already exist in the publish manifest, so no key moves).
 */
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
 * The per-platform package dir/suffix for a build host: `<platform>-<arch>`
 * exactly as the binary packages spell it (`darwin`/`arm64` → `darwin-arm64`,
 * `linux`/`x64` → `linux-x64`) — the same string the runtime loader
 * (`src/native.ts`) assembles when it resolves
 * `@bjornpagen/bumbledb-<platform>-<arch>` by name. Only the platforms the
 * native build compiles on are accepted; anything else fails loudly here
 * rather than placing an artifact under a name no loader will ever ask for.
 */
function localPlatformTarget(platform: string, arch: string): string {
	if (platform !== "darwin" && platform !== "linux") {
		throw errors.new(`unsupported platform for the bumbledb native build: ${platform}`)
	}
	return `${platform}-${arch}`
}

/**
 * Cargo's cdylib artifact name on a build host: darwin `.dylib`, linux
 * `.so`. The build copies this file to `bumbledb.node` inside the local
 * platform package dir — the name the loader resolves at runtime.
 */
function nativeArtifactName(platform: string): string {
	if (platform === "darwin") {
		return "libbumbledb_node.dylib"
	}
	if (platform === "linux") {
		return "libbumbledb_node.so"
	}
	throw errors.new(`unsupported platform for the bumbledb native build: ${platform}`)
}

export { deriveDevTwinManifest, localPlatformTarget, nativeArtifactName, PUBLISH_PLATFORM }
