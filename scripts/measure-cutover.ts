import assert from "node:assert/strict"
import { randomUUID } from "node:crypto"
import fs from "node:fs"
import os from "node:os"
import path from "node:path"
import {
	bool,
	ChangeSet,
	Compute,
	Db,
	type Fact,
	i64,
	NativeRuntime,
	query,
	relation,
	Schema,
	schema,
	u64,
	v
} from "@bjornpagen/bumbledb"
import {
	Command,
	DatabaseId,
	IncarnationId,
	LocalHistory,
	OperationId,
	RequestId,
	Transition
} from "@bjornpagen/bumbledb-log"
import { schemaSnapshot } from "@bjornpagen/bumbledb-log/schema"
import { Effect, ManagedRuntime, Result, Stream } from "effect"
import * as old from "./fixtures/transition/source.ts"
import * as next from "./fixtures/transition/target.ts"

const mode = process.argv[2]
const count = Number(process.argv[3])
assert(Number.isSafeInteger(count) && count > 0)
const directory = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-cutover-data-"))
const runtime = ManagedRuntime.make(NativeRuntime.layer({ workers: 2, nativeHandleCapacity: 128 }))
let peakHeap = 0
let peakExternal = 0
function sample() {
	const memory = process.memoryUsage()
	peakHeap = Math.max(peakHeap, memory.heapUsed)
	peakExternal = Math.max(peakExternal, memory.external)
}
const timer = setInterval(sample, 5)
function ok<A, E>(value: Result.Result<A, E>): A {
	assert(Result.isSuccess(value))
	return value.success
}
const Entry = relation("Entry", { id: u64, amount: i64, previous: i64, original: bool })
const Theory = schema("Heads", { Entry }, [])
const original = query(Theory).rule((r) => {
	const { id, amount } = v(Entry)
	return r.match(Entry, { id, amount, original: true }).find({ id, amount })
})
const correction = query(Theory).rule((r) => {
	const { id, amount, previous } = v(Entry)
	return r
		.match(Entry, { id, amount, previous, original: false })
		.find({ id, amount: Compute.subtract(amount, previous) })
})
const direct = original.rule((r) => {
	const { id, amount, previous } = v(Entry)
	return r
		.match(Entry, { id, amount, previous, original: false })
		.find({ id, amount: Compute.subtract(amount, previous) })
})
const imported = original.rule((r) => {
	const row = v(correction)
	return r.match(correction, row).find(row)
})
const unmixed = original.rule((r) => {
	const { id, amount } = v(Entry)
	return r.match(Entry, { id, amount, original: false }).find({ id, amount })
})
const sourceJoin = query(old.schema).rule((r) => {
	const { id, method, recordedAt } = v(old.r1)
	const { reference } = v(old.r2)
	return r
		.match(old.r1, { id, method, recordedAt })
		.match(old.r2, { submission: id, reference })
		.find({ id, method, recordedAt, reference })
})
function median(values: number[]): number {
	const value = values.toSorted((a, b) => a - b)[Math.floor(values.length / 2)]
	assert.notEqual(value, undefined)
	assert(typeof value === "number")
	return value
}
function heads() {
	return Effect.gen(function* () {
		const db = yield* Db.create(path.join(directory, "db"), Theory)
		for (let start = 0; start < count; start += 1000) {
			yield* Effect.scoped(
				Effect.gen(function* () {
					const builder = yield* ChangeSet.builder(Theory)
					yield* builder.insert(
						Entry,
						Array.from({ length: Math.min(1000, count - start) }, (_, offset) => ({
							id: BigInt(start + offset),
							amount: BigInt(start + offset),
							previous: 0n,
							original: offset % 2 === 0
						}))
					)
					yield* db.apply(yield* builder.finish(), { expected: { kind: "any" } })
				})
			)
		}
		const reader = yield* db.snapshot()
		const reports: Record<string, { preparationMs: number; executionMs: number }> = {}
		// Rotate order each round and discard the first warmup round. All forms
		// have the same two match conditions and exact identity-valued results.
		const forms = [
			{ name: "direct", query: direct, preparation: [] as number[], execution: [] as number[] },
			{ name: "imported", query: imported, preparation: [] as number[], execution: [] as number[] },
			{ name: "unmixed", query: unmixed, preparation: [] as number[], execution: [] as number[] }
		]
		for (let round = 0; round < 8; round++) {
			for (let offset = 0; offset < forms.length; offset++) {
				const index = (round + offset) % forms.length
				const form = forms[index]
				assert(form)
				yield* Effect.scoped(
					Effect.gen(function* () {
						let clock = performance.now()
						const prepared = yield* reader.prepare(form.query)
						const preparation = performance.now() - clock
						clock = performance.now()
						const result = yield* prepared.execute({})
						const execution = performance.now() - clock
						let seen = 0
						let sum = 0n
						yield* Stream.runForEach(result.pages(), (rows) =>
							Effect.sync(() => {
								for (const row of rows) {
									assert.equal(row.id, row.amount)
									seen++
									sum += row.id
								}
							})
						)
						assert.equal(seen, count)
						assert.equal(sum, (BigInt(count) * BigInt(count - 1)) / 2n)
						if (round > 0) {
							form.preparation.push(preparation)
							form.execution.push(execution)
						}
					})
				)
			}
		}
		for (const form of forms)
			reports[form.name] = { preparationMs: median(form.preparation), executionMs: median(form.execution) }
		return reports
	})
}
function transition() {
	return Effect.gen(function* () {
		const identity = {
			databaseId: ok(DatabaseId.parse(randomUUID())),
			incarnationId: ok(IncarnationId.parse(randomUUID())),
			schemaId: (yield* Schema.compile(old.schema)).schemaId
		}
		const source = yield* LocalHistory.create({ kind: "local", directory, identity }, old.schema, {
			creation: {
				operationId: ok(OperationId.parse(randomUUID())),
				artifact: new TextEncoder().encode(yield* schemaSnapshot(old.schema))
			}
		})
		for (let start = 0; start < count; start += 1000) {
			yield* Effect.scoped(
				Effect.gen(function* () {
					const parents: Fact<typeof old.r1>[] = Array.from({ length: Math.min(1000, count - start) }, (_, offset) => ({
						id: BigInt(start + offset),
						method: ["Imported", "Electronic", "Postal"][(start + offset) % 3] as "Imported" | "Electronic" | "Postal",
						recordedAt: -BigInt(start + offset)
					}))
					const builder = yield* ChangeSet.builder(old.schema)
					yield* builder.insert(old.r1, parents)
					yield* builder.insert(
						old.r2,
						parents.map((row) => ({ submission: row.id, reference: `proof-${row.id}` }))
					)
					const command = yield* Command.seal({
						scope: identity,
						id: { receiptEpoch: source.receiptEpoch, requestId: ok(RequestId.parse(randomUUID())) },
						changes: yield* builder.finish(),
						precondition: { kind: "blind" },
						result: {}
					})
					const outcome = yield* source.submit(command, { attempts: 1, backoff: { baseMillis: 0, capMillis: 0 } })
					assert(outcome.kind === "decided" && outcome.receipt.outcome.kind === "committed")
				})
			)
		}
		globalThis.gc?.()
		const memoryBefore = process.memoryUsage()
		peakHeap = memoryBefore.heapUsed
		peakExternal = memoryBefore.external
		let clock = performance.now()
		const start = yield* Transition.begin(source, next.schema, {
			source: identity,
			target: {
				...identity,
				incarnationId: ok(IncarnationId.parse(randomUUID())),
				schemaId: (yield* Schema.compile(next.schema)).schemaId
			},
			operation: ok(OperationId.parse(randomUUID())),
			commitment: "ab".repeat(32)
		})
		assert(start.kind === "populating")
		const captureMs = performance.now() - clock
		const prepared = yield* start.source.prepare(sourceJoin)
		clock = performance.now()
		const result = yield* prepared.execute({})
		const joinMs = performance.now() - clock
		clock = performance.now()
		let batches = 0
		let seen = 0
		let largestPage = 0
		const batchTimes: number[] = []
		yield* Stream.runForEach(result.pages(), (rows) =>
			Effect.gen(function* () {
				largestPage = Math.max(largestPage, rows.length)
				for (let offset = 0; offset < rows.length; offset += 1000) {
					yield* Effect.scoped(
						Effect.gen(function* () {
							const batchStart = performance.now()
							const page = rows.slice(offset, offset + 1000)
							const builder = yield* ChangeSet.builder(next.schema)
							yield* builder.insert(
								next.r1,
								page.map(({ id, method, recordedAt }) => ({ id, method, recordedAt }))
							)
							for (const [method, relation] of [
								["Imported", next.r3],
								["Electronic", next.r4],
								["Postal", next.r5]
							] as const) {
								yield* builder.insert(
									relation,
									page
										.filter((row) => row.method === method)
										.map((row) => ({ submission: row.id, reference: row.reference }))
								)
							}
							yield* start.population.apply(yield* builder.finish())
							seen += page.length
							batches++
							batchTimes.push(performance.now() - batchStart)
							sample()
						})
					)
				}
			})
		)
		const populationMs = performance.now() - clock
		assert.equal(seen, count)
		clock = performance.now()
		const ready = yield* start.population.finish()
		assert.equal(ready.kind, "ready")
		const finalizeMs = performance.now() - clock
		sample()
		const width = Math.max(1, Math.floor(batchTimes.length / 4))
		return {
			captureMs,
			joinMs,
			populationMs,
			finalizeMs,
			batches,
			largestPage,
			factsWritten: 2 * count,
			batchFirstQuarterMs: median(batchTimes.slice(0, width)),
			batchLastQuarterMs: median(batchTimes.slice(-width)),
			memoryBefore,
			peakHeap,
			peakExternal
		}
	})
}
try {
	const result =
		mode === "heads"
			? await runtime.runPromise(Effect.scoped(heads()))
			: await runtime.runPromise(Effect.scoped(transition()))
	const counts = await runtime.runPromise(Effect.flatMap(NativeRuntime, (native) => native.inspect()))
	assert.equal(counts.natives, 0n)
	assert.equal(counts.owners, 0n)
	console.log(
		JSON.stringify({ mode, count, result, peakRssBytes: process.resourceUsage().maxRSS * 1024, peakHeap, peakExternal })
	)
} finally {
	clearInterval(timer)
	await runtime.dispose()
	fs.rmSync(directory, { recursive: true, force: true })
}
