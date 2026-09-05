import { spawnSync } from "node:child_process"
import * as fs from "node:fs"
import { createRequire } from "node:module"
import * as os from "node:os"
import * as path from "node:path"
import { fileURLToPath } from "node:url"
import { Result } from "effect"
import { assertDeclarationsAreIsolated, assertPackedImports, rewriteDeclarationImports } from "./declarations.ts"
import { ScriptError } from "./errors.ts"
import {
	deriveDevTwinManifest,
	isPublishPlatform,
	localPlatformTarget,
	nativeArtifactName,
	PUBLISH_PLATFORMS
} from "./platform.ts"

const LOCAL_PLATFORM = localPlatformTarget(process.platform, process.arch)

function build(): void {
	const packageRoot = fileURLToPath(new URL("..", import.meta.url))
	const distDir = path.join(packageRoot, "dist")
	const crateManifest = path.join(packageRoot, "crate", "Cargo.toml")
	const shapePackageDir = path.join(packageRoot, "npm", PUBLISH_PLATFORMS[0])
	const localPackageDir = path.join(packageRoot, "npm", LOCAL_PLATFORM)

	const version = assertVersionLockstep(packageRoot)
	console.log(
		`bumbledb build: version ${version} (main == platform == napi crate == engine; the platform pin injects at pack)`
	)

	fs.rmSync(distDir, { recursive: true, force: true })

	// Incremental release artifacts keep the previous CARGO_PKG_VERSION in
	// engineVersion(); a lockstep bump must remint bumbledb-node.
	const clean = spawnSync("cargo", ["clean", "-p", "bumbledb-node", "--release", "--manifest-path", crateManifest], {
		stdio: "inherit"
	})
	if (clean.error) {
		throw new ScriptError({ message: "spawn cargo clean", cause: clean.error })
	}
	if (clean.status !== 0) {
		throw new ScriptError({ message: `cargo clean exited with status ${clean.status}` })
	}

	const cargo = spawnSync("cargo", ["build", "--release", "--manifest-path", crateManifest], {
		stdio: "inherit"
	})
	if (cargo.error) {
		throw new ScriptError({ message: "spawn cargo", cause: cargo.error })
	}
	if (cargo.status !== 0) {
		throw new ScriptError({ message: `cargo build exited with status ${cargo.status}` })
	}

	ensureLocalPlatformPackage(shapePackageDir, localPackageDir)
	const targetDir = process.env.CARGO_TARGET_DIR ?? path.join(packageRoot, "crate", "target")
	const artifact = path.join(targetDir, "release", nativeArtifactName(process.platform))
	const nodeBinary = path.join(localPackageDir, "bumbledb.node")
	fs.copyFileSync(artifact, nodeBinary)

	linkPlatformPackage(packageRoot, localPackageDir)
	smokeLoad(packageRoot, version)

	const tsc = spawnSync("tsc", ["-p", "tsconfig.build.json"], {
		stdio: "inherit",
		cwd: packageRoot
	})
	if (tsc.error) {
		throw new ScriptError({ message: "spawn tsc", cause: tsc.error })
	}
	if (tsc.status !== 0) {
		throw new ScriptError({ message: `tsc exited with status ${tsc.status}` })
	}

	rewriteDeclarationImports(distDir)
	assertDeclarationsAreIsolated(distDir)

	verifyPack(packageRoot, localPackageDir, version)
}

const VERSION_ROSTER = "scripts/version-roster.txt"

function workspacePackageVersion(repoRoot: string): string {
	const manifestPath = path.join(repoRoot, "Cargo.toml")
	const crate = Result.try(() => fs.readFileSync(manifestPath, "utf8"))
	if (Result.isFailure(crate)) {
		throw new ScriptError({ message: `read ${manifestPath}`, cause: crate.failure })
	}
	const block = /\[workspace\.package\]\s*([\s\S]*?)(?:\n\[|$)/.exec(crate.success)
	if (block === null || typeof block[1] !== "string") {
		throw new ScriptError({ message: `${manifestPath} is missing [workspace.package]` })
	}
	const version = /^version = "([^"]+)"$/m.exec(block[1])?.[1]
	if (typeof version !== "string" || version === "") {
		throw new ScriptError({ message: `${manifestPath} [workspace.package] is missing a version` })
	}
	return version
}

function cargoPackageVersion(manifestPath: string): string {
	const crate = Result.try(() => fs.readFileSync(manifestPath, "utf8"))
	if (Result.isFailure(crate)) {
		throw new ScriptError({ message: `read ${manifestPath}`, cause: crate.failure })
	}
	const crateVersion = /^version = "([^"]+)"$/m.exec(crate.success)?.[1]
	if (typeof crateVersion !== "string" || crateVersion === "") {
		throw new ScriptError({ message: `${manifestPath} is missing a package version` })
	}
	return crateVersion
}

function npmPackageVersion(manifestPath: string): string {
	const manifest = readJson(manifestPath)
	const version = manifest.version
	if (typeof version !== "string" || version === "") {
		throw new ScriptError({ message: `${manifestPath} is missing a string version` })
	}
	return version
}

function manifestVersion(repoRoot: string, relPath: string): string {
	const abs = path.join(repoRoot, relPath)
	if (relPath.endsWith("Cargo.toml")) {
		return cargoPackageVersion(abs)
	}
	if (relPath.endsWith("package.json")) {
		return npmPackageVersion(abs)
	}
	throw new ScriptError({ message: `${relPath} is not a versioned manifest` })
}

function readVersionRoster(repoRoot: string): string[] {
	const rosterPath = path.join(repoRoot, VERSION_ROSTER)
	const text = Result.try(() => fs.readFileSync(rosterPath, "utf8"))
	if (Result.isFailure(text)) {
		throw new ScriptError({ message: `read ${rosterPath}`, cause: text.failure })
	}
	const paths = text.success.split("\n").flatMap((line) => {
		const trimmed = line.trim()
		return trimmed === "" || trimmed.startsWith("#") ? [] : [trimmed]
	})
	if (paths.length === 0) {
		throw new ScriptError({ message: `${VERSION_ROSTER} is empty` })
	}
	const seen = new Set<string>()
	for (const rel of paths) {
		if (seen.has(rel)) {
			throw new ScriptError({ message: `${VERSION_ROSTER} lists ${rel} twice` })
		}
		seen.add(rel)
	}
	return paths
}

function isVersionBearing(repoRoot: string, relPath: string): boolean {
	const abs = path.join(repoRoot, relPath)
	const base = path.basename(relPath)
	if (base === "Cargo.toml") {
		const text = Result.try(() => fs.readFileSync(abs, "utf8"))
		if (Result.isFailure(text)) {
			throw new ScriptError({ message: `read ${abs}`, cause: text.failure })
		}
		return /\[package\]/.test(text.success) && /^version = "/m.test(text.success)
	}
	if (base === "package.json") {
		const manifest = readJson(abs)
		return typeof manifest.version === "string" && manifest.version !== ""
	}
	return false
}

function trackedManifests(repoRoot: string): string[] {
	// `-c safe.directory` is process-scoped so `git ls-files` works when
	// the checkout owner differs from the process (Actions containers,
	// docker, odd mounts) without writing the user's global gitconfig.
	const listed = spawnSync("git", ["-c", `safe.directory=${repoRoot}`, "-C", repoRoot, "ls-files", "-z"])
	if (listed.error) {
		throw new ScriptError({ message: "spawn git ls-files", cause: listed.error })
	}
	if (listed.status !== 0) {
		throw new ScriptError({ message: `git ls-files exited with status ${listed.status}: ${listed.stderr.toString()}` })
	}
	return listed.stdout
		.toString("utf8")
		.split("\0")
		.flatMap((file) => {
			const base = path.posix.basename(file)
			return base === "Cargo.toml" || base === "package.json" ? [file] : []
		})
}

function versionBearingManifests(repoRoot: string): string[] {
	return trackedManifests(repoRoot).filter((rel) => isVersionBearing(repoRoot, rel))
}

function assertRosterComplete(repoRoot: string, roster: readonly string[]): void {
	const found = versionBearingManifests(repoRoot)
	const rosterSet = new Set(roster)
	const extra = found.filter((rel) => !rosterSet.has(rel))
	if (extra.length > 0) {
		throw new ScriptError({
			message: `version lockstep broken: version-bearing manifest off-roster: ${extra.join(", ")}`
		})
	}
	const missing = roster.filter((rel) => !found.includes(rel))
	if (missing.length > 0) {
		throw new ScriptError({
			message: `version lockstep broken: roster names a manifest the tree sweep did not find: ${missing.join(", ")}`
		})
	}
}

function assertTsLogPeer(repoRoot: string, version: string): void {
	const manifest = readJson(path.join(repoRoot, "ts-log", "package.json"))
	const peers =
		typeof manifest.peerDependencies === "object" && manifest.peerDependencies !== null
			? (manifest.peerDependencies as Record<string, unknown>)
			: undefined
	if (peers === undefined) {
		throw new ScriptError({ message: "ts-log/package.json is missing peerDependencies" })
	}
	const peer = peers["@bjornpagen/bumbledb"]
	const expected = version
	if (peer !== expected) {
		throw new ScriptError({
			message: `version lockstep broken: ts-log peerDependencies["@bjornpagen/bumbledb"] is ${String(peer)}, expected ${expected}`
		})
	}
}

/**
 * The version-lockstep gate: `[workspace.package] version` is the one
 * writer. Every path on `scripts/version-roster.txt` carries that
 * version exactly; a sweep of tracked `Cargo.toml` and `package.json`
 * files proves the roster lists every version-bearing manifest; `ts-log`'s
 * peer on `@bjornpagen/bumbledb` is exactly `<workspace version>`: a log
 * package cannot silently select a different native command/runtime contract.
 * The FFI ABI is not semver-stable — a main package may only ever resolve
 * its own-version binary; `engineVersion` bakes
 * `CARGO_PKG_VERSION` into the shipped binary. The platform PIN is not a
 * repo field: the repo manifest carries no `optionalDependencies`;
 * `scripts/pin.ts` injects the pin into the PACKED manifest at prepack,
 * exact-version by construction from the one source, and `verifyPack`
 * proves the injected pin on a real tarball. A divergence fails the build
 * before anything is produced. Pure manifest reads, so the gate holds on
 * every build host.
 */
function assertVersionLockstep(packageRoot: string): string {
	const repoRoot = path.join(packageRoot, "..")
	const version = workspacePackageVersion(repoRoot)
	const main = readJson(path.join(packageRoot, "package.json"))
	if ("optionalDependencies" in main) {
		throw new ScriptError({
			message:
				"the repo package.json carries optionalDependencies — the platform pin lives only in the PACKED manifest (scripts/pin.ts injects it at prepack; a committed pin recreates the sdk lane's frozen-lockfile bootstrap window)"
		})
	}
	const roster = readVersionRoster(repoRoot)
	for (const rel of roster) {
		const got = manifestVersion(repoRoot, rel)
		if (got !== version) {
			throw new ScriptError({ message: `version lockstep broken: workspace is ${version} but ${rel} is ${got}` })
		}
	}
	assertRosterComplete(repoRoot, roster)
	assertTsLogPeer(repoRoot, version)
	for (const platform of PUBLISH_PLATFORMS) {
		const platformName = `@bjornpagen/bumbledb-${platform}`
		const manifest = readJson(path.join(packageRoot, "npm", platform, "package.json"))
		if (manifest.name !== platformName) {
			throw new ScriptError({
				message: `platform package.json name is ${String(manifest.name)}, expected ${platformName}`
			})
		}
	}
	return version
}

/** Reads and parses a JSON file, wrapping either failure. */
function readJson(file: string): Record<string, unknown> {
	const text = Result.try(() => fs.readFileSync(file, "utf8"))
	if (Result.isFailure(text)) {
		throw new ScriptError({ message: `read ${file}`, cause: text.failure })
	}
	const parsed = Result.try(() => JSON.parse(text.success) as Record<string, unknown>)
	if (Result.isFailure(parsed)) {
		throw new ScriptError({ message: `parse ${file}`, cause: parsed.failure })
	}
	return parsed.success
}

/**
 * Guarantees the LOCAL platform package dir exists with a loadable manifest.
 * On a shipped platform this is the committed `npm/<target>` tree and
 * nothing is written. On any other build host (a compile-allowlisted
 * checkout outside the publish set) the dir is SYNTHESIZED — a
 * dev-tree-only, gitignored manifest DERIVED from a committed publish
 * manifest: only `name`, `description`, `os`, and `cpu` are rewritten for
 * the host; every other field (`version`, `main`, `files`, `engines`,
 * `repository`, `publishConfig`, …) is inherited BY CONSTRUCTION, so the
 * twin can never drift from the publish shape field by field. The LICENSE
 * rides along, so the by-name link, the smoke-load, and the tarball proof
 * all exercise the exact shape a published platform package would have.
 * Publishing is untouched: the publish runbook names each shipped
 * `./npm/<target>` explicitly and a synthesized twin never enters the
 * registry.
 */
function ensureLocalPlatformPackage(shapePackageDir: string, localPackageDir: string): void {
	fs.mkdirSync(localPackageDir, { recursive: true })
	if (isPublishPlatform(LOCAL_PLATFORM)) {
		return
	}
	const manifest = deriveDevTwinManifest(
		readJson(path.join(shapePackageDir, "package.json")),
		LOCAL_PLATFORM,
		process.platform,
		process.arch
	)
	fs.writeFileSync(path.join(localPackageDir, "package.json"), `${JSON.stringify(manifest, null, "\t")}\n`)
	fs.copyFileSync(path.join(shapePackageDir, "LICENSE"), path.join(localPackageDir, "LICENSE"))
}

/**
 * Links the freshly built platform package into this package's
 * `node_modules` so `@bjornpagen/bumbledb-<platform>-<arch>` resolves BY
 * NAME — exactly as npm/pnpm would place the published optional dependency
 * on a matching host. Without this the dev tree cannot resolve the platform
 * package, and both the smoke-load and `node --test` (which drive the real
 * loader) would take the unsupported-platform path on the build host itself.
 * Purely a dev-tree convenience; `node_modules` is gitignored and rebuilt
 * each run.
 */
function linkPlatformPackage(packageRoot: string, localPackageDir: string): void {
	const scopeDir = path.join(packageRoot, "node_modules", "@bjornpagen")
	const link = path.join(scopeDir, `bumbledb-${LOCAL_PLATFORM}`)
	fs.mkdirSync(scopeDir, { recursive: true })
	fs.rmSync(link, { recursive: true, force: true })
	const target = path.relative(scopeDir, localPackageDir)
	fs.symlinkSync(target, link, "dir")
}

function smokeLoad(packageRoot: string, release: string): void {
	const requireNative = createRequire(path.join(packageRoot, "scripts", "build.ts"))
	const platformPackage = `@bjornpagen/bumbledb-${LOCAL_PLATFORM}`
	const loaded = Result.try(() => requireNative(platformPackage))
	if (Result.isFailure(loaded)) {
		throw new ScriptError({
			message: `smoke-load ${platformPackage} through the by-name loader path`,
			cause: loaded.failure
		})
	}
	const binding: { engineVersion(): string } = loaded.success
	const version = Result.try(() => binding.engineVersion())
	if (Result.isFailure(version)) {
		throw new ScriptError({ message: "smoke call engineVersion()", cause: version.failure })
	}
	if (typeof version.success !== "string" || !version.success.includes(release)) {
		throw new ScriptError({
			message: `smoke assertion failed: engineVersion() must carry the release version ${release}, got ${String(version.success)}`
		})
	}
}

function verifyPack(packageRoot: string, localPackageDir: string, version: string): void {
	const mainFiles = packDryRun(packageRoot)
	const binary = mainFiles.find((file) => file.endsWith(".node"))
	if (binary !== undefined) {
		throw new ScriptError({ message: `main package tarball must carry no native binary, found ${binary}` })
	}
	if (!mainFiles.includes("package.json")) {
		throw new ScriptError({ message: "main package tarball is missing package.json" })
	}
	if (!mainFiles.some((file) => file.startsWith("dist/"))) {
		throw new ScriptError({ message: "main package tarball carries no dist/ output" })
	}

	const platformFiles = packDryRun(localPackageDir).toSorted()
	const expected = ["LICENSE", "bumbledb.node", "package.json"]
	if (JSON.stringify(platformFiles) !== JSON.stringify(expected)) {
		throw new ScriptError({
			message: `platform package tarball must contain exactly ${JSON.stringify(expected)}, found ${JSON.stringify(platformFiles)}`
		})
	}

	verifyInjectedPin(packageRoot, version)

	console.log(
		"bumbledb build: tarball manifests verified (main has no binary; platform has only the binary; the packed pin is exact)"
	)
}

function verifyInjectedPin(packageRoot: string, version: string): void {
	const scratch = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-pack-"))
	try {
		const tarball = path.join(scratch, "main.tgz")
		const pack = spawnSync("pnpm", ["pack", "--out", tarball], { cwd: packageRoot })
		if (pack.error) {
			throw new ScriptError({ message: "spawn pnpm pack", cause: pack.error })
		}
		if (pack.status !== 0) {
			throw new ScriptError({ message: `pnpm pack exited with status ${pack.status}: ${pack.stderr.toString()}` })
		}
		const extract = spawnSync("tar", ["-xzOf", tarball, "package/package.json"], { cwd: scratch })
		if (extract.error) {
			throw new ScriptError({ message: "spawn tar", cause: extract.error })
		}
		if (extract.status !== 0) {
			throw new ScriptError({ message: `tar exited with status ${extract.status}: ${extract.stderr.toString()}` })
		}
		const packed = Result.try(() => JSON.parse(extract.stdout.toString()) as Record<string, unknown>)
		if (Result.isFailure(packed)) {
			throw new ScriptError({ message: "parse the packed package.json", cause: packed.failure })
		}
		const optional =
			typeof packed.success.optionalDependencies === "object" && packed.success.optionalDependencies !== null
				? (packed.success.optionalDependencies as Record<string, unknown>)
				: {}
		for (const platform of PUBLISH_PLATFORMS) {
			const platformName = `@bjornpagen/bumbledb-${platform}`
			const pin = optional[platformName]
			if (pin !== version) {
				throw new ScriptError({
					message: `the packed manifest's optionalDependencies["${platformName}"] is ${String(pin)}, expected the exact release version ${version} (scripts/pin.ts injects it at prepack)`
				})
			}
		}
		assertPackedImports(packed.success)
	} finally {
		fs.rmSync(scratch, { recursive: true, force: true })
	}
	const repo = readJson(path.join(packageRoot, "package.json"))
	if ("optionalDependencies" in repo) {
		throw new ScriptError({
			message:
				"the repo package.json still carries optionalDependencies after pack — postpack's restore failed (the committed manifest must stay pin-free)"
		})
	}
}

function packDryRun(dir: string): string[] {
	const result = spawnSync("pnpm", ["pack", "--dry-run", "--json"], { cwd: dir })
	if (result.error) {
		throw new ScriptError({ message: "spawn pnpm pack", cause: result.error })
	}
	if (result.status !== 0) {
		throw new ScriptError({ message: `pnpm pack exited with status ${result.status}: ${result.stderr.toString()}` })
	}
	const parsed = Result.try(() => JSON.parse(result.stdout.toString()) as { files: ReadonlyArray<{ path: string }> })
	if (Result.isFailure(parsed)) {
		throw new ScriptError({ message: "parse pnpm pack --json output", cause: parsed.failure })
	}
	return parsed.success.files.map((file) => file.path)
}

build()
