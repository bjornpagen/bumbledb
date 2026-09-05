import { spawnSync } from "node:child_process"
import * as fs from "node:fs"
import * as path from "node:path"
import { fileURLToPath } from "node:url"
import { Result } from "effect"
import { assertDeclarationsAreIsolated, assertPackedImports, rewriteDeclarationImports } from "./declarations.ts"
import { BuildInputError, BuildOperationError } from "./errors.ts"

function build(): void {
	const packageRoot = fileURLToPath(new URL("..", import.meta.url))
	const distDir = path.join(packageRoot, "dist")
	fs.rmSync(distDir, { recursive: true, force: true })
	const tsc = spawnSync("tsc", ["-p", "tsconfig.build.json"], {
		stdio: "inherit",
		cwd: packageRoot
	})
	if (tsc.error) {
		throw new BuildOperationError({ message: "spawn tsc", cause: tsc.error })
	}
	if (tsc.status !== 0) {
		throw new BuildInputError({ message: `tsc exited with status ${tsc.status}` })
	}
	rewriteDeclarationImports(distDir)
	assertDeclarationsAreIsolated(distDir)
	verifyPack(packageRoot)
}
function verifyPack(packageRoot: string): void {
	const files = packDryRun(packageRoot)
	if (!files.includes("package.json")) {
		throw new BuildInputError({ message: "package tarball is missing package.json" })
	}
	if (!files.includes("dist/index.js")) {
		throw new BuildInputError({ message: "package tarball is missing dist/index.js" })
	}
	if (!files.includes("dist/index.d.ts")) {
		throw new BuildInputError({ message: "package tarball is missing dist/index.d.ts" })
	}
	const leakedSrc = files.filter((file) => file.startsWith("src/"))
	if (leakedSrc.length > 0) {
		throw new BuildInputError({ message: `package tarball must not carry src/, found ${leakedSrc.join(", ")}` })
	}
	const manifest = readJson(path.join(packageRoot, "package.json"))
	assertPackedImports(manifest)
	const exports = manifest.exports
	if (typeof exports !== "object" || exports === null) {
		throw new BuildInputError({ message: "package.json is missing exports" })
	}
	const root = (exports as Record<string, unknown>)["."]
	if (typeof root !== "object" || root === null) {
		throw new BuildInputError({ message: 'package.json is missing exports["."]' })
	}
	const entry = root as Record<string, unknown>
	if (entry.types !== "./dist/index.d.ts" || entry.default !== "./dist/index.js") {
		throw new BuildInputError({
			message: 'exports["."] must point types at dist/index.d.ts and default at dist/index.js'
		})
	}
	console.log("bumbledb-log build: tarball carries dist JS and declarations")
}
function packDryRun(dir: string): string[] {
	const result = spawnSync("pnpm", ["pack", "--dry-run", "--json"], { cwd: dir })
	if (result.error) {
		throw new BuildOperationError({ message: "spawn pnpm pack", cause: result.error })
	}
	if (result.status !== 0) {
		throw new BuildInputError({ message: `pnpm pack exited with status ${result.status}: ${result.stderr.toString()}` })
	}
	const parsed = Result.try(
		() =>
			JSON.parse(result.stdout.toString()) as {
				files: ReadonlyArray<{
					path: string
				}>
			}
	)
	if (Result.isFailure(parsed)) {
		throw new BuildOperationError({ message: "parse pnpm pack --json output", cause: parsed.failure })
	}
	return parsed.success.files.map((file) => file.path)
}
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
build()
