/** Copied into the isolated tarball consumer; never imports workspace source. */
import assert from "node:assert/strict"
import { createRequire } from "node:module"
import { fileURLToPath } from "node:url"
import {
	AuthoringError,
	type Db,
	DbError,
	internalBlake3,
	relation,
	schema,
	span,
	str,
	u64
} from "@bjornpagen/bumbledb"
import { ErrStore, memStore, openReplica, openWriter, storeKey } from "@bjornpagen/bumbledb-log"
import { Effect } from "effect"

const consumer = createRequire(import.meta.url)
const core = createRequire(consumer.resolve("@bjornpagen/bumbledb"))
const log = createRequire(consumer.resolve("@bjornpagen/bumbledb-log"))
assert.equal(log.resolve("@bjornpagen/bumbledb"), consumer.resolve("@bjornpagen/bumbledb"))
assert.equal(core.resolve("effect"), consumer.resolve("effect"))
assert.equal(log.resolve("effect"), consumer.resolve("effect"))
assert.equal(internalBlake3(new Uint8Array()).length, 32)
assert.equal(storeKey("manifest"), "manifest")
assert.equal(typeof openReplica, "function")

assert.throws(
	() => span(1n, 1n),
	(cause: unknown) => cause instanceof AuthoringError
)
export const authoringRecovery = Effect.gen(function* () {
	return yield* new AuthoringError({ message: "packed authoring refusal" })
}).pipe(Effect.catchTag("AuthoringError", (failure) => Effect.succeed(failure.message)))
const typedAuthoring: Effect.Effect<string> = authoringRecovery
assert.equal(Effect.runSync(typedAuthoring), "packed authoring refusal")

const resourceError = new DbError({
	operation: "packed-consumer",
	reason: {
		_tag: "ResourceLimit",
		dimension: "workingBytes",
		used: 0n,
		requested: 10n,
		limit: 9n
	}
})
export const resourceRecovery = Effect.fail(resourceError).pipe(
	Effect.catchReason("DbError", "ResourceLimit", (reason) => Effect.succeed(reason.limit))
)
const typedResource: Effect.Effect<bigint, DbError> = resourceRecovery
// The pinned RC retains DbError in E after catchReason; do not erase it.
const retainedError: Effect.Error<typeof resourceRecovery> = resourceError
assert.equal(retainedError, resourceError)
assert.equal(Effect.runSync(typedResource), 9n)

const providerCause = { provider: "packed consumer", retry: false }
export const storeFailure = new ErrStore({ message: "provider refusal", cause: providerCause })
export const storeRecovery = Effect.gen(function* () {
	return yield* storeFailure
}).pipe(Effect.catchTag("LogStoreFailure", (failure) => Effect.succeed(failure.cause)))
const typedStore: Effect.Effect<unknown> = storeRecovery
assert.equal(Effect.runSync(typedStore), providerCause)

const PackedRow = relation("PackedRow", { id: u64.fresh, name: str })
const theory = schema("PackedConsumer", { PackedRow }, [])
const writer = await openWriter({
	store: memStore(),
	prefix: "packed/consumer",
	dir: fileURLToPath(new URL("./data/writer", import.meta.url)),
	theory
})
try {
	// Core type/schema identity and the real native-backed replica handle.
	const db: Db<{ PackedRow: typeof PackedRow }> = writer.replica.db
	assert.equal(db.schema, theory)
	const outcome = await writer.commit((batch) => batch.insert(PackedRow, [{ id: 1n, name: "packed" }]))
	assert.equal(outcome.tag, "accepted")
	assert.deepEqual(
		db.read((instance) => instance.scan(PackedRow)),
		[{ id: 1n, name: "packed" }]
	)
} finally {
	await writer.replica[Symbol.asyncDispose]()
}
