import assert from "node:assert/strict"
import { spawnSync } from "node:child_process"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { test } from "node:test"

const packageRoot = path.join(import.meta.dirname, "..")
const cookbookPath = path.join(packageRoot, "COOKBOOK.md")

const RECIPE_COUNT = 32

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

interface Section {
	readonly heading: string
	readonly body: string
}

/** Splits the document at `##` headings; the preamble (before the first heading) rides heading "". */
function sections(markdown: string): Section[] {
	const parts = markdown.split(/^(## .*)$/m)
	const first = parts[0]
	assert.ok(first !== undefined, "the split always yields a leading chunk")
	const out: Section[] = [{ heading: "", body: first }]
	for (let i = 1; i < parts.length; i += 2) {
		const heading = parts[i]
		const body = parts[i + 1]
		assert.ok(heading !== undefined && body !== undefined, "headings and bodies alternate")
		out.push({ heading, body })
	}
	return out
}

test("every ts fence in COOKBOOK.md type-checks against src/index.ts at HEAD, section by section", function cookbookDocPin() {
	const markdown = fs.readFileSync(cookbookPath, "utf8")
	const parts = sections(markdown)
	const preamble = parts[0]
	assert.ok(preamble !== undefined, "the document has a preamble")
	const imports = tsFences(preamble.body)
	assert.equal(imports.length, 1, "the preamble carries exactly one ts fence — the one-package-entry imports")
	const prelude = imports[0]
	assert.ok(prelude !== undefined, "the imports fence has a body")

	const recipes = parts.filter(function isRecipe(section) {
		return /^## \d+\. /.test(section.heading)
	})
	assert.equal(recipes.length, RECIPE_COUNT, "the cookbook holds all 32 recipes")
	recipes.forEach(function numbered(section, index) {
		assert.ok(
			section.heading.startsWith(`## ${index + 1}. `),
			`recipe numbering follows the roster: ${section.heading}`
		)
		assert.ok(tsFences(section.body).length > 0, `${section.heading} carries at least one ts fence`)
	})

	const projectDir = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-cookbook-doc-"))
	try {
		const files: string[] = []
		parts.forEach(function writeSection(section, index) {
			if (section.heading === "") {
				return
			}
			const fences = tsFences(section.body)
			if (fences.length === 0) {
				return
			}
			const file = path.join(projectDir, `section-${index}.ts`)
			fs.writeFileSync(file, `${prelude}\n${fences.join("\n")}`)
			files.push(file)
		})
		assert.ok(files.length >= RECIPE_COUNT, "every recipe produced a section file")

		const tsconfig = {
			extends: path.join(packageRoot, "tsconfig.json"),
			compilerOptions: {
				paths: { "@bjornpagen/bumbledb": [path.join(packageRoot, "src", "index.ts")] },
				typeRoots: [path.join(packageRoot, "node_modules", "@types")],
				noUnusedLocals: false,
				noUnusedParameters: false
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
			`a COOKBOOK ts fence no longer compiles against the real surface:\n${tsc.stdout}${tsc.stderr}`
		)
	} finally {
		fs.rmSync(projectDir, { recursive: true, force: true })
	}
})
