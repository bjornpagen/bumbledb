/**
 * The README's compile pin (PRD-T3). `ts/README.md` is the npm-facing
 * document and its examples once rotted all the way back to a deleted API —
 * this test is what makes that impossible again. Every ```ts fence is
 * extracted from the README MECHANICALLY at test time (read the file, slice
 * the fences — never a hand-maintained copy), written to a throwaway
 * project whose tsconfig EXTENDS the package's own (identical strictness),
 * with the bare `@bjornpagen/bumbledb` specifier mapped onto `src/index.ts`
 * at HEAD, and type-checked by the package's own `tsc`. An edit to the
 * README's code that stops compiling against the real surface fails
 * `node --test`; so does a README whose fences vanish entirely.
 */

import assert from "node:assert/strict"
import { spawnSync } from "node:child_process"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { test } from "node:test"

const packageRoot = path.join(import.meta.dirname, "..")
const readmePath = path.join(packageRoot, "README.md")

/**
 * Every ```ts fence body of a markdown document, in document order — the
 * opener must be exactly ```ts on its own line (```tsx or an indented fence
 * would be a different claim), the closer exactly ``` on its own line.
 */
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

test("every ts fence in README.md type-checks against src/index.ts at HEAD", function readmePin() {
	const markdown = fs.readFileSync(readmePath, "utf8")
	const fences = tsFences(markdown)
	assert.ok(fences.length > 0, "the README carries at least one ts fence — the pin has something to hold")

	const projectDir = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-readme-"))
	try {
		const files = fences.map(function writeFence(body, index) {
			const file = path.join(projectDir, `fence-${index}.ts`)
			fs.writeFileSync(file, body)
			return file
		})
		// The package's own tsconfig, extended verbatim (same strictness, same
		// resolution), with exactly two additions: the README's bare package
		// specifier resolves to the source entry at HEAD, and the type roots
		// still point at the package's node_modules (the temp dir has none).
		const tsconfig = {
			extends: path.join(packageRoot, "tsconfig.json"),
			compilerOptions: {
				paths: { "@bjornpagen/bumbledb": [path.join(packageRoot, "src", "index.ts")] },
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
