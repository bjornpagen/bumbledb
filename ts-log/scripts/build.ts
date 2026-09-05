import { spawnSync } from "node:child_process"
import * as fs from "node:fs"
import * as path from "node:path"
import { fileURLToPath } from "node:url"
import * as errors from "@superbuilders/errors"
import { assertDeclarationsAreIsolated, assertPackedImports, rewriteDeclarationImports } from "./declarations.ts"

function build(): void {
	const packageRoot = fileURLToPath(new URL("..", import.meta.url))
	const distDir = path.join(packageRoot, "dist")

	fs.rmSync(distDir, { recursive: true, force: true })

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
	verifyPack(packageRoot)
}

function verifyPack(packageRoot: string): void {
	const files = packDryRun(packageRoot)
	if (!files.includes("package.json")) {
		throw errors.new("package tarball is missing package.json")
	}
	if (!files.includes("dist/index.js")) {
		throw errors.new("package tarball is missing dist/index.js")
	}
	if (!files.includes("dist/index.d.ts")) {
		throw errors.new("package tarball is missing dist/index.d.ts")
	}
	const leakedSrc = files.filter((file) => file.startsWith("src/"))
	if (leakedSrc.length > 0) {
		throw errors.new(`package tarball must not carry src/, found ${leakedSrc.join(", ")}`)
	}

	const manifest = readJson(path.join(packageRoot, "package.json"))
	assertPackedImports(manifest)
	const exports = manifest.exports
	if (typeof exports !== "object" || exports === null) {
		throw errors.new("package.json is missing exports")
	}
	const root = (exports as Record<string, unknown>)["."]
	if (typeof root !== "object" || root === null) {
		throw errors.new('package.json is missing exports["."]')
	}
	const entry = root as Record<string, unknown>
	if (entry.types !== "./dist/index.d.ts" || entry.default !== "./dist/index.js") {
		throw errors.new('exports["."] must point types at dist/index.d.ts and default at dist/index.js')
	}

	console.log("bumbledb-log build: tarball carries dist JS and declarations")
}

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

build()
