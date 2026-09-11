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
import type { NativeRuntimeOptions } from "@bjornpagen/bumbledb"
import { ChangeSet, key, NativeRuntime, relation, Schema, schema, str, u64 } from "@bjornpagen/bumbledb"
import { lower } from "@bjornpagen/bumbledb/internal/log"
import { Effect, Exit, ManagedRuntime, Result } from "effect"
import { backup, restore, verifyBackup } from "#admin.ts"
import { Command } from "#command.ts"
import { LocalHistory } from "#history.ts"
import type { DatabaseIdentity } from "#identity.ts"
import { DatabaseId, IncarnationId, OperationId, ReceiptEpoch, RequestId } from "#identity.ts"
import { productionCodec } from "#migrations/native.ts"

const Entry = relation("Entry", { id: u64, body: str })
const EntryById = key(Entry, ["id"])
const Ledger = schema("Ledger", { Entry }, [EntryById])

const runtimeOptions: NativeRuntimeOptions = {
	workers: 2,
	queueCapacity: 16,
	cleanupCapacity: 16,
	ownerCapacity: 16,
	nativeHandleCapacity: 64,
	cleanupTimeout: "2 seconds"
}
const submitOptions = { attempts: 4, backoff: { baseMillis: 1, capMillis: 10 } }
const readOptions = { consistency: { kind: "cached" } as const }

function ok<A, E>(result: Result.Result<A, E>): A {
	assert.ok(Result.isSuccess(result), "expected success")
	return result.success
}

test("native command recovery and independent backup → verify → writable restore", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	const dir = fs.mkdtempSync(path.join(os.tmpdir(), "bdb-roundtrip-"))
	try {
		const exit = await runtime.runPromiseExit(
			Effect.gen(function* () {
				const compiled = yield* Schema.compile(Ledger)
				// The sanctioned creation artifact: native-rendered canonical
				// schema snapshot whose fingerprint IS the identity's schemaId.
				const rendered = yield* productionCodec.schemaIdentity(lower(Ledger))
				assert.equal(rendered.schemaId, compiled.schemaId, "schema_file fingerprint == core compile fingerprint")
				const tenant: DatabaseIdentity = {
					databaseId: ok(DatabaseId.parse("abababab-abab-abab-abab-abababababab")),
					incarnationId: ok(IncarnationId.parse("cdcdcdcd-cdcd-cdcd-cdcd-cdcdcdcdcdcd")),
					schemaId: compiled.schemaId
				}
				const binding = { kind: "local", directory: dir, identity: tenant } as const
				// Scope 1: create the history, seal one insert, submit to a
				// decided receipt, retain the ref and receipt past the scope.
				const { ref, receipt } = yield* Effect.scoped(
					Effect.gen(function* () {
						const history = yield* LocalHistory.create(binding, Ledger, {
							creation: {
								operationId: ok(OperationId.parse("e1e1e1e1-e1e1-e1e1-e1e1-e1e1e1e1e1e1")),
								artifact: new TextEncoder().encode(rendered.snapshot)
							}
						})
						const draft = yield* ChangeSet.builder(Ledger)
						yield* draft.insert(Entry, [{ id: 42n, body: "round trip" }])
						const changes = yield* draft.finish()
						const command = yield* Command.seal({
							scope: history.identity,
							id: {
								receiptEpoch: ok(ReceiptEpoch.from(1n)),
								requestId: ok(RequestId.parse("0b0b0b0b-0b0b-0b0b-0b0b-0b0b0b0b0b0b"))
							},
							changes,
							precondition: { kind: "blind" },
							result: {}
						})
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
						const history = yield* LocalHistory.open(binding, Ledger)
						const inspected = yield* history.inspect()
						const file = fs.statSync(path.join(dir, "db", "data.mdb"), { bigint: true })
						assert.equal(inspected.storage.populatedFileBytes, file.size)
						assert.ok(inspected.storage.virtualMapBytes >= file.size)
						assert.ok(inspected.storage.nonFreePageBytes > 0n)
						assert.equal(inspected.storage.allocatedDiskBytes, process.platform === "win32" ? null : file.blocks * 512n)
						assert.equal("accounted" in inspected, false)
						const resolved = yield* history.resolve(ref)
						assert.equal(resolved.kind, "found", "the retained ref resolves after reopen")
						if (resolved.kind === "found") {
							assert.equal(resolved.receipt.outcome.kind, "committed")
							assert.deepEqual(resolved.receipt.decisionAt, receipt.decisionAt)
						}
						const snapshot = yield* history.snapshot(readOptions)
						const fact = yield* snapshot.get(EntryById, { id: 42n })
						assert.ok(fact._tag === "Some", "the committed fact reads back after reopen")
						if (fact._tag === "Some") {
							assert.equal(fact.value.body, "round trip")
						}
					})
				)
				// A cold administrative open captures the standard backup artifact.
				// Restore relies only on that destination, with a new incarnation.
				const destination = { kind: "filesystem", directory: path.join(dir, "backup") } as const
				const backupId = ok(OperationId.parse("11111111-1111-1111-1111-111111111111"))
				const backed = yield* backup(binding, { operationId: backupId, destination, schema: Ledger })
				assert.equal(backed.kind, "completed")
				const verified = yield* verifyBackup(destination, { backup: backupId })
				assert.deepEqual(verified.identity, tenant)
				const again = yield* backup(binding, { operationId: backupId, destination, schema: Ledger })
				assert.deepEqual(again, backed)
				fs.rmSync(path.join(dir, "db"), { recursive: true })
				const restored = yield* restore(
					destination,
					{
						...binding,
						directory: path.join(dir, "restored"),
						identity: { ...tenant, incarnationId: ok(IncarnationId.parse("22222222-2222-2222-2222-222222222222")) }
					},
					{
						operationId: ok(OperationId.parse("33333333-3333-3333-3333-333333333333")),
						backup: backupId,
						schema: Ledger
					}
				)
				assert.equal(restored.kind, "completed")
				if (restored.kind !== "completed") return yield* Effect.die("restore did not complete")
				assert.notEqual(restored.value.identity.incarnationId, tenant.incarnationId)
				const target = restored.value.binding
				assert.equal(target.kind, "local")
				if (target.kind !== "local") return yield* Effect.die("expected local restore")
				yield* Effect.scoped(
					Effect.gen(function* () {
						const history = yield* LocalHistory.open(target, Ledger)
						const snapshot = yield* history.snapshot(readOptions)
						const fact = yield* snapshot.get(EntryById, { id: 42n })
						assert.equal(fact._tag, "Some")
						if (fact._tag === "Some") assert.deepEqual(fact.value, { id: 42n, body: "round trip" })
						const draft = yield* ChangeSet.builder(Ledger)
						yield* draft.insert(Entry, [{ id: 43n, body: "after restore" }])
						const command = yield* Command.seal({
							scope: history.identity,
							id: {
								receiptEpoch: history.receiptEpoch,
								requestId: ok(RequestId.parse("44444444-4444-4444-4444-444444444444"))
							},
							changes: yield* draft.finish(),
							precondition: { kind: "exact-state", at: snapshot.stateStamp },
							result: {}
						})
						const posted = yield* history.submit(command, submitOptions)
						assert.equal(posted.kind, "decided")
						if (posted.kind === "decided") assert.equal(posted.receipt.outcome.kind, "committed")
					})
				)
				return true
			})
		)
		assert.ok(
			Exit.isSuccess(exit),
			`round trip: ${JSON.stringify(exit, (_, value) => (typeof value === "bigint" ? value.toString() : value))}`
		)
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
		fs.rmSync(dir, { recursive: true, force: true })
	}
})
