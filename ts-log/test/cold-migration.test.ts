import assert from "node:assert/strict"
import { mkdtemp, readFile, rm } from "node:fs/promises"
import { tmpdir } from "node:os"
import * as path from "node:path"
import { test } from "node:test"
import { ChangeSet, key, NativeRuntime, query, relation, Scalar, Schema, schema, u64, v } from "@bjornpagen/bumbledb"
import { Effect, ManagedRuntime, Result } from "effect"
import { Command } from "#command.ts"
import { LocalHistory } from "#history.ts"
import { DatabaseId, IncarnationId, OperationId, RequestId } from "#identity.ts"
import { activateMigration, initialize, migrate, migrationStatus } from "#migration-ops.ts"
import { decodeGeneratedMigrations } from "#migrations/decode.ts"
import { backfill, migrationIntent } from "#migrations/intent.ts"
import { generateMigrations } from "#migrations/workflow.ts"

const OldStock = relation("Stock", { id: u64, units: u64 })
const Old = schema("Stock", { Stock: OldStock }, [key(OldStock, ["id"])])
const NewStock = relation("Stock", { id: u64, units: u64, next: u64 })
const New = schema("Stock", { Stock: NewStock }, [key(NewStock, ["id"])])
const operation = (digit: string) =>
	Result.getOrThrow(
		OperationId.parse(`${digit.repeat(8)}-${digit.repeat(4)}-${digit.repeat(4)}-${digit.repeat(4)}-${digit.repeat(12)}`)
	)
const loadPlans = (directory: string) =>
	Effect.promise(async () => {
		const manifest = JSON.parse(await readFile(path.join(directory, "manifest.json"), "utf8"))
		const plans = []
		for (const entry of manifest.entries)
			plans.push(JSON.parse(await readFile(path.join(directory, `${entry.id}.plan.json`), "utf8")))
		const decoded = decodeGeneratedMigrations({
			manifest,
			plans,
			snapshots: JSON.parse(await readFile(path.join(directory, "snapshots.json"), "utf8"))
		})
		assert.ok(decoded.ok)
		return decoded.value
	})

test("cold migration opens the source from verified snapshots without a retired schema module", async () => {
	const directory = await mkdtemp(path.join(tmpdir(), "bdb-cold-migration-"))
	const runtime = ManagedRuntime.make(NativeRuntime.layer())
	try {
		await runtime.runPromise(
			Effect.gen(function* () {
				const repository = { directory: path.join(directory, "migrations") }
				yield* generateMigrations({ schema: Old, repository })
				const binding = {
					kind: "local" as const,
					directory: path.join(directory, "store"),
					identity: {
						databaseId: Result.getOrThrow(DatabaseId.parse("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa")),
						incarnationId: Result.getOrThrow(IncarnationId.parse("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb")),
						schemaId: (yield* Schema.compile(Old)).schemaId
					}
				}
				assert.equal(
					(yield* initialize(binding, yield* loadPlans(repository.directory), { operationId: operation("1") })).kind,
					"completed"
				)
				yield* Effect.scoped(
					Effect.gen(function* () {
						const history = yield* LocalHistory.open(binding, Old)
						const draft = yield* ChangeSet.builder(Old)
						yield* draft.insert(OldStock, [{ id: 42n, units: 3n }])
						const command = yield* Command.seal({
							scope: history.identity,
							id: {
								receiptEpoch: history.receiptEpoch,
								requestId: Result.getOrThrow(RequestId.parse("cccccccc-cccc-cccc-cccc-cccccccccccc"))
							},
							precondition: { kind: "blind" },
							changes: yield* draft.finish(),
							result: {}
						})
						const submitted = yield* history.submit(command, { attempts: 2, backoff: { baseMillis: 1, capMillis: 10 } })
						assert.equal(submitted.kind, "decided")
					})
				)
				yield* generateMigrations({
					schema: New,
					repository,
					intent: migrationIntent(New, [backfill(NewStock, "next", Scalar.add(Scalar.field("units"), Scalar.u64(1n)))])
				})
				const plans = yield* loadPlans(repository.directory)
				assert.equal((yield* migrationStatus(binding, plans, {})).kind, "pending")
				const migrated = yield* migrate(binding, plans, { operationId: operation("2") })
				assert.equal(migrated.kind, "completed")
				if (migrated.kind !== "completed") throw new Error(JSON.stringify(migrated))
				assert.equal(migrated.value.kind, "ready-to-switch")
				if (migrated.value.kind !== "ready-to-switch") throw new Error("Migration did not produce a target")
				const { activation, deploymentBinding } = migrated.value
				assert.equal((yield* activateMigration(activation, { binding, schema: New })).kind, "completed")
				assert.equal(deploymentBinding.kind, "local")
				if (deploymentBinding.kind !== "local") throw new Error("Expected local binding")
				yield* Effect.scoped(
					Effect.gen(function* () {
						const restored = yield* LocalHistory.open(deploymentBinding, New)
						const snapshot = yield* restored.snapshot({ consistency: { kind: "latest" } })
						const all = query(New).rule((r) => {
							const row = v(NewStock)
							return r.match(NewStock, row).find(row)
						})
						assert.deepEqual(yield* (yield* snapshot.execute(all, {})).collect(), [{ id: 42n, units: 3n, next: 4n }])
					})
				)
				assert.equal((yield* migrationStatus(deploymentBinding, plans, {})).kind, "up-to-date")
				const retry = yield* migrate(binding, plans, { operationId: operation("2") })
				assert.equal(retry.kind, "completed")
				if (retry.kind !== "completed" || retry.value.kind !== "up-to-date")
					throw new Error("Activated retry did not resolve")
				assert.deepEqual(retry.value.binding, deploymentBinding)
				const edited = { ...plans, snapshots: plans.snapshots.slice(1) }
				assert.ok(Result.isFailure(yield* Effect.result(migrationStatus(deploymentBinding, edited, {}))))
			})
		)
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
		await rm(directory, { recursive: true, force: true })
	}
})
