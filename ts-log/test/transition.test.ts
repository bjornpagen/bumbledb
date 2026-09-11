import assert from "node:assert/strict"
import { randomUUID } from "node:crypto"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { test } from "node:test"
import type { Fact } from "@bjornpagen/bumbledb"
import { ChangeSet, key, NativeRuntime, query, Schema, v } from "@bjornpagen/bumbledb"
import type { History, Population, PublishedSnapshot, TransitionContract } from "@bjornpagen/bumbledb-log"
import {
	Command,
	DatabaseId,
	IncarnationId,
	LocalHistory,
	OperationId,
	RequestId,
	Transition
} from "@bjornpagen/bumbledb-log"
import { schemaBindings, schemaSnapshot } from "@bjornpagen/bumbledb-log/schema"
import { Effect, ManagedRuntime, Result, Stream } from "effect"
import * as old from "./fixtures/transition/source.ts"
import * as next from "./fixtures/transition/target.ts"

function ok<A, E>(result: Result.Result<A, E>): A {
	assert.ok(Result.isSuccess(result))
	return result.success
}
const submissions: readonly Fact<typeof old.r1>[] = [
	{ id: 1n, method: "Imported", recordedAt: -2208988800000n },
	{ id: 2n, method: "Electronic", recordedAt: 0n },
	{ id: 3n, method: "Postal", recordedAt: 9223372036854775807n }
]
const proofs = submissions.map((row) => ({ submission: row.id, reference: `proof ${row.id} 😀` }))
const documents = [{ id: 41n, digest: "unchanged document" }]
const submissionKey = key(next.r1, ["id"])
const documentKey = key(next.r2, ["id"])
const sourceJoin = query(old.schema).rule((r) => {
	const { id, method, recordedAt } = v(old.r1)
	const { reference } = v(old.r2)
	return r
		.match(old.r1, { id, method, recordedAt })
		.match(old.r2, { submission: id, reference })
		.find({ id, method, recordedAt, reference })
})
const sourceDocuments = query(old.schema).rule((r) => {
	const row = v(old.r3)
	return r.match(old.r3, row).find(row)
})
const readOptions = { consistency: { kind: "cached" } } as const
const submitOptions = { attempts: 1, backoff: { baseMillis: 0, capMillis: 0 } }

const createSource = (directory: string) =>
	Effect.gen(function* () {
		const source = {
			databaseId: ok(DatabaseId.parse(randomUUID())),
			incarnationId: ok(IncarnationId.parse(randomUUID())),
			schemaId: (yield* Schema.compile(old.schema)).schemaId
		}
		const binding = { kind: "local", directory, identity: source } as const
		const history = yield* LocalHistory.create(binding, old.schema, {
			creation: {
				operationId: ok(OperationId.parse(randomUUID())),
				artifact: new TextEncoder().encode(yield* schemaSnapshot(old.schema))
			}
		})
		yield* Effect.scoped(
			Effect.gen(function* () {
				const builder = yield* ChangeSet.builder(old.schema)
				yield* builder.insert(old.r1, submissions)
				yield* builder.insert(old.r2, proofs)
				yield* builder.insert(old.r3, documents)
				const command = yield* Command.seal({
					scope: source,
					id: { receiptEpoch: history.receiptEpoch, requestId: ok(RequestId.parse(randomUUID())) },
					changes: yield* builder.finish(),
					precondition: { kind: "blind" },
					result: {}
				})
				const receipt = yield* history.submit(command, submitOptions)
				assert.equal(receipt.kind, "decided")
				if (receipt.kind === "decided") assert.equal(receipt.receipt.outcome.kind, "committed")
			})
		)
		const contract: TransitionContract = {
			source,
			target: {
				...source,
				incarnationId: ok(IncarnationId.parse(randomUUID())),
				schemaId: (yield* Schema.compile(next.schema)).schemaId
			},
			operation: ok(OperationId.parse(randomUUID())),
			commitment: "ab".repeat(32)
		}
		return { history, contract, binding }
	})

// The application owns this ordinary, typechecked transform. No migration
// row model or interpreter: just native query pages and normal ChangeSets.
function transform(
	source: PublishedSnapshot<typeof old.schema>,
	target: Population<typeof next.schema>,
	omitDocument = false,
	omitProof = false,
	childrenFirst = false
) {
	return Effect.gen(function* () {
		const prepared = yield* source.prepare(sourceJoin)
		const result = yield* prepared.execute({})
		yield* Stream.runForEach(result.pages(), (rows) =>
			Effect.scoped(
				Effect.gen(function* () {
					const parents = yield* ChangeSet.builder(next.schema)
					const children = yield* ChangeSet.builder(next.schema)
					for (const row of rows) {
						yield* parents.insert(next.r1, [{ id: row.id, method: row.method, recordedAt: row.recordedAt }])
						const fact = { submission: row.id, reference: row.reference }
						if (omitProof && row.method === "Postal") continue
						switch (row.method) {
							case "Imported":
								yield* children.insert(next.r3, [fact])
								break
							case "Electronic":
								yield* children.insert(next.r4, [fact])
								break
							case "Postal":
								yield* children.insert(next.r5, [fact])
								break
						}
					}
					const parentChanges = yield* parents.finish()
					const childChanges = yield* children.finish()
					for (const batch of childrenFirst ? [childChanges, parentChanges] : [parentChanges, childChanges])
						yield* target.apply(batch)
				})
			)
		)
		if (!omitDocument) {
			const result = yield* (yield* source.prepare(sourceDocuments)).execute({})
			yield* Stream.runForEach(result.pages(), (rows) =>
				Effect.scoped(
					Effect.gen(function* () {
						const builder = yield* ChangeSet.builder(next.schema)
						yield* builder.insert(next.r2, rows)
						yield* target.apply(yield* builder.finish())
					})
				)
			)
		}
	})
}

function compare(source: PublishedSnapshot<typeof old.schema>, target: PublishedSnapshot<typeof next.schema>) {
	return Effect.gen(function* () {
		const result = yield* (yield* source.prepare(sourceJoin)).execute({})
		yield* Stream.runForEach(result.pages(), (rows) =>
			Effect.gen(function* () {
				for (const row of rows) {
					const parent = yield* target.get(submissionKey, { id: row.id })
					assert.ok(parent._tag === "Some")
					assert.deepEqual(parent.value, { id: row.id, method: row.method, recordedAt: row.recordedAt })
					const arm = { Imported: next.r3, Electronic: next.r4, Postal: next.r5 }[row.method]
					const child = yield* target.get(key(arm, ["submission"]), { submission: row.id })
					assert.ok(child._tag === "Some")
					assert.deepEqual(child.value, { submission: row.id, reference: row.reference })
				}
			})
		)
		const resultDocuments = yield* (yield* source.prepare(sourceDocuments)).execute({})
		let preserved = true
		yield* Stream.runForEach(resultDocuments.pages(), (rows) =>
			Effect.gen(function* () {
				for (const row of rows) {
					const actual = yield* target.get(documentKey, { id: row.id })
					preserved &&= actual._tag === "Some" && actual.value.digest === row.digest
				}
			})
		)
		// Membership in both directions proves equality without buffering
		// all documents in JavaScript. Keys make each lookup unambiguous.
		const targetDocuments = query(next.schema).rule((r) => {
			const row = v(next.r2)
			return r.match(next.r2, row).find(row)
		})
		const reverse = yield* target.execute(targetDocuments, {})
		yield* Stream.runForEach(reverse.pages(), (rows) =>
			Effect.gen(function* () {
				for (const row of rows) {
					const actual = yield* source.get(key(old.r3, ["id"]), { id: row.id })
					preserved &&= actual._tag === "Some" && actual.value.digest === row.digest
				}
			})
		)
		const targetSubmissions = query(next.schema).rule((r) => {
			const row = v(next.r1)
			return r.match(next.r1, row).find(row)
		})
		yield* Stream.runForEach((yield* target.execute(targetSubmissions, {})).pages(), (rows) =>
			Effect.gen(function* () {
				for (const row of rows) {
					const actual = yield* source.get(key(old.r1, ["id"]), { id: row.id })
					preserved &&=
						actual._tag === "Some" && actual.value.method === row.method && actual.value.recordedAt === row.recordedAt
				}
			})
		)
		return preserved
	})
}

async function run<A, E>(
	program: (directory: string) => Effect.Effect<A, E, import("effect").Scope.Scope | NativeRuntime>
) {
	const directory = fs.mkdtempSync(path.join(os.tmpdir(), "bdb-transition-public-"))
	const runtime = ManagedRuntime.make(NativeRuntime.layer({ workers: 4, nativeHandleCapacity: 128 }))
	try {
		const result = await runtime.runPromise(Effect.scoped(program(directory)))
		const counts = await runtime.runPromise(Effect.flatMap(NativeRuntime, (native) => native.inspect()))
		assert.equal(counts.natives, 0n)
		assert.equal(counts.owners, 0n)
		return result
	} finally {
		await runtime.dispose()
		fs.rmSync(directory, { recursive: true, force: true })
	}
}

test("the retained source and target declarations are independently generated exact native bindings", async () => {
	await run(() =>
		Effect.gen(function* () {
			for (const [name, schema] of [
				["source", old.schema],
				["target", next.schema]
			] as const) {
				const directory = path.join(import.meta.dirname, "fixtures/transition")
				const snapshot = fs.readFileSync(path.join(directory, `${name}.json`), "utf8")
				assert.equal(yield* schemaBindings(snapshot), fs.readFileSync(path.join(directory, `${name}.ts`), "utf8"))
				assert.equal(yield* schemaSnapshot(schema), snapshot)
			}
		})
	)
})

for (const childrenFirst of [false, true])
	test(`ordinary imperative transition, comparison, activation and replay (children first: ${childrenFirst})`, async () => {
		await run((directory) =>
			Effect.gen(function* () {
				const { history, contract } = yield* createSource(directory)
				let invocations = 0
				const start = yield* Transition.begin(history, next.schema, contract)
				assert.ok(start.kind === "populating")
				assert.deepEqual(start.source.decisionStamp, start.captured.decision)
				assert.deepEqual(start.source.stateStamp, start.captured.state)
				invocations++
				yield* transform(start.source, start.population, false, false, childrenFirst)
				const ready = yield* start.population.finish()
				const stale = yield* Effect.result(start.population.finish())
				assert.equal(stale._tag, "Failure")
				assert.deepEqual(yield* Transition.resolve(history, next.schema, contract), ready)
				const retry = yield* Transition.begin(history, next.schema, contract)
				if (retry.kind === "populating") {
					invocations++
					yield* transform(retry.source, retry.population)
				}
				assert.equal(retry.kind, "ready")
				assert.equal(invocations, 1)
				yield* Effect.scoped(
					Effect.gen(function* () {
						const target = yield* LocalHistory.open(ready.binding, next.schema)
						assert.equal((yield* target.inspect()).accessMode, "frozen")
						const reader = yield* Transition.inspect(target, ready.installed)
						assert.ok(yield* compare(start.source, reader))
					})
				)
				const activated = yield* Transition.activate(history, next.schema, ready.installed)
				assert.equal((yield* history.inspect()).accessMode, "frozen")
				yield* Effect.scoped(
					Effect.gen(function* () {
						const target = yield* LocalHistory.open(activated.binding, next.schema)
						assert.equal((yield* target.inspect()).accessMode, "active")
						const builder = yield* ChangeSet.builder(next.schema)
						yield* builder.insert(next.r2, [{ id: 42n, digest: "after activation" }])
						const command = yield* Command.seal({
							scope: target.identity,
							id: { receiptEpoch: target.receiptEpoch, requestId: ok(RequestId.parse(randomUUID())) },
							changes: yield* builder.finish(),
							precondition: { kind: "blind" },
							result: {}
						})
						assert.equal((yield* target.submit(command, submitOptions)).kind, "decided")
						assert.deepEqual(yield* Transition.resolve(history, next.schema, contract), activated)
						assert.deepEqual(yield* Transition.activate(history, next.schema, ready.installed), activated)
						assert.equal((yield* Effect.result(Transition.abort(history, next.schema, contract)))._tag, "Failure")
						const reader = yield* target.snapshot(readOptions)
						assert.equal((yield* reader.get(documentKey, { id: 42n }))._tag, "Some")
						assert.equal(
							yield* compare(start.source, reader),
							false,
							"new target data is not part of the frozen source"
						)
					})
				)
				assert.equal(
					(yield* Effect.result(Transition.resolve(history, next.schema, { ...contract, commitment: "cd".repeat(32) })))
						._tag,
					"Failure"
				)
				const changedTarget = {
					...contract,
					target: { ...contract.target, incarnationId: ok(IncarnationId.parse(randomUUID())) }
				}
				const changedResolution = yield* Effect.result(Transition.resolve(history, next.schema, changedTarget))
				assert.ok(Result.isFailure(changedResolution))
				assert.equal(changedResolution.failure.code, "OperationConflict")
			})
		)
	})

test("native admission rejects a missing proof; unconstrained document loss requires application comparison", async () => {
	await run((directory) =>
		Effect.gen(function* () {
			const { history, contract } = yield* createSource(directory)
			const first = yield* Transition.begin(history, next.schema, contract)
			assert.ok(first.kind === "populating")
			yield* transform(first.source, first.population, false, true)
			const refused = yield* Effect.result(first.population.finish())
			assert.ok(Result.isFailure(refused))
			assert.equal(refused.failure.code, "Engine")
			assert.ok("reason" in refused.failure && refused.failure.reason._tag === "Engine")
			assert.match(refused.failure.reason.message, /JudgeRefused.*StatementId\(11\)/)
			assert.equal((yield* Transition.resolve(history, next.schema, contract)).kind, "uninstalled")
			const second = yield* Transition.begin(history, next.schema, contract)
			assert.ok(second.kind === "populating")
			yield* transform(second.source, second.population, true)
			const ready = yield* second.population.finish()
			yield* Effect.scoped(
				Effect.gen(function* () {
					const target = yield* LocalHistory.open(ready.binding, next.schema)
					const reader = yield* Transition.inspect(target, ready.installed)
					assert.equal(yield* compare(second.source, reader), false)
				})
			)
			yield* Transition.abort(history, next.schema, contract)
			assert.equal((yield* history.inspect()).accessMode, "active")
			assert.equal((yield* Transition.begin(history, next.schema, contract)).kind, "aborted")
		})
	)
})

// Type errors must be rejected by the generated ordinary SDK types.
function typePins(population: Population<typeof next.schema>, source: History<typeof old.schema>) {
	const builder = ChangeSet.builder(next.schema)
	// @ts-expect-error Closed rosters are immutable schema data, not changeable relations.
	Effect.flatMap(builder, (draft) => draft.delete(next.r0, [{ id: "Imported" }]))
	void builder
	// @ts-expect-error Query belongs to the old schema, not the population.
	population.apply(sourceJoin)
	// @ts-expect-error Epoch milliseconds retain signed bigint values.
	const bad: Fact<typeof next.r1> = { id: 1n, method: "Imported", recordedAt: 0 }
	// @ts-expect-error Closed handles cannot invent a method.
	const unknown: Fact<typeof old.r1> = { id: 1n, method: "Fax", recordedAt: 0n }
	// @ts-expect-error No plan or transformation function is accepted by native begin.
	Transition.begin(source, next.schema, () => {})
	void [bad, unknown]
}
void typePins

for (const defect of ["wrong-arm", "orphan", "key-conflict", "missing-parent"] as const) {
	test(`complete native judgment refuses ${defect} across ordinary population batches`, async () => {
		await run((directory) =>
			Effect.gen(function* () {
				const { history, contract } = yield* createSource(directory)
				const start = yield* Transition.begin(history, next.schema, contract)
				assert.ok(start.kind === "populating")
				yield* transform(start.source, start.population)
				yield* Effect.scoped(
					Effect.gen(function* () {
						const builder = yield* ChangeSet.builder(next.schema)
						const firstSubmission = submissions[0]
						const firstProof = proofs[0]
						assert.ok(firstSubmission && firstProof)
						switch (defect) {
							case "wrong-arm":
								yield* builder.delete(next.r3, [{ submission: 1n, reference: firstProof.reference }])
								yield* builder.insert(next.r4, [{ submission: 1n, reference: firstProof.reference }])
								break
							case "orphan":
								yield* builder.insert(next.r3, [{ submission: 99n, reference: "orphan" }])
								break
							case "key-conflict":
								yield* builder.insert(next.r3, [{ submission: 1n, reference: "different" }])
								break
							case "missing-parent":
								yield* builder.delete(next.r1, [firstSubmission])
								break
						}
						yield* start.population.apply(yield* builder.finish())
					})
				)
				const result = yield* Effect.result(start.population.finish())
				assert.ok(Result.isFailure(result))
				assert.equal(result.failure.code, "Engine")
				assert.ok("reason" in result.failure && result.failure.reason._tag === "Engine")
				assert.match(result.failure.reason.message, /JudgeRefused/)
				assert.equal((yield* Transition.resolve(history, next.schema, contract)).kind, "uninstalled")
				yield* Transition.abort(history, next.schema, contract)
				assert.equal((yield* history.inspect()).accessMode, "active")
			})
		)
	})
}
