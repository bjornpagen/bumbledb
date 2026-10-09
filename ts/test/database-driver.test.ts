import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, Exit, Fiber, Schedule, Scope } from "effect"
import type { DriverOptions, MachinePort, Step } from "../src/database/driver.ts"
import { Driver } from "../src/database/driver.ts"
import type { IoRequest, IoResponse, ObjectStore } from "../src/database/io.ts"
import { MemStore } from "../src/database/mem.ts"
import { DbError } from "../src/runtime-errors.ts"

type Submit = { readonly bytes: Uint8Array }
type Settled = { readonly _tag: "Decided"; readonly seq: number } | { readonly _tag: "Closed" }

const seqKey = (seq: number) => `log/${seq.toString().padStart(20, "0")}`
const sameBytes = (left: Uint8Array, right: Uint8Array) =>
	left.length === right.length && left.every((byte, index) => byte === right[index])

/**
 * A minimal create-only log in TS, enough to drive the driver: write at tip+1; on a refused or
 * ambiguous create, read the key back and keep it if the bytes are ours, else move on a slot.
 */
function fakeLog(): MachinePort<Submit, Settled> {
	let tip = 0
	let ioId = 0n
	const open = new Map<bigint, { bytes: Uint8Array; seq: number }>()
	const inFlight = new Map<bigint, { ticket: bigint; verb: "put" | "get" }>()
	const step = (io: IoRequest[], done: Step<Settled>["done"] = []): Step<Settled> => ({ io, done })
	const putFor = (ticket: bigint): IoRequest => {
		const state = open.get(ticket)
		if (state === undefined) throw new Error("unknown ticket")
		ioId += 1n
		inFlight.set(ioId, { ticket, verb: "put" })
		return {
			id: ioId,
			bucket: "Log",
			key: seqKey(state.seq),
			op: { _tag: "PutIfAbsent", body: { _tag: "Bytes", bytes: state.bytes } }
		}
	}
	const getFor = (ticket: bigint, seq: number): IoRequest => {
		ioId += 1n
		inFlight.set(ioId, { ticket, verb: "get" })
		return { id: ioId, bucket: "Log", key: seqKey(seq), op: { _tag: "Get", target: { _tag: "Memory" } } }
	}
	const decide = (ticket: bigint, seq: number): Step<Settled> => {
		open.delete(ticket)
		tip = Math.max(tip, seq)
		return step([], [{ ticket, settled: { _tag: "Decided", seq } }])
	}
	return {
		request: (ticket, submit) =>
			Effect.sync(() => {
				open.set(ticket, { bytes: submit.bytes, seq: tip + 1 })
				return step([putFor(ticket)])
			}),
		respond: (response: IoResponse) =>
			Effect.sync(() => {
				const pending = inFlight.get(response.id)
				inFlight.delete(response.id)
				const state = pending === undefined ? undefined : open.get(pending.ticket)
				if (pending === undefined || state === undefined) return step([])
				const result = response.result
				if (pending.verb === "put") {
					return result._tag === "Created"
						? decide(pending.ticket, state.seq)
						: step([getFor(pending.ticket, state.seq)])
				}
				if (result._tag === "Body") {
					if (sameBytes(result.bytes, state.bytes)) return decide(pending.ticket, state.seq)
					tip = Math.max(tip, state.seq)
					state.seq = tip + 1
					return step([putFor(pending.ticket)])
				}
				return result._tag === "Missing" ? step([putFor(pending.ticket)]) : step([getFor(pending.ticket, state.seq)])
			}),
		close: Effect.sync(() => {
			const done = [...open.keys()].map((ticket) => ({ ticket, settled: { _tag: "Closed" } as const }))
			open.clear()
			return step([], done)
		})
	}
}

const options: DriverOptions = { concurrency: 4, executor: { timeout: "1 second", readRetry: Schedule.recurs(5) } }
const submission = (index: number) => ({ bytes: new TextEncoder().encode(`submission ${index}`) })

function decidedSeqs(settled: readonly Settled[]): number[] {
	return settled.map((value) => {
		assert.equal(value._tag, "Decided")
		return value._tag === "Decided" ? value.seq : -1
	})
}

test("concurrent submissions through two machines on one store each land in their own slot", async () => {
	const store = MemStore.make()
	const settled = await Effect.runPromise(
		Effect.scoped(
			Effect.gen(function* () {
				const left = yield* Driver.make(fakeLog(), store, options)
				const right = yield* Driver.make(fakeLog(), store, options)
				return yield* Effect.forEach(
					Array.from({ length: 16 }, (_, index) => index),
					(index) => (index % 2 === 0 ? left : right).run(submission(index)),
					{ concurrency: "unbounded" }
				)
			})
		)
	)
	const seqs = decidedSeqs(settled)
	assert.deepEqual(
		[...seqs].sort((a, b) => a - b),
		Array.from({ length: 16 }, (_, index) => index + 1)
	)
	const log = store.objects("Log")
	for (const [index, seq] of seqs.entries()) assert.deepEqual(log.get(seqKey(seq)), submission(index).bytes)
})

test("a create whose response is lost is resolved by reading it back, so it lands exactly once", async () => {
	let puts = 0
	const store = MemStore.make({
		fault: ({ verb }) => {
			if (verb !== "putIfAbsent") return undefined
			puts += 1
			return puts % 2 === 1 ? "Lose" : undefined
		}
	})
	const settled = await Effect.runPromise(
		Effect.scoped(
			Effect.gen(function* () {
				const driver = yield* Driver.make(fakeLog(), store, options)
				return yield* Effect.forEach([0, 1, 2, 3], (index) => driver.run(submission(index)), {
					concurrency: "unbounded"
				})
			})
		)
	)
	assert.deepEqual([...decidedSeqs(settled)].sort(), [1, 2, 3, 4])
	assert.equal(store.objects("Log").size, 4)
})

test("closing the scope settles a request whose store call never answers, and later runs are refused", async () => {
	const stuck: ObjectStore = { ...MemStore.make(), putIfAbsent: () => Effect.never }
	const driverOptions: DriverOptions = { ...options, executor: { ...options.executor, timeout: "1 hour" } }
	const [settled, later] = await Effect.runPromise(
		Effect.gen(function* () {
			const scope = yield* Scope.make()
			const driver = yield* Driver.make(fakeLog(), stuck, driverOptions).pipe(Scope.provide(scope))
			const pending = yield* Effect.forkChild(driver.run(submission(0)), { startImmediately: true })
			yield* Scope.close(scope, Exit.void)
			return [yield* Fiber.join(pending), yield* Effect.exit(driver.run(submission(1)))] as const
		})
	)
	assert.deepEqual(settled, { _tag: "Closed" })
	assert.ok(Exit.isFailure(later))
})

test("a machine step failure fails the request that caused it and every request after it", async () => {
	const broken = new DbError({ operation: "machine", reason: { _tag: "Internal" } })
	const port: MachinePort<Submit, Settled> = { ...fakeLog(), request: () => Effect.fail(broken) }
	const exits = await Effect.runPromise(
		Effect.scoped(
			Effect.gen(function* () {
				const driver = yield* Driver.make(port, MemStore.make(), options)
				const first = yield* Effect.exit(driver.run(submission(0)))
				const second = yield* Effect.exit(driver.run(submission(1)))
				return [first, second]
			})
		)
	)
	for (const exit of exits) {
		assert.ok(Exit.isFailure(exit))
	}
})
