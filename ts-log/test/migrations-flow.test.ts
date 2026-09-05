/**
 * The Drizzle-like edit → generate → review → (apply-boundary) repo workflow
 * over the scripted codec (TS-MIG-01/03/04/10 language-layer side; OPS-001
 * generator half). Covers: initial generation from the empty base, unchanged
 * reruns writing nothing, deterministic byte-identical output across
 * repositories, ambiguous/destructive refusal with zero writes, the complete
 * staged example history handed to P13, seed lowering through the CORE cell
 * codec, one-shot seed iterables, budget refusal, and history replay — the
 * generated data decodes back to exactly the `GeneratedMigrations` value the
 * `migrate()` runner consumes. Native digests/execution are P09 + F3 lanes.
 */
import assert from "node:assert/strict"
import { mkdtemp, readdir, readFile, writeFile } from "node:fs/promises"
import { tmpdir } from "node:os"
import * as path from "node:path"
import { describe, test } from "node:test"
import type { NativeRuntime } from "@bjornpagen/bumbledb"
import { Effect, Exit } from "effect"
import { ProtocolError } from "#errors.ts"
import { decodeGeneratedMigrations } from "#migrations/decode.ts"
import { makeGenerator } from "#migrations/generate.ts"
import type { GeneratedMigrations, MigrationPlan } from "#migrations/types.ts"
import { scriptedCodec, withStubRuntime, WORK } from "#test/migrations-double.ts"
import { App0, App1, App2, App3, evolution1, evolution2, evolution3 } from "#test/migrations-example.ts"
import { migrationIntent, seed } from "#migrations/intent.ts"

const gen = makeGenerator(scriptedCodec())

function run<A, E>(effect: Effect.Effect<A, E, NativeRuntime>): Promise<A> {
	return Effect.runPromise(withStubRuntime(effect))
}

function runExit<A, E>(effect: Effect.Effect<A, E, NativeRuntime>): Promise<Exit.Exit<A, E>> {
	return Effect.runPromiseExit(withStubRuntime(effect))
}

function repoDir(): Promise<string> {
	return mkdtemp(path.join(tmpdir(), "bumbledb-migrations-"))
}

/** Every repo file (recursive), name → exact text. */
async function repoFiles(directory: string): Promise<Map<string, string>> {
	const out = new Map<string, string>()
	let names: string[]
	try {
		names = await readdir(directory, { recursive: true })
	} catch {
		return out
	}
	for (const name of names.sort()) {
		try {
			out.set(name, await readFile(path.join(directory, name), "utf8"))
		} catch {
			// a directory entry
		}
	}
	return out
}

/** Build the exact value the generated static index would export. */
async function loadGenerated(directory: string): Promise<GeneratedMigrations> {
	const manifest = JSON.parse(await readFile(path.join(directory, "manifest.json"), "utf8")) as GeneratedMigrations["manifest"]
	const plans: MigrationPlan[] = []
	for (const entry of manifest.entries) {
		plans.push(JSON.parse(await readFile(path.join(directory, `${entry.id}.plan.json`), "utf8")) as MigrationPlan)
	}
	return { manifest, plans }
}

function expectIntentRequired(exit: Exit.Exit<unknown, unknown>): readonly { code: string; relation: string; field: string | null }[] {
	assert.ok(Exit.isFailure(exit) && Exit.hasFails(exit), "expected a typed refusal")
	const failure = Exit.findErrorOption(exit)
	assert.ok(failure._tag === "Some")
	const error = failure.value
	assert.ok(error instanceof ProtocolError, "expected ProtocolError")
	const reason = error.reason as { _tag: string; requirements?: readonly { code: string; relation: string; field: string | null }[] }
	assert.equal(reason._tag, "MigrationIntentRequired")
	assert.ok(Array.isArray(reason.requirements))
	return reason.requirements ?? []
}

describe("generate / check flow", function suite() {
	test("initial generation from the empty base records the whole chain commit", async function initial() {
		const directory = await repoDir()
		const report = await run(gen.generateMigrations({ schema: App0, repository: { directory }, work: WORK }))
		assert.equal(report.status, "generated")
		assert.equal(report.planId, "0000-initialize")
		assert.deepEqual(report.files, [
			"meta/0000.schema.json",
			"0000-initialize.plan.json",
			"manifest.json",
			"index.ts",
			"runtime-contract.json"
		])
		const files = await repoFiles(directory)
		for (const name of report.files) {
			assert.ok(files.has(name), `missing generated file ${name}`)
		}
		const generated = await loadGenerated(directory)
		const decoded = decodeGeneratedMigrations(generated)
		assert.ok(decoded.ok, "generated data must decode as runner input")
		assert.equal(decoded.value.manifest.entries.length, 1)
		const plan = decoded.value.plans[0]
		assert.ok(plan !== undefined)
		// Initial plan: create the relation empty, then the required final judgment.
		assert.deepEqual(
			plan.operations.map((op) => op.kind),
			["empty-relation", "validate-schema"]
		)
		const last = plan.operations[plan.operations.length - 1]
		assert.ok(last !== undefined && last.kind === "validate-schema")
		assert.equal(last.schemaId, plan.toSchemaId)
		// The runtime contract carries the recorded head expectation.
		assert.equal(report.contract.schemaId, plan.toSchemaId)
		assert.equal(report.contract.steps, "1")
		const contractText = files.get("runtime-contract.json")
		assert.ok(contractText !== undefined && contractText.includes(plan.toSchemaId))
	})

	test("rerun is unchanged and writes nothing; check is clean and writes nothing", async function rerun() {
		const directory = await repoDir()
		await run(gen.generateMigrations({ schema: App0, repository: { directory }, work: WORK }))
		const before = await repoFiles(directory)
		const again = await run(gen.generateMigrations({ schema: App0, repository: { directory }, work: WORK }))
		assert.equal(again.status, "unchanged")
		assert.equal(again.planId, null)
		assert.deepEqual(again.files, [])
		assert.deepEqual(again.removed, [])
		const check = await run(gen.checkMigrations({ schema: App0, repository: { directory }, work: WORK }))
		assert.equal(check.status, "clean")
		const after = await repoFiles(directory)
		assert.deepEqual([...after.entries()], [...before.entries()], "unchanged/check must not modify the repository")
	})

	test("output is deterministic: two fresh repositories agree byte for byte", async function deterministic() {
		const a = await repoDir()
		const b = await repoDir()
		await run(gen.generateMigrations({ schema: App0, repository: { directory: a }, work: WORK }))
		await run(gen.generateMigrations({ schema: App1, intent: evolution1, repository: { directory: a }, work: WORK }))
		await run(gen.generateMigrations({ schema: App0, repository: { directory: b }, work: WORK }))
		await run(gen.generateMigrations({ schema: App1, intent: evolution1, repository: { directory: b }, work: WORK }))
		assert.deepEqual([...(await repoFiles(a)).entries()], [...(await repoFiles(b)).entries()])
	})

	test("a new required field refuses without typed intent, writing nothing", async function ambiguous() {
		const directory = await repoDir()
		await run(gen.generateMigrations({ schema: App0, repository: { directory }, work: WORK }))
		const before = await repoFiles(directory)
		const exit = await runExit(gen.generateMigrations({ schema: App1, repository: { directory }, work: WORK }))
		const requirements = expectIntentRequired(exit)
		assert.deepEqual(
			requirements.map((entry) => ({ code: entry.code, relation: entry.relation, field: entry.field })),
			[{ code: "missing-backfill", relation: "Note", field: "pinned" }]
		)
		const after = await repoFiles(directory)
		assert.deepEqual([...after.entries()], [...before.entries()], "a refusal must not write")
		// check refuses identically — same computation, no files.
		const checkExit = await runExit(gen.checkMigrations({ schema: App1, repository: { directory }, work: WORK }))
		expectIntentRequired(checkExit)
	})

	test("the complete staged example history generates end to end (P13 handoff)", async function example() {
		const directory = await repoDir()
		await run(gen.generateMigrations({ schema: App0, repository: { directory }, work: WORK }))
		const one = await run(gen.generateMigrations({ schema: App1, intent: evolution1, repository: { directory }, work: WORK }))
		assert.equal(one.planId, "0001-note")
		const two = await run(gen.generateMigrations({ schema: App2, intent: evolution2, repository: { directory }, work: WORK }))
		assert.equal(two.planId, "0002-create-tag-seed-tag")
		const three = await run(gen.generateMigrations({ schema: App3, intent: evolution3, repository: { directory }, work: WORK }))
		assert.equal(three.planId, "0003-note")
		const generated = await loadGenerated(directory)
		const decoded = decodeGeneratedMigrations(generated)
		assert.ok(decoded.ok)
		const { manifest, plans } = decoded.value
		assert.equal(manifest.entries.length, 4)
		// The chain is contiguous: every step starts where its predecessor ended.
		for (let index = 1; index < manifest.entries.length; index += 1) {
			assert.equal(manifest.entries[index]?.fromSchemaId, manifest.entries[index - 1]?.toSchemaId)
		}
		assert.equal(manifest.entries[0]?.fromSchemaId, manifest.baseSchemaId)
		// 0001: chapter 33's worked example — complete projection, typed literal.
		const noteMap = plans[1]?.operations[0]
		assert.ok(noteMap !== undefined && noteMap.kind === "map-relation")
		assert.equal(noteMap.source, "Note")
		assert.equal(noteMap.target, "Note")
		assert.deepEqual(noteMap.fields, [
			{ target: "id", expression: { kind: "field", name: "id" } },
			{ target: "body", expression: { kind: "field", name: "body" } },
			{ target: "pinned", expression: { kind: "literal", value: { bool: false } } }
		])
		assert.deepEqual(plans[1]?.destructive, [])
		// 0002: unchanged Note preserved automatically; Tag created and seeded
		// through the CORE cell codec into canonical one-arm values.
		const kinds2 = plans[2]?.operations.map((op) => op.kind)
		assert.deepEqual(kinds2, ["map-relation", "empty-relation", "seed", "validate-schema"])
		const seedOp = plans[2]?.operations[2]
		assert.ok(seedOp !== undefined && seedOp.kind === "seed")
		assert.equal(seedOp.target, "Tag")
		assert.deepEqual(seedOp.rows, [
			[{ u64: "1" }, { string: "inbox" }],
			[{ u64: "2" }, { string: "archive" }]
		])
		// 0003: the rename is an identity projection from the OLD field name;
		// nothing is lost, so no destructive acknowledgement exists.
		const renameMap = plans[3]?.operations[0]
		assert.ok(renameMap !== undefined && renameMap.kind === "map-relation")
		assert.deepEqual(renameMap.fields, [
			{ target: "id", expression: { kind: "field", name: "id" } },
			{ target: "text", expression: { kind: "field", name: "body" } },
			{ target: "pinned", expression: { kind: "field", name: "pinned" } }
		])
		assert.deepEqual(plans[3]?.destructive, [])
		// History replay: the whole recorded chain re-verifies cleanly.
		const check = await run(gen.checkMigrations({ schema: App3, repository: { directory }, work: WORK }))
		assert.equal(check.status, "clean")
		assert.equal(check.contract.steps, "4")
	})

	test("seed ingestion is budgeted and reads the caller iterable exactly once", async function seeds() {
		// Fresh repo initialized at the App1 stage; the seed stage lands next.
		const dir2 = await repoDir()
		await run(gen.generateMigrations({ schema: App1, repository: { directory: dir2 }, work: WORK }))
		// Budget refusal: two seed rows against a one-row budget.
		const tight = { ...WORK, rows: 1n }
		const exit = await runExit(
			gen.generateMigrations({ schema: App2, intent: evolution2, repository: { directory: dir2 }, work: tight })
		)
		assert.ok(Exit.isFailure(exit) && Exit.hasFails(exit))
		const failure = Exit.findErrorOption(exit)
		assert.ok(failure._tag === "Some")
		const error = failure.value as { reason?: { _tag?: string } }
		assert.equal(error.reason?._tag, "ResourceLimit", "seed budgets use the core resource reason")
		// One-shot iterables are read once and never replayed by the SDK.
		let pulls = 0
		function* once(): Generator<{ id: bigint; name: string }> {
			pulls += 1
			yield { id: 7n, name: "solo" }
		}
		const oneShot = migrationIntent(App2, [seed(App2.relations.Tag, once())])
		assert.equal(pulls, 0, "constructing intent must not consume the iterable")
		const report = await run(
			gen.generateMigrations({ schema: App2, intent: oneShot, repository: { directory: dir2 }, work: WORK })
		)
		assert.equal(report.status, "generated")
		assert.equal(pulls, 1)
		const generated = await loadGenerated(dir2)
		const decoded = decodeGeneratedMigrations(generated)
		assert.ok(decoded.ok)
		const seeded = decoded.value.plans[decoded.value.plans.length - 1]?.operations.find((op) => op.kind === "seed")
		assert.ok(seeded !== undefined && seeded.kind === "seed")
		assert.deepEqual(seeded.rows, [[{ u64: "7" }, { string: "solo" }]])
	})

	test("a throwing seed iterator is a typed input failure, never a partial artifact", async function hostile() {
		const directory = await repoDir()
		await run(gen.generateMigrations({ schema: App1, repository: { directory }, work: WORK }))
		function* poison(): Generator<{ id: bigint; name: string }> {
			yield { id: 1n, name: "ok" }
			throw new Error("hostile iterator")
		}
		const before = await repoFiles(directory)
		const exit = await runExit(
			gen.generateMigrations({
				schema: App2,
				intent: migrationIntent(App2, [seed(App2.relations.Tag, poison())]),
				repository: { directory },
				work: WORK
			})
		)
		assert.ok(Exit.isFailure(exit) && Exit.hasFails(exit))
		assert.deepEqual([...(await repoFiles(directory)).entries()], [...before.entries()])
	})

	test("an interrupted-generation leftover is rewritten deterministically, not drift", async function leftovers() {
		const directory = await repoDir()
		await run(gen.generateMigrations({ schema: App0, repository: { directory }, work: WORK }))
		// Simulate a crash between plan write and manifest write: an unrecorded
		// next-sequence plan under a DIFFERENT derived label.
		await writeFile(path.join(directory, "0001-abandoned.plan.json"), "{}\n", "utf8")
		const check = await run(gen.checkMigrations({ schema: App0, repository: { directory }, work: WORK }))
		assert.equal(check.status, "generation-required")
		const report = await run(
			gen.generateMigrations({ schema: App1, intent: evolution1, repository: { directory }, work: WORK })
		)
		assert.equal(report.status, "generated")
		assert.deepEqual(report.removed, ["0001-abandoned.plan.json"])
		const files = await repoFiles(directory)
		assert.ok(!files.has("0001-abandoned.plan.json"), "the unrecorded leftover is gone")
		assert.ok(files.has("0001-note.plan.json"))
		// Recorded intent is consumed intent: the later check runs WITHOUT it
		// (a leftover intent matching no change is a stale-intent refusal).
		const clean = await run(gen.checkMigrations({ schema: App1, repository: { directory }, work: WORK }))
		assert.equal(clean.status, "clean")
	})

	test("already-recorded intent is stale on the next run, never silently ignored", async function consumed() {
		const directory = await repoDir()
		await run(gen.generateMigrations({ schema: App0, repository: { directory }, work: WORK }))
		await run(gen.generateMigrations({ schema: App1, intent: evolution1, repository: { directory }, work: WORK }))
		const exit = await runExit(gen.checkMigrations({ schema: App1, intent: evolution1, repository: { directory }, work: WORK }))
		const requirements = expectIntentRequired(exit)
		assert.deepEqual(requirements.map((entry) => entry.code), ["stale-intent"])
	})

	test("intent declared for a different schema value refuses before any work", async function foreignIntent() {
		const directory = await repoDir()
		const exit = await runExit(
			gen.generateMigrations({ schema: App0, intent: evolution1, repository: { directory }, work: WORK })
		)
		assert.ok(Exit.isFailure(exit) && Exit.hasFails(exit))
		assert.deepEqual([...(await repoFiles(directory)).entries()], [], "nothing is written")
	})
})
