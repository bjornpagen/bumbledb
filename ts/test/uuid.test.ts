import assert from "node:assert/strict"
import { randomUUID } from "node:crypto"
import { test } from "node:test"
import { Effect, ManagedRuntime, Result } from "effect"
import { ChangeSet } from "#changes.ts"
import { Db } from "#db.ts"
import { query } from "#query/lower.ts"
import { v } from "#query/scope.ts"
import { NativeRuntime } from "#runtime.ts"
import { Learning, runtimeOptions, Student, storeDir } from "#test/fixtures/learning.ts"
import { Uuid } from "#uuid.ts"

const CANONICAL = "00112233-4455-6677-8899-aabbccddeeff"

test("UUID native persistence and ordered predicates preserve both halves", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	const ids: Uuid[] = [
		"00000000-0000-0000-0000-000000000000",
		"00000000-0000-0000-0000-000000000001",
		"00000000-0000-0001-0000-000000000000",
		"ffffffff-ffff-ffff-ffff-ffffffffffff"
	]
	const after = query(Learning).rule((r) => {
		const { id } = v(Student)
		return r
			.match(Student, { id })
			.where(r.gt(id, r.param("floor")))
			.find({ id })
	})
	const path = storeDir("uuid-order")
	try {
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.create(path, Learning)
					const draft = yield* ChangeSet.builder(Learning)
					yield* draft.insert(
						Student,
						ids.map((id) => ({ id, name: "UUID", budget: 10n }))
					)
					const changes = yield* draft.finish()
					assert.equal((yield* db.apply(changes, { expected: { kind: "any" } })).kind, "accepted")
				})
			)
		)
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.open(path, Learning)
					const snapshot = yield* db.snapshot()
					for (const floor of ids) {
						const result = yield* snapshot.execute(after, { floor })
						const rows = yield* result.collect()
						assert.deepEqual(
							rows.map((row) => row.id).sort(),
							ids.filter((id) => id > floor)
						)
					}
				})
			)
		)
	} finally {
		await runtime.dispose()
	}
})

test("UUID parsing normalizes host input but canonical boundary validation does not", () => {
	for (const text of [CANONICAL, CANONICAL.toUpperCase(), randomUUID()]) {
		const value = Result.getOrThrow(Uuid.parse(text))
		assert.equal(value, text.toLowerCase())
		assert.ok(Uuid.isUuid(value))
	}
	assert.equal(Uuid.isUuid(CANONICAL.toUpperCase()), false)
	for (const text of [
		"",
		"00112233445566778899aabbccddeeff",
		`${CANONICAL}\n`,
		` ${CANONICAL}`,
		CANONICAL.replace("a", "g")
	]) {
		assert.ok(Result.isFailure(Uuid.parse(text)))
		assert.equal(Uuid.isUuid(text), false)
	}
	assert.equal(Uuid.isUuid(123), false)
})

test("UUID bytes and canonical text share exact unsigned order across both word halves", () => {
	const payloads = [new Uint8Array(16), new Uint8Array(16).fill(255)]
	for (let position = 0; position < 16; position += 1) {
		for (const byte of [1, 127, 128, 255]) {
			const bytes = new Uint8Array(16)
			bytes[position] = byte
			payloads.push(bytes)
		}
	}
	const ordered = payloads.sort((left, right) => Buffer.compare(left, right))
	const values = ordered.map((bytes) => Result.getOrThrow(Uuid.fromBytes(bytes)))
	assert.deepEqual(values, [...values].sort())
	for (const [index, value] of values.entries()) {
		assert.deepEqual(Result.getOrThrow(Uuid.toBytes(value)), ordered[index])
		const copy = Result.getOrThrow(Uuid.toBytes(value))
		copy.fill(42)
		assert.deepEqual(Result.getOrThrow(Uuid.toBytes(value)), ordered[index])
	}
	for (const length of [0, 15, 17, 36]) {
		assert.ok(Result.isFailure(Uuid.fromBytes(new Uint8Array(length))))
	}
})

test("structural UUID inputs are validated, not trusted as a proof", () => {
	const malformed: Uuid = "a-b-c-d-e"
	const uppercase: Uuid = "00112233-4455-6677-8899-AABBCCDDEEFF"
	assert.ok(Result.isFailure(Uuid.toBytes(malformed)))
	assert.ok(Result.isFailure(Uuid.toBytes(uppercase)))
	assert.ok(Result.isSuccess(Uuid.toBytes(randomUUID())))
})
