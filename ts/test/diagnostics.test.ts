import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, ManagedRuntime, Result } from "effect"
import { ChangeSet } from "#changes.ts"
import { closed } from "#closed.ts"
import { decodeBoundaryRows, decodeRows, encodeBoundaryRows, encodeRows, rowShape } from "#codec.ts"
import { Db } from "#db.ts"
import { str, u64 } from "#fields.ts"
import { query } from "#query/lower.ts"
import { relation } from "#relation.ts"
import { NativeRuntime } from "#runtime.ts"
import type { DbError } from "#runtime-errors.ts"
import { schema } from "#schema.ts"
import { key } from "#statements.ts"
import { runtimeOptions, storeDir } from "#test/fixtures/learning.ts"

const Row = relation("Row", { id: u64, value: str })
const ById = key(Row, ["id"])
const Theory = schema("Diagnostics", { Row }, [ById])

function diagnostic(error: DbError) {
	assert.equal(error.reason._tag, "InvalidArgument")
	if (error.reason._tag !== "InvalidArgument") throw new Error("expected an argument failure")
	assert.ok(error.reason.detail?.diagnostic)
	assert.equal(error.message, `${error.operation}: InvalidArgument`)
	return error.reason.detail.diagnostic
}

test("JSON codecs preserve missing, extra and invalid-field diagnostics", () => {
	const missing = encodeBoundaryRows(Row, [{ id: 1n }] as never)
	assert.ok(Result.isFailure(missing))
	assert.deepEqual(diagnostic(missing.failure), {
		code: "MissingField",
		context: "relation Row.value",
		expected: "a required own field"
	})
	const extra = encodeBoundaryRows(Row, [{ id: 1n, value: "sensitive content", obsolete: 2n }] as never)
	assert.ok(Result.isFailure(extra))
	assert.equal(diagnostic(extra.failure).code, "UnknownField")
	assert.equal(diagnostic(extra.failure).context, "relation Row.obsolete")
	assert.equal(extra.failure.message.includes("sensitive content"), false)
	const invalid = decodeBoundaryRows(Row, [{ id: "-0", value: "sensitive content" }])
	assert.ok(Result.isFailure(invalid))
	assert.equal(diagnostic(invalid.failure).context, "relation Row.id")
	assert.equal(diagnostic(invalid.failure).code, "InvalidValue")
})

test("typed writes, keys and both query execution forms retain authoring detail", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.create(storeDir("diagnostics"), Theory)
					let shapeReads = 0
					const getterShape = {
						get schema() {
							shapeReads++
							return Theory
						},
						relation: Row
					}
					for (const shape of [undefined, getterShape, { schema: Theory, relation: undefined }]) {
						assert.ok(Result.isFailure(yield* Effect.result(encodeRows(shape as never, []))))
						assert.ok(Result.isFailure(yield* Effect.result(decodeRows(shape as never, new Uint8Array()))))
					}
					assert.equal(shapeReads, 0)
					const snapshot = yield* db.snapshot()
					const badKey = yield* Effect.result(snapshot.get(ById, { id: -1n }))
					assert.ok(Result.isFailure(badKey))
					assert.equal(diagnostic(badKey.failure).code, "InvalidValue")
					const q = query(Theory).rule((r) => r.match(Row, { id: r.param("id") }).find({ count: r.count() }))
					const prepared = yield* snapshot.prepare(q)
					for (const execution of [snapshot.execute(q, { id: -1n }), prepared.execute({ id: -1n })]) {
						const result = yield* Effect.result(execution)
						assert.ok(Result.isFailure(result))
						assert.deepEqual(diagnostic(result.failure), {
							code: "InvalidValue",
							context: "param id",
							expected: "u64 bigint in range"
						})
					}
					const draft = yield* ChangeSet.builder(Theory)
					let reads = 0
					const getter = {
						id: 1n,
						get value() {
							reads += 1
							return "sensitive content"
						}
					}
					const result = yield* Effect.result(draft.insert(Row, [getter]))
					assert.ok(Result.isFailure(result))
					assert.equal(diagnostic(result.failure).code, "InvalidRecord")
					assert.equal(reads, 0, "input validation must precede sizing and field reads")
					assert.ok(Result.isFailure(yield* Effect.result(draft.finish())))
				})
			)
		)
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
	}
})

test("native schema refusal retains its engine family and detail instead of Internal", async () => {
	// The native theory disallows str payload columns on a closed relation.
	const Invalid = closed("Invalid", ["Only"], { label: str }, { Only: { label: "sensitive content" } })
	const invalidTheory = schema("InvalidTheory", { Invalid, Row }, [ById])
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		const result = await runtime.runPromise(Effect.result(encodeRows(rowShape(invalidTheory, Row), [])))
		assert.ok(Result.isFailure(result))
		assert.equal(result.failure.reason._tag, "Engine")
		if (result.failure.reason._tag === "Engine") {
			assert.equal(result.failure.reason.kind, "schema")
			assert.match(result.failure.reason.message, /str on a closed relation/)
		}
		assert.equal(result.failure.message, "encodeRows: Engine")
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
	}
})
