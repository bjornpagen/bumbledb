import { spawnSync } from "node:child_process"
import * as fs from "node:fs"
import { createRequire } from "node:module"
import * as os from "node:os"
import * as path from "node:path"
import { fileURLToPath } from "node:url"
import * as errors from "@superbuilders/errors"
import { assertDeclarationsAreIsolated, assertPackedImports, rewriteDeclarationImports } from "./declarations.ts"
import { deriveDevTwinManifest, localPlatformTarget, nativeArtifactName, PUBLISH_PLATFORM } from "./platform.ts"

/**
 * The package build, end to end, so `pnpm run build` owns both publishable
 * trees with zero steps outside it (PRD-03): the pure-JS MAIN package
 * (`dist/*.js` + declarations, no binary) and the per-platform BINARY package
 * (`npm/<platform>-<arch>/bumbledb.node` under its `os`/`cpu`-gated manifest).
 *
 * Two platform concepts, never conflated: the PUBLISH platform (the one
 * package this release ships to the registry — darwin-arm64, a deliberate
 * list) owns the version-lockstep gate; the LOCAL platform (this host,
 * derived from `process.platform`/`process.arch` in `platform.ts`) owns
 * artifact placement, the by-name link, the smoke-load, and the platform
 * tarball proof — so a linux host builds, links, and verifies its own `.so`
 * under its own name instead of misfiling it under the darwin one.
 *
 * Order: assert version lockstep (one source of truth — npm main, platform
 * package, napi crate, engine crate, C ABI crate, workspace members) →
 * clean dist → cargo-compile the napi bridge against the in-repo engine →
 * place the `.node` in the LOCAL platform package dir → link that package
 * into `node_modules` so it resolves by name exactly as the published
 * optional dep would → smoke-load THROUGH the loader's by-name resolution
 * path (a build whose artifact cannot load or link fails here) → emit JS +
 * declarations with tsc → rewrite `#` specifiers out of `.d.ts` so the
 * published type graph is a closed relative tree → prove both tarballs carry exactly the intended
 * files and the packed main manifest carries the exact-version platform pin
 * (injected at prepack by `scripts/pin.ts`; the repo manifest stays
 * pin-free). All spawns are raw argv arrays — no shell strings, no
 * shell-in-JS libraries.
 */

/** This host's platform target — where placement, link, and smoke-load go. */
const LOCAL_PLATFORM = localPlatformTarget(process.platform, process.arch)

function build(): void {
	const packageRoot = fileURLToPath(new URL("..", import.meta.url))
	const distDir = path.join(packageRoot, "dist")
	const crateManifest = path.join(packageRoot, "crate", "Cargo.toml")
	const publishPackageDir = path.join(packageRoot, "npm", PUBLISH_PLATFORM)
	const localPackageDir = path.join(packageRoot, "npm", LOCAL_PLATFORM)

	const version = assertVersionLockstep(packageRoot, publishPackageDir, crateManifest)
	console.log(
		`bumbledb build: version ${version} (main == platform == napi crate == engine == C ABI; the platform pin injects at pack)`
	)

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

	ensureLocalPlatformPackage(publishPackageDir, localPackageDir)
	const artifact = path.join(packageRoot, "crate", "target", "release", nativeArtifactName(process.platform))
	const nodeBinary = path.join(localPackageDir, "bumbledb.node")
	fs.copyFileSync(artifact, nodeBinary)

	linkPlatformPackage(packageRoot, localPackageDir)
	smokeLoad(packageRoot, version)

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

	rewriteDeclarationImports(distDir)
	assertDeclarationsAreIsolated(distDir)

	verifyPack(packageRoot, localPackageDir, version)
}

/**
 * The version-lockstep gate: the main manifest's `version` is the single
 * source. The PUBLISH platform manifest, the napi crate, the engine crate,
 * `bumbledb-c`, and the other workspace members must equal it EXACTLY.
 * The FFI ABI is not semver-stable — a main package may only ever resolve
 * its own-version binary; `engineVersion()` and `bdb_version()` bake
 * `CARGO_PKG_VERSION` into the shipped binary. The platform PIN is not a
 * repo field: the repo manifest must carry NO `optionalDependencies` (a
 * committed exact pin of the current unpublished version made every
 * release a red-CI window — the frozen lockfile can never resolve it);
 * `scripts/pin.ts` injects the pin into the PACKED manifest at prepack,
 * exact-version by construction from the one source, and `verifyPack`
 * proves the injected pin on a real tarball. A divergence fails the build
 * before anything is produced. Pure manifest reads, so the gate holds on
 * EVERY build host, not just the publishing one.
 */
function cargoPackageVersion(manifestPath: string): string {
	const crate = errors.trySync(() => fs.readFileSync(manifestPath, "utf8"))
	if (crate.error) {
		throw errors.wrap(crate.error, `read ${manifestPath}`)
	}
	const crateVersion = /^version = "([^"]+)"$/m.exec(crate.data)?.[1]
	if (typeof crateVersion !== "string" || crateVersion === "") {
		throw errors.new(`${manifestPath} is missing a package version`)
	}
	return crateVersion
}

function assertCargoLockstep(manifestPath: string, version: string, label: string): void {
	const got = cargoPackageVersion(manifestPath)
	if (got !== version) {
		throw errors.new(`version lockstep broken: main is ${version} but ${label} is ${got}`)
	}
}

function assertVersionLockstep(packageRoot: string, publishPackageDir: string, crateManifest: string): string {
	const main = readJson(path.join(packageRoot, "package.json"))
	const platform = readJson(path.join(publishPackageDir, "package.json"))
	const platformName = `@bjornpagen/bumbledb-${PUBLISH_PLATFORM}`

	const version = main.version
	if (typeof version !== "string" || version === "") {
		throw errors.new("main package.json is missing a string version")
	}
	if ("optionalDependencies" in main) {
		throw errors.new(
			"the repo package.json carries optionalDependencies — the platform pin lives only in the PACKED manifest (scripts/pin.ts injects it at prepack; a committed pin recreates the sdk lane's frozen-lockfile bootstrap window)"
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

	const repoRoot = path.join(packageRoot, "..")
	assertCargoLockstep(crateManifest, version, "ts/crate/Cargo.toml")
	assertCargoLockstep(path.join(repoRoot, "crates/bumbledb-c/Cargo.toml"), version, "crates/bumbledb-c/Cargo.toml")
	for (const member of workspaceMemberManifests(repoRoot)) {
		assertCargoLockstep(member, version, path.relative(repoRoot, member))
	}
	return version
}

/**
 * Every engine-workspace member must share the product identity. Parsed
 * from the root `members = [...]` list so a new crate cannot skip the
 * gate. `bumbledb-c` and `ts/crate` are workspace-excluded leaves and
 * are asserted separately — those are the C ABI and napi spellings the
 * TS three-way used to miss.
 */
function workspaceMemberManifests(repoRoot: string): string[] {
	const manifestPath = path.join(repoRoot, "Cargo.toml")
	const text = errors.trySync(() => fs.readFileSync(manifestPath, "utf8"))
	if (text.error) {
		throw errors.wrap(text.error, `read ${manifestPath}`)
	}
	const block = /members\s*=\s*\[([\s\S]*?)\]/.exec(text.data)
	if (block === null || typeof block[1] !== "string") {
		throw errors.new(`${manifestPath} is missing a workspace members list`)
	}
	const members = [...block[1].matchAll(/"([^"]+)"/g)].flatMap((match) => {
		const member = match[1]
		return member === undefined ? [] : [path.join(repoRoot, member, "Cargo.toml")]
	})
	if (members.length === 0) {
		throw errors.new(`${manifestPath} workspace members list is empty`)
	}
	return members
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
 * Guarantees the LOCAL platform package dir exists with a loadable manifest.
 * On the publish platform this is the committed `npm/darwin-arm64` tree and
 * nothing is written. On any other build host (a linux checkout) the dir is
 * SYNTHESIZED — a dev-tree-only, gitignored manifest DERIVED from the
 * committed publish manifest: only `name`, `description`, `os`, and `cpu`
 * are rewritten for the host; every other field (`version`, `main`,
 * `files`, `engines`, `repository`, `publishConfig`, …) is inherited BY
 * CONSTRUCTION, so the twin can never drift from the publish shape field
 * by field (the old hand-written literal had already drifted). The LICENSE
 * rides along, so the by-name link, the smoke-load, and the tarball proof
 * all exercise the exact shape a published platform package would have.
 * Publishing is untouched: the publish runbook names `./npm/darwin-arm64`
 * explicitly and this dir never enters the registry.
 */
function ensureLocalPlatformPackage(publishPackageDir: string, localPackageDir: string): void {
	fs.mkdirSync(localPackageDir, { recursive: true })
	if (LOCAL_PLATFORM === PUBLISH_PLATFORM) {
		return
	}
	const manifest = deriveDevTwinManifest(
		readJson(path.join(publishPackageDir, "package.json")),
		LOCAL_PLATFORM,
		process.platform,
		process.arch
	)
	fs.writeFileSync(path.join(localPackageDir, "package.json"), `${JSON.stringify(manifest, null, "\t")}\n`)
	fs.copyFileSync(path.join(publishPackageDir, "LICENSE"), path.join(localPackageDir, "LICENSE"))
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

/**
 * The build's self-assertion (PRD-03 item 4): resolve the LOCAL platform
 * package BY NAME through the same `createRequire` path the loader uses,
 * require its `bumbledb.node`, and assert `engineVersion()` carries the
 * release version — so a build whose artifact cannot load, whose path
 * dependency did not link, whose platform package is not resolvable, or
 * whose binary self-reports a foreign version fails here instead of at
 * first runtime use. The smoke check witnesses identity, not mere life.
 */
function smokeLoad(packageRoot: string, release: string): void {
	// createRequire anchored inside the package so its node_modules (with the
	// just-linked platform package) is on the resolution path.
	const requireNative = createRequire(path.join(packageRoot, "scripts", "build.ts"))
	const platformPackage = `@bjornpagen/bumbledb-${LOCAL_PLATFORM}`
	const loaded = errors.trySync(() => requireNative(platformPackage))
	if (loaded.error) {
		throw errors.wrap(loaded.error, `smoke-load ${platformPackage} through the by-name loader path`)
	}
	const binding: { engineVersion(): string } = loaded.data
	const version = errors.trySync(() => binding.engineVersion())
	if (version.error) {
		throw errors.wrap(version.error, "smoke call engineVersion()")
	}
	if (typeof version.data !== "string" || !version.data.includes(release)) {
		throw errors.new(
			`smoke assertion failed: engineVersion() must carry the release version ${release}, got ${String(version.data)}`
		)
	}
}

/**
 * Tarball proof (PRD-08 item 4): run `pnpm pack --dry-run --json` (the pnpm
 * equivalent of `npm pack --dry-run`) on both package dirs and assert their
 * file manifests, so a wrong `files`/`.npmignore` fails the build rather than
 * shipping a mispacked tarball. The MAIN tarball
 * must carry NO `.node` (the binary lives only in the platform package); the
 * LOCAL platform tarball (identical to the publish tarball on the publish
 * host, the synthesized dev twin elsewhere) must carry EXACTLY
 * `bumbledb.node` + `package.json` + `LICENSE` and nothing else. The PIN
 * proof then packs the main package FOR REAL (the prepack/postpack pair
 * runs, exactly as `pnpm publish` runs it), extracts the tarball's
 * `package.json`, and asserts the injected pin equals the release version
 * exactly — and that the repo manifest came back PIN-FREE (the restore
 * held). A tarball without the pin would install with no platform binary
 * anywhere; a repo manifest left with the pin would re-open the sdk lane's
 * frozen-lockfile window.
 */
function verifyPack(packageRoot: string, localPackageDir: string, version: string): void {
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

	const platformFiles = packDryRun(localPackageDir).toSorted()
	const expected = ["LICENSE", "bumbledb.node", "package.json"]
	if (JSON.stringify(platformFiles) !== JSON.stringify(expected)) {
		throw errors.new(
			`platform package tarball must contain exactly ${JSON.stringify(expected)}, found ${JSON.stringify(platformFiles)}`
		)
	}

	verifyInjectedPin(packageRoot, version)

	console.log(
		"bumbledb build: tarball manifests verified (main has no binary; platform has only the binary; the packed pin is exact)"
	)
}

/**
 * The injected-pin proof: a REAL `pnpm pack` of the main package (into a
 * scratch dir, so the lifecycle pair runs as publish would run it), the
 * packed `package.json` read straight out of the tarball, the
 * `optionalDependencies` pin asserted EXACTLY the release version — then
 * the repo manifest asserted pin-free (postpack restored it).
 */
function verifyInjectedPin(packageRoot: string, version: string): void {
	const scratch = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-pack-"))
	try {
		const tarball = path.join(scratch, "main.tgz")
		const pack = spawnSync("pnpm", ["pack", "--out", tarball], { cwd: packageRoot })
		if (pack.error) {
			throw errors.wrap(pack.error, "spawn pnpm pack")
		}
		if (pack.status !== 0) {
			throw errors.new(`pnpm pack exited with status ${pack.status}: ${pack.stderr.toString()}`)
		}
		const extract = spawnSync("tar", ["-xzOf", tarball, "package/package.json"], { cwd: scratch })
		if (extract.error) {
			throw errors.wrap(extract.error, "spawn tar")
		}
		if (extract.status !== 0) {
			throw errors.new(`tar exited with status ${extract.status}: ${extract.stderr.toString()}`)
		}
		const packed = errors.trySync(() => JSON.parse(extract.stdout.toString()) as Record<string, unknown>)
		if (packed.error) {
			throw errors.wrap(packed.error, "parse the packed package.json")
		}
		const platformName = `@bjornpagen/bumbledb-${PUBLISH_PLATFORM}`
		const optional = packed.data.optionalDependencies
		const pin =
			typeof optional === "object" && optional !== null
				? (optional as Record<string, unknown>)[platformName]
				: undefined
		if (pin !== version) {
			throw errors.new(
				`the packed manifest's optionalDependencies["${platformName}"] is ${String(pin)}, expected the exact release version ${version} (scripts/pin.ts injects it at prepack)`
			)
		}
		assertPackedImports(packed.data)
	} finally {
		fs.rmSync(scratch, { recursive: true, force: true })
	}
	const repo = readJson(path.join(packageRoot, "package.json"))
	if ("optionalDependencies" in repo) {
		throw errors.new(
			"the repo package.json still carries optionalDependencies after pack — postpack's restore failed (the committed manifest must stay pin-free)"
		)
	}
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
