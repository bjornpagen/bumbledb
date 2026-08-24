import * as fs from "node:fs"
import * as path from "node:path"
import { fileURLToPath } from "node:url"
import * as errors from "@superbuilders/errors"
import { PUBLISH_PLATFORMS } from "./platform.ts"

/**
 * The platform pins, injected where they mean something — the PACKED
 * manifest (the napi prepublish pattern). The repo `package.json` carries
 * NO `optionalDependencies`: a lockfile can never pin the CURRENT
 * unpublished version, so a committed pin put every release in a red-CI
 * window (`--frozen-lockfile` refused the unresolvable exact pin) until a
 * post-publish lockfile regeneration. Instead, `prepack` writes
 * `optionalDependencies` with an exact-version pin for every shipped
 * platform package (the version read from the manifest itself — one
 * source) and `postpack` removes it, so every tarball `pnpm pack` /
 * `pnpm publish` produces carries the pins while the committed tree stays
 * registry-independent forever. The build's tarball proof
 * (`scripts/build.ts` `verifyPack`) packs for real and asserts every
 * injected pin — and that the repo manifest came back pin-free.
 *
 * Both commands are idempotent (inject twice writes the same pins;
 * restore with no pin is a no-op), because `pnpm publish` runs
 * `prepublishOnly` (the full build, whose pack proof runs this pair)
 * before its own prepack/postpack pair. NOTHING is printed on success:
 * pack lifecycle output can share a stream with `pnpm pack --json`,
 * which the build parses.
 */

const PACKAGE_ROOT = fileURLToPath(new URL("..", import.meta.url))

function readManifest(file: string): Record<string, unknown> {
	const text = errors.trySync(() => fs.readFileSync(file, "utf8"))
	if (text.error) {
		throw errors.wrap(text.error, `read ${file}`)
	}
	const parsed = errors.trySync(() => JSON.parse(text.data) as Record<string, unknown>)
	if (parsed.error) {
		throw errors.wrap(parsed.error, `parse ${file}`)
	}
	return parsed.data
}

function writeManifest(file: string, manifest: Record<string, unknown>): void {
	fs.writeFileSync(file, `${JSON.stringify(manifest, null, "\t")}\n`)
}

function inject(file: string): void {
	const manifest = readManifest(file)
	const version = manifest.version
	if (typeof version !== "string" || version === "") {
		throw errors.new(`${file} is missing a string version`)
	}
	const pins: Record<string, string> = {}
	for (const platform of PUBLISH_PLATFORMS) {
		pins[`@bjornpagen/bumbledb-${platform}`] = version
	}
	manifest.optionalDependencies = pins
	writeManifest(file, manifest)
}

function restore(file: string): void {
	const manifest = readManifest(file)
	if (!("optionalDependencies" in manifest)) {
		return
	}
	delete manifest.optionalDependencies
	writeManifest(file, manifest)
}

function main(): void {
	const file = path.join(PACKAGE_ROOT, "package.json")
	const command = process.argv[2]
	switch (command) {
		case "inject":
			inject(file)
			return
		case "restore":
			restore(file)
			return
		default:
			throw errors.new(`pin.ts: unknown command ${String(command)} (inject | restore)`)
	}
}

main()
