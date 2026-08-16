/**
 * Published-declaration isolation. A consumer with `skipLibCheck` and
 * `exactOptionalPropertyTypes` must typecheck ONLY `dist/*.d.ts` — never
 * `src/*.ts`. The in-repo `#*.ts` map is a dual: this package's
 * `customConditions: ["bumbledb-src"]` sees sources; everyone else sees
 * declarations. `tsc` leaves `#` specifiers in `.d.ts`; the build rewrites
 * them to a closed relative graph so isolation does not depend on the
 * consumer leaving `bumbledb-src` unset.
 */

import assert from "node:assert/strict"
import { spawnSync } from "node:child_process"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { describe, test } from "node:test"

import {
	assertDeclarationsAreIsolated,
	assertPackedImports,
	PUBLISHED_HASH_IMPORTS,
	relativeFromHash,
	rewriteDeclarationImports
} from "../scripts/declarations.ts"

const packageRoot = path.resolve(import.meta.dirname, "..")
const tscBin = path.join(packageRoot, "node_modules", ".bin", "tsc")

describe("the published #*.ts import map", function suite() {
	test("the committed manifest IS the published isolation contract", function committedMap() {
		const manifest = JSON.parse(fs.readFileSync(path.join(packageRoot, "package.json"), "utf8")) as {
			imports: { "#*.ts": Record<string, string> }
		}
		assert.deepEqual(manifest.imports["#*.ts"], { ...PUBLISHED_HASH_IMPORTS })
		assertPackedImports(manifest)
	})

	test("types resolve to dist, never src", function typesTarget() {
		assert.equal(PUBLISHED_HASH_IMPORTS.types, "./dist/*.d.ts")
		assert.notEqual(PUBLISHED_HASH_IMPORTS.types, "./src/*.ts")
	})
})

describe("declaration specifier rewrite", function suite() {
	test("hash specifiers become relative .js paths from any dist depth", function relatives() {
		const dist = "/pkg/dist"
		assert.equal(relativeFromHash("/pkg/dist/index.d.ts", "#db.ts", dist), "./db.js")
		assert.equal(relativeFromHash("/pkg/dist/db.d.ts", "#query/atom.ts", dist), "./query/atom.js")
		assert.equal(relativeFromHash("/pkg/dist/query/lower.d.ts", "#fields.ts", dist), "../fields.js")
		assert.equal(relativeFromHash("/pkg/dist/query/lower.d.ts", "#query/atom.ts", dist), "./atom.js")
	})

	test("a # specifier that is not a .ts path is refused", function refuse() {
		assert.throws(function bad() {
			relativeFromHash("/pkg/dist/index.d.ts", "#db.js", "/pkg/dist")
		}, /#\*\.ts specifier/)
	})

	test("rewrite closes the declaration graph and the isolation gate holds", function rewrite() {
		const dir = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-dts-"))
		try {
			fs.mkdirSync(path.join(dir, "query"))
			fs.writeFileSync(path.join(dir, "index.d.ts"), 'export type { V } from "#db.ts";\n')
			fs.writeFileSync(path.join(dir, "db.d.ts"), 'import type { X } from "#query/atom.ts";\nexport type V = X;\n')
			fs.writeFileSync(path.join(dir, "query/atom.d.ts"), "export type X = string;\n")
			rewriteDeclarationImports(dir)
			assert.equal(fs.readFileSync(path.join(dir, "index.d.ts"), "utf8"), 'export type { V } from "./db.js";\n')
			assert.equal(
				fs.readFileSync(path.join(dir, "db.d.ts"), "utf8"),
				'import type { X } from "./query/atom.js";\nexport type V = X;\n'
			)
			assertDeclarationsAreIsolated(dir)
		} finally {
			fs.rmSync(dir, { recursive: true, force: true })
		}
	})

	test("a leftover # import fails the isolation gate", function leftover() {
		const dir = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-dts-leak-"))
		try {
			fs.writeFileSync(path.join(dir, "index.d.ts"), 'export type { V } from "#db.ts";\n')
			assert.throws(function leak() {
				assertDeclarationsAreIsolated(dir)
			}, /must not import # specifiers/)
		} finally {
			fs.rmSync(dir, { recursive: true, force: true })
		}
	})
})

describe("a strict downstream consumer", function suite() {
	test("exactOptionalPropertyTypes + skipLibCheck typechecks dist, never src", function consumer() {
		const emitDir = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-emit-"))
		const projectDir = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-consumer-"))
		try {
			const emitted = spawnSync(tscBin, ["-p", "tsconfig.build.json", "--emitDeclarationOnly", "--outDir", emitDir], {
				cwd: packageRoot,
				encoding: "utf8"
			})
			assert.equal(emitted.status, 0, `emit failed:\n${emitted.stdout}${emitted.stderr}`)
			rewriteDeclarationImports(emitDir)
			assertDeclarationsAreIsolated(emitDir)

			const pkg = path.join(projectDir, "node_modules", "@bjornpagen", "bumbledb")
			fs.mkdirSync(pkg, { recursive: true })
			const manifest = JSON.parse(fs.readFileSync(path.join(packageRoot, "package.json"), "utf8")) as Record<
				string,
				unknown
			>
			fs.writeFileSync(path.join(pkg, "package.json"), `${JSON.stringify(manifest, null, "\t")}\n`)
			fs.cpSync(emitDir, path.join(pkg, "dist"), { recursive: true })
			fs.cpSync(path.join(packageRoot, "src"), path.join(pkg, "src"), { recursive: true })

			const useFile = path.join(projectDir, "use.ts")
			fs.writeFileSync(useFile, consumerProgram())
			fs.writeFileSync(
				path.join(projectDir, "tsconfig.json"),
				`${JSON.stringify(
					{
						compilerOptions: {
							strict: true,
							exactOptionalPropertyTypes: true,
							skipLibCheck: true,
							module: "ESNext",
							moduleResolution: "Bundler",
							target: "esnext",
							noEmit: true,
							types: []
						},
						files: [useFile]
					},
					null,
					"\t"
				)}\n`
			)

			const listed = spawnSync(tscBin, ["-p", projectDir, "--listFiles", "--pretty", "false"], {
				encoding: "utf8"
			})
			assert.equal(listed.status, 0, `consumer typecheck failed:\n${listed.stdout}${listed.stderr}`)
			const files = listed.stdout.split("\n").filter(function nonempty(line) {
				return line.length > 0
			})
			const packageFiles = files.filter(function fromPackage(file) {
				return file.includes(`${path.sep}node_modules${path.sep}@bjornpagen${path.sep}bumbledb${path.sep}`)
			})
			assert.ok(
				packageFiles.some(function hasIndex(file) {
					return file.endsWith(`${path.sep}dist${path.sep}index.d.ts`)
				}),
				"the consumer must load dist/index.d.ts"
			)
			assert.ok(
				packageFiles.some(function hasDb(file) {
					return file.endsWith(`${path.sep}dist${path.sep}db.d.ts`)
				}),
				"the consumer must load dist/db.d.ts"
			)
			const leakedSrc = packageFiles.filter(function src(file) {
				return file.includes(`${path.sep}src${path.sep}`) && file.endsWith(".ts") && !file.endsWith(".d.ts")
			})
			assert.deepEqual(leakedSrc, [], "the consumer must not typecheck implementation sources")
		} finally {
			fs.rmSync(emitDir, { recursive: true, force: true })
			fs.rmSync(projectDir, { recursive: true, force: true })
		}
	})
})

/** A consumer program that constructs every Violation arm under exactOptionalPropertyTypes. */
function consumerProgram(): string {
	return `import type {
	CapacityViolation,
	ContainmentViolation,
	DeclaredKeyViolation,
	ImpliedKeyViolation,
	InsertFact,
	MirrorViolation,
	Violation
} from "@bjornpagen/bumbledb"
import { contained, key, on, relation, schema, str, u64 } from "@bjornpagen/bumbledb"

const Holder = relation("Holder", { id: u64.fresh, name: str })
const Account = relation("Account", { id: u64.fresh, holder: u64 })
const Terms = relation("Terms", { account: u64, rate: u64 })
const termsKey = key(Terms, ["account"])
const holderOf = contained(on(Account, "holder"), on(Holder, "id"))
const Theory = schema("T", { Holder, Account, Terms }, [termsKey, holderOf])
type Rels = (typeof Theory)["relations"]

const implied: ImpliedKeyViolation<Rels> = {
	kind: "functionality",
	statement: undefined,
	canonical: "Holder(id) -> Holder",
	facts: []
}
const declared: DeclaredKeyViolation<Rels> = {
	kind: "functionality",
	statement: termsKey,
	canonical: "Terms(account) -> Terms",
	facts: []
}
const containment: ContainmentViolation<Rels> = {
	kind: "containment",
	statement: holderOf,
	canonical: "Account(holder) <= Holder(id)",
	direction: "sourceUnsatisfied",
	facts: []
}
const mirrored: MirrorViolation<Rels> = {
	kind: "containment",
	statement: holderOf,
	canonical: "Account(holder) == Holder(id)",
	direction: "targetRequired",
	orientation: "written",
	facts: []
}

const violations: readonly Violation<Rels>[] = [implied, declared, containment, mirrored]
export const statements = violations.map(function statementOf(violation: Violation<Rels>) {
	return violation.statement
})

function impliedStatement(violation: ImpliedKeyViolation<Rels>): undefined {
	return violation.statement
}
function declaredStatement(violation: DeclaredKeyViolation<Rels> | ContainmentViolation<Rels> | CapacityViolation<Rels>) {
	return violation.statement
}
impliedStatement(implied)
declaredStatement(declared)
declaredStatement(containment)

const insert: InsertFact<typeof Holder> = { name: "Ada", id: undefined }
export const inserted = insert

function containmentRejectsUndefined(): Violation<Rels> {
	// @ts-expect-error — declared containments always carry the SDK statement
	return {
		kind: "containment",
		statement: undefined,
		canonical: "",
		direction: "sourceUnsatisfied",
		facts: []
	}
}
export type Reject = ReturnType<typeof containmentRejectsUndefined>
`
}
