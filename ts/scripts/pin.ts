import { ScriptError } from "./errors.ts"
import { PUBLISH_PLATFORMS } from "./platform.ts"

/**
 * Packed-manifest derivation — pure functions over the committed repo
 * manifest, consumed by `scripts/stage.ts`. The committed `package.json`
 * is NEVER rewritten: the earlier prepack/postpack pair that injected and
 * removed `optionalDependencies` in place is deleted
 * (docs/reference/packaging.md PKG-02: no prepack/postpack hook rewrites
 * the developer's package.json or depends on an interrupted post-hook
 * repairing it). Instead the exact platform
 * pins are written into the STAGED manifest only, inside an isolated
 * staging tree, and `pnpm pack` runs there. An interrupted stage leaves
 * the checkout byte-identical by construction.
 *
 * The pin policy is unchanged: every shipped platform package is pinned
 * to the exact release version read from the manifest itself (one
 * source), and the committed manifest stays registry-independent so a
 * lockfile can never demand the current unpublished version
 * (`--frozen-lockfile` bootstrap window).
 */

/** The one selected TypeScript peer/dev dependency (chapter 35). */
const EFFECT_PIN = "4.0.0-rc.112"

function stringField(manifest: Record<string, unknown>, field: string, where: string): string {
	const value = manifest[field]
	if (typeof value !== "string" || value === "") {
		throw new ScriptError({ message: `${where} is missing a string ${field}` })
	}
	return value
}

function record(value: unknown): Record<string, unknown> | undefined {
	return typeof value === "object" && value !== null ? (value as Record<string, unknown>) : undefined
}

/**
 * Asserts the exact Effect handshake on a manifest: `effect` is the
 * pinned RC as a peerDependency, and (when the manifest carries
 * devDependencies — the repo manifest does, a packed manifest does not)
 * the same exact devDependency. No range, no optional adapter, no second
 * Effect version anywhere in the dependency graph we control.
 */
function assertEffectPin(manifest: Record<string, unknown>, where: string): void {
	const peers = record(manifest.peerDependencies)
	if (peers?.effect !== EFFECT_PIN) {
		throw new ScriptError({
			message: `${where} peerDependencies.effect is ${String(peers?.effect)}, expected the exact pin ${EFFECT_PIN}`
		})
	}
	const dev = record(manifest.devDependencies)
	if (dev !== undefined && dev.effect !== undefined && dev.effect !== EFFECT_PIN) {
		throw new ScriptError({
			message: `${where} devDependencies.effect is ${String(dev.effect)}, expected the exact pin ${EFFECT_PIN}`
		})
	}
}

/**
 * Derives the manifest a packed `@bjornpagen/bumbledb` tarball ships:
 * the committed manifest plus exact-version platform pins, minus the
 * repo-only fields. `scripts` is dropped whole (the tarball carries no
 * scripts/ directory and no lifecycle hook may run from it) and
 * `devDependencies` is dropped (repo tooling, never a consumer input).
 * Everything else — exports, imports isolation map, files roster, exact
 * Effect peer — is inherited by construction.
 */
function packedMainManifest(repoManifest: Record<string, unknown>): Record<string, unknown> {
	if ("optionalDependencies" in repoManifest) {
		throw new ScriptError({
			message:
				"the committed package.json carries optionalDependencies — the platform pin lives only in the STAGED manifest (scripts/stage.ts derives it; a committed pin recreates the frozen-lockfile bootstrap window)"
		})
	}
	assertEffectPin(repoManifest, "ts/package.json")
	const version = stringField(repoManifest, "version", "ts/package.json")
	const pins: Record<string, string> = {}
	for (const platform of PUBLISH_PLATFORMS) {
		pins[`@bjornpagen/bumbledb-${platform}`] = version
	}
	const staged: Record<string, unknown> = { ...repoManifest, optionalDependencies: pins }
	delete staged.scripts
	delete staged.devDependencies
	delete staged.packageManager
	return staged
}

/** A platform package manifest ships as committed; this only validates it. */
function packedPlatformManifest(
	manifest: Record<string, unknown>,
	platform: string,
	version: string
): Record<string, unknown> {
	const name = stringField(manifest, "name", `ts/npm/${platform}/package.json`)
	if (name !== `@bjornpagen/bumbledb-${platform}`) {
		throw new ScriptError({
			message: `ts/npm/${platform}/package.json names ${name}, expected @bjornpagen/bumbledb-${platform}`
		})
	}
	const got = stringField(manifest, "version", `ts/npm/${platform}/package.json`)
	if (got !== version) {
		throw new ScriptError({
			message: `ts/npm/${platform}/package.json is ${got}, expected the release version ${version}`
		})
	}
	return manifest
}

export { assertEffectPin, EFFECT_PIN, packedMainManifest, packedPlatformManifest }
