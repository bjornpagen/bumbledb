/** Copied into the isolated tarball consumer; never imports workspace source. */
import assert from "node:assert/strict"
import { createRequire } from "node:module"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import {
	AuthoringError,
	ChangeSet,
	Db,
	DbError,
	Id128,
	internalBlake3,
	key,
	NativeRuntime,
	relation,
	schema,
	span,
	str,
	u64
} from "@bjornpagen/bumbledb"
import type { ExecutionPolicy, NativeRuntimeOptions } from "@bjornpagen/bumbledb"
import { ProtocolError, protocolErrorCodes } from "@bjornpagen/bumbledb-log"
import { Effect, Option, Result } from "effect"

// One shared module graph: the log resolves the SAME core and effect.
const consumer = createRequire(import.meta.url)
const core = createRequire(consumer.resolve("@bjornpagen/bumbledb"))
const log = createRequire(consumer.resolve("@bjornpagen/bumbledb-log"))
assert.equal(log.resolve("@bjornpagen/bumbledb"), consumer.resolve("@bjornpagen/bumbledb"))
assert.equal(core.resolve("effect"), consumer.resolve("effect"))
assert.equal(log.resolve("effect"), consumer.resolve("effect"))
assert.equal(internalBlake3(new Uint8Array()).length, 32)

// Pure fallible parsing is Result; no throw-twin exists.
assert.ok(Result.isFailure(span(1n, 1n)))
assert.ok(Result.isFailure(Id128.fromHex("not-hex")))
assert.ok(Result.isSuccess(Id128.fromHex("00112233445566778899aabbccddeeff")))

// Typed authoring refusal recovers through catchTag with exact inference.
export const authoringRecovery = Effect.gen(function* () {
	return yield* new AuthoringError({ message: "packed authoring refusal" })
}).pipe(Effect.catchTag("AuthoringError", (failure) => Effect.succeed(failure.message)))
const typedAuthoring: Effect.Effect<string> = authoringRecovery
assert.equal(Effect.runSync(typedAuthoring), "packed authoring refusal")

// Reason-level recovery keeps DbError in E on the pinned RC.
const resourceError = new DbError({
	operation: "packed-consumer",
	reason: { _tag: "ResourceLimit", dimension: "workingBytes", used: 0n, requested: 10n, limit: 9n }
})
export const resourceRecovery = Effect.fail(resourceError).pipe(
	Effect.catchReason("DbError", "ResourceLimit", (reason) => Effect.succeed(reason.limit))
)
const typedResource: Effect.Effect<bigint, DbError> = resourceRecovery
const retainedError: Effect.Error<typeof resourceRecovery> = resourceError
assert.equal(retainedError, resourceError)
assert.equal(Effect.runSync(typedResource), 9n)

// The log's own vocabulary is a direct tagged class beside the core's.
assert.ok(protocolErrorCodes.includes("ForeignIdentity"))
const protocolFailure = new ProtocolError({ operation: "packed-consumer", reason: { _tag: "Contention", attempts: 3 } })
export const protocolRecovery = Effect.fail(protocolFailure).pipe(
	Effect.catchTag("ProtocolError", (failure) => Effect.succeed(failure.code))
)
assert.equal(Effect.runSync(protocolRecovery), "Contention")

// Real native work through the staged artifacts: create, apply, read.
const PackedRow = relation("PackedRow", { id: u64, name: str })
const theory = schema("PackedConsumer", { PackedRow }, [key(PackedRow, ["id"])])
const runtimeOptions: NativeRuntimeOptions = {
	workers: 1,
	queueCapacity: 8,
	cleanupCapacity: 8,
	ownerCapacity: 4,
	nativeHandleCapacity: 32,
	inputBytes: 4_000_000n,
	workingBytes: 16_000_000n,
	scratchBytes: 16_000_000n,
	resultBytes: 4_000_000n,
	chunkBytes: 500_000n,
	cleanupTimeout: "2 seconds"
}
const work: ExecutionPolicy = {
	inputBytes: 1_000_000n,
	workingBytes: 8_000_000n,
	scratchBytes: 8_000_000n,
	resultBytes: 1_000_000n,
	rows: 1_000n,
	workUnits: 1_000_000n,
	timeout: "10 seconds"
}
const dir = fs.mkdtempSync(path.join(os.tmpdir(), "packed-consumer-"))
const program = Effect.scoped(
	Effect.gen(function* () {
		const db = yield* Db.create(path.join(dir, "store"), theory, work)
		const draft = yield* ChangeSet.builder(theory, work)
		yield* draft.insert(PackedRow, [{ id: 1n, name: "packed" }])
		const changes = yield* draft.finish()
		const outcome = yield* db.apply(changes, { ...work, expected: { kind: "any" } })
		assert.equal(outcome.kind, "accepted")
		const snapshot = yield* db.snapshot(work)
		const found = yield* snapshot.get(PackedRow, { id: 1n }, work)
		assert.ok(Option.isSome(found))
		assert.equal(found.value.name, "packed")
		return found.value
	})
)
try {
	await Effect.runPromise(program.pipe(Effect.provide(NativeRuntime.layer(runtimeOptions))))
} finally {
	fs.rmSync(dir, { recursive: true, force: true })
}
