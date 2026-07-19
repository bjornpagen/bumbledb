/**
 * The cookbook DOCUMENT's compile pin — the TS twin of the Rust cookbook's
 * `doc_blocks_match_the_compiled_copies` sync test, in this host's idiom
 * (no macro exists to stringify compiling tokens, so the pin runs the
 * other direction: the document's own code is extracted and compiled).
 * Every ```ts fence of `ts/COOKBOOK.md` is sliced out MECHANICALLY at test
 * time — never a hand-maintained copy — grouped by its `##` section (a
 * recipe's fences share one scope: recipe 24's host loop reads the query
 * its first fence declared), prefixed with the document's own preamble
 * imports fence ("everything below imports from the one package entry"),
 * and type-checked against `src/index.ts` at HEAD with the package's own
 * `tsc`. Unused-declaration checking is OFF for these throwaway projects —
 * a recipe declares values for the READER (queries it never executes), so
 * unused-ness is the cookbook's nature, not drift; every other strictness
 * flag is the package tsconfig's own. An edit to COOKBOOK.md whose code
 * stops compiling against the real surface fails `node --test`; so does a
 * cookbook whose recipe roster or fences vanish. The runtime half of the
 * cookbook's claim — admission on a real store, cross-host fingerprints,
 * every query prepared — is `test/cookbook.test.ts`'s, over the compiled
 * copies.
 */

import assert from "node:assert/strict"
import { spawnSync } from "node:child_process"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { test } from "node:test"

const packageRoot = path.join(import.meta.dirname, "..")
const cookbookPath = path.join(packageRoot, "COOKBOOK.md")

/** The cookbook's recipe count — the roster the Rust twin also pins. */
const RECIPE_COUNT = 29

/**
 * Every ```ts fence body of a markdown chunk, in document order — the
 * opener must be exactly ```ts on its own line, the closer exactly ``` on
 * its own line (the readme pin's discipline).
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

/** One `##` section of the document: its heading line and its body up to the next heading. */
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

	// The recipe roster: 1..29 in order, every recipe carrying at least one fence.
	const recipes = parts.filter(function isRecipe(section) {
		return /^## \d+\. /.test(section.heading)
	})
	assert.equal(recipes.length, RECIPE_COUNT, "the cookbook holds all 29 recipes")
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
		// The package's own tsconfig, extended verbatim, with the readme pin's
		// two additions (the bare specifier resolves to src at HEAD; type roots
		// stay the package's) and ONE deliberate relaxation: unused-declaration
		// checking off — a recipe declares values for the reader.
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
