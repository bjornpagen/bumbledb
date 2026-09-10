import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, Schema as EffectSchema, ManagedRuntime, Option, Result } from "effect"
import { ChangeSet } from "#changes.ts"
import { decodeBoundaryRows, decodeRows, encodeBoundaryRows, encodeRows, rowSchema, rowShape } from "#codec.ts"
import { Db } from "#db.ts"
import { dbNative } from "#db-native.ts"
import { bool, bytes, f64, i64, interval, literalOf, str, u64, uuid } from "#fields.ts"
import { lower } from "#lower.ts"
import { lowerQuery, query } from "#query/lower.ts"
import { wireParams } from "#query/run.ts"
import { v } from "#query/scope.ts"
import type { Fact } from "#relation.ts"
import { relation } from "#relation.ts"
import { cellOf, flatRowsOf } from "#rows.ts"
import { NativeRuntime, nativeOperationWith, runtimeHandle } from "#runtime.ts"
import { schema } from "#schema.ts"
import { key } from "#statements.ts"
import { runtimeOptions, storeDir } from "#test/fixtures/learning.ts"
import { I64_MAX, I64_MIN, U64_MAX } from "#values.ts"

const fields = {
	n: u64,
	signed: i64,
	span: interval(i64),
	fixed: interval(u64, 5n),
	dense: interval(f64),
	bytes: bytes(2),
	text: str,
	identity: uuid,
	flag: bool
}
const Row = relation("Row", fields)
const RowByN = key(Row, ["n"])
const Theory = schema("ValueConformance", { Row }, [RowByN])
const shape = rowShape(Theory, Row)
const good: Fact<typeof Row> = {
	n: U64_MAX,
	signed: I64_MIN,
	span: { start: I64_MIN, end: I64_MAX },
	fixed: { start: 0n, end: 5n },
	dense: { start: Number.NEGATIVE_INFINITY, end: Number.POSITIVE_INFINITY },
	bytes: new Uint8Array([1, 2]),
	text: "bee 🐝",
	identity: "00112233-4455-6677-8899-aabbccddeeff",
	flag: true
}
const invalidValues: ReadonlyArray<readonly [keyof typeof fields, unknown]> = [
	["n", -1n],
	["n", U64_MAX + 1n],
	["n", 1],
	["signed", I64_MIN - 1n],
	["signed", I64_MAX + 1n],
	["span", { start: 2n, end: 1n }],
	["span", { start: 1n, end: 1n }],
	["span", { start: I64_MIN - 1n, end: 0n }],
	["span", { start: 0n, end: I64_MAX + 1n }],
	["fixed", { start: 0n, end: 4n }],
	["fixed", { start: U64_MAX - 5n, end: U64_MAX }],
	["dense", { start: Number.NaN, end: 1 }],
	["dense", { start: 1, end: 1 }],
	["dense", { start: 2, end: 1 }],
	["bytes", new Uint8Array(1)],
	["text", "\ud800"],
	["text", "\udc00"],
	["text", "valid\ud800then-invalid"],
	["identity", good.identity.toUpperCase()],
	["flag", 1]
]

const checkRow = EffectSchema.is(rowSchema(Row))

test("facts, selections, host cells, and JSON codecs share the complete native value domain", () => {
	assert.ok(checkRow(good))
	const encoded = encodeBoundaryRows(Row, [good])
	assert.ok(Result.isSuccess(encoded))
	assert.deepEqual(Result.getOrThrow(decodeBoundaryRows(Row, encoded.success)), [good])
	for (const [name, value] of invalidValues) {
		const row = { ...good, [name]: value }
		assert.equal(checkRow(row), false, `${name}: row schema`)
		assert.ok(Result.isFailure(encodeBoundaryRows(Row, [row])), `${name}: JSON encoder`)
		assert.throws(() => flatRowsOf(Row, [row]), `${name}: fact projector`)
		assert.throws(() => cellOf(name, fields[name], value), `${name}: host cell`)
		assert.throws(() => literalOf(fields[name], value), `${name}: schema literal`)
	}
	assert.throws(() => interval(u64, U64_MAX + 1n), /width/)
})

test("unknown, inherited, absent, symbolic, and nested fields are never silently dropped", () => {
	const invalid = [
		{ ...good, rir: 2n },
		{ ...good, obsolete: undefined },
		{ ...good, n: undefined },
		{ ...good, [Symbol("unknown")]: 1 },
		{ ...good, span: { ...good.span, extra: 1 } },
		{ ...good, dense: { ...good.dense, extra: 1 } },
		Object.create(good),
		null,
		[],
		() => good
	]
	for (const row of invalid) {
		assert.equal(checkRow(row), false)
		assert.ok(Result.isFailure(encodeBoundaryRows(Row, [row] as never)))
		assert.throws(() => flatRowsOf(Row, [row] as never))
	}
	const encoded = Result.getOrThrow(encodeBoundaryRows(Row, [good]))[0]
	assert.ok(encoded)
	for (const row of [
		{ ...encoded, n: "-0" },
		{ ...encoded, signed: "-0" },
		{ ...encoded, span: { start: "-0", end: "1" } },
		{ ...encoded, fixed: { start: "0", end: "4" } },
		{ ...encoded, dense: { ...(encoded.dense as object), extra: 1 } },
		{ ...encoded, obsolete: undefined },
		{ ...encoded, [Symbol("unknown")]: 1 },
		Object.create(encoded)
	])
		assert.ok(Result.isFailure(decodeBoundaryRows(Row, [row])))
	assert.ok(Result.isFailure(encodeBoundaryRows(Row, [{ ...good, bytes: new Uint8Array(new SharedArrayBuffer(2)) }])))
})

test("query parameters use the same interpreter and exact supplied parameter fields", () => {
	const q = query(Theory).rule((r) => r.match(Row, { n: r.param("n") }).find({ count: r.count() }))
	assert.deepEqual(wireParams(q.data.params, { n: 1n }), [{ kind: "u64", value: 1n }])
	for (const params of [{}, { n: -1n }, { n: U64_MAX + 1n }, { n: 1n, extra: undefined }, Object.create({ n: 1n })]) {
		assert.throws(() => wireParams(q.data.params, params))
	}
	const sets = query(Theory).rule((r) => r.match(Row, { n: r.inSet("ns") }).find({ count: r.count() }))
	assert.throws(() => wireParams(sets.data.params, { ns: [1n, -1n] }))
	let elementReads = 0
	const accessor = Object.defineProperty([1n], "0", {
		get() {
			elementReads += 1
			return 1n
		},
		enumerable: true
	})
	for (const ns of [new Array(1), accessor, Object.assign([1n], { extra: true })]) {
		assert.throws(() => wireParams(sets.data.params, { ns }))
	}
	assert.equal(elementReads, 0)
	const points = query(Theory).rule((r) => {
		const { span } = v(Row)
		return r
			.match(Row, { span })
			.where(r.pointIn(r.param("point"), span))
			.find({ span })
	})
	assert.deepEqual(wireParams(points.data.params, { point: 0n }), [{ kind: "i64", value: 0n }])
	assert.throws(() => wireParams(points.data.params, { point: I64_MAX + 1n }))
})

test("authored queries own mutable binding and comparison literals", () => {
	const ByteRow = relation("ByteRow", { value: bytes(2), span: interval(i64) })
	const theory = schema("LiteralOwnership", { ByteRow }, [])
	const boundBytes = new Uint8Array([1, 2])
	const comparedBytes = new Uint8Array([3, 4])
	const span = { start: 0n, end: 10n }
	const q = query(theory).rule((r) => {
		const row = v(ByteRow)
		return r
			.match(ByteRow, { value: boundBytes, span: row.span })
			.match(ByteRow, { value: row.value, span })
			.where(r.eq(row.value, comparedBytes))
			.find({ value: row.value })
	})
	const before = lowerQuery(q)
	boundBytes[0] = 9
	comparedBytes[0] = 9
	span.end = 99n
	assert.deepEqual(lowerQuery(q), before)
	assert.equal(Object.isFrozen(span), false, "caller input is not frozen")
	const inspected = q.data.rules[0]?.items[0]
	assert.equal(inspected?.kind, "atom")
	if (inspected?.kind !== "atom") throw new Error("expected atom")
	const literal = inspected.atom.bindings.find((binding) => binding.field === "value")?.term
	assert.equal(literal?.kind, "literal")
	if (literal?.kind !== "literal" || !(literal.value instanceof Uint8Array)) throw new Error("expected bytes")
	literal.value[0] = 99
	assert.deepEqual(lowerQuery(q), before, "inspection cannot change query meaning")
	const imported = query(theory).rule((r) => {
		const refs = v(q)
		return r.match(q, refs).find(refs)
	})
	assert.equal(lowerQuery(imported).interiors.length, 1)
	const nested = query(theory).rule((r) => {
		const refs = v(imported)
		return r.match(imported, refs).find(refs)
	})
	assert.equal(lowerQuery(nested).interiors.length, 2)
	assert.equal(
		lowerQuery(
			imported.rule((r) => {
				const refs = v(q)
				return r.match(q, refs).find(refs)
			})
		).interiors.length,
		1
	)
})

test("query bindings and result records reject getters and explicit undefined fields", () => {
	let reads = 0
	const getter = {
		get n() {
			reads += 1
			return 1n
		}
	}
	for (const bindings of [
		getter,
		{ n: undefined },
		{ extra: undefined },
		{ [Symbol("extra")]: 1n },
		Object.create({ n: 1n })
	]) {
		assert.throws(() => query(Theory).rule((r) => r.match(Row, bindings as never).find({ count: r.count() })))
		assert.throws(() =>
			query(Theory).rule((r) =>
				r
					.match(Row, {})
					.where(r.not(Row, bindings as never))
					.find({ count: r.count() })
			)
		)
	}
	assert.throws(() => query(Theory).rule((r) => r.match(Row, {}).find({ count: undefined } as never)))
	assert.throws(() =>
		query(Theory).rule((r) =>
			r.match(Row, {}).find({
				get count() {
					reads += 1
					return r.count()
				}
			})
		)
	)
	assert.equal(reads, 0)
})

function nativeEncode(row: Readonly<Record<string, unknown>>) {
	return Effect.gen(function* () {
		const runtime = yield* runtimeHandle()
		// Deliberately bypass host validation to test the native domain independently.
		const cells = Object.keys(Row.fields).map((name) => row[name])
		return yield* nativeOperationWith(
			"native value conformance",
			(callback) => dbNative.runtimeEncodeRows(runtime, lower(Theory), 0, 1n, cells as never, callback),
			dbNative.runtimeBytesTake,
			(value) => value
		)
	})
}

test("host admission matches the real native codec; rejected facts do not enter a database", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		const native = await runtime.runPromise(nativeEncode(good))
		assert.deepEqual(await runtime.runPromise(decodeRows(shape, native)), [good])
		assert.deepEqual(await runtime.runPromise(encodeRows(shape, [good])), native)
		for (const [name, value] of invalidValues) {
			const result = await runtime.runPromise(Effect.result(nativeEncode({ ...good, [name]: value })))
			assert.ok(Result.isFailure(result), `${name}: native refusal`)
		}
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.create(storeDir("strict-values"), Theory)
					const snapshot = yield* db.snapshot()
					assert.ok(Result.isFailure(yield* Effect.result(snapshot.get(RowByN, { n: -1n }))))
					const draft = yield* ChangeSet.builder(Theory)
					assert.ok(Result.isFailure(yield* Effect.result(draft.insert(Row, [{ ...good, obsolete: true }] as never))))
					assert.ok(Result.isFailure(yield* Effect.result(draft.finish())), "an invalid row spends the draft")
					assert.equal((yield* db.inspect()).generation, snapshot.witness.generation)
				})
			)
		)
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
	}
})

test("native names and query parameters reject lossy Unicode conversion", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	const original = dbNative.runtimeSnapshotExecute
	try {
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.create(storeDir("native-text"), Theory)
					const snapshot = yield* db.snapshot()
					const q = query(Theory).rule((r) => r.match(Row, { text: r.param("text") }).find({ count: r.count() }))
					for (const text of ["bee 🐝", "\ud800", "\udc00"]) {
						// Replace only the already-validated host parameter to exercise native ingress independently.
						dbNative.runtimeSnapshotExecute = (snapshot, query, _params, callback) =>
							original(snapshot, query, [{ kind: "string", value: text }], callback)
						const result = yield* Effect.result(Effect.scoped(snapshot.execute(q, { text: "valid" })))
						assert.equal(Result.isSuccess(result), text.isWellFormed())
					}
					dbNative.runtimeSnapshotExecute = original
					const handle = yield* runtimeHandle()
					for (const name of ["well-formed 🐝", "bad\ud800"]) {
						const result = yield* Effect.result(
							nativeOperationWith(
								"native name text",
								(callback) =>
									dbNative.runtimeSchemaCompile(
										handle,
										{
											relations: [{ name, fields: [], closed: undefined }],
											statements: []
										},
										callback
									),
								dbNative.runtimeSchemaTake,
								(value) => value
							)
						)
						assert.equal(Result.isSuccess(result), name.isWellFormed())
					}
				})
			)
		)
	} finally {
		dbNative.runtimeSnapshotExecute = original
		await Effect.runPromise(runtime.disposeEffect)
	}
})

test("prototype-sensitive field and result names are ordinary owned data", async () => {
	const Special = relation("Special", { id: u64, ["__proto__"]: interval(i64), constructor: str })
	const ById = key(Special, ["id"])
	const SpecialTheory = schema("SpecialFields", { Special }, [ById])
	const row = { id: 1n, ["__proto__"]: { start: 0n, end: 1n }, constructor: "data" }
	const find = query(SpecialTheory).rule((r) => {
		const fields = v(Special)
		return r.match(Special, fields).find(fields)
	})
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const shape = rowShape(SpecialTheory, Special)
					assert.deepEqual(yield* decodeRows(shape, yield* encodeRows(shape, [row])), [row])
					const db = yield* Db.create(storeDir("special-fields"), SpecialTheory)
					const draft = yield* ChangeSet.builder(SpecialTheory)
					yield* draft.insert(Special, [row])
					assert.equal((yield* db.apply(yield* draft.finish(), { expected: { kind: "any" } })).kind, "accepted")
					const snapshot = yield* db.snapshot()
					const decoded = Option.getOrThrow(yield* snapshot.get(ById, { id: 1n }))
					assert.deepEqual(decoded, row)
					assert.equal(Object.getPrototypeOf(decoded), Object.prototype)
					assert.ok(Object.hasOwn(decoded, "__proto__"))
					assert.ok(Object.isFrozen(decoded))
					assert.deepEqual(yield* (yield* snapshot.execute(find, {})).collect(), [row])
				})
			)
		)
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
	}
})
