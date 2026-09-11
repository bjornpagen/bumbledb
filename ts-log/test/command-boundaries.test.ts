import assert from "node:assert/strict"
import { test } from "node:test"
import { ChangeSet, DbError, NativeRuntime, relation, Schema, schema, str } from "@bjornpagen/bumbledb"
import { Effect, ManagedRuntime, Result } from "effect"
import { Command } from "#command.ts"
import { ProtocolError } from "#errors.ts"
import { DatabaseId, IncarnationId, ReceiptEpoch, RequestId } from "#identity.ts"

const Entry = relation("Entry", { body: str })
const Theory = schema("CommandBoundaries", { Entry }, [])
const ENVELOPE_BYTES = 4 << 20
const RESULT_BYTES = 1 << 20

function ok<A, E>(value: Result.Result<A, E>): A {
	assert.ok(Result.isSuccess(value))
	return value.success
}

test("native command boundaries retain exact sizes and distinguish caller misuse", async () => {
	const runtime = ManagedRuntime.make(
		NativeRuntime.layer({
			workers: 2,
			queueCapacity: 16,
			cleanupCapacity: 16,
			ownerCapacity: 16,
			nativeHandleCapacity: 64,
			cleanupTimeout: "2 seconds"
		})
	)
	try {
		await runtime.runPromise(
			Effect.gen(function* () {
				const compiled = yield* Schema.compile(Theory)
				const scope = {
					databaseId: ok(DatabaseId.parse("11111111-1111-1111-1111-111111111111")),
					incarnationId: ok(IncarnationId.parse("22222222-2222-2222-2222-222222222222")),
					schemaId: compiled.schemaId
				}
				const id = {
					receiptEpoch: ok(ReceiptEpoch.from(1n)),
					requestId: ok(RequestId.parse("33333333-3333-3333-3333-333333333333"))
				}
				yield* Effect.scoped(
					Effect.gen(function* () {
						const changes = yield* (yield* ChangeSet.builder(Theory)).finish()
						yield* changes.close()
						const refused = yield* Effect.result(
							Command.seal({
								scope,
								id,
								changes,
								precondition: { kind: "blind" },
								result: {}
							})
						)
						assert.ok(Result.isFailure(refused))
						assert.ok(refused.failure instanceof DbError)
						assert.equal(refused.failure.code, "ClosedHandle")
					})
				)
				const encode = (body: string, result: Readonly<Record<string, string>> = {}) =>
					Effect.scoped(
						Effect.gen(function* () {
							const draft = yield* ChangeSet.builder(Theory)
							yield* draft.insert(Entry, [{ body }])
							const command = yield* Command.seal({
								scope,
								id,
								changes: yield* draft.finish(),
								precondition: { kind: "blind" },
								result
							})
							return yield* Command.encode(command)
						})
					)
				const decode = (bytes: Uint8Array) =>
					Effect.scoped(
						Effect.gen(function* () {
							const command = yield* Command.decode(bytes, Theory)
							return yield* Command.encode(command)
						})
					)
				const tiny = yield* encode("x")
				const atLimitBody = "x".repeat(ENVELOPE_BYTES - tiny.byteLength + 1)
				const exact = yield* encode(atLimitBody)
				assert.equal(exact.byteLength, ENVELOPE_BYTES)
				assert.deepEqual(yield* decode(exact), exact)
				for (const failure of [
					yield* Effect.result(encode(`${atLimitBody}x`)),
					yield* Effect.result(decode(new Uint8Array(ENVELOPE_BYTES + 1)))
				]) {
					assert.ok(Result.isFailure(failure))
					assert.ok(failure.failure instanceof DbError)
					assert.deepEqual(failure.failure.reason, {
						_tag: "ResourceLimit",
						dimension: "envelope",
						used: 0n,
						requested: BigInt(ENVELOPE_BYTES + 1),
						limit: BigInt(ENVELOPE_BYTES)
					})
				}

				// Measure envelope growth with the actual public codec; result
				// framing stays native and the test does not duplicate its writer.
				const withResult = yield* encode("x", { body: "x" })
				const resultOverhead = withResult.byteLength - tiny.byteLength - 1
				const resultAtLimit = "x".repeat(RESULT_BYTES - resultOverhead)
				const exactResult = yield* encode("x", { body: resultAtLimit })
				assert.deepEqual(yield* decode(exactResult), exactResult)
				const resultFailure = yield* Effect.result(encode("x", { body: `${resultAtLimit}x` }))
				assert.ok(Result.isFailure(resultFailure))
				assert.ok(resultFailure.failure instanceof DbError)
				assert.deepEqual(resultFailure.failure.reason, {
					_tag: "ResourceLimit",
					dimension: "result",
					used: 0n,
					requested: BigInt(RESULT_BYTES + 1),
					limit: BigInt(RESULT_BYTES)
				})

				const foreignFamily = tiny.slice()
				foreignFamily[0] = 0
				const badResult = withResult.slice()
				const marker = new TextEncoder().encode("bumbledb.result.v1\0")
				const resultOffset = Buffer.from(badResult).indexOf(marker)
				assert.ok(resultOffset > 0)
				badResult[resultOffset] = 0
				for (const [bytes, detail] of [
					[tiny.slice(0, 10), /Truncated/],
					[foreignFamily, /Family/],
					[badResult, /declared-result record: Family/]
				] as const) {
					const failure = yield* Effect.result(decode(bytes))
					assert.ok(Result.isFailure(failure))
					assert.ok(failure.failure instanceof ProtocolError)
					assert.ok(failure.failure.reason._tag === "Misuse")
					assert.match(failure.failure.reason.detail ?? "", detail)
				}
			})
		)
	} finally {
		await runtime.dispose()
	}
})
