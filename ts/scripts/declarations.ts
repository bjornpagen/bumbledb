import * as fs from "node:fs"
import * as path from "node:path"
import * as errors from "@superbuilders/errors"

/**
 * Published-declaration isolation, extracted from `build.ts` so the
 * rewrite and the packed-imports contract are units the test suite pins
 * without executing the build (`test/declaration-isolation.test.ts`).
 *
 * `tsc` preserves in-repo `#foo.ts` specifiers (`rewriteRelativeImportExtensions`
 * only rewrites RELATIVE imports). A `#` specifier in a `.d.ts` is resolved
 * through `package.json#imports`. Pointing that `types` condition at
 * `src/*.ts` made every consumer typecheck the implementation — `skipLibCheck`
 * skips `.d.ts`, not `.ts`. The published contract is therefore:
 *
 * - `imports["#*.ts"].types` → `./dist/*.d.ts`
 * - `imports["#*.ts"].default` → `./dist/*.js`
 * - `imports["#*.ts"].bumbledb-src` → `./src/*.ts` (this repo's
 *   `customConditions` only)
 * - every emitted `.d.ts` is rewritten to a relative `.js` specifier, so
 *   the published type graph does not consult `imports` at all
 */

/** The packed `imports["#*.ts"]` map — key order is the TypeScript condition order. */
const PUBLISHED_HASH_IMPORTS = {
	"bumbledb-src": "./src/*.ts",
	types: "./dist/*.d.ts",
	default: "./dist/*.js"
} as const

/**
 * `#query/atom.ts` from `dist/db.d.ts` → `./query/atom.js`. TypeScript's
 * published-ESM convention: the specifier names the `.js`; the `.d.ts`
 * sits next to it.
 */
function relativeFromHash(fromFile: string, hashSpecifier: string, distDir: string): string {
	if (!hashSpecifier.startsWith("#") || !hashSpecifier.endsWith(".ts")) {
		throw errors.new(`declaration rewrite expected a #*.ts specifier, got ${hashSpecifier}`)
	}
	const targetJs = path.join(distDir, hashSpecifier.slice(1).replace(/\.ts$/, ".js"))
	const relative = path.relative(path.dirname(fromFile), targetJs).split(path.sep).join("/")
	return relative.startsWith(".") ? relative : `./${relative}`
}

/** Rewrites every `from "#foo.ts"` in emitted `.d.ts` files to a relative `.js` specifier. */
function rewriteDeclarationImports(distDir: string): void {
	for (const file of declarationFiles(distDir)) {
		const source = fs.readFileSync(file, "utf8")
		const rewritten = source.replace(/from "(#[^"]+\.ts)"/g, function relative(_match, specifier: string) {
			return `from "${relativeFromHash(file, specifier, distDir)}"`
		})
		if (rewritten !== source) {
			fs.writeFileSync(file, rewritten)
		}
	}
}

/** Fails if any emitted declaration still mentions a `#` specifier. */
function assertDeclarationsAreIsolated(distDir: string): void {
	const leaked: string[] = []
	for (const file of declarationFiles(distDir)) {
		if (fs.readFileSync(file, "utf8").includes('from "#')) {
			leaked.push(path.relative(distDir, file))
		}
	}
	if (leaked.length > 0) {
		throw errors.new(
			`published declarations must not import # specifiers (consumers would resolve them through package.json imports); leaked: ${leaked.join(", ")}`
		)
	}
}

/**
 * The packed manifest's `#*.ts` map is the isolation contract: consumers
 * resolve types to `dist/*.d.ts`, Node resolves runtime to `dist/*.js`,
 * and only this repo's `customConditions: ["bumbledb-src"]` sees `src`.
 */
function assertPackedImports(packed: Record<string, unknown>): void {
	const imports = packed.imports
	if (typeof imports !== "object" || imports === null) {
		throw errors.new("the packed manifest is missing imports")
	}
	const hash = (imports as Record<string, unknown>)["#*.ts"]
	if (typeof hash !== "object" || hash === null) {
		throw errors.new('the packed manifest is missing imports["#*.ts"]')
	}
	const conditions = hash as Record<string, unknown>
	for (const [key, value] of Object.entries(PUBLISHED_HASH_IMPORTS)) {
		if (conditions[key] !== value) {
			throw errors.new(
				`the packed imports["#*.ts"].${key} is ${String(conditions[key])}, expected ${value} (published types must not resolve to src)`
			)
		}
	}
}

function declarationFiles(distDir: string): string[] {
	const files: string[] = []
	function walk(dir: string): void {
		for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
			const next = path.join(dir, entry.name)
			if (entry.isDirectory()) {
				walk(next)
				continue
			}
			if (entry.name.endsWith(".d.ts") && !entry.name.endsWith(".d.ts.map")) {
				files.push(next)
			}
		}
	}
	walk(distDir)
	return files
}

export {
	assertDeclarationsAreIsolated,
	assertPackedImports,
	PUBLISHED_HASH_IMPORTS,
	relativeFromHash,
	rewriteDeclarationImports
}
