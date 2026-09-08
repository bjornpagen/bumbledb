/** Isolated tarball runner for D07/D22. Specimens do not self-provide. */
import assert from "node:assert/strict"
import { createRequire } from "node:module"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { Db, DbError } from "@bjornpagen/bumbledb"
import { ProtocolError, protocolErrorCodes } from "@bjornpagen/bumbledb-log"
import { AuthoringError } from "@bjornpagen/bumbledb"
import { Effect, Exit, Stream } from "effect"
import {
	Learning,
	attemptsFor,
	coreProgram,
	drainPages,
	incrementUnits,
	incrementUnitsAsF64,
	makeConsumerRuntime,
	newAttempt,
	readAttempts
} from "./core-ts/consumer.ts"
import { incrementUnitsIntent, knownInvalidMixRefuses, mintIntent } from "./log-ts/consumer.ts"
import { readPublishedAttempts, mintCommand } from "./native-ledger/consumer.ts"

const consumer = createRequire(import.meta.url)
const core = createRequire(consumer.resolve("@bjornpagen/bumbledb"))
const log = createRequire(consumer.resolve("@bjornpagen/bumbledb-log"))
assert.equal(log.resolve("@bjornpagen/bumbledb"), consumer.resolve("@bjornpagen/bumbledb"))
assert.equal(core.resolve("effect"), consumer.resolve("effect"))
assert.equal(log.resolve("effect"), consumer.resolve("effect"))

assert.equal(incrementUnits.kind, "add")
assert.equal(incrementUnits.result, "unresolved")
assert.equal(incrementUnitsAsF64.kind, "cast")
if (incrementUnitsAsF64.kind === "cast") {
	assert.equal(incrementUnitsAsF64.cast, "toF64")
}
const convertUnits = incrementUnitsIntent.entries[0]
assert.equal(convertUnits?.kind, "convert")
assert.equal(convertUnits && "field" in convertUnits ? convertUnits.field : "", "units")
assert.equal(incrementUnitsIntent.schema.name, "Learning")
assert.ok(knownInvalidMixRefuses, "D27: I64/U64 mixing refuses at authoring")

const authoringRecovery = Effect.gen(function* () {
	return yield* new AuthoringError({ message: "packed authoring refusal" })
}).pipe(Effect.catchTag("AuthoringError", (failure) => Effect.succeed(failure.message)))
assert.equal(Effect.runSync(authoringRecovery), "packed authoring refusal")

const cancelledError = new DbError({
	operation: "packed-consumer",
	reason: { _tag: "Cancelled" }
})
assert.equal(
	Effect.runSync(
		Effect.fail(cancelledError).pipe(
			Effect.catchReason("DbError", "Cancelled", (reason) => Effect.succeed(reason._tag))
		)
	),
	"Cancelled"
)
assert.ok(protocolErrorCodes.includes("ForeignIdentity"))
assert.equal(
	Effect.runSync(
		Effect.fail(
			new ProtocolError({ operation: "packed-consumer", reason: { _tag: "Contention", attempts: 3 } })
		).pipe(Effect.catchTag("ProtocolError", (failure) => Effect.succeed(failure.code)))
	),
	"Contention"
)

const dir = fs.mkdtempSync(path.join(os.tmpdir(), "packed-consumer-"))
const runtime = makeConsumerRuntime()

try {
	const created = await runtime.runPromise(coreProgram(path.join(dir, "core")))
	assert.ok(created.outcome.kind === "accepted" || created.outcome.kind === "no-change")
	assert.ok(Array.isArray(created.rows))
	assert.ok(created.closed)

	const d07 = await runtime.runPromise(
		Effect.scoped(
			Effect.gen(function* () {
				const studentId = crypto.randomUUID()
				const attemptId = crypto.randomUUID()
				const store = path.join(dir, "d07")
				const db = yield* Db.create(store, Learning)
				const changes = yield* newAttempt(studentId, attemptId)
				const outcome = yield* db.apply(changes, { expected: { kind: "any" } })
				assert.ok(outcome.kind === "accepted" || outcome.kind === "no-change")
				const snapshot = yield* db.snapshot()
				const rows = yield* readAttempts(snapshot, studentId)
				assert.ok(rows.length >= 1)
				const paged = yield* drainPages(snapshot, studentId)
				assert.equal(paged, rows.length, "D07: pages and collect must agree on admitted rows")
				assert.deepEqual(yield* readPublishedAttempts(snapshot, studentId), rows)
				const result = yield* snapshot.execute(attemptsFor, { student: studentId })
				assert.deepEqual(yield* result.collect(), rows)
				assert.deepEqual(yield* result.collect(), rows, "collection does not consume the result")
				const pages = result.pages()
				assert.equal(yield* pages.pipe(Stream.runFold(() => 0, (count, page) => count + page.length)), rows.length)
				assert.ok(Exit.isFailure(yield* Effect.exit(result.collect())), "paging transfers the result once")
				return yield* db.close()
			})
		)
	)
	assert.ok(d07)

	const intent = await runtime.runPromise(mintIntent)
	assert.ok(intent.studentId)
	const command = await runtime.runPromise(mintCommand(crypto.randomUUID()))
	assert.ok(command.requestId)
	assert.ok(command.receiptEpoch)
	assert.ok(intent.commandId.requestId)
} finally {
	await runtime.dispose()
	fs.rmSync(dir, { recursive: true, force: true })
}
