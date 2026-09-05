/**
 * Repository drift/fork/tamper refusal (TS-MIG-01/02 generator side, MIG-07
 * language layer): recorded artifacts are immutable — an edited plan, an
 * edited or reordered manifest, a missing snapshot or a stray unrecorded file
 * refuses BEFORE any write, by exact recomputation from the file bytes
 * actually on disk. Bounded reads refuse oversize files before loading them.
 */
import assert from "node:assert/strict"
import { mkdir, mkdtemp, readdir, readFile, rm, writeFile } from "node:fs/promises"
import { tmpdir } from "node:os"
import * as path from "node:path"
import { describe, test } from "node:test"
import { key, relation, schema, str, u64 } from "@bjornpagen/bumbledb"
import type { NativeRuntime } from "@bjornpagen/bumbledb"
import { Effect, Exit } from "effect"
import { ProtocolError } from "#errors.ts"
import { makeGenerator } from "#migrations/generate.ts"
import { readBounded, writeDerived, writeImmutable, writeManifest } from "#migrations/fsops.ts"
import { readRepository } from "#migrations/repo.ts"
import { scriptedCodec, scriptedExclusion, withStubRuntime, WORK } from "#test/migrations-double.ts"
import { App0, App1, evolution1, Note0 } from "#test/migrations-example.ts"

const gen = makeGenerator(scriptedCodec(), scriptedExclusion())

function run<A, E>(effect: Effect.Effect<A, E, NativeRuntime>): Promise<A> {
	return Effect.runPromise(withStubRuntime(effect))
}

function runExit<A, E>(effect: Effect.Effect<A, E, NativeRuntime>): Promise<Exit.Exit<A, E>> {
	return Effect.runPromiseExit(withStubRuntime(effect))
}

function repoDir(): Promise<string> {
	return mkdtemp(path.join(tmpdir(), "bumbledb-migrations-repo-"))
}

async function history(directory: string): Promise<void> {
	await run(gen.generateMigrations({ schema: App0, repository: { directory }, work: WORK }))
	await run(gen.generateMigrations({ schema: App1, intent: evolution1, repository: { directory }, work: WORK }))
}

function expectRefusal(exit: Exit.Exit<unknown, unknown>, tag: string): void {
	assert.ok(Exit.isFailure(exit) && Exit.hasFails(exit), "expected a typed refusal")
	const failure = Exit.findErrorOption(exit)
	assert.ok(failure._tag === "Some")
	const error = failure.value
	assert.ok(error instanceof ProtocolError, `expected ProtocolError, got ${String(error)}`)
	assert.equal(error.reason._tag, tag)
}

describe("drift, fork and tamper refuse before writes", function suite() {
	test("an edited recorded plan no longer matches its recorded digest", async function tamper() {
		const directory = await repoDir()
		await history(directory)
		const planPath = path.join(directory, "0001-note.plan.json")
		const text = await readFile(planPath, "utf8")
		// Semantics-preserving-looking edit: flip the backfilled default.
		const edited = text.replace('"bool": false', '"bool": true')
		assert.notEqual(edited, text, "the fixture edit must hit the literal")
		await writeFile(planPath, edited, "utf8")
		expectRefusal(await runExit(gen.checkMigrations({ schema: App1, repository: { directory }, work: WORK })), "MigrationDrift")
		// Generation refuses on the same recomputation, before any write.
		expectRefusal(
			await runExit(gen.generateMigrations({ schema: App1, repository: { directory }, work: WORK })),
			"MigrationDrift"
		)
	})

	test("an edited manifest entry refuses: identity is recomputation, not text", async function manifest() {
		const directory = await repoDir()
		await history(directory)
		const manifestPath = path.join(directory, "manifest.json")
		const tree = JSON.parse(await readFile(manifestPath, "utf8")) as {
			entries: { planDigest: string; prefixDigest: string }[]
		}
		const first = tree.entries[0]
		assert.ok(first !== undefined)
		first.planDigest = first.planDigest.split("").reverse().join("")
		await writeFile(manifestPath, `${JSON.stringify(tree, null, "\t")}\n`, "utf8")
		expectRefusal(await runExit(gen.checkMigrations({ schema: App1, repository: { directory }, work: WORK })), "MigrationDrift")
	})

	test("reordered manifest entries refuse as a broken chain", async function reorder() {
		const directory = await repoDir()
		await history(directory)
		const manifestPath = path.join(directory, "manifest.json")
		const tree = JSON.parse(await readFile(manifestPath, "utf8")) as { entries: unknown[] }
		tree.entries.reverse()
		await writeFile(manifestPath, `${JSON.stringify(tree, null, "\t")}\n`, "utf8")
		expectRefusal(await runExit(gen.checkMigrations({ schema: App1, repository: { directory }, work: WORK })), "MigrationDrift")
	})

	test("a missing recorded snapshot or plan file is drift, not a fresh start", async function missing() {
		const directory = await repoDir()
		await history(directory)
		await rm(path.join(directory, "meta", "0001.schema.json"))
		expectRefusal(await runExit(gen.checkMigrations({ schema: App1, repository: { directory }, work: WORK })), "MigrationDrift")
		const second = await repoDir()
		await history(second)
		await rm(path.join(second, "0000-initialize.plan.json"))
		expectRefusal(await runExit(gen.checkMigrations({ schema: App1, repository: { directory: second }, work: WORK })), "MigrationDrift")
		const third = await repoDir()
		await history(third)
		await rm(path.join(third, "meta", "base.schema.json"))
		expectRefusal(await runExit(gen.checkMigrations({ schema: App1, repository: { directory: third }, work: WORK })), "MigrationDrift")
	})

	test("same-process duplicate generation refuses while the first holds exclusion", async function sameProcess() {
		const directory = await repoDir()
		const held = await run(scriptedExclusion().acquire("test", directory, WORK))
		const busy = await runExit(gen.generateMigrations({ schema: App0, repository: { directory }, work: WORK }))
		expectRefusal(busy, "MigrationRepository")
		await run(held.release)
		const first = await run(gen.generateMigrations({ schema: App0, repository: { directory }, work: WORK }))
		assert.equal(first.status, "generated")
	})

	test("a stray plan that is not the next sequence is drift, never adopted", async function stray() {
		const directory = await repoDir()
		await history(directory)
		await writeFile(path.join(directory, "0007-imposter.plan.json"), "{}\n", "utf8")
		expectRefusal(await runExit(gen.checkMigrations({ schema: App1, repository: { directory }, work: WORK })), "MigrationDrift")
	})

	test("a forked twin repository cannot lend its recorded plan bytes", async function fork() {
		// Two repos share plan 0, then diverge; a plan file transplanted from
		// the fork under A's recorded name is caught by A's recorded digest.
		const Extra = relation("Extra", { id: u64, note: str })
		const AppFork = schema("App", { Note: Note0, Extra }, [key(Note0, ["id"]), key(Extra, ["id"])])
		const a = await repoDir()
		const b = await repoDir()
		await history(a)
		await run(gen.generateMigrations({ schema: App0, repository: { directory: b }, work: WORK }))
		const forked = await run(gen.generateMigrations({ schema: AppFork, repository: { directory: b }, work: WORK }))
		assert.equal(forked.planId, "0001-create-extra")
		// Transplant B's 0001 plan into A under A's recorded name.
		const foreign = await readFile(path.join(b, "0001-create-extra.plan.json"), "utf8")
		await writeFile(path.join(a, "0001-note.plan.json"), foreign, "utf8")
		expectRefusal(await runExit(gen.checkMigrations({ schema: App1, repository: { directory: a }, work: WORK })), "MigrationDrift")
	})

	test("readRepository reports interrupted next-sequence drafts and refuses partial chains", async function drafts() {
		const directory = await repoDir()
		// A plan file with NO manifest is a partial chain (only sequence 0 is a
		// tolerated interrupted first generation).
		await writeFile(path.join(directory, "0003-ghost.plan.json"), "{}\n", "utf8")
		const exit = await runExit(readRepository({ directory }))
		expectRefusal(exit, "MigrationDrift")
		await rm(path.join(directory, "0003-ghost.plan.json"))
		await writeFile(path.join(directory, "0000-first-try.plan.json"), "{}\n", "utf8")
		const state = await run(readRepository({ directory }))
		assert.deepEqual(state.staleDrafts, ["0000-first-try.plan.json"])
		assert.equal(state.manifest, null)
	})
})

describe("bounded interruption-safe filesystem work", function suite() {
	test("reads are size-bounded before any byte is loaded; absent is null, not empty", async function bounded() {
		const directory = await repoDir()
		const file = path.join(directory, "big.json")
		await writeFile(file, "x".repeat(64), "utf8")
		const capped = await runExit(readBounded("test", file, 16))
		assert.ok(Exit.isFailure(capped))
		const fits = await run(readBounded("test", file, 1024))
		assert.equal(fits, "x".repeat(64))
		const absent = await run(readBounded("test", path.join(directory, "missing.json"), 16))
		assert.equal(absent, null)
	})

	test("writeManifest commits whole files and leaves no temporary behind", async function atomic() {
		const directory = await repoDir()
		const file = path.join(directory, "artifact.json")
		await run(writeManifest("test", file, "one\n"))
		await run(writeManifest("test", file, "two\n"))
		assert.equal(await readFile(file, "utf8"), "two\n")
		const names = await readdir(directory)
		assert.deepEqual(names, ["artifact.json"], "no temporary sibling survives a completed commit")
		const clash = path.join(directory, "clash")
		await mkdir(clash)
		await mkdir(path.join(clash, "sub"))
		const failed = await runExit(writeDerived("test", clash, "text\n"))
		assert.ok(Exit.isFailure(failed))
		const after = await readdir(directory)
		assert.deepEqual(after.sort(), ["artifact.json", "clash"], "the temporary was removed on failure")
	})

	test("writeImmutable is no-clobber and accepts only identical existing content", async function noclobber() {
		const directory = await repoDir()
		const file = path.join(directory, "plan.json")
		await run(writeImmutable("test", file, "same\n"))
		await run(writeImmutable("test", file, "same\n"))
		assert.equal(await readFile(file, "utf8"), "same\n")
		const clash = await runExit(writeImmutable("test", file, "other\n"))
		assert.ok(Exit.isFailure(clash))
		assert.equal(await readFile(file, "utf8"), "same\n")
	})

	test("PID/stale-lock and stat-then-read predecessors are deleted", async function deletions() {
		const fsops = await import("#migrations/fsops.ts")
		assert.equal("acquireGenerationLock" in fsops, false)
		assert.equal("releaseGenerationLock" in fsops, false)
		assert.equal("writeAtomic" in fsops, false)
		assert.equal("processAlive" in fsops, false)
		assert.equal("readLockPid" in fsops, false)
	})

	test("invalid UTF-8 on the same descriptor is fatal", async function utf8() {
		const directory = await repoDir()
		const file = path.join(directory, "bad.json")
		await writeFile(file, Buffer.from([0xff, 0xfe, 0x00]))
		const exit = await runExit(readBounded("test", file, 1024))
		assert.ok(Exit.isFailure(exit))
	})
})
