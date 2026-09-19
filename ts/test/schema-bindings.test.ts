import assert from "node:assert/strict"
import { execFileSync } from "node:child_process"
import * as fs from "node:fs"
import * as path from "node:path"
import { test } from "node:test"
import { fileURLToPath, pathToFileURL } from "node:url"
import { Effect, ManagedRuntime } from "effect"
import * as db from "#index.ts"
import { lower } from "#lower.ts"
import { internalSchemaBindings, internalSchemaSnapshot } from "#schema-file.ts"
import { runtimeOptions } from "#test/fixtures/learning.ts"

const packageRoot = fileURLToPath(new URL("..", import.meta.url))

function checkSource(directory: string) {
	fs.writeFileSync(
		path.join(directory, "tsconfig.json"),
		JSON.stringify({
			compilerOptions: {
				noEmit: false,
				outDir: "compiled",
				rewriteRelativeImportExtensions: true,
				strict: true,
				noUnusedLocals: true,
				noUncheckedIndexedAccess: true,
				exactOptionalPropertyTypes: true,
				module: "ESNext",
				moduleResolution: "Bundler",
				target: "ESNext",
				skipLibCheck: true,
				allowImportingTsExtensions: true
			},
			include: ["*.ts"]
		})
	)
	execFileSync(path.join(packageRoot, "node_modules/.bin/tsc"), ["-p", directory], {
		cwd: packageRoot,
		stdio: "pipe",
		encoding: "utf8"
	})
}

async function roundtrip(theory: db.AnySchema | string, pins = "") {
	const runtime = ManagedRuntime.make(db.NativeRuntime.layer(runtimeOptions))
	const directory = fs.mkdtempSync(path.join(packageRoot, "node_modules/.schema-bindings-"))
	try {
		fs.mkdirSync(path.join(directory, "node_modules/@bjornpagen"), { recursive: true })
		fs.symlinkSync(packageRoot, path.join(directory, "node_modules/@bjornpagen/bumbledb"), "dir")
		fs.writeFileSync(path.join(directory, "package.json"), JSON.stringify({ type: "module" }))
		const original = typeof theory === "string" ? undefined : await runtime.runPromise(db.Schema.compile(theory))
		const snapshot =
			typeof theory === "string" ? theory : await runtime.runPromise(internalSchemaSnapshot(lower(theory)))
		const source = await runtime.runPromise(internalSchemaBindings(snapshot))
		assert.equal(await runtime.runPromise(internalSchemaBindings(snapshot)), source, "deterministic emission")
		const file = path.join(directory, "bindings.ts")
		fs.writeFileSync(file, source)
		if (pins) fs.writeFileSync(path.join(directory, "pins.ts"), pins)
		checkSource(directory)
		const generated = await import(pathToFileURL(path.join(directory, "compiled/bindings.js")).href)
		const compiled = await runtime.runPromise(db.Schema.compile(generated.schema))
		if (original !== undefined) {
			assert.equal(compiled.schemaId, original.schemaId)
			assert.deepEqual(compiled.descriptor.relations, original.descriptor.relations)
			assert.deepEqual(compiled.descriptor.statements, original.descriptor.statements)
		}
		assert.deepEqual(
			JSON.parse(await runtime.runPromise(internalSchemaSnapshot(lower(generated.schema)))),
			JSON.parse(snapshot)
		)
		return source
	} finally {
		await runtime.dispose()
		fs.rmSync(directory, { recursive: true, force: true })
	}
}

test("snapshot bindings preserve owned Event field types and dependency identity", async () => {
	const Regions = db.relation("Regions", { id: db.u64, value: db.event })
	const theory = db.schema("Events", { Regions }, [db.key(Regions, ["id"])])
	await roundtrip(
		theory,
		`import type { Event, Fact } from "@bjornpagen/bumbledb"
import { r0 } from "./bindings.ts"
declare const row: Fact<typeof r0>
const value: Event = row.value
// @ts-expect-error Event is an owned opaque value, not raw bytes.
const invalid: Fact<typeof r0>["value"] = new Uint8Array()
void [value, invalid]
`
	)
})

test("snapshot bindings preserve contextual full projections and explicit closed full keys", async () => {
	const Roster = db.relation("Roster", { group: db.u64 })
	const Branch = db.relation("Branch", { group: db.u64, when: db.event })
	const Config = db.relation("Config", { true: db.bool, value: db.u64 })
	const Codes = db.closed("Codes", ["One"], { code: db.u64 }, { One: { code: 1n } })
	const source = await roundtrip(
		db.schema("Full", { Roster, Branch, Config, Codes }, [
			db.key(Roster, ["group", true]),
			db.key(Branch, ["group", "when"]),
			db.key(Config, [true]),
			db.key(Config, ["true"]),
			db.key(Codes, ["code", true]),
			db.mirrors(db.on(Roster, ["group", true]), db.on(Branch, ["group", "when"])),
			db.contained(db.on(Roster, ["group", true]), db.on(Codes, ["code", true]))
		])
	)
	assert.match(source, /db\.key\(r0, \["group", true\]\)/)
	assert.match(source, /db\.key\(r2, \[true\]\)/)
	assert.match(source, /db\.key\(r2, \["true"\]\)/)
	assert.match(source, /db\.key\(r3, \["code", true\]\)/)
})

test("snapshot bindings preserve every field kind, exact payload values, names and field order", async () => {
	const columns = {
		unsigned: db.u64,
		signed: db.i64,
		float: db.f64,
		text: db.str,
		boolean: db.bool,
		uuid: db.uuid,
		bytes: db.bytes(3),
		u: db.interval(db.u64),
		i: db.interval(db.i64),
		f: db.interval(db.f64),
		fu: db.interval(db.u64, 3n),
		fi: db.interval(db.i64, 4n)
	}
	const payload = {
		unsigned: (1n << 64n) - 1n,
		signed: -(1n << 63n),
		float: Number.NaN,
		// biome-ignore lint/suspicious/noTemplateCurlyInString: emission must preserve literal interpolation syntax
		text: 'quote" slash\\ newline\n 😀 ${neverExecuted}',
		boolean: true,
		uuid: "abcdefab-1234-4321-9876-abcdefabcdef" as const,
		bytes: new Uint8Array([0, 128, 255]),
		u: { start: 1n, end: 4n },
		i: { start: -8n, end: 7n },
		f: { start: Number.NEGATIVE_INFINITY, end: Number.POSITIVE_INFINITY },
		fu: { start: 7n, end: 10n },
		fi: { start: -3n, end: 1n }
	}
	const { text: _text, ...closedColumns } = columns
	const { text: _textValue, ...values } = payload
	void [_text, _textValue]
	const closedPayload = { ...values, f: { start: -1.25, end: 3.5 } }
	const Catalog = db.closed("weird-name", ["2", "1", "a.b", "__proto__", '😀"\n'], closedColumns, {
		"2": closedPayload,
		"1": { ...closedPayload, float: Number.MIN_VALUE },
		"a.b": { ...closedPayload, float: Number.MAX_VALUE },
		["__proto__"]: { ...closedPayload, float: -0 },
		'😀"\n': { ...closedPayload, float: Number.NEGATIVE_INFINITY }
	})
	const Rows = db.relation("__proto__", { constructor: db.u64, ["__proto__"]: db.closedId(Catalog), ...columns })
	const theory = db.schema("Awkward", { ["__proto__"]: Rows, "weird-name": Catalog }, [
		db.key(Rows, ["constructor"]),
		db.contained(db.on(Rows, "__proto__"), db.on(Catalog, "id"))
	])
	await roundtrip(
		theory,
		`import type { Fact } from "@bjornpagen/bumbledb"
import { r0 } from "./bindings.ts"
declare const row: Fact<typeof r0>
const handle: ${Catalog.handles.map((handle) => JSON.stringify(handle)).join(" | ")} = row.__proto__
const amount: bigint = row.signed
// @ts-expect-error signed integer values are bigint, not number
const wrong: number = row.signed
// @ts-expect-error handles are a closed roster
const invalid: Fact<typeof r0>["__proto__"] = "missing"
void [handle, amount, wrong, invalid]
`
	)
})

test("independent bindings preserve selected mirror directions, capacities and expanded statement IDs", async () => {
	const Kind = db.closed("Kind", ["Imported", "Electronic", "Postal"])
	const Parent = db.relation("Parent", { id: db.u64, kind: db.closedId(Kind), budget: db.u64 })
	const Child = db.relation("Child", { parent: db.u64, bytes: db.u64 })
	const parent = db.key(Parent, ["id"])
	const child = db.key(Child, ["parent"])
	await roundtrip(
		db.schema("Selected", { Kind, Parent, Child }, [
			parent,
			child,
			db.contained(db.on(Parent, "kind"), db.on(Kind, "id")),
			db.mirrors(db.on(db.select(Parent, { kind: ["Imported", "Postal"] }), "id"), db.on(Child, "parent")),
			db.capacity(db.on(Parent, "id"), {
				from: db.on(Child, "parent"),
				within: db.within(0n, db.ref("budget")),
				weight: db.weigh("bytes")
			})
		])
	)
})

test("emission refuses unrepresentable conditional roster pairing and invalid declaration names", async () => {
	const runtime = ManagedRuntime.make(db.NativeRuntime.layer(runtimeOptions))
	try {
		const snapshot = JSON.stringify({
			relations: [
				{ name: "Kind", fields: [], extension: [{ handle: "Only", values: [] }] },
				{
					name: "Row",
					fields: [
						{ name: "kind", type: "u64" },
						{ name: "enabled", type: "bool" }
					]
				}
			],
			statements: [
				{
					containment: {
						source: { relation: 1, projection: [0], selection: [[1, [{ bool: true }]]] },
						target: { relation: 0, projection: [0] }
					}
				}
			]
		})
		const failure = await runtime.runPromise(Effect.flip(internalSchemaBindings(snapshot)))
		assert.equal(failure.reason._tag, "Engine")
		if (failure.reason._tag === "Engine")
			assert.match(failure.reason.message, /Row.kind.*Kind.id.*conditional membership/)
		for (const name of ["0", "123", "a.b"]) {
			const input = JSON.stringify({ relations: [{ name, fields: [{ name: "value", type: "u64" }] }], statements: [] })
			const refused = await runtime.runPromise(Effect.flip(internalSchemaBindings(input)))
			assert.equal(refused.reason._tag, "Engine")
			if (refused.reason._tag === "Engine") assert.match(refused.reason.message, /SDK declaration names/)
		}
	} finally {
		await runtime.dispose()
	}
})

test("bindings derive transitive total roster membership without losing or adding laws", async () => {
	const source = await roundtrip(
		JSON.stringify({
			relations: [
				{ name: "Kind", fields: [], extension: [{ handle: "Only", values: [] }] },
				{ name: "Relay", fields: [{ name: "kind", type: "u64" }] },
				{ name: "Row", fields: [{ name: "kind", type: "u64" }] }
			],
			statements: [
				{ functionality: { relation: 1, projection: [0] } },
				{ containment: { source: { relation: 1, projection: [0] }, target: { relation: 0, projection: [0] } } },
				{ containment: { source: { relation: 2, projection: [0] }, target: { relation: 1, projection: [0] } } }
			]
		})
	)
	assert.equal(source.match(/\["kind"\]: c0/g)?.length, 2)
})

test("mutually referring closed payloads use the existing field descriptors without declaration cycles", async () => {
	await roundtrip(
		JSON.stringify({
			relations: [
				{ name: "A", fields: [{ name: "peer", type: "u64" }], extension: [{ handle: "Left", values: [{ u64: "0" }] }] },
				{ name: "B", fields: [{ name: "peer", type: "u64" }], extension: [{ handle: "Right", values: [{ u64: "0" }] }] }
			],
			statements: [
				{ containment: { source: { relation: 0, projection: [1] }, target: { relation: 1, projection: [0] } } },
				{ containment: { source: { relation: 1, projection: [1] }, target: { relation: 0, projection: [0] } } }
			]
		})
	)
})

test("snapshot bindings preserve canonical Event selection literals and schema identity", async () => {
	const runtime = ManagedRuntime.make(db.NativeRuntime.layer(runtimeOptions))
	try {
		const [a, b] = await runtime.runPromise(
			Effect.gen(function* () {
				const full = yield* db.Event.space(new Uint8Array(32).fill(117), 2n)
				return [yield* db.Event.coordinate(full, 0n), yield* db.Event.coordinate(full, 1n)] as const
			})
		)
		const Child = db.relation("Child", { group: db.u64, filter: db.event })
		const Parent = db.relation("Parent", { group: db.u64, filter: db.event })
		await runtime.dispose()
		const source = await roundtrip(
			db.schema("Selection", { Child, Parent }, [
				db.key(Parent, ["group"]),
				db.contained(
					db.on(db.select(Child, { filter: [a, b] }), "group"),
					db.on(db.select(Parent, { filter: a }), "group")
				)
			])
		)
		assert.match(source, /Result.getOrThrow\(db.Event.fromBytes/)
	} finally {
		await runtime.dispose()
	}
})

test("snapshot bindings preserve closed Event axioms and their explicit pointwise keys", async () => {
	const runtime = ManagedRuntime.make(db.NativeRuntime.layer(runtimeOptions))
	try {
		const [a, b, empty] = await runtime.runPromise(
			Effect.gen(function* () {
				const full = yield* db.Event.space(new Uint8Array(32).fill(130), 2n)
				const a = yield* db.Event.coordinate(full, 0n)
				return [a, yield* db.Event.complement(a), yield* db.Event.empty(full)] as const
			})
		)
		await runtime.dispose()
		const Catalog = db.closed(
			"Catalog",
			["A", "B", "Empty"],
			{ group: db.u64, when: db.event },
			{
				A: { group: 7n, when: a },
				B: { group: 7n, when: b },
				Empty: { group: 7n, when: empty }
			}
		)
		const Claim = db.relation("Claim", { group: db.u64, when: db.event })
		const source = await roundtrip(
			db.schema("Ground", { Catalog, Claim }, [
				db.key(Catalog, ["group", "when"]),
				db.contained(db.on(Claim, ["group", "when"]), db.on(Catalog, ["group", "when"]))
			]),
			`import type { Event } from "@bjornpagen/bumbledb"
import { r0 } from "./bindings.ts"
const value: Event = r0.axioms.A.when
void value
`
		)
		assert.match(source, /import \{ Result \} from "effect"/)
		assert.match(source, /db\.key\(r0, \["group", "when"\]\)/)
	} finally {
		await runtime.dispose()
	}
})
