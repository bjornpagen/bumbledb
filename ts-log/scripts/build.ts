import { spawnSync } from "node:child_process"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { fileURLToPath } from "node:url"
import { Result } from "effect"
import { assertDeclarationsAreIsolated, assertPackedImports, rewriteDeclarationImports } from "./declarations.ts"
import { BuildInputError, BuildOperationError } from "./errors.ts"
import { stageLogPackage, tarballFile, tarballFiles } from "./stage.ts"

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

/**
 * The tarball proof over IMMUTABLE STAGING: pack the staged tree for
 * real in a scratch dir and assert the shipped shape on the actual
 * tarball — dist entry points and the migrations CLI present, no src/
 * leak, the packed manifest's exports/bin/imports map exact, no repo
 * tooling fields, and the committed manifest untouched (asserted again
 * inside stageLogPackage). The peer handshake (exact Effect RC, exact
 * same-version core peer) is refused at staging time.
 */
function verifyPack(packageRoot: string): void {
	const scratch = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-log-pack-"))
	try {
		const staging = path.join(scratch, "staging")
		fs.mkdirSync(staging, { recursive: true })
		const tarball = stageLogPackage(packageRoot, staging, scratch)

		const files = tarballFiles(tarball)
		for (const required of ["package.json", "dist/index.js", "dist/index.d.ts", "dist/schema.js", "dist/migrations/index.js", "dist/migrations/bin.js"]) {
			if (!files.includes(required)) {
				throw new BuildInputError({ message: `package tarball is missing ${required}` })
			}
		}
		const leakedSrc = files.filter((file) => file.startsWith("src/"))
		if (leakedSrc.length > 0) {
			throw new BuildInputError({ message: `package tarball must not carry src/, found ${leakedSrc.join(", ")}` })
		}

		const packed = Result.try(() => JSON.parse(tarballFile(tarball, "package.json")) as Record<string, unknown>)
		if (Result.isFailure(packed)) {
			throw new BuildOperationError({ message: "parse the packed package.json", cause: packed.failure })
		}
		if ("scripts" in packed.success || "devDependencies" in packed.success) {
			throw new BuildInputError({
				message: "the packed manifest must not carry scripts or devDependencies (repo tooling never ships)"
			})
		}
		assertPackedImports(packed.success)
		const exports = packed.success.exports
		if (typeof exports !== "object" || exports === null) {
			throw new BuildInputError({ message: "package.json is missing exports" })
		}
		const table = exports as Record<string, unknown>
		assertEntry(table, ".", "./dist/index.d.ts", "./dist/index.js")
		assertEntry(table, "./schema", "./dist/schema.d.ts", "./dist/schema.js")
		assertEntry(table, "./migrations", "./dist/migrations/index.d.ts", "./dist/migrations/index.js")
		const bin = packed.success.bin
		if (typeof bin !== "object" || bin === null || (bin as Record<string, unknown>)["bumbledb-log"] !== "./dist/migrations/bin.js") {
			throw new BuildInputError({ message: 'package.json bin must map "bumbledb-log" to ./dist/migrations/bin.js' })
		}
	} finally {
		fs.rmSync(scratch, { recursive: true, force: true })
	}
	console.log("bumbledb-log build: staged tarball carries dist JS, declarations, subpaths and the CLI; checkout untouched")
}

function assertEntry(exports: Record<string, unknown>, entry: string, types: string, main: string): void {
	const value = exports[entry]
	if (typeof value !== "object" || value === null) {
		throw new BuildInputError({ message: `package.json is missing exports["${entry}"]` })
	}
	const record = value as Record<string, unknown>
	if (record.types !== types || record.default !== main) {
		throw new BuildInputError({
			message: `exports["${entry}"] must point types at ${types} and default at ${main}`
		})
	}
}

build()
