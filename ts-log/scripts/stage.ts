import { spawnSync } from "node:child_process"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { fileURLToPath, pathToFileURL } from "node:url"
import { Result } from "effect"
import { BuildInputError, BuildOperationError } from "./errors.ts"

/**
 * Immutable pack staging for `@bjornpagen/bumbledb-log` (chapter 32
 * "Build and publish without mutating the checkout"). The already-built
 * `dist/` and the committed docs are copied into an isolated staging
 * tree, the packed manifest is derived there (repo-only fields dropped;
 * the exact peer handshake asserted), and `pnpm pack` runs inside that
 * tree. No lifecycle hook touches the checkout; interruption at any
 * phase leaves it byte-identical.
 *
 * The exact peer handshake this stage refuses to pack without:
 *  - `peerDependencies.effect === "4.0.0-rc.112"` (chapter 35's one pin),
 *  - `peerDependencies["@bjornpagen/bumbledb"] === <this version>` — the
 *    log package can never silently select a different native
 *    command/runtime contract; both packages resolve the SAME shared
 *    native artifact through the core loader (there is no log addon).
 *
 * CLI: `node scripts/stage.ts --out <dir>` →
 *   `<dir>/bjornpagen-bumbledb-log-<v>.tgz`.
 */

/** The one selected TypeScript peer/dev dependency (chapter 35). */
const EFFECT_PIN = "4.0.0-rc.112"

/** Files staged beside dist/ when present (pnpm also allowlists these). */
const EXTRA_FILES = ["README.md", "LICENSE"] as const

function readJson(file: string): Record<string, unknown> {
	const text = Result.try(() => fs.readFileSync(file, "utf8"))
	if (Result.isFailure(text)) {
		throw new BuildOperationError({ message: `read ${file}`, cause: text.failure })
	}
	const parsed = Result.try(() => JSON.parse(text.success) as Record<string, unknown>)
	if (Result.isFailure(parsed)) {
		throw new BuildOperationError({ message: `parse ${file}`, cause: parsed.failure })
	}
	return parsed.success
}

function record(value: unknown): Record<string, unknown> | undefined {
	return typeof value === "object" && value !== null ? (value as Record<string, unknown>) : undefined
}

/**
 * Derives the manifest a packed log tarball ships: the committed
 * manifest minus repo tooling (`scripts`, `devDependencies` — which
 * carries the workspace `link:../ts` twin — and `packageManager`),
 * with the exact peer handshake asserted.
 */
function packedLogManifest(repoManifest: Record<string, unknown>): Record<string, unknown> {
	const version = repoManifest.version
	if (typeof version !== "string" || version === "") {
		throw new BuildInputError({ message: "ts-log/package.json is missing a string version" })
	}
	const peers = record(repoManifest.peerDependencies)
	if (peers?.effect !== EFFECT_PIN) {
		throw new BuildInputError({
			message: `ts-log peerDependencies.effect is ${String(peers?.effect)}, expected the exact pin ${EFFECT_PIN}`
		})
	}
	if (peers["@bjornpagen/bumbledb"] !== version) {
		throw new BuildInputError({
			message: `ts-log peerDependencies["@bjornpagen/bumbledb"] is ${String(peers["@bjornpagen/bumbledb"])}, expected the exact release version ${version}`
		})
	}
	const dev = record(repoManifest.devDependencies)
	if (dev !== undefined && dev.effect !== undefined && dev.effect !== EFFECT_PIN) {
		throw new BuildInputError({
			message: `ts-log devDependencies.effect is ${String(dev.effect)}, expected the exact pin ${EFFECT_PIN}`
		})
	}
	const staged: Record<string, unknown> = { ...repoManifest }
	delete staged.scripts
	delete staged.devDependencies
	delete staged.packageManager
	return staged
}

function packInto(dir: string, outFile: string): string {
	const pack = spawnSync("pnpm", ["pack", "--out", outFile], { cwd: dir })
	if (pack.error) {
		throw new BuildOperationError({ message: "spawn pnpm pack", cause: pack.error })
	}
	if (pack.status !== 0) {
		throw new BuildInputError({ message: `pnpm pack exited with status ${pack.status}: ${pack.stderr.toString()}` })
	}
	if (!fs.existsSync(outFile)) {
		throw new BuildInputError({ message: `pnpm pack reported success but ${outFile} does not exist` })
	}
	return outFile
}

/** Lists a tarball's package-relative paths. */
function tarballFiles(tarball: string): string[] {
	const listed = spawnSync("tar", ["-tzf", tarball])
	if (listed.error) {
		throw new BuildOperationError({ message: "spawn tar -tzf", cause: listed.error })
	}
	if (listed.status !== 0) {
		throw new BuildInputError({ message: `tar -tzf exited with status ${listed.status}: ${listed.stderr.toString()}` })
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

/** Extracts one file's text from a tarball. */
function tarballFile(tarball: string, entry: string): string {
	const extract = spawnSync("tar", ["-xzOf", tarball, `package/${entry}`])
	if (extract.error) {
		throw new BuildOperationError({ message: "spawn tar -xzOf", cause: extract.error })
	}
	if (extract.status !== 0) {
		throw new BuildInputError({
			message: `tar -xzOf ${entry} exited with status ${extract.status}: ${extract.stderr.toString()}`
		})
	}
	return extract.stdout.toString("utf8")
}

/**
 * Stages the log package into `<stagingDir>/bumbledb-log` and packs it
 * into `<outDir>`. Requires a built `dist/`; never touches the checkout.
 */
function stageLogPackage(packageRoot: string, stagingDir: string, outDir: string): string {
	const repoManifestPath = path.join(packageRoot, "package.json")
	const before = fs.readFileSync(repoManifestPath, "utf8")
	const manifest = packedLogManifest(readJson(repoManifestPath))
	const version = manifest.version as string

	const distDir = path.join(packageRoot, "dist")
	if (!fs.existsSync(path.join(distDir, "index.js")) || !fs.existsSync(path.join(distDir, "index.d.ts"))) {
		throw new BuildInputError({ message: "stage the log package after a build: dist/index.js|d.ts missing" })
	}

	const staged = path.join(stagingDir, "bumbledb-log")
	fs.mkdirSync(staged, { recursive: true })
	fs.cpSync(distDir, path.join(staged, "dist"), { recursive: true })
	for (const extra of EXTRA_FILES) {
		const from = path.join(packageRoot, extra)
		if (fs.existsSync(from)) {
			fs.copyFileSync(from, path.join(staged, extra))
		}
	}
	fs.writeFileSync(path.join(staged, "package.json"), `${JSON.stringify(manifest, null, "\t")}\n`)

	const tarball = path.join(outDir, `bjornpagen-bumbledb-log-${version}.tgz`)
	packInto(staged, tarball)

	const after = fs.readFileSync(repoManifestPath, "utf8")
	if (after !== before) {
		throw new BuildInputError({
			message: "staging mutated the committed ts-log/package.json — immutable staging is broken"
		})
	}
	return tarball
}

function main(): void {
	const packageRoot = fileURLToPath(new URL("..", import.meta.url))
	const args = process.argv.slice(2)
	const outFlag = args.indexOf("--out")
	if (outFlag === -1 || typeof args[outFlag + 1] !== "string") {
		throw new BuildInputError({ message: "stage.ts: --out <dir> is required" })
	}
	const outDir = path.resolve(args[outFlag + 1] as string)
	fs.mkdirSync(outDir, { recursive: true })
	const stagingDir = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-log-stage-"))
	try {
		const tarball = stageLogPackage(packageRoot, stagingDir, outDir)
		console.log(`staged: ${path.basename(tarball)}`)
	} finally {
		fs.rmSync(stagingDir, { recursive: true, force: true })
	}
}

const invokedDirectly = process.argv[1] !== undefined && import.meta.url === pathToFileURL(process.argv[1]).href

if (invokedDirectly) {
	main()
}

export { EFFECT_PIN, packedLogManifest, stageLogPackage, tarballFile, tarballFiles }
