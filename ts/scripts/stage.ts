import { spawnSync } from "node:child_process"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { fileURLToPath, pathToFileURL } from "node:url"
import { Result } from "effect"
import { ScriptError } from "./errors.ts"
import { packedMainManifest, packedPlatformManifest } from "./pin.ts"
import { PUBLISH_PLATFORMS } from "./platform.ts"

/** Repository root (three levels above ts/scripts/). */
const repoRoot = fileURLToPath(new URL("../..", import.meta.url))

/** Pack provenance bound to the same candidate/spec digests as release qualification. */
export type PackProvenance = {
	candidateSourceDigest: string
	specificationRevision: string
	stagedAt: string
	package: string
	version: string
}

function readReleaseDigest(flag: "--candidate-digest" | "--specification-revision"): string {
	const out = spawnSync("node", ["scripts/release-results.mjs", flag], {
		cwd: repoRoot,
		encoding: "utf8"
	})
	if (out.error) {
		throw new ScriptError({ message: `spawn release-results.mjs ${flag}`, cause: out.error })
	}
	if (out.status !== 0) {
		throw new ScriptError({
			message: `release-results.mjs ${flag} exited with status ${out.status}: ${out.stderr}`
		})
	}
	const digest = out.stdout.trim()
	if (!/^[a-f0-9]{64}$/.test(digest)) {
		throw new ScriptError({ message: `release-results.mjs ${flag} returned an invalid digest` })
	}
	return digest
}

export function packProvenance(packageName: string, version: string): PackProvenance {
	return {
		candidateSourceDigest: readReleaseDigest("--candidate-digest"),
		specificationRevision: readReleaseDigest("--specification-revision"),
		stagedAt: new Date().toISOString(),
		package: packageName,
		version
	}
}

function writePackProvenance(stagedDir: string, provenance: PackProvenance): void {
	fs.writeFileSync(path.join(stagedDir, "pack-provenance.json"), `${JSON.stringify(provenance, null, "\t")}\n`)
}

/**
 * Immutable pack staging (docs/reference/packaging.md). Packing copies
 * the already-built outputs and the
 * committed sources into an isolated staging tree, writes the derived
 * packed manifest (exact platform pins injected there, never in the
 * checkout), and runs `pnpm pack` inside that tree. The committed
 * manifests carry no lifecycle pack hooks, so an interruption at any
 * phase leaves the checkout byte-identical and a retry restages from
 * scratch. Release/pre-promotion lanes (`scripts/packed-import.sh`)
 * consume these tarballs; `pnpm publish` publishes the staged tarball,
 * never the live checkout. Local packing is not PKG-07B.
 *
 * CLI: `node scripts/stage.ts --out <dir> [--skip-binary]`
 *   Stages and packs the main package plus every `npm/<target>` platform
 *   package whose binary is present. `--skip-binary` permits platform
 *   packages without a built `bumbledb.node` to be skipped (source-only
 *   hosts); the main tarball is always produced. Exact tarball names:
 *   `bjornpagen-bumbledb-<v>.tgz`, `bjornpagen-bumbledb-<target>-<v>.tgz`.
 */

/** The exact file roster a staged main package contains, beyond dist/src. */
const MAIN_EXTRA_FILES = ["COOKBOOK.md", "LICENSE", "README.md"] as const

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

function writeManifest(file: string, manifest: Record<string, unknown>): void {
	fs.writeFileSync(file, `${JSON.stringify(manifest, null, "\t")}\n`)
}

function copyTree(from: string, to: string): void {
	fs.cpSync(from, to, { recursive: true, verbatimSymlinks: false })
}

/** `pnpm pack` inside `dir`, emitting exactly `outFile`; returns its path. */
function packInto(dir: string, outFile: string): string {
	const pack = spawnSync("pnpm", ["pack", "--out", outFile], { cwd: dir })
	if (pack.error) {
		throw new ScriptError({ message: "spawn pnpm pack", cause: pack.error })
	}
	if (pack.status !== 0) {
		throw new ScriptError({ message: `pnpm pack exited with status ${pack.status}: ${pack.stderr.toString()}` })
	}
	if (!fs.existsSync(outFile)) {
		throw new ScriptError({ message: `pnpm pack reported success but ${outFile} does not exist` })
	}
	return outFile
}

/** Lists a tarball's package-relative paths (`tar -tzf`, `package/` stripped). */
function tarballFiles(tarball: string): string[] {
	const listed = spawnSync("tar", ["-tzf", tarball])
	if (listed.error) {
		throw new ScriptError({ message: "spawn tar -tzf", cause: listed.error })
	}
	if (listed.status !== 0) {
		throw new ScriptError({ message: `tar -tzf exited with status ${listed.status}: ${listed.stderr.toString()}` })
	}
	return listed.stdout
		.toString("utf8")
		.split("\n")
		.flatMap((line) => {
			const trimmed = line.trim()
			if (trimmed === "" || !trimmed.startsWith("package/")) {
				return []
			}
			return [trimmed.slice("package/".length)]
		})
}

/** Extracts one file's bytes from a tarball. */
function tarballFile(tarball: string, entry: string): string {
	const extract = spawnSync("tar", ["-xzOf", tarball, `package/${entry}`])
	if (extract.error) {
		throw new ScriptError({ message: "spawn tar -xzOf", cause: extract.error })
	}
	if (extract.status !== 0) {
		throw new ScriptError({
			message: `tar -xzOf ${entry} exited with status ${extract.status}: ${extract.stderr.toString()}`
		})
	}
	return extract.stdout.toString("utf8")
}

/**
 * Stages the main `@bjornpagen/bumbledb` package into
 * `<stagingDir>/bumbledb` and packs it into `<outDir>`. Requires a built
 * `dist/`; never touches the checkout. Returns the tarball path.
 */
function stageMainPackage(packageRoot: string, stagingDir: string, outDir: string): string {
	const repoManifestPath = path.join(packageRoot, "package.json")
	const before = fs.readFileSync(repoManifestPath, "utf8")
	const manifest = packedMainManifest(readJson(repoManifestPath))
	const version = manifest.version as string

	const distDir = path.join(packageRoot, "dist")
	if (!fs.existsSync(path.join(distDir, "index.js")) || !fs.existsSync(path.join(distDir, "index.d.ts"))) {
		throw new ScriptError({ message: "stage the main package after a build: dist/index.js|d.ts missing" })
	}

	const staged = path.join(stagingDir, "bumbledb")
	fs.mkdirSync(staged, { recursive: true })
	copyTree(distDir, path.join(staged, "dist"))
	copyTree(path.join(packageRoot, "src"), path.join(staged, "src"))
	for (const extra of MAIN_EXTRA_FILES) {
		const from = path.join(packageRoot, extra)
		if (fs.existsSync(from)) {
			fs.copyFileSync(from, path.join(staged, extra))
		}
	}
	writeManifest(path.join(staged, "package.json"), manifest)
	writePackProvenance(staged, packProvenance("@bjornpagen/bumbledb", version))

	const tarball = path.join(outDir, `bjornpagen-bumbledb-${version}.tgz`)
	packInto(staged, tarball)

	const after = fs.readFileSync(repoManifestPath, "utf8")
	if (after !== before) {
		throw new ScriptError({
			message: "staging mutated the committed ts/package.json — immutable staging is broken"
		})
	}
	return tarball
}

/**
 * Stages one committed `npm/<target>` platform package and packs it.
 * The platform tree is already immutable (manifest + LICENSE + binary,
 * no hooks); staging copies it so the pack never runs inside the
 * checkout either. Returns the tarball path, or null when the binary is
 * absent and `skipMissingBinary` permits skipping.
 */
function stagePlatformPackage(
	packageRoot: string,
	platform: string,
	stagingDir: string,
	outDir: string,
	skipMissingBinary: boolean
): string | null {
	const sourceDir = path.join(packageRoot, "npm", platform)
	const manifest = readJson(path.join(sourceDir, "package.json"))
	const version = readJson(path.join(packageRoot, "package.json")).version
	if (typeof version !== "string" || version === "") {
		throw new ScriptError({ message: "ts/package.json is missing a string version" })
	}
	packedPlatformManifest(manifest, platform, version)

	const binary = path.join(sourceDir, "bumbledb.node")
	if (!fs.existsSync(binary)) {
		if (skipMissingBinary) {
			return null
		}
		throw new ScriptError({
			message: `ts/npm/${platform}/bumbledb.node is missing — build the native artifact for ${platform} first (or pass --skip-binary to stage source-only)`
		})
	}

	const staged = path.join(stagingDir, `bumbledb-${platform}`)
	fs.mkdirSync(staged, { recursive: true })
	writeManifest(path.join(staged, "package.json"), manifest)
	writePackProvenance(staged, packProvenance(`@bjornpagen/bumbledb-${platform}`, version))
	fs.copyFileSync(path.join(sourceDir, "LICENSE"), path.join(staged, "LICENSE"))
	fs.copyFileSync(binary, path.join(staged, "bumbledb.node"))

	const tarball = path.join(outDir, `bjornpagen-bumbledb-${platform}-${version}.tgz`)
	packInto(staged, tarball)

	const files = tarballFiles(tarball).toSorted()
	const expected = ["LICENSE", "bumbledb.node", "pack-provenance.json", "package.json"]
	if (JSON.stringify(files) !== JSON.stringify(expected)) {
		throw new ScriptError({
			message: `the ${platform} tarball must contain exactly ${JSON.stringify(expected)}, found ${JSON.stringify(files)}`
		})
	}
	return tarball
}

function main(): void {
	const packageRoot = fileURLToPath(new URL("..", import.meta.url))
	const args = process.argv.slice(2)
	const outFlag = args.indexOf("--out")
	if (outFlag === -1 || typeof args[outFlag + 1] !== "string") {
		throw new ScriptError({ message: "stage.ts: --out <dir> is required" })
	}
	const outDir = path.resolve(args[outFlag + 1] as string)
	const skipBinary = args.includes("--skip-binary")
	fs.mkdirSync(outDir, { recursive: true })
	const stagingDir = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-stage-"))
	try {
		const mainTarball = stageMainPackage(packageRoot, stagingDir, outDir)
		console.log(`staged: ${path.basename(mainTarball)}`)
		for (const platform of PUBLISH_PLATFORMS) {
			const tarball = stagePlatformPackage(packageRoot, platform, stagingDir, outDir, skipBinary)
			if (tarball === null) {
				console.log(`skipped: ${platform} (no built binary)`)
				continue
			}
			console.log(`staged: ${path.basename(tarball)}`)
		}
	} finally {
		fs.rmSync(stagingDir, { recursive: true, force: true })
	}
}

const invokedDirectly = process.argv[1] !== undefined && import.meta.url === pathToFileURL(process.argv[1]).href

if (invokedDirectly) {
	main()
}

export { packInto, packProvenance, readJson, stageMainPackage, stagePlatformPackage, tarballFile, tarballFiles, writeManifest }
