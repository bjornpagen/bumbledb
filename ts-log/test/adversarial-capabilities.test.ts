/**
 * P12 adversarial integration at the PUBLIC log surface over the REAL native
 * runtime (no scripted double): foreign tenant/cache origin attacks,
 * retained-ref resolution after reopen, retained receipts after close, and
 * closed/foreign capability refusal — REC-03/STORE-08 TS half, ARCH-004,
 * SDK-016, PROTO-02/17 client side, RUN-06, G14.
 *
 * These lanes exercise the production `LocalHistory`/`Command` path end to
 * end and are RED until the wave-C native log verbs (P06R2/P05 over P08's
 * declared `LogNative` roster) integrate — recorded in
 * implementation/packets/P12.md as a cross-lane dependency, owed green at
 * F3. Verification: NotRun (F2 authors, does not execute).
 */
import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { test } from "node:test"
import {
	ChangeSet,
	NativeRuntime,
	relation,
	Schema,
	schema,
	str,
	u64
} from "@bjornpagen/bumbledb"
import type { ExecutionPolicy, NativeRuntimeOptions } from "@bjornpagen/bumbledb"
import { Effect, Exit, ManagedRuntime, Result } from "effect"
import { Command } from "#command.ts"
import { LocalHistory } from "#history.ts"
import type { DatabaseIdentity } from "#identity.ts"
import { DatabaseId, IncarnationId, OperationId, ReceiptEpoch, RequestId } from "#identity.ts"
import type { LocalBinding } from "#options.ts"

const Note = relation("Note", { id: u64, body: str })
const Journal = schema("Journal", { Note }, [])

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

function tempDir(tag: string): string {
	return fs.mkdtempSync(path.join(os.tmpdir(), `bdb-p12-logcap-${tag}-`))
}

function identityOf(schemaId: DatabaseIdentity["schemaId"], seed: string): DatabaseIdentity {
	return {
		databaseId: ok(DatabaseId.fromHex(seed.repeat(16))),
		incarnationId: ok(IncarnationId.fromHex(seed.repeat(16))),
		schemaId
	}
}

const creation = (seed: string) => ({
	operationId: ok(OperationId.fromHex(seed.repeat(16))),
	artifact: new Uint8Array(0)
})

function binding(directory: string, identity: DatabaseIdentity): LocalBinding {
	return { kind: "local", directory, identity }
}

/** Seal one insert command under the given history scope. */
const sealInsert = (
	scope: DatabaseIdentity,
	changes: ChangeSet<typeof Journal>,
	requestSeed: string
) =>
	Command.seal(
		{
			scope,
			id: {
				receiptEpoch: ok(ReceiptEpoch.from(1n)),
				requestId: ok(RequestId.fromHex(requestSeed.repeat(16)))
			},
			changes,
			precondition: { kind: "blind" },
			result: {}
		},
		work
	)

const buildChanges = (rows: ReadonlyArray<{ id: bigint; body: string }>) =>
	Effect.gen(function* () {
		const draft = yield* ChangeSet.builder(Journal, work)
		yield* draft.insert(Note, rows)
		return yield* draft.finish()
	})

test("same-schema cross-origin caches refuse before serving or mutating anything", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	const dirA = tempDir("origin-a")
	const dirB = tempDir("origin-b")
	try {
		const exit = await runtime.runPromiseExit(
			Effect.scoped(
				Effect.gen(function* () {
					const compiled = yield* Schema.compile(Journal, work)
					const tenantA = identityOf(compiled.schemaId, "aa")
					const tenantB = identityOf(compiled.schemaId, "bb")
					// Two tenants, same schema, distinct directories.
					const historyA = yield* LocalHistory.create(binding(dirA, tenantA), Journal, {
						...work,
						creation: creation("a1")
					})
					const changes = yield* buildChanges([{ id: 1n, body: "tenant-a-secret" }])
					const command = yield* sealInsert(historyA.identity, changes, "0a")
					const outcome = yield* historyA.submit(command, submitOptions)
					assert.equal(outcome.kind, "decided")
					yield* historyA.close()
					// The ATTACK: open tenant A's cache directory under tenant
					// B's identity binding. Equal schema and equal revision must
					// not be enough — the origin binding refuses before any
					// fact crosses scope.
					const attack = yield* Effect.exit(
						LocalHistory.open(binding(dirA, tenantB), Journal, work)
					)
					assert.ok(Exit.hasFailures(attack), "the foreign-origin open refuses")
					// Tenant A's data is untouched and still served to A.
					const reopened = yield* LocalHistory.open(binding(dirA, tenantA), Journal, work)
					const snapshot = yield* reopened.snapshot(readOptions)
					const stillThere = yield* snapshot.get(Note, { id: 1n })
					assert.ok(stillThere._tag === "Some", "the refused attack mutated nothing")
					yield* reopened.close()
					return true
				})
			)
		)
		assert.ok(Exit.isSuccess(exit), `cross-origin isolation flow: ${String(exit)}`)
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
		fs.rmSync(dirA, { recursive: true, force: true })
		fs.rmSync(dirB, { recursive: true, force: true })
	}
})

test("a retained command ref resolves after reopen and receipts outlive their scope", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	const dir = tempDir("reopen")
	try {
		const exit = await runtime.runPromiseExit(
			Effect.gen(function* () {
				const compiled = yield* Schema.compile(Journal, work)
				const tenant = identityOf(compiled.schemaId, "cc")
				// Scope 1: create, submit, retain the ref and receipt, close.
				const { ref, receipt } = yield* Effect.scoped(
					Effect.gen(function* () {
						const history = yield* LocalHistory.create(binding(dir, tenant), Journal, {
							...work,
							creation: creation("c1")
						})
						const changes = yield* buildChanges([{ id: 7n, body: "durable" }])
						const command = yield* sealInsert(history.identity, changes, "0c")
						const ref = command.ref
						const outcome = yield* history.submit(command, submitOptions)
						assert.equal(outcome.kind, "decided")
						if (outcome.kind !== "decided") return yield* Effect.die("unreachable")
						return { ref, receipt: outcome.receipt }
					})
				)
				// The retained receipt is plain owned data after its scope died.
				assert.equal(receipt.outcome.kind, "committed")
				// Scope 2: reopen; the pre-dispatch retained ref resolves to the
				// exact recorded outcome (never NotSubmitted, never invented).
				yield* Effect.scoped(
					Effect.gen(function* () {
						const history = yield* LocalHistory.open(binding(dir, tenant), Journal, work)
						const resolved = yield* history.resolve(ref, work)
						assert.equal(resolved.kind, "found")
						if (resolved.kind === "found") {
							assert.equal(resolved.receipt.outcome.kind, "committed")
							assert.deepEqual(resolved.receipt.decisionAt, receipt.decisionAt)
						}
					})
				)
				return true
			})
		)
		assert.ok(Exit.isSuccess(exit), `retained-ref flow: ${String(exit)}`)
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
		fs.rmSync(dir, { recursive: true, force: true })
	}
})

test("closed and foreign capabilities refuse typed without dispatching", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	const dirA = tempDir("cap-a")
	const dirB = tempDir("cap-b")
	try {
		const exit = await runtime.runPromiseExit(
			Effect.scoped(
				Effect.gen(function* () {
					const compiled = yield* Schema.compile(Journal, work)
					const tenantA = identityOf(compiled.schemaId, "dd")
					const tenantB = identityOf(compiled.schemaId, "ee")
					const historyA = yield* LocalHistory.create(binding(dirA, tenantA), Journal, {
						...work,
						creation: creation("d1")
					})
					const historyB = yield* LocalHistory.create(binding(dirB, tenantB), Journal, {
						...work,
						creation: creation("e1")
					})
					// A command sealed for tenant B submitted through tenant A's
					// live handle: refused as certainty (not-submitted with a
					// typed error), zero authoritative dispatch.
					const changes = yield* buildChanges([{ id: 9n, body: "misdirected" }])
					const foreign = yield* sealInsert(historyB.identity, changes, "0e")
					const misdirected = yield* historyA.submit(foreign, submitOptions)
					assert.equal(misdirected.kind, "not-submitted")
					// Close A, then every verb on the retained wrapper refuses
					// typed without dispatch; the receipt table it held is gone
					// from THIS handle but not from the durable directory.
					yield* historyA.close()
					const late = yield* Effect.exit(historyA.inspect(work))
					assert.ok(Exit.hasFailures(late), "a closed capability refuses typed")
					// Tenant B is completely unaffected by A's lifecycle.
					const own = yield* historyB.submit(foreign, submitOptions)
					assert.equal(own.kind, "decided")
					yield* historyB.close()
					return true
				})
			)
		)
		assert.ok(Exit.isSuccess(exit), `capability flow: ${String(exit)}`)
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
		fs.rmSync(dirA, { recursive: true, force: true })
		fs.rmSync(dirB, { recursive: true, force: true })
	}
})

test("open never creates: a missing configured database is a typed refusal, not genesis", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	const dir = tempDir("missing")
	try {
		const exit = await runtime.runPromiseExit(
			Effect.scoped(
				Effect.gen(function* () {
					const compiled = yield* Schema.compile(Journal, work)
					const tenant = identityOf(compiled.schemaId, "0f")
					const attempt = yield* Effect.exit(
						LocalHistory.open(binding(dir, tenant), Journal, work)
					)
					assert.ok(Exit.hasFailures(attempt), "open of a missing database refuses")
					return true
				})
			)
		)
		assert.ok(Exit.isSuccess(exit), `missing-database flow: ${String(exit)}`)
		// The refused open initialized nothing: the directory has no store.
		const entries = fs.existsSync(dir) ? fs.readdirSync(dir) : []
		assert.deepEqual(entries, [], "no hidden genesis, no empty replacement")
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
		fs.rmSync(dir, { recursive: true, force: true })
	}
})

test("a hosted binding under the local constructor refuses before any native work", async () => {
	// Pure pre-dispatch discrimination: no runtime service is even provided.
	const hostile = {
		kind: "hosted",
		origin: { bucket: "b", prefix: "p", region: "r" },
		directory: tempDir("hostile"),
		identity: {
			databaseId: ok(DatabaseId.fromHex("aa".repeat(16))),
			incarnationId: ok(IncarnationId.fromHex("bb".repeat(16))),
			schemaId: "2d".repeat(32) as DatabaseIdentity["schemaId"]
		}
	}
	const exit = await Effect.runPromiseExit(
		Effect.scoped(
			// Deliberate wrong-binding forgery crossing the typed wall.
			LocalHistory.open(hostile as unknown as LocalBinding, Journal, work)
		) as Effect.Effect<unknown, unknown, never>
	)
	assert.ok(Exit.hasFailures(exit), "the backend-discriminated binding refuses pre-dispatch")
	fs.rmSync(hostile.directory, { recursive: true, force: true })
})
