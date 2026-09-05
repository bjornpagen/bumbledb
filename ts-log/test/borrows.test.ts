/**
 * Independent tenant borrows over the ONE native registry. Every acquire is
 * a distinct one-shot borrow; releasing frees only that borrow (never the
 * shared owner, never a sibling); a stale borrow refuses with ClosedHandle;
 * double release is harmless; evicting a borrowed slot refuses instead of
 * revoking another request; same-schema different-origin bindings stay
 * isolated. There is no TTL, renewal, timer or JS eviction logic to test —
 * the successor deleted them. Maps to RUN-01 (borrow isolation), RUN-06
 * (cache isolation, layer side), chapter 35 "double/stale borrows" and
 * "same-schema cross-origin isolation"; OPS-006.
 */
import assert from "node:assert/strict"
import { describe, test } from "node:test"
import type { AnySchema } from "@bjornpagen/bumbledb"
import { Effect, Exit } from "effect"
import { makeLogMachine } from "#machine.ts"
import type { TenantCacheOptions } from "#options.ts"
import {
	handleWire,
	identityWire,
	localBinding,
	makeIntegration,
	makeWireDouble,
	otherIdentityWire,
	otherLocalBinding,
	provideRuntime,
	receiptWire,
	refWire,
	work
} from "#test/double.ts"

const schema = { name: "TestSchema" } as unknown as AnySchema

const cacheOptions: TenantCacheOptions = {
	maxOpen: 8,
	budgetBytes: 1n << 30n,
	maintenance: work
}

type Double = ReturnType<typeof makeWireDouble>
type Machine = ReturnType<typeof makeLogMachine>

function plannedCache(double: Double, machine: Machine) {
	double.plan("logCacheMake", { result: { __cache: true } })
	return machine.TenantCache.make(schema, cacheOptions)
}

describe("borrow lifecycle", function suite() {
	test("two acquires of one binding are DISTINCT borrows; releasing one leaves the other live", async function distinct() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		await Effect.runPromise(
			provideRuntime(
				Effect.scoped(
					Effect.gen(function* () {
						const cache = yield* plannedCache(double, machine)
						double.plan("logCacheAcquire", { result: handleWire() })
						double.plan("logCacheAcquire", { result: handleWire() })
						const first = yield* cache.acquire(localBinding, work)
						const second = yield* cache.acquire(localBinding, work)
						assert.notEqual(first, second)

						const released = yield* first.release()
						assert.equal(released.kind, "closed")
						// Only the borrow-release verb ran: no owner close, no cache close.
						assert.equal(double.calls.filter((call) => call.verb === "logBorrowRelease").length, 1)
						assert.equal(double.calls.filter((call) => call.verb === "logHistoryClose").length, 0)
						assert.equal(double.calls.filter((call) => call.verb === "logCacheClose").length, 0)

						// The sibling borrow still dispatches.
						double.plan("logHistoryCall", {
							result: { verb: "resolve", outcome: { kind: "found", receipt: receiptWire } }
						})
						const ref = {
							identity: second.identity,
							id: { receiptEpoch: second.receiptEpoch, requestId: refWire.requestId },
							digest: refWire.digest
						} as unknown as Parameters<typeof second.resolve>[0]
						const outcome = yield* second.resolve(ref, work)
						assert.equal(outcome.kind, "found")
					})
				)
			)
		)
	})

	test("a stale borrow refuses with ClosedHandle and dispatches nothing", async function stale() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		await Effect.runPromise(
			provideRuntime(
				Effect.scoped(
					Effect.gen(function* () {
						const cache = yield* plannedCache(double, machine)
						double.plan("logCacheAcquire", { result: handleWire() })
						const borrow = yield* cache.acquire(localBinding, work)
						yield* borrow.release()
						const dispatched = double.calls.filter((call) => call.verb === "logHistoryCall").length
						const exit = yield* Effect.exit(borrow.inspect(work))
						assert.ok(Exit.isFailure(exit))
						const error = Exit.findErrorOption(exit)
						assert.ok(error._tag === "Some")
						assert.equal(error.value.code, "ClosedHandle")
						assert.equal(double.calls.filter((call) => call.verb === "logHistoryCall").length, dispatched)
					})
				)
			)
		)
	})

	test("double release is harmless and joins the native transition", async function doubleRelease() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		await Effect.runPromise(
			provideRuntime(
				Effect.scoped(
					Effect.gen(function* () {
						const cache = yield* plannedCache(double, machine)
						double.plan("logCacheAcquire", { result: handleWire() })
						const borrow = yield* cache.acquire(localBinding, work)
						const first = yield* borrow.release()
						const second = yield* borrow.release()
						assert.equal(first.kind, "closed")
						assert.equal(second.kind, "closed")
					})
				)
			)
		)
	})

	test("scope close releases the borrow exactly once (idempotent with explicit release)", async function scopeRelease() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		await Effect.runPromise(
			provideRuntime(
				Effect.scoped(
					Effect.gen(function* () {
						const cache = yield* plannedCache(double, machine)
						double.plan("logCacheAcquire", { result: handleWire() })
						yield* Effect.scoped(
							Effect.gen(function* () {
								yield* cache.acquire(localBinding, work)
							})
						)
						// The inner scope released the borrow; the owner cache is intact.
						assert.equal(double.calls.filter((call) => call.verb === "logBorrowRelease").length, 1)
						assert.equal(double.calls.filter((call) => call.verb === "logCacheClose").length, 0)
					})
				)
			)
		)
	})
})

describe("cache surface", function suite() {
	test("evicting a borrowed slot refuses instead of revoking the borrow", async function evictBorrowed() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		await Effect.runPromise(
			provideRuntime(
				Effect.scoped(
					Effect.gen(function* () {
						const cache = yield* plannedCache(double, machine)
						double.plan("logCacheAcquire", { result: handleWire() })
						const borrow = yield* cache.acquire(localBinding, work)
						double.plan("logCacheEvict", {
							refuse: { source: "protocol", reason: { _tag: "SlotBorrowed" } }
						})
						const exit = yield* Effect.exit(cache.evict(localBinding))
						assert.ok(Exit.isFailure(exit))
						const error = Exit.findErrorOption(exit)
						assert.ok(error._tag === "Some")
						assert.equal(error.value.code, "SlotBorrowed")
						// The refused eviction revoked nothing: the borrow still works.
						double.plan("logHistoryCall", {
							result: { verb: "resolve", outcome: { kind: "command-epoch-closed" } }
						})
						const ref = {
							identity: borrow.identity,
							id: { receiptEpoch: borrow.receiptEpoch, requestId: refWire.requestId },
							digest: refWire.digest
						} as unknown as Parameters<typeof borrow.resolve>[0]
						const outcome = yield* borrow.resolve(ref, work)
						assert.equal(outcome.kind, "command-epoch-closed")
					})
				)
			)
		)
	})

	test("same-schema cross-origin bindings acquire isolated capabilities", async function crossOrigin() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		await Effect.runPromise(
			provideRuntime(
				Effect.scoped(
					Effect.gen(function* () {
						const cache = yield* plannedCache(double, machine)
						double.plan("logCacheAcquire", { result: handleWire(identityWire) })
						double.plan("logCacheAcquire", { result: handleWire(otherIdentityWire) })
						const first = yield* cache.acquire(localBinding, work)
						const second = yield* cache.acquire(otherLocalBinding, work)
						// Same schema, different origin: distinct identities, distinct slots.
						assert.equal(first.identity.schemaId, second.identity.schemaId)
						assert.notEqual(first.identity.databaseId, second.identity.databaseId)
						const acquires = double.calls.filter((call) => call.verb === "logCacheAcquire")
						assert.equal(acquires.length, 2)
						const bindings = acquires.map(
							(call) =>
								(call.request as { request: { binding: { directory: string; identity: { databaseId: string } } } })
									.request.binding
						)
						assert.notEqual(bindings[0]?.directory, bindings[1]?.directory)
						assert.notEqual(bindings[0]?.identity.databaseId, bindings[1]?.identity.databaseId)
						// Releasing the foreign tenant's borrow does not touch the first.
						yield* second.release()
						double.plan("logHistoryCall", {
							result: { verb: "resolve", outcome: { kind: "receipt-expired-unknown" } }
						})
						const ref = {
							identity: first.identity,
							id: { receiptEpoch: first.receiptEpoch, requestId: refWire.requestId },
							digest: refWire.digest
						} as unknown as Parameters<typeof first.resolve>[0]
						const outcome = yield* first.resolve(ref, work)
						assert.equal(outcome.kind, "receipt-expired-unknown")
					})
				)
			)
		)
	})

	test("cache close is the owner decision; a released cache refuses new acquires", async function cacheClose() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		await Effect.runPromise(
			provideRuntime(
				Effect.scoped(
					Effect.gen(function* () {
						const cache = yield* plannedCache(double, machine)
						const report = yield* cache.close()
						assert.equal(report.kind, "closed")
						const exit = yield* Effect.exit(Effect.scoped(cache.acquire(localBinding, work)))
						assert.ok(Exit.isFailure(exit))
						const error = Exit.findErrorOption(exit)
						assert.ok(error._tag === "Some")
						assert.equal(error.value.code, "ClosedHandle")
					})
				)
			)
		)
	})

	test("inspection decodes the bounded native report", async function inspection() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		const report = await Effect.runPromise(
			provideRuntime(
				Effect.scoped(
					Effect.gen(function* () {
						const cache = yield* plannedCache(double, machine)
						double.plan("logCacheInspect", {
							result: {
								openCount: 2,
								opening: 1,
								budgetBytes: 1n << 30n,
								maxOpen: 8,
								evictions: 5n,
								slots: [{ binding: "ab".repeat(16), state: "ready", borrows: 1, diskBytes: 4096n }]
							}
						})
						return yield* cache.inspect(work)
					})
				)
			)
		)
		assert.equal(report.openCount, 2)
		assert.equal(report.budget.maxOpen, 8)
		assert.equal(report.slots[0]?.state, "ready")
		assert.equal(report.slots[0]?.diskBytes, 4096n)
	})
})
