import { spawnSync } from "node:child_process"
import * as fs from "node:fs"
import { createRequire } from "node:module"
import * as path from "node:path"
import { fileURLToPath } from "node:url"
import * as errors from "@superbuilders/errors"

/**
 * The package build, end to end, so `pnpm run build` owns both publishable
 * trees with zero steps outside it (PRD-03): the pure-JS MAIN package
 * (`dist/*.js` + declarations, no binary) and the per-platform BINARY package
 * (`npm/darwin-arm64/bumbledb.node` under its `os`/`cpu`-gated manifest).
 *
 * Order: assert version lockstep (one source of truth, the main manifest) →
 * clean dist → cargo-compile the napi bridge against the in-repo engine →
 * place the `.node` in the PLATFORM package dir → link that package into
 * `node_modules` so it resolves by name exactly as the published optional dep
 * would → smoke-load THROUGH the loader's by-name resolution path (a build
 * whose artifact cannot load or link fails here) → emit JS + declarations
 * with tsc → prove both tarballs carry exactly the intended files. All spawns
 * are raw argv arrays — no shell strings, no shell-in-JS libraries.
 */

/** The single platform this release ships; its package dir is `npm/<target>`. */
const TARGET_PLATFORM = "darwin-arm64"

function build(): void {
	const packageRoot = fileURLToPath(new URL("..", import.meta.url))
	const distDir = path.join(packageRoot, "dist")
	const crateManifest = path.join(packageRoot, "crate", "Cargo.toml")
	const platformPackageDir = path.join(packageRoot, "npm", TARGET_PLATFORM)

	const version = assertVersionLockstep(packageRoot, platformPackageDir)
	console.log(`bumbledb build: version ${version} (main == platform == optionalDependencies pin)`)

	fs.rmSync(distDir, { recursive: true, force: true })

	const cargo = spawnSync("cargo", ["build", "--release", "--manifest-path", crateManifest], {
		stdio: "inherit"
	})
	if (cargo.error) {
		throw errors.wrap(cargo.error, "spawn cargo")
	}
	if (cargo.status !== 0) {
		throw errors.new(`cargo build exited with status ${cargo.status}`)
	}

	const artifact = path.join(packageRoot, "crate", "target", "release", nativeArtifactName())
	const nodeBinary = path.join(platformPackageDir, "bumbledb.node")
	fs.mkdirSync(platformPackageDir, { recursive: true })
	fs.copyFileSync(artifact, nodeBinary)

	linkPlatformPackage(packageRoot, platformPackageDir)
	smokeLoad(packageRoot)

	const tsc = spawnSync("tsc", ["-p", "tsconfig.build.json"], {
		stdio: "inherit",
		cwd: packageRoot
	})
	if (tsc.error) {
		throw errors.wrap(tsc.error, "spawn tsc")
	}
	if (tsc.status !== 0) {
		throw errors.new(`tsc exited with status ${tsc.status}`)
	}

	verifyPack(packageRoot, platformPackageDir)
}

/**
 * The version-lockstep gate (PRD-03 item 5): the main manifest's `version` is
 * the single source; the platform manifest's `version` and the main's
 * `optionalDependencies` pin for the platform package must equal it EXACTLY
 * (the FFI ABI is not semver-stable — a main package may only ever resolve
 * its own-version binary). A divergence fails the build before anything is
 * produced, so a release bump is one edit that this gate then enforces.
 */
function assertVersionLockstep(packageRoot: string, platformPackageDir: string): string {
	const main = readJson(path.join(packageRoot, "package.json"))
	const platform = readJson(path.join(platformPackageDir, "package.json"))
	const platformName = `@bjornpagen/bumbledb-${TARGET_PLATFORM}`

	const version = main.version
	if (typeof version !== "string" || version === "") {
		throw errors.new("main package.json is missing a string version")
	}
	const optional = main.optionalDependencies
	const pin =
		typeof optional === "object" && optional !== null ? (optional as Record<string, unknown>)[platformName] : undefined
	if (pin !== version) {
		throw errors.new(
			`version lockstep broken: main is ${version} but optionalDependencies["${platformName}"] is ${String(pin)} (must be an EXACT match)`
		)
	}
	if (platform.version !== version) {
		throw errors.new(
			`version lockstep broken: main is ${version} but ${platformName} package.json is ${String(platform.version)}`
		)
	}
	if (platform.name !== platformName) {
		throw errors.new(`platform package.json name is ${String(platform.name)}, expected ${platformName}`)
	}
	return version
}

/** Reads and parses a JSON file, wrapping either failure. */
function readJson(file: string): Record<string, unknown> {
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

/**
 * Cargo's cdylib artifact name for the host platform: darwin `.dylib`, linux
 * `.so`. The published target is darwin-arm64 only; the `.node` this build
 * places into the platform package (`npm/darwin-arm64/bumbledb.node`) is what
 * the loader resolves by name at runtime.
 */
function nativeArtifactName(): string {
	if (process.platform === "darwin") {
		return "libbumbledb_node.dylib"
	}
	if (process.platform === "linux") {
		return "libbumbledb_node.so"
	}
	throw errors.new(`unsupported platform for the bumbledb native build: ${process.platform}`)
}

/**
 * Links the freshly built platform package into this package's
 * `node_modules` so `@bjornpagen/bumbledb-darwin-arm64` resolves BY NAME —
 * exactly as npm/pnpm would place the published optional dependency on a
 * matching host. Without this the dev tree cannot resolve the platform
 * package, and both the smoke-load and `node --test` (which drive the real
 * loader) would take the unsupported-platform path on darwin-arm64. Purely a
 * dev-tree convenience; `node_modules` is gitignored and rebuilt each run.
 */
function linkPlatformPackage(packageRoot: string, platformPackageDir: string): void {
	const scopeDir = path.join(packageRoot, "node_modules", "@bjornpagen")
	const link = path.join(scopeDir, `bumbledb-${TARGET_PLATFORM}`)
	fs.mkdirSync(scopeDir, { recursive: true })
	fs.rmSync(link, { recursive: true, force: true })
	const target = path.relative(scopeDir, platformPackageDir)
	fs.symlinkSync(target, link, "dir")
}

/**
 * The build's self-assertion (PRD-03 item 4): resolve the platform package BY
 * NAME through the same `createRequire` path the loader uses, require its
 * `bumbledb.node`, and assert `engineVersion()` returns a non-empty string —
 * so a build whose artifact cannot load, whose path dependency did not link,
 * or whose platform package is not resolvable fails here instead of at first
 * runtime use.
 */
function smokeLoad(packageRoot: string): void {
	// createRequire anchored inside the package so its node_modules (with the
	// just-linked platform package) is on the resolution path.
	const requireNative = createRequire(path.join(packageRoot, "scripts", "build.ts"))
	const platformPackage = `@bjornpagen/bumbledb-${TARGET_PLATFORM}`
	const loaded = errors.trySync(() => requireNative(platformPackage))
	if (loaded.error) {
		throw errors.wrap(loaded.error, `smoke-load ${platformPackage} through the by-name loader path`)
	}
	const binding: { engineVersion(): string } = loaded.data
	const version = errors.trySync(() => binding.engineVersion())
	if (version.error) {
		throw errors.wrap(version.error, "smoke call engineVersion()")
	}
	if (typeof version.data !== "string" || version.data === "") {
		throw errors.new("smoke assertion failed: engineVersion() must return a non-empty string")
	}
}

/**
 * Tarball proof (PRD-08 item 4): run `pnpm pack --dry-run --json` (the pnpm
 * equivalent of `npm pack --dry-run`) on both package dirs and assert their
 * file manifests, so a wrong `files`/`.npmignore` fails the build rather than
 * shipping a mispacked tarball. The MAIN tarball
 * must carry NO `.node` (the binary lives only in the platform package); the
 * PLATFORM tarball must carry EXACTLY `bumbledb.node` + `package.json` +
 * `LICENSE` and nothing else.
 */
function verifyPack(packageRoot: string, platformPackageDir: string): void {
	const mainFiles = packDryRun(packageRoot)
	const binary = mainFiles.find((file) => file.endsWith(".node"))
	if (binary !== undefined) {
		throw errors.new(`main package tarball must carry no native binary, found ${binary}`)
	}
	if (!mainFiles.includes("package.json")) {
		throw errors.new("main package tarball is missing package.json")
	}
	if (!mainFiles.some((file) => file.startsWith("dist/"))) {
		throw errors.new("main package tarball carries no dist/ output")
	}

	const platformFiles = packDryRun(platformPackageDir).toSorted()
	const expected = ["LICENSE", "bumbledb.node", "package.json"]
	if (JSON.stringify(platformFiles) !== JSON.stringify(expected)) {
		throw errors.new(
			`platform package tarball must contain exactly ${JSON.stringify(expected)}, found ${JSON.stringify(platformFiles)}`
		)
	}

	console.log("bumbledb build: tarball manifests verified (main has no binary; platform has only the binary)")
}

/** Runs `pnpm pack --dry-run --json` in `dir` and returns its packed file paths. */
function packDryRun(dir: string): string[] {
	const result = spawnSync("pnpm", ["pack", "--dry-run", "--json"], { cwd: dir })
	if (result.error) {
		throw errors.wrap(result.error, "spawn pnpm pack")
	}
	if (result.status !== 0) {
		throw errors.new(`pnpm pack exited with status ${result.status}: ${result.stderr.toString()}`)
	}
	const parsed = errors.trySync(
		() => JSON.parse(result.stdout.toString()) as { files: ReadonlyArray<{ path: string }> }
	)
	if (parsed.error) {
		throw errors.wrap(parsed.error, "parse pnpm pack --json output")
	}
	return parsed.data.files.map((file) => file.path)
}

build()
