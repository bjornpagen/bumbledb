import assert from "node:assert/strict"
import { spawnSync } from "node:child_process"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { test } from "node:test"

const packageRoot = path.join(import.meta.dirname, "..")
const readmePaths = [path.join(packageRoot, "README.md"), path.join(packageRoot, "..", "README.md")]

function tsFences(markdown: string): string[] {
	const fences: string[] = []
	const pattern = /^```ts\n([\s\S]*?)^```$/gm
	for (const matched of markdown.matchAll(pattern)) {
		const body = matched[1]
		assert.ok(body !== undefined, "a matched fence carries its captured body")
		fences.push(body)
	}
	return fences
}

test("every ts fence in the package and repository READMEs type-checks against src/index.ts", function readmePin() {
	const fences = readmePaths.flatMap(function readmeFences(readmePath) {
		const found = tsFences(fs.readFileSync(readmePath, "utf8"))
		assert.ok(found.length > 0, `${readmePath} carries at least one ts fence`)
		return found
	})

	const projectDir = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-readme-"))
	try {
		fs.symlinkSync(path.join(packageRoot, "node_modules"), path.join(projectDir, "node_modules"), "dir")
		const files = fences.map(function writeFence(body, index) {
			const file = path.join(projectDir, `fence-${index}.ts`)
			fs.writeFileSync(file, body)
			return file
		})

		const tsconfig = {
			extends: path.join(packageRoot, "tsconfig.json"),
			compilerOptions: {
				paths: {
					"@bjornpagen/bumbledb": [path.join(packageRoot, "src", "index.ts")],
					"@bjornpagen/bumbledb/engine": [path.join(packageRoot, "src", "engine.ts")]
				},
				typeRoots: [path.join(packageRoot, "node_modules", "@types")]
			},
			include: [],
			files
		}
		fs.writeFileSync(path.join(projectDir, "tsconfig.json"), JSON.stringify(tsconfig, null, "\t"))

		const tsc = spawnSync(path.join(packageRoot, "node_modules", ".bin", "tsc"), ["-p", projectDir], {
			encoding: "utf8"
		})
		assert.equal(tsc.error, undefined, `spawn tsc: ${String(tsc.error)}`)
		assert.equal(
			tsc.status,
			0,
			`a README ts fence no longer compiles against the real surface:\n${tsc.stdout}${tsc.stderr}`
		)
	} finally {
		fs.rmSync(projectDir, { recursive: true, force: true })
	}
})
