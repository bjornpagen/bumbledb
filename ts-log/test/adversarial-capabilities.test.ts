/**
 * P12 adversarial integration at the PUBLIC log surface over the REAL native
 * runtime (no scripted double): foreign tenant/cache origin attacks,
 * retained-ref resolution after reopen, retained receipts after close, and
 * closed/foreign capability refusal — REC-03/STORE-08 TS half, ARCH-004,
 * SDK-016, PROTO-02/17 client side, RUN-06, G14.
 *
 * These lanes exercise the production `LocalHistory`/`Command` path end to
 * end over the real native runtime. Creation uses the sanctioned checked
 * initialization artifact: the NATIVE-rendered canonical schema snapshot
 * (`productionCodec.schemaIdentity` → `schema_file::render`), whose core v6
 * fingerprint is exactly the creation identity's schemaId — never fabricated
 * client-side bytes.
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
import type { LocalBinding } from "#options.ts"

const Note = relation("Note", { id: u64, body: str })
// `get` reads through the primary (first-declared) key — declare it.
const Journal = schema("Journal", { Note }, [key(Note, ["id"])])

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

/**
 * Mints the VALID checked initialization artifact through the sanctioned
 * flow: the native migration codec renders the canonical schema snapshot
 * (`schema_file::render`) whose fingerprint IS the compiled schemaId the
 * creation identity carries; `check_artifact` re-judges both natively.
 */
const creation = (seed: string) =>
	Effect.gen(function* () {
		const identity = yield* productionCodec.schemaIdentity(lower(Journal), work)
		return {
			operationId: ok(OperationId.fromHex(seed.repeat(16))),
			artifact: new TextEncoder().encode(identity.snapshot)
		}
	})

function binding(directory: string, identity: DatabaseIdentity): LocalBinding {
	return { kind: "local", directory, identity }
}

/** Seal one insert command under the given history scope. */
const sealInsert = (scope: DatabaseIdentity, changes: ChangeSet<typeof Journal>, requestSeed: string) =>
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
						creation: yield* creation("a1")
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
					const attack = yield* Effect.exit(LocalHistory.open(binding(dirA, tenantB), Journal, work))
					assert.ok(Exit.hasFails(attack), "the foreign-origin open refuses")
					// Tenant A's data is untouched and still served to A.
					const reopened = yield* LocalHistory.open(binding(dirA, tenantA), Journal, work)
					const snapshot = yield* reopened.snapshot(readOptions)
					const stillThere = yield* snapshot.get(Note, { id: 1n }, work)
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
							creation: yield* creation("c1")
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
						creation: yield* creation("d1")
					})
					const historyB = yield* LocalHistory.create(binding(dirB, tenantB), Journal, {
						...work,
						creation: yield* creation("e1")
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
					assert.ok(Exit.hasFails(late), "a closed capability refuses typed")
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
					const attempt = yield* Effect.exit(LocalHistory.open(binding(dir, tenant), Journal, work))
					assert.ok(Exit.hasFails(attempt), "open of a missing database refuses")
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

test("a hosted binding under the local constructor refuses typed with no genesis", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	const dir = tempDir("hostile")
	const hostile = {
		kind: "hosted",
		origin: { bucket: "b", prefix: "p", region: "r" },
		directory: dir,
		identity: {
			databaseId: ok(DatabaseId.fromHex("aa".repeat(16))),
			incarnationId: ok(IncarnationId.fromHex("bb".repeat(16))),
			schemaId: "2d".repeat(32) as DatabaseIdentity["schemaId"]
		}
	}
	try {
		const exit = await runtime.runPromiseExit(
			Effect.scoped(
				// Deliberate wrong-binding forgery crossing the typed wall.
				LocalHistory.open(hostile as unknown as LocalBinding, Journal, work)
			)
		)
		assert.ok(Exit.hasFails(exit), "the backend-discriminated binding refuses TYPED, never a defect")
		// The refusal did no native work: the directory holds no genesis.
		assert.deepEqual(fs.readdirSync(dir), [], "the refused forgery materialized nothing")
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
		fs.rmSync(dir, { recursive: true, force: true })
	}
})
