import * as errors from "@superbuilders/errors"

/**
 * The build-host platform derivations, extracted from `build.ts` so the
 * mapping is a unit the test suite pins (`test/build-platform.test.ts`)
 * without executing the build. Two distinct concepts meet in the build and
 * only ONE of them lives here: the LOCAL platform (where did this host's
 * cargo build land, where must it be placed/linked/smoke-loaded — derived
 * from the running process) is computed; the PUBLISH set (which platform
 * packages ship to the registry) is a deliberate hand-written list in
 * `build.ts` and is never derived from a host.
 */

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

export { localPlatformTarget, nativeArtifactName }
