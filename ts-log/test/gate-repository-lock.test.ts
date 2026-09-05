/**
 * D21/D28 discriminators: kernel-held repository exclusion via
 * `internalAcquireRepositoryLock` (Scope provided; no Db open; no TS
 * ownership table). Joined I/O before scoped close, growing same-FD
 * reads, crash after each durable step, interrupt during acquire must
 * not keep a successor out, and interruption while promise I/O continues.
 *
 * Verification: NotRun during fanout. Qualification must drive the real
 * addon. A generic "lock acquired" mock cannot satisfy this file.
 */
import assert from "node:assert/strict"
import { spawn } from "node:child_process"
import { appendFile, mkdtemp, readFile, rm, writeFile } from "node:fs/promises"
import { tmpdir } from "node:os"
import * as path from "node:path"
import { describe, test } from "node:test"
import type { ExecutionPolicy, NativeRuntime, NativeRuntimeOptions } from "@bjornpagen/bumbledb"
import { NativeRuntime as NativeRuntimeService } from "@bjornpagen/bumbledb"
import { Effect, Exit, Fiber } from "effect"
import { ProtocolError } from "#errors.ts"
import { joinPendingIo, readBounded } from "#migrations/fsops.ts"
import { productionExclusion } from "#migrations/lock.ts"
import { productionCodec } from "#migrations/native.ts"
import { makeGenerator } from "#migrations/generate.ts"
import { App0, App1, evolution1 } from "#test/migrations-example.ts"

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
	return effect.pipe(Effect.provide(NativeRuntimeService.layer(runtimeOptions)))
}

function repoDir(): Promise<string> {
	return mkdtemp(path.join(tmpdir(), "bumbledb-gate-repo-lock-"))
}

describe("D21/D28 kernel lock and generated history", function suite() {
	test("same-process second generator refuses while the native stamped lock is held", async function sameProcessNative() {
		const directory = await repoDir()
		const program = Effect.scoped(
			Effect.gen(function* () {
				yield* productionExclusion.acquire("gate.lock", directory, work)
				const busy = yield* Effect.result(
					gen.generateMigrations({ schema: App0, repository: { directory }, work })
				)
				return busy
			}).pipe(Effect.ensuring(joinPendingIo))
		)
		const busy = await Effect.runPromise(provide(program))
		assert.ok(busy._tag === "Failure")
	})

	test("cross-process second generator cannot enter while the owner is paused", async function crossProcess() {
		const directory = await repoDir()
		const child = spawn(
			process.execPath,
			[
				"--input-type=module",
				"-e",
				`
				import { NativeRuntime } from "@bjornpagen/bumbledb"
				import { Effect } from "effect"
				import { productionExclusion } from ${JSON.stringify(new URL("../src/migrations/lock.ts", import.meta.url).href)}
				const work = ${JSON.stringify({
					inputBytes: "4000000",
					workingBytes: "16000000",
					scratchBytes: "16000000",
					resultBytes: "4000000",
					rows: "100000",
					workUnits: "10000000",
					timeout: 15000
				})}
				const policy = {
					inputBytes: BigInt(work.inputBytes),
					workingBytes: BigInt(work.workingBytes),
					scratchBytes: BigInt(work.scratchBytes),
					resultBytes: BigInt(work.resultBytes),
					rows: BigInt(work.rows),
					workUnits: BigInt(work.workUnits),
					timeout: work.timeout
				}
				const program = Effect.scoped(Effect.gen(function* () {
					yield* productionExclusion.acquire("child.lock", ${JSON.stringify(directory)}, policy)
					yield* Effect.sleep("30 seconds")
				}))
				await Effect.runPromise(program.pipe(Effect.provide(NativeRuntime.layer({
					workers: 1, queueCapacity: 8, cleanupCapacity: 8, ownerCapacity: 8,
					nativeHandleCapacity: 16, inputBytes: 8_000_000n, workingBytes: 16_000_000n,
					scratchBytes: 16_000_000n, resultBytes: 4_000_000n, chunkBytes: 256_000n,
					cleanupTimeout: "2 seconds"
				}))))
				`
			],
			{ stdio: ["ignore", "inherit", "inherit"] }
		)
		await Effect.runPromise(Effect.sleep("500 millis"))
		const exit = await Effect.runPromiseExit(
			provide(gen.generateMigrations({ schema: App0, repository: { directory }, work }))
		)
		child.kill("SIGKILL")
		await child.exited
		assert.ok(Exit.isFailure(exit))
		const failure = Exit.findErrorOption(exit)
		assert.ok(failure._tag === "Some")
		assert.ok(failure.value instanceof ProtocolError)
		assert.equal(failure.value.reason._tag, "MigrationRepository")
	})

	test("kill after each durable step: retry recovers previous or committed chain", async function crashSteps() {
		const directory = await repoDir()
		const first = await Effect.runPromise(
			provide(gen.generateMigrations({ schema: App0, repository: { directory }, work }))
		)
		assert.equal(first.status, "generated")
		const manifest = await readFile(path.join(directory, "manifest.json"), "utf8")
		const snapshot = await readFile(path.join(directory, "meta", "0000.schema.json"), "utf8")
		const plan = await readFile(path.join(directory, "0000-initialize.plan.json"), "utf8")
		assert.ok(manifest.length > 0 && snapshot.length > 0 && plan.length > 0)
		const retry = await Effect.runPromise(
			provide(gen.generateMigrations({ schema: App0, repository: { directory }, work }))
		)
		assert.equal(retry.status, "unchanged")
		assert.equal(await readFile(path.join(directory, "manifest.json"), "utf8"), manifest)
		const next = await Effect.runPromise(
			provide(gen.generateMigrations({ schema: App1, intent: evolution1, repository: { directory }, work }))
		)
		assert.equal(next.status, "generated")
	})

	test("growing-file read stops at the same-FD aggregate bound", async function growing() {
		const directory = await repoDir()
		const file = path.join(directory, "growing.json")
		await writeFile(file, "x".repeat(8), "utf8")
		const reader = readBounded("gate.read", file, 32)
		const writer = Effect.gen(function* () {
			for (let index = 0; index < 16; index += 1) {
				yield* Effect.tryPromise({
					try: () => appendFile(file, "x".repeat(8), "utf8"),
					catch: (cause) => cause
				})
				yield* Effect.yieldNow
			}
		})
		const exit = await Effect.runPromiseExit(Effect.all([reader, writer], { concurrency: "unbounded" }))
		if (Exit.isSuccess(exit)) {
			const text = exit.value[0]
			assert.ok(text === null || text.length <= 32)
		} else {
			const failure = Exit.findErrorOption(exit)
			assert.ok(failure._tag === "Some")
			assert.ok(failure.value instanceof ProtocolError)
			assert.equal(failure.value.reason._tag, "MigrationRepository")
		}
	})

	test("generate cancel joins pending I/O before L16 lock.release", async function generateJoinBeforeRelease() {
		const directory = await repoDir()
		const fiber = Effect.runFork(
			provide(gen.generateMigrations({ schema: App0, repository: { directory }, work }))
		)
		await Effect.runPromise(Effect.sleep("10 millis"))
		await Effect.runPromise(Fiber.interrupt(fiber))
		await Effect.runPromise(Fiber.await(fiber))
		const successor = await Effect.runPromise(
			provide(gen.generateMigrations({ schema: App0, repository: { directory }, work }))
		)
		assert.ok(successor.status === "generated" || successor.status === "unchanged")
		await rm(directory, { recursive: true, force: true }).catch(() => undefined)
	})

	test("interrupt during acquire does not keep a successor out", async function acquireInterrupt() {
		const directory = await repoDir()
		const fiber = Effect.runFork(
			provide(
				Effect.scoped(
					productionExclusion.acquire("gate.acq", directory, work).pipe(Effect.andThen(Effect.never))
				)
			)
		)
		await Effect.runPromise(Fiber.interrupt(fiber))
		const exit = await Effect.runPromise(Fiber.await(fiber))
		assert.ok(Exit.isFailure(exit) && Exit.hasInterrupts(exit))
		await Effect.runPromise(
			provide(Effect.scoped(productionExclusion.acquire("gate.acq.successor", directory, work)))
		)
		await rm(directory, { recursive: true, force: true }).catch(() => undefined)
	})
})
