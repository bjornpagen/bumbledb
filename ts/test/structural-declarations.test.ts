import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, ManagedRuntime, Option, Result } from "effect"
import { ChangeSet } from "#changes.ts"
import { closed, closedId, memberDescriptor, membersAgree } from "#closed.ts"
import { decodeRows, encodeRows, rowShape } from "#codec.ts"
import { Schema, schemaTables } from "#compile.ts"
import { Db } from "#db.ts"
import { on } from "#face.ts"
import { bytes, str, u64 } from "#fields.ts"
import { isImmutable } from "#immutable.ts"
import { lower } from "#lower.ts"
import { lowerQuery, query } from "#query/lower.ts"
import { v } from "#query/scope.ts"
import { relation } from "#relation.ts"
import { NativeRuntime } from "#runtime.ts"
import { schema, schemaDescriptor, schemasAgree } from "#schema.ts"
import { contained, key, statementDescriptor } from "#statements.ts"
import { runtimeOptions, storeDir } from "#test/fixtures/learning.ts"

function declarations() {
	const Kind = closed(
		"Kind",
		["Active", "Inactive"],
		{ rank: u64 },
		{
			Active: { rank: 1n },
			Inactive: { rank: 2n }
		}
	)
	const Item = relation("Item", { id: u64, sequence: u64, kind: closedId(Kind) })
	const ById = key(Item, ["id"])
	const Theory = schema("Structural", { Kind, Item }, [ById, contained(on(Item, "kind"), on(Kind, "id"))])
	return { Kind, Item, ById, Theory }
}

test("generated declarations and factories describe the same ordered theory", () => {
	const { Kind, Item, Theory } = declarations()
	const generated = {
		kind: "relation",
		name: "Item",
		fields: {
			id: { kind: "u64" },
			sequence: { kind: "u64" },
			kind: { kind: "u64", closed: { name: "Kind", handles: ["Active", "Inactive"] } }
		}
	} as const
	const generatedKey = { kind: "key", owner: generated, projection: ["id"] } as const
	const equivalent = schema("Generated", { Kind, Item: generated }, [
		generatedKey,
		contained(on(generated, "kind"), on(Kind, "id"))
	])
	assert.ok(membersAgree(Item, generated))
	assert.ok(schemasAgree(Theory, equivalent))
	assert.deepEqual(lower(Theory), lower(equivalent))
	const refs = v(generated)
	assert.deepEqual(
		lowerQuery(query(Theory).rule((r) => r.match(generated, refs).find(refs))),
		lowerQuery(query(equivalent).rule((r) => r.match(Item, refs).find(refs)))
	)
})

test("same names cannot hide changed field order, scalar kinds, rosters or ground axioms", () => {
	const { Kind, Item, Theory } = declarations()
	const reordered = relation("Item", { sequence: u64, id: u64, kind: closedId(Kind) })
	const changed = relation("Item", { id: str, sequence: u64, kind: closedId(Kind) })
	for (const candidate of [reordered, changed]) {
		assert.equal(membersAgree(Item, candidate), false)
		assert.throws(
			() => query(Theory).rule((r) => r.match(candidate as never, {}).find({ count: r.count() })),
			/does not declare/
		)
	}
	const reorderedVars = v(reordered)
	assert.throws(
		() => query(Theory).rule((r) => r.match(Item, { id: reorderedVars.id }).find({ id: reorderedVars.id })),
		/does not declare/
	)
	for (const candidate of [
		closed("Kind", ["Inactive", "Active"], { rank: u64 }, { Active: { rank: 1n }, Inactive: { rank: 2n } }),
		closed("Kind", ["Active", "Inactive"], { rank: u64 }, { Active: { rank: 3n }, Inactive: { rank: 2n } })
	]) {
		assert.equal(membersAgree(Kind, candidate), false)
		assert.throws(
			() => query(Theory).rule((r) => r.match(candidate as never, {}).find({ count: r.count() })),
			/does not declare/
		)
	}
})

test("schema and variable construction own mutable input records without freezing callers", () => {
	const fields = { id: { kind: "u64" as const }, label: { kind: "str" as const } }
	const generated = { kind: "relation" as const, name: "Mutable" as const, fields }
	const refs = v(generated)
	const Theory = schema("Owned", { Mutable: generated }, [key(generated, ["id"])])
	assert.equal(Object.isFrozen(fields), false)
	fields.label = { kind: "str" }
	Object.assign(fields.id, { kind: "str" })
	assert.equal(refs.id.field.kind, "u64")
	assert.equal(Theory.relations.Mutable.fields.id.kind, "u64")
	assert.equal(membersAgree(Theory.relations.Mutable, generated), false)
	assert.throws(() => memberDescriptor({ ...generated, obsolete: true }), /only the declared fields/)
	let reads = 0
	const getter = {
		get kind() {
			reads += 1
			return "relation"
		},
		name: "Mutable",
		fields
	}
	assert.throws(() => memberDescriptor(getter), /own data fields/)
	assert.throws(
		() =>
			statementDescriptor({
				get kind() {
					reads += 1
					return "key"
				},
				owner: generated,
				projection: ["id"]
			} as never),
		/own data fields/
	)
	assert.equal(reads, 0, "shape validation must not run an input accessor")
})

test("checked schema reuse requires an immutable graph and validates classes and data arrays", () => {
	const { Theory, Item } = declarations()
	assert.equal(isImmutable(Theory), true)
	const checked = schemaDescriptor(Theory)
	assert.equal(schemaDescriptor(Theory), checked)
	assert.equal(schemaDescriptor(checked), checked)
	assert.equal(memberDescriptor(Item), memberDescriptor(Item))
	const mutable = structuredClone(Theory)
	assert.notEqual(schemaDescriptor(mutable), schemaDescriptor(mutable))
	const tables = schemaTables(mutable)
	Reflect.set(mutable, "statements", [])
	assert.notEqual(schemaTables(mutable), tables)
	assert.equal(schemaTables(mutable).statementIds.size, 0)
	assert.throws(() => schemaDescriptor({ ...Theory, classes: {} }), /required own field/)
	assert.throws(
		() =>
			schemaDescriptor({ ...Theory, classes: { ...Theory.classes, Item: { ...Theory.classes.Item, kind: "wrong" } } }),
		/incorrect class/
	)
	let reads = 0
	const statements = [key(Item, ["id"])]
	Object.defineProperty(statements, "0", {
		get() {
			reads++
			return key(Item, ["id"])
		},
		enumerable: true
	})
	assert.throws(() => schema("Getter", { Item }, statements), /own data elements/)
	assert.equal(reads, 0)
	const handles = ["Only"] as [string]
	Object.defineProperty(handles, "0", {
		get() {
			reads++
			return "Only"
		},
		enumerable: true
	})
	assert.throws(() => closed("GetterHandles", handles), /own data elements/)
	assert.equal(reads, 0)
	const Binary = closed("Binary", ["Only"], { value: bytes(2) }, { Only: { value: new Uint8Array([1, 2]) } })
	const binarySchema = schema("BinarySchema", { Binary }, [])
	assert.equal(isImmutable(binarySchema), false)
	const before = schemaDescriptor(binarySchema)
	binarySchema.relations.Binary.axioms.Only.value[0] = 3
	assert.equal(before.relations.Binary.axioms.Only.value[0], 1)
	assert.equal(schemaDescriptor(binarySchema).relations.Binary.axioms.Only.value[0], 3)
	assert.equal(schemasAgree(before, binarySchema), false)
})

test("compilation and live resources own schema metadata; malformed schemas fail in the Effect channel", async () => {
	const { Theory, Item, ById } = declarations()
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const mutable = structuredClone(Theory)
					const compiled = yield* Schema.compile(mutable)
					const Binary = closed("Binary", ["Only"], { value: bytes(2) }, { Only: { value: new Uint8Array([1, 2]) } })
					const binaryTheory = schema("CompiledBytes", { Binary }, [])
					const binaryCompiled = yield* Schema.compile(binaryTheory)
					binaryCompiled.schema.relations.Binary.axioms.Only.value[0] = 99
					const descriptorValue = binaryCompiled.descriptor.relations[0]?.extension?.[0]?.values.find(
						(field) => field.name === "value"
					)?.value
					assert.ok(descriptorValue instanceof Uint8Array)
					descriptorValue[0] = 99
					assert.equal(binaryCompiled.schema.relations.Binary.axioms.Only.value[0], 1)
					assert.deepEqual(
						binaryCompiled.descriptor.relations[0]?.extension?.[0]?.values.find((field) => field.name === "value")
							?.value,
						new Uint8Array([1, 2])
					)
					const db = yield* Db.create(storeDir("owned-schema"), mutable)
					const draft = yield* ChangeSet.builder(mutable)
					Reflect.set(mutable.relations.Item.fields, "id", str)
					Reflect.set(mutable, "statements", [])
					assert.equal(compiled.schema.relations.Item.fields.id.kind, "u64")
					const row = { id: 4n, sequence: 8n, kind: "Active" as const }
					yield* draft.insert(Item, [row])
					assert.equal((yield* db.apply(yield* draft.finish(), { expected: { kind: "any" } })).kind, "accepted")
					assert.deepEqual(yield* (yield* db.snapshot()).get(ById, { id: 4n }), Option.some(row))
					for (const bad of [undefined, { ...Theory, classes: {} }, { ...Theory, statements: [undefined] }]) {
						const result = yield* Effect.result(Schema.compile(bad as never))
						assert.ok(Result.isFailure(result))
						assert.equal(result.failure.code, "InvalidArgument")
					}
				})
			)
		)
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
	}
})

test("equivalent declarations work across codecs, writes, keys, query unions, imports and prepared execution", async () => {
	const original = declarations()
	const equivalent = declarations()
	const row = { id: 3n, sequence: 7n, kind: "Active" as const }
	const direct = query(equivalent.Theory).rule((r) => {
		const refs = v(original.Item)
		return r.match(equivalent.Item, refs).find(refs)
	})
	const imported = query(original.Theory).rule((r) => {
		const refs = v(direct)
		return r.match(direct, refs).find(refs)
	})
	const union = query(original.Theory)
		.rule((r) => {
			const { id } = v(original.Kind)
			return r.match(original.Kind, { id }).find({ kind: id })
		})
		.rule((r) => {
			const { id } = v(equivalent.Kind)
			return r.match(equivalent.Kind, { id }).find({ kind: id })
		})
	const parameter = query(original.Theory)
		.rule((r) => r.match(original.Item, { kind: r.param("kind") }).find({ count: r.count() }))
		.rule((r) => r.match(equivalent.Item, { kind: r.param("kind") }).find({ count: r.count() }))
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const shape = rowShape(original.Theory, equivalent.Item)
					assert.deepEqual(yield* decodeRows(shape, yield* encodeRows(shape, [row])), [row])
					const db = yield* Db.create(storeDir("structural-declarations"), original.Theory)
					const draft = yield* ChangeSet.builder(original.Theory)
					yield* draft.insert(equivalent.Item, [row])
					assert.equal((yield* db.apply(yield* draft.finish(), { expected: { kind: "any" } })).kind, "accepted")
					const snapshot = yield* db.snapshot()
					assert.deepEqual(yield* snapshot.get(equivalent.ById, { id: 3n }), Option.some(row))
					assert.deepEqual(yield* (yield* snapshot.execute(direct, {})).collect(), [row])
					assert.deepEqual(yield* (yield* snapshot.execute(imported, {})).collect(), [row])
					assert.deepEqual(yield* (yield* snapshot.execute(union, {})).collect(), [
						{ kind: "Active" },
						{ kind: "Inactive" }
					])
					assert.deepEqual(yield* (yield* snapshot.execute(parameter, { kind: "Active" })).collect(), [{ count: 1n }])
					const prepared = yield* snapshot.prepare(direct)
					assert.deepEqual(yield* (yield* prepared.execute({})).collect(), [row])
					const differentTheory = schema(
						"Structural",
						{ Item: original.Item, Kind: original.Kind },
						original.Theory.statements
					)
					const wrongOrdinal = query(differentTheory).rule((r) => {
						const refs = v(original.Item)
						return r.match(original.Item, refs).find(refs)
					})
					assert.ok(Result.isFailure(yield* Effect.result(snapshot.execute(wrongOrdinal, {}))))
					assert.ok(Result.isFailure(yield* Effect.result(snapshot.prepare(wrongOrdinal))))
				})
			)
		)
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
	}
})
