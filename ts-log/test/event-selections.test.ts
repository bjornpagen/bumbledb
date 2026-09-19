import assert from "node:assert/strict"
import { randomUUID } from "node:crypto"
import { mkdtempSync, rmSync } from "node:fs"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { test } from "node:test"
import {
	ChangeSet,
	closed,
	contained,
	Event,
	event,
	key,
	NativeRuntime,
	on,
	query,
	relation,
	Schema,
	schema,
	select,
	u64,
	v
} from "@bjornpagen/bumbledb"
import { Effect, ManagedRuntime, Result } from "effect"
import { backup, restore, verifyBackup } from "#admin.ts"
import { Command } from "#command.ts"
import { LocalHistory } from "#history.ts"
import { DatabaseId, IncarnationId, OperationId, RequestId } from "#identity.ts"
import { schemaSnapshot } from "#schema.ts"
import { TenantCache } from "#tenants.ts"
import { Transition } from "#transition.ts"

const Child = relation("Child", { id: u64, filter: event })
const Parent = relation("Parent", { id: u64, filter: event })
const ChildById = key(Child, ["id"])
const theory = (literal: Event) => {
	const Known = closed("Known", ["Literal"], { when: event }, { Literal: { when: literal } })
	return schema("SelectedLog", { Child, Parent, Known }, [
		ChildById,
		key(Parent, ["id"]),
		key(Known, ["when"]),
		contained(on(select(Child, { filter: literal }), "id"), on(select(Parent, { filter: literal }), "id"))
	])
}

const options = {
	workers: 2,
	queueCapacity: 16,
	cleanupCapacity: 16,
	ownerCapacity: 16,
	nativeHandleCapacity: 128,
	cleanupTimeout: "2 seconds"
} as const
const submit = { attempts: 1, backoff: { baseMillis: 0, capMillis: 0 } }
const read = { consistency: { kind: "cached" } } as const
const operationId = () => Result.getOrThrow(OperationId.parse(randomUUID()))
const requestId = () => Result.getOrThrow(RequestId.parse(randomUUID()))
const incarnationId = () => Result.getOrThrow(IncarnationId.parse(randomUUID()))

test("Event-selected log schemas survive command recovery, runtime release, cache, backup and restore", async () => {
	const directory = mkdtempSync(join(tmpdir(), "bumbledb-event-log-"))
	let runtime = ManagedRuntime.make(NativeRuntime.layer(options))
	try {
		const saved = await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const full = yield* Event.space(new Uint8Array(32).fill(211), 2n)
					const literal = yield* Event.coordinate(full, 0n)
					const s = theory(literal)
					const identity = {
						databaseId: Result.getOrThrow(DatabaseId.parse(randomUUID())),
						incarnationId: incarnationId(),
						schemaId: (yield* Schema.compile(s)).schemaId
					}
					const binding = { kind: "local", directory, identity } as const
					const history = yield* LocalHistory.create(binding, s, {
						creation: {
							operationId: operationId(),
							artifact: new TextEncoder().encode(yield* schemaSnapshot(s))
						}
					})
					const builder = yield* ChangeSet.builder(s)
					yield* builder.insert(Child, [
						{ id: 1n, filter: literal },
						{ id: 2n, filter: full }
					])
					yield* builder.insert(Parent, [{ id: 1n, filter: literal }])
					const command = yield* Command.seal({
						scope: identity,
						id: { receiptEpoch: history.receiptEpoch, requestId: requestId() },
						changes: yield* builder.finish(),
						precondition: { kind: "blind" },
						result: { saved: true }
					})
					const bytes = yield* Command.encode(command)
					const decoded = yield* Command.decode(
						bytes,
						theory(Result.getOrThrow(Event.fromBytes(Event.toBytes(literal))))
					)
					assert.deepEqual(decoded.ref, command.ref)
					const posted = yield* history.submit(decoded, submit)
					assert.equal(posted.kind, "decided")
					if (posted.kind === "decided") assert.equal(posted.receipt.outcome.kind, "committed")
					return { binding, bytes, literal: Event.toBytes(literal), full: Event.toBytes(full), ref: command.ref }
				})
			)
		)
		await runtime.dispose()
		runtime = ManagedRuntime.make(NativeRuntime.layer(options))
		await runtime.runPromise(
			Effect.gen(function* () {
				const literal = Result.getOrThrow(Event.fromBytes(saved.literal))
				const full = Result.getOrThrow(Event.fromBytes(saved.full))
				const s = theory(literal)
				yield* Effect.scoped(
					Effect.gen(function* () {
						const cache = yield* TenantCache.make(s, { maxOpen: 1 })
						const history = yield* cache.acquire(saved.binding)
						const decoded = yield* Command.decode(saved.bytes, s)
						assert.deepEqual(decoded.ref, saved.ref)
						assert.equal((yield* history.resolve(saved.ref)).kind, "found")
						const snapshot = yield* history.snapshot(read)
						const row = yield* snapshot.get(ChildById, { id: 1n })
						assert.ok(row._tag === "Some")
						assert.deepEqual(Event.toBytes(row.value.filter), saved.literal)
						const ground = query(s).rule((r) => {
							const row = v(s.relations.Known)
							return r.match(s.relations.Known, row).find({ when: row.when })
						})
						const groundRows = yield* (yield* snapshot.execute(ground, {})).collect()
						assert.equal(groundRows.length, 1)
						assert.ok(groundRows[0])
						assert.deepEqual(Event.toBytes(groundRows[0].when), saved.literal)
						// Overlap cannot replace an exact selected parent, including on
						// a recovered history whose Event owners were all reconstructed.
						const draft = yield* ChangeSet.builder(s)
						yield* draft.delete(Parent, [{ id: 1n, filter: literal }])
						yield* draft.insert(Parent, [{ id: 1n, filter: full }])
						const replacement = yield* Command.seal({
							scope: history.identity,
							id: { receiptEpoch: history.receiptEpoch, requestId: requestId() },
							changes: yield* draft.finish(),
							precondition: { kind: "blind" },
							result: {}
						})
						const outcome = yield* history.submit(replacement, submit)
						assert.equal(outcome.kind, "decided")
						if (outcome.kind === "decided") assert.equal(outcome.receipt.outcome.kind, "invariant-rejected")
					})
				)
				const destination = { kind: "filesystem", directory: join(directory, "backup") } as const
				const backupId = operationId()
				assert.equal(
					(yield* backup(saved.binding, { operationId: backupId, destination, schema: s })).kind,
					"completed"
				)
				assert.deepEqual((yield* verifyBackup(destination, { backup: backupId })).identity, saved.binding.identity)
				const restored = yield* restore(
					destination,
					{
						...saved.binding,
						directory: join(directory, "restored"),
						identity: { ...saved.binding.identity, incarnationId: incarnationId() }
					},
					{ operationId: operationId(), backup: backupId, schema: s }
				)
				assert.ok(restored.kind === "completed")
				const target = restored.value.binding
				assert.ok(target.kind === "local")
				yield* Effect.scoped(
					Effect.gen(function* () {
						const history = yield* LocalHistory.open(target, s)
						const row = yield* (yield* history.snapshot(read)).get(ChildById, { id: 1n })
						assert.ok(row._tag === "Some")
						assert.deepEqual(Event.toBytes(row.value.filter), saved.literal)
					})
				)
			})
		)
	} finally {
		await runtime.dispose()
		rmSync(directory, { recursive: true, force: true })
	}
})

test("Event selections participate in transition admission, replay, activation and abort", async () => {
	const directory = mkdtempSync(join(tmpdir(), "bumbledb-event-transition-"))
	const runtime = ManagedRuntime.make(NativeRuntime.layer(options))
	try {
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const literal = yield* Event.space(new Uint8Array(32).fill(212), 2n)
					const target = theory(literal)
					const source = schema("Source", { Child, Parent }, [ChildById, key(Parent, ["id"])])
					const identity = {
						databaseId: Result.getOrThrow(DatabaseId.parse(randomUUID())),
						incarnationId: incarnationId(),
						schemaId: (yield* Schema.compile(source)).schemaId
					}
					const history = yield* LocalHistory.create({ kind: "local", directory, identity }, source, {
						creation: { operationId: operationId(), artifact: new TextEncoder().encode(yield* schemaSnapshot(source)) }
					})
					const contract = {
						source: identity,
						target: { ...identity, incarnationId: incarnationId(), schemaId: (yield* Schema.compile(target)).schemaId },
						operation: operationId(),
						commitment: "12".repeat(32)
					}
					const bad = yield* Transition.begin(history, target, contract)
					assert.ok(bad.kind === "populating")
					const orphan = yield* ChangeSet.builder(target)
					yield* orphan.insert(Child, [{ id: 1n, filter: literal }])
					yield* bad.population.apply(yield* orphan.finish())
					assert.equal((yield* Effect.flip(bad.population.finish())).code, "Engine")
					assert.equal((yield* Transition.resolve(history, target, contract)).kind, "uninstalled")
					const start = yield* Transition.begin(history, target, contract)
					assert.ok(start.kind === "populating")
					const valid = yield* ChangeSet.builder(target)
					yield* valid.insert(Child, [{ id: 1n, filter: literal }])
					yield* valid.insert(Parent, [{ id: 1n, filter: literal }])
					yield* start.population.apply(yield* valid.finish())
					const ready = yield* start.population.finish()
					assert.deepEqual(yield* Transition.resolve(history, target, contract), ready)
					assert.equal((yield* Transition.begin(history, target, contract)).kind, "ready")
					const malformed = theory(Result.getOrThrow(Event.fromBytes(Buffer.from("BEVT\x01"))))
					const refused = yield* Effect.flip(Transition.activate(history, malformed, ready.installed))
					assert.equal(refused.code, "Engine")
					assert.equal((yield* Transition.resolve(history, target, contract)).kind, "ready")
					const activated = yield* Transition.activate(history, target, ready.installed)
					assert.deepEqual(yield* Transition.resolve(history, target, contract), activated)
					yield* Effect.scoped(
						Effect.gen(function* () {
							const next = yield* LocalHistory.open(activated.binding, target)
							const row = yield* (yield* next.snapshot(read)).get(ChildById, { id: 1n })
							assert.ok(row._tag === "Some")
							assert.deepEqual(Event.toBytes(row.value.filter), Event.toBytes(literal))
							const abortContract = {
								...contract,
								source: next.identity,
								target: { ...next.identity, incarnationId: incarnationId() },
								operation: operationId()
							}
							const attempt = yield* Transition.begin(next, target, abortContract)
							assert.ok(attempt.kind === "populating")
							yield* attempt.population.close()
							yield* Transition.abort(next, target, abortContract)
							assert.equal((yield* Transition.resolve(next, target, abortContract)).kind, "aborted")
							assert.equal((yield* next.inspect()).accessMode, "active")
						})
					)
				})
			)
		)
	} finally {
		await runtime.dispose()
		rmSync(directory, { recursive: true, force: true })
	}
})
