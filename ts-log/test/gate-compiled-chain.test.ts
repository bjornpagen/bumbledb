/**
 * D20/D27 discriminators: every snapshot is mandatory; native verify/append
 * receive the full snapshot chain plus `compiledMappings` JSON over every
 * plan; symbolic source-field arithmetic is passed through before any
 * manifest write; empty source is not a shortcut; edited/missing snapshots
 * refuse. Uses the production codec (actual native compile).
 *
 * Verification: NotRun during fanout.
 */
import assert from "node:assert/strict"
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises"
import { tmpdir } from "node:os"
import * as path from "node:path"
import { describe, test } from "node:test"
import { NativeRuntime, Scalar, key, relation, schema, u64 } from "@bjornpagen/bumbledb"
import type { ExecutionPolicy, NativeRuntimeOptions } from "@bjornpagen/bumbledb"
import { Effect, Exit } from "effect"
import { ProtocolError } from "#errors.ts"
import { backfill, convert, migrationIntent } from "#migrations/intent.ts"
import { productionExclusion } from "#migrations/lock.ts"
import { productionCodec } from "#migrations/native.ts"
import { makeGenerator } from "#migrations/generate.ts"
import { planExpressionOf } from "#migrations/expr.ts"

const Units0 = relation("Stock", { id: u64, units: u64 })
const AppUnits0 = schema("Units", { Stock: Units0 }, [key(Units0, ["id"])])
const Units1 = relation("Stock", { id: u64, units: u64, next: u64 })
const AppUnits1 = schema("Units", { Stock: Units1 }, [key(Units1, ["id"])])

const runtimeOptions: NativeRuntimeOptions = {
	workers: 2,
	queueCapacity: 16,
	cleanupCapacity: 16,
	ownerCapacity: 16,
	nativeHandleCapacity: 64,
	inputBytes: 16_000_000n,
	workingBytes: 64_000_000n,
	scratchBytes: 64_000_000n,
	resultBytes: 16_000_000n,
	chunkBytes: 1_000_000n,
	cleanupTimeout: "2 seconds"
}

const work: ExecutionPolicy = {
	inputBytes: 4_000_000n,
	workingBytes: 16_000_000n,
	scratchBytes: 16_000_000n,
	resultBytes: 4_000_000n,
	rows: 100_000n,
	workUnits: 10_000_000n,
	timeout: "15 seconds"
}

const gen = makeGenerator(productionCodec, productionExclusion)

function provide<A, E>(effect: Effect.Effect<A, E, NativeRuntime>) {
	return effect.pipe(Effect.provide(NativeRuntime.layer(runtimeOptions)))
}

function repoDir(): Promise<string> {
	return mkdtemp(path.join(tmpdir(), "bumbledb-gate-chain-"))
}

function expectRefusal(exit: Exit.Exit<unknown, unknown>, tag: string): void {
	assert.ok(Exit.isFailure(exit) && Exit.hasFails(exit), "expected a typed refusal")
	const failure = Exit.findErrorOption(exit)
	assert.ok(failure._tag === "Some")
	assert.ok(failure.value instanceof ProtocolError)
	assert.equal(failure.value.reason._tag, tag)
}

describe("D20/D27 full-chain compile and symbolic field arithmetic", function suite() {
	test("Scalar.field arithmetic is authoring AST, not a JS evaluator", function ast() {
		const authored = Scalar.add(Scalar.field("units"), Scalar.u64(1n))
		const outcome = planExpressionOf(authored)
		assert.equal(outcome.ok, true)
		if (outcome.ok) {
			assert.equal(outcome.expression.kind, "add")
			assert.deepEqual(outcome.fields, ["units"])
		}
	})

	test("generate compiles field arithmetic against the verified source snapshot", async function fieldBackfill() {
		const directory = await repoDir()
		const first = await Effect.runPromise(
			provide(gen.generateMigrations({ schema: AppUnits0, repository: { directory }, work }))
		)
		assert.equal(first.status, "generated")
		const evolution = migrationIntent(AppUnits1, [
			backfill(Units1, "next", Scalar.add(Scalar.field("units"), Scalar.u64(1n)))
		])
		const second = await Effect.runPromise(
			provide(
				gen.generateMigrations({
					schema: AppUnits1,
					intent: evolution,
					repository: { directory },
					work
				})
			)
		)
		assert.equal(second.status, "generated")
		const plan = JSON.parse(await readFile(path.join(directory, `${second.planId}.plan.json`), "utf8")) as {
			operations: { kind: string; fields?: { expression: { kind: string } }[] }[]
		}
		const mapped = plan.operations.find((operation) => operation.kind === "map-relation")
		assert.ok(mapped?.fields?.some((field) => field.expression.kind === "add"))
		await rm(directory, { recursive: true, force: true }).catch(() => undefined)
	})

	test("same-schema convert records units+1 instead of returning unchanged", async function sameSchemaConvert() {
		const directory = await repoDir()
		const first = await Effect.runPromise(
			provide(gen.generateMigrations({ schema: AppUnits0, repository: { directory }, work }))
		)
		assert.equal(first.status, "generated")
		const evolution = migrationIntent(AppUnits0, [
			convert(Units0, "units", Scalar.add(Scalar.field("units"), Scalar.u64(1n)))
		])
		const second = await Effect.runPromise(
			provide(
				gen.generateMigrations({
					schema: AppUnits0,
					intent: evolution,
					label: "increment-units",
					repository: { directory },
					work
				})
			)
		)
		assert.equal(second.status, "generated")
		assert.notEqual(second.status, "unchanged")
		const plan = JSON.parse(await readFile(path.join(directory, `${second.planId}.plan.json`), "utf8")) as {
			fromSchemaId: string
			toSchemaId: string
			operations: { kind: string; fields?: { target: string; expression: { kind: string } }[] }[]
		}
		assert.equal(plan.fromSchemaId, plan.toSchemaId)
		const mapped = plan.operations.find((operation) => operation.kind === "map-relation")
		assert.ok(mapped?.fields?.some((field) => field.target === "units" && field.expression.kind === "add"))
		await rm(directory, { recursive: true, force: true }).catch(() => undefined)
	})

	test("edited or missing snapshots refuse before a new manifest is written", async function snapshotsRequired() {
		const directory = await repoDir()
		await Effect.runPromise(provide(gen.generateMigrations({ schema: AppUnits0, repository: { directory }, work })))
		const manifestBefore = await readFile(path.join(directory, "manifest.json"), "utf8")
		await rm(path.join(directory, "meta", "base.schema.json"))
		expectRefusal(
			await Effect.runPromiseExit(
				provide(gen.generateMigrations({ schema: AppUnits0, repository: { directory }, work }))
			),
			"MigrationDrift"
		)
		assert.equal(await readFile(path.join(directory, "manifest.json"), "utf8"), manifestBefore)
		await writeFile(path.join(directory, "meta", "base.schema.json"), "{ \"relations\": [] }\n", "utf8")
		expectRefusal(
			await Effect.runPromiseExit(
				provide(gen.generateMigrations({ schema: AppUnits0, repository: { directory }, work }))
			),
			"MigrationDrift"
		)
		assert.equal(await readFile(path.join(directory, "manifest.json"), "utf8"), manifestBefore)
		await rm(directory, { recursive: true, force: true }).catch(() => undefined)
	})

	test("empty source still supplies the empty-base snapshot to compile", async function emptySource() {
		const directory = await repoDir()
		const report = await Effect.runPromise(
			provide(gen.generateMigrations({ schema: AppUnits0, repository: { directory }, work }))
		)
		assert.equal(report.status, "generated")
		const snapshots = JSON.parse(await readFile(path.join(directory, "snapshots.json"), "utf8")) as string[]
		assert.equal(snapshots.length, 2)
		assert.ok(snapshots[0] !== undefined && snapshots[0].length > 0)
		await rm(directory, { recursive: true, force: true }).catch(() => undefined)
	})
})
