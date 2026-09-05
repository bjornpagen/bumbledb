/**
 * The ONE complete PUBLIC log round trip over the REAL native bridge (no
 * scripted double): create → seal → submit(decided) → receipt → resolve
 * after reopen — chapter 35's acceptance shape, small and behavioral.
 *
 * Creation uses the sanctioned checked initialization artifact: the
 * NATIVE-rendered canonical schema snapshot (`productionCodec.schemaIdentity`
 * → `schema_file::render`), whose core v6 fingerprint is exactly the
 * creation identity's schemaId. Nothing here is hand-forged bytes.
 */
import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { test } from "node:test"
import type { ExecutionPolicy, NativeRuntimeOptions } from "@bjornpagen/bumbledb"
import { ChangeSet, key, lower, NativeRuntime, relation, Schema, schema, str, u64 } from "@bjornpagen/bumbledb"
import { Effect, Exit, ManagedRuntime, Result } from "effect"
import { Command } from "#command.ts"
import { LocalHistory } from "#history.ts"
import type { DatabaseIdentity } from "#identity.ts"
import { DatabaseId, IncarnationId, OperationId, ReceiptEpoch, RequestId } from "#identity.ts"
import { productionCodec } from "#migrations/native.ts"

const Entry = relation("Entry", { id: u64, body: str })
const Ledger = schema("Ledger", { Entry }, [key(Entry, ["id"])])

const runtimeOptions: NativeRuntimeOptions = {
	workers: 2,
	queueCapacity: 16,
	cleanupCapacity: 16,
	ownerCapacity: 16,
	nativeHandleCapacity: 64,
	inputBytes: 8_000_000n,
	workingBytes: 8_000_000n,
	scratchBytes: 8_000_000n,
	resultBytes: 1_000_000n,
	chunkBytes: 1_000_000n,
	cleanupTimeout: "2 seconds"
}
const work: ExecutionPolicy = {
	inputBytes: 1_000_000n,
	workingBytes: 1_000_000n,
	scratchBytes: 1_000_000n,
	resultBytes: 100_000n,
	rows: 100_000n,
	workUnits: 10_000_000n,
	timeout: "10 seconds"
}
const submitOptions = { ...work, attempts: 4, backoff: { baseMillis: 1, capMillis: 10 } }
const readOptions = { ...work, consistency: { kind: "cached" } as const }

function ok<A, E>(result: Result.Result<A, E>): A {
	assert.ok(Result.isSuccess(result), "expected success")
	return result.success
}

test("create → seal → submit(decided) → receipt → resolve after reopen, over the real bridge", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	const dir = fs.mkdtempSync(path.join(os.tmpdir(), "bdb-roundtrip-"))
	try {
		const exit = await runtime.runPromiseExit(
			Effect.gen(function* () {
				const compiled = yield* Schema.compile(Ledger, work)
				// The sanctioned creation artifact: native-rendered canonical
				// schema snapshot whose fingerprint IS the identity's schemaId.
				const rendered = yield* productionCodec.schemaIdentity(lower(Ledger), work)
				assert.equal(rendered.schemaId, compiled.schemaId, "schema_file fingerprint == core compile fingerprint")
				const tenant: DatabaseIdentity = {
					databaseId: ok(DatabaseId.fromHex("ab".repeat(16))),
					incarnationId: ok(IncarnationId.fromHex("cd".repeat(16))),
					schemaId: compiled.schemaId
				}
				const binding = { kind: "local", directory: dir, identity: tenant } as const
				// Scope 1: create the history, seal one insert, submit to a
				// decided receipt, retain the ref and receipt past the scope.
				const { ref, receipt } = yield* Effect.scoped(
					Effect.gen(function* () {
						const history = yield* LocalHistory.create(binding, Ledger, {
							...work,
							creation: {
								operationId: ok(OperationId.fromHex("e1".repeat(16))),
								artifact: new TextEncoder().encode(rendered.snapshot)
							}
						})
						const draft = yield* ChangeSet.builder(Ledger, work)
						yield* draft.insert(Entry, [{ id: 42n, body: "round trip" }])
						const changes = yield* draft.finish()
						const command = yield* Command.seal(
							{
								scope: history.identity,
								id: {
									receiptEpoch: ok(ReceiptEpoch.from(1n)),
									requestId: ok(RequestId.fromHex("0b".repeat(16)))
								},
								changes,
								precondition: { kind: "blind" },
								result: {}
							},
							work
						)
						const outcome = yield* history.submit(command, submitOptions)
						assert.equal(outcome.kind, "decided", "the submit decided")
						if (outcome.kind !== "decided") {
							return yield* Effect.die("unreachable")
						}
						assert.equal(outcome.receipt.outcome.kind, "committed", "the decided receipt committed")
						return { ref: command.ref, receipt: outcome.receipt }
					})
				)
				// Scope 2: reopen the durable history; the retained ref resolves
				// to the exact recorded receipt, and the fact reads back.
				yield* Effect.scoped(
					Effect.gen(function* () {
						const history = yield* LocalHistory.open(binding, Ledger, work)
						const resolved = yield* history.resolve(ref, work)
						assert.equal(resolved.kind, "found", "the retained ref resolves after reopen")
						if (resolved.kind === "found") {
							assert.equal(resolved.receipt.outcome.kind, "committed")
							assert.deepEqual(resolved.receipt.decisionAt, receipt.decisionAt)
						}
						const snapshot = yield* history.snapshot(readOptions)
						const fact = yield* snapshot.get(Entry, { id: 42n }, work)
						assert.ok(fact._tag === "Some", "the committed fact reads back after reopen")
						if (fact._tag === "Some") {
							assert.equal(fact.value.body, "round trip")
						}
					})
				)
				return true
			})
		)
		assert.ok(Exit.isSuccess(exit), `round trip: ${String(exit)}`)
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
		fs.rmSync(dir, { recursive: true, force: true })
	}
})
