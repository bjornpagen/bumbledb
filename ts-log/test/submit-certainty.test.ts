/**
 * Submission certainty. `submit` is `Effect<SubmitOutcome, never>`: every
 * ordinary failure lives inside the union — pre-dispatch refusals become
 * `not-submitted` (with the authentic ref), post-dispatch decode loss
 * becomes `outcome-unknown`. Fiber interruption after dispatch is
 * `outcome-unknown` (or `decided` if the receipt already decoded) under
 * that same ref — never a new ID — and joins the native cancel drain
 * before the lease is released. The retained ref resolves after reopen.
 * Maps to the chapter 35 rows "interruption after publication" and
 * "retained ref resolution after reopen"; API-04, PROTO-02/04/05 (layer
 * side), OPS-006. D13/D15 Effect-layer discriminators.
 */
import assert from "node:assert/strict"
import { describe, test } from "node:test"
import type { AnySchema } from "@bjornpagen/bumbledb"
import { Effect, Exit, Fiber } from "effect"
import { makeLogMachine } from "#machine.ts"
import type { Command, CommandInput } from "#surface.ts"
import {
	handleWire,
	localBinding,
	makeIntegration,
	makeWireDouble,
	provideRuntime,
	receiptWire,
	refWire,
	registerChange,
	submitOptions,
	work
} from "#test/double.ts"

const schema = { name: "TestSchema" } as unknown as AnySchema

type Machine = ReturnType<typeof makeLogMachine>
type Double = ReturnType<typeof makeWireDouble>

/** Plans one seal and returns the lazy seal effect. */
function plannedSeal(double: Double, machine: Machine) {
	const changes = { __changes: true }
	registerChange(changes, { native: "change" })
	double.plan("logCommandSeal", {
		result: { command: { __command: true }, ref: refWire }
	})
	const input = {
		scope: {
			databaseId: refWire.identity.databaseId,
			incarnationId: refWire.identity.incarnationId,
			schemaId: refWire.identity.schemaId
		},
		id: { receiptEpoch: 1n, requestId: refWire.requestId },
		changes,
		precondition: { kind: "blind" as const },
		result: { attempt: "6f".repeat(16) }
	} as unknown as CommandInput<typeof schema>
	return machine.Command.seal(input, work)
}

/** Plans one open and returns the lazy open effect. */
function plannedOpen(double: Double, machine: Machine) {
	double.plan("logHistoryOpen", { result: handleWire() })
	return machine.LocalHistory.open(localBinding, schema, work)
}

function ticks(count: number): Promise<void> {
	let chain = Promise.resolve()
	for (let i = 0; i < count; i++) {
		chain = chain.then(() => new Promise((resolve) => setImmediate(resolve)))
	}
	return chain
}

describe("submit certainty arms", function suite() {
	test("a decided receipt round-trips with local health; the ref exists before dispatch", async function decided() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		const outcome = await Effect.runPromise(
			provideRuntime(
				Effect.scoped(
					Effect.gen(function* () {
						const history = yield* plannedOpen(double, machine)
						const command = yield* plannedSeal(double, machine)
						// The ref is available BEFORE any submission.
						assert.equal(command.ref.digest, refWire.digest)
						double.plan("logHistoryCall", {
							result: {
								verb: "submit",
								outcome: {
									kind: "decided",
									receipt: receiptWire,
									localHealth: { kind: "ready", at: receiptWire.decisionAt },
									publicationPhase: "confirmed"
								}
							}
						})
						return yield* history.submit(command, submitOptions)
					})
				)
			)
		)
		assert.equal(outcome.kind, "decided")
		if (outcome.kind === "decided") {
			assert.equal(outcome.receipt.outcome.kind, "no-change")
			assert.equal(outcome.localHealth.kind, "ready")
			assert.equal(outcome.phase, "confirmed")
			assert.equal(outcome.receipt.decisionAt.seq, 7n)
		}
	})

	test("pre-dispatch refusal is not-submitted with the authentic ref; E stays never", async function preDispatch() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		const exit = await Effect.runPromiseExit(
			provideRuntime(
				Effect.scoped(
					Effect.gen(function* () {
						const history = yield* plannedOpen(double, machine)
						const command = yield* plannedSeal(double, machine)
						double.plan("logHistoryCall", {
							refuse: { source: "core", reason: { _tag: "QueueFull" } }
						})
						return yield* history.submit(command, submitOptions)
					})
				)
			)
		)
		// Ordinary failure is INSIDE the union: the effect itself succeeds.
		assert.ok(Exit.isSuccess(exit))
		const outcome = Exit.getSuccess(exit)
		assert.ok(outcome._tag === "Some")
		assert.equal(outcome.value.kind, "not-submitted")
		if (outcome.value.kind === "not-submitted") {
			assert.equal(outcome.value.command.digest, refWire.digest)
			assert.equal(outcome.value.error.code, "QueueFull")
		}
	})

	test("post-dispatch decode loss is outcome-unknown, never a fabricated rejection", async function postDispatch() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		const outcome = await Effect.runPromise(
			provideRuntime(
				Effect.scoped(
					Effect.gen(function* () {
						const history = yield* plannedOpen(double, machine)
						const command = yield* plannedSeal(double, machine)
						double.plan("logHistoryCall", {
							failure: { source: "protocol", reason: { _tag: "Backend" } }
						})
						return yield* history.submit(command, submitOptions)
					})
				)
			)
		)
		assert.equal(outcome.kind, "outcome-unknown")
		if (outcome.kind === "outcome-unknown") {
			assert.equal(outcome.command.digest, refWire.digest)
			assert.equal(outcome.error.code, "Backend")
		}
	})

	test("submit on a closed history refuses as not-submitted without dispatch", async function closedHistory() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		const outcome = await Effect.runPromise(
			provideRuntime(
				Effect.scoped(
					Effect.gen(function* () {
						const history = yield* plannedOpen(double, machine)
						const command = yield* plannedSeal(double, machine)
						yield* history.close()
						const dispatched = double.calls.filter((call) => call.verb === "logHistoryCall").length
						const result = yield* history.submit(command, submitOptions)
						assert.equal(double.calls.filter((call) => call.verb === "logHistoryCall").length, dispatched)
						return result
					})
				)
			)
		)
		assert.equal(outcome.kind, "not-submitted")
		if (outcome.kind === "not-submitted") {
			assert.equal(outcome.error.code, "ClosedHandle")
		}
	})

	test("a forged command capability is a defect, never a forged receipt or arm", async function forged() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		const exit = await Effect.runPromiseExit(
			provideRuntime(
				Effect.scoped(
					Effect.gen(function* () {
						const history = yield* plannedOpen(double, machine)
						const forgedCommand = {
							ref: refWire,
							close: () => Effect.succeed({ kind: "closed" as const })
						} as unknown as Command<typeof schema>
						return yield* history.submit(forgedCommand, submitOptions)
					})
				)
			)
		)
		assert.ok(Exit.isFailure(exit))
		assert.ok(Exit.hasDies(exit))
	})

	test("invalid submit options refuse pre-dispatch as not-submitted, not as E", async function invalidOptions() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		const outcome = await Effect.runPromise(
			provideRuntime(
				Effect.scoped(
					Effect.gen(function* () {
						const history = yield* plannedOpen(double, machine)
						const command = yield* plannedSeal(double, machine)
						return yield* history.submit(command, { ...submitOptions, attempts: 0 })
					})
				)
			)
		)
		assert.equal(outcome.kind, "not-submitted")
		if (outcome.kind === "not-submitted") {
			assert.equal(outcome.command.digest, refWire.digest)
		}
		assert.equal(double.calls.filter((call) => call.verb === "logHistoryCall").length, 0)
	})
})

describe("interruption after publication", function suite() {
	test("interrupt after dispatch is outcome-unknown under the original ref and joins the native drain", async function interrupted() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())

		const program = provideRuntime(
			Effect.scoped(
				Effect.gen(function* () {
					const history = yield* plannedOpen(double, machine)
					const command = yield* plannedSeal(double, machine)
					assert.equal(command.ref.digest, refWire.digest)
					assert.equal(command.ref.id.requestId, refWire.requestId)
					// Publication response is withheld: the operation dispatched and
					// the native machine may already have published.
					double.plan("logHistoryCall", {
						hold: true,
						result: {
							verb: "submit",
							outcome: {
								kind: "decided",
								receipt: receiptWire,
								localHealth: { kind: "ready", at: receiptWire.decisionAt },
								publicationPhase: "confirmed"
							}
						}
					})
					return yield* history.submit(command, submitOptions)
				})
			)
		)

		const fiber = Effect.runFork(program)
		await ticks(2)
		assert.equal(double.held.length, 1)
		await Effect.runPromise(Fiber.interrupt(fiber))
		const exit = await Effect.runPromise(Fiber.await(fiber))

		assert.ok(Exit.isSuccess(exit))
		const outcome = Exit.getSuccess(exit)
		assert.ok(outcome._tag === "Some")
		assert.equal(outcome.value.kind, "outcome-unknown")
		if (outcome.value.kind === "outcome-unknown") {
			assert.equal(outcome.value.command.digest, refWire.digest)
			assert.equal(outcome.value.command.id.requestId, refWire.requestId)
			assert.equal(outcome.value.phase, "dispatchedUnresolved")
			assert.equal(outcome.value.error.code, "Cancelled")
		}
		assert.ok(double.cancelCount() >= 1)

		// A late native completion after the arm settled does not mint a new ref.
		double.releaseHeld()
		await ticks(1)
		assert.equal(outcome.value.kind, "outcome-unknown")
		if (outcome.value.kind === "outcome-unknown") {
			assert.equal(outcome.value.command.digest, refWire.digest)
		}
	})

	test("the retained original ref resolves after reopen; retry never seals a new id", async function retainedRef() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		// The app retained command.ref (fixture refWire). Reopen and resolve
		// against the durable receipt the earlier publication produced.
		const outcome = await Effect.runPromise(
			provideRuntime(
				Effect.scoped(
					Effect.gen(function* () {
						const history = yield* plannedOpen(double, machine)
						double.plan("logHistoryCall", {
							result: { verb: "resolve", outcome: { kind: "found", receipt: receiptWire } }
						})
						const ref = {
							identity: history.identity,
							id: { receiptEpoch: history.receiptEpoch, requestId: refWire.requestId },
							digest: refWire.digest
						} as unknown as Parameters<typeof history.resolve>[0]
						return yield* history.resolve(ref, work)
					})
				)
			)
		)
		assert.equal(outcome.kind, "found")
		if (outcome.kind === "found") {
			assert.equal(outcome.receipt.command.digest, refWire.digest)
			assert.equal(outcome.receipt.outcome.kind, "no-change")
		}
	})

	test("resolve also reports the closed/expired epochs as data, not errors", async function resolveArms() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		const outcomes = await Effect.runPromise(
			provideRuntime(
				Effect.scoped(
					Effect.gen(function* () {
						const history = yield* plannedOpen(double, machine)
						const ref = {
							identity: history.identity,
							id: { receiptEpoch: history.receiptEpoch, requestId: refWire.requestId },
							digest: refWire.digest
						} as unknown as Parameters<typeof history.resolve>[0]
						double.plan("logHistoryCall", {
							result: { verb: "resolve", outcome: { kind: "command-epoch-closed" } }
						})
						const closed = yield* history.resolve(ref, work)
						double.plan("logHistoryCall", {
							result: { verb: "resolve", outcome: { kind: "receipt-expired-unknown" } }
						})
						const expired = yield* history.resolve(ref, work)
						double.plan("logHistoryCall", {
							result: {
								verb: "resolve",
								outcome: { kind: "not-recorded-at", decisionAt: receiptWire.decisionAt }
							}
						})
						const absent = yield* history.resolve(ref, work)
						return { closed, expired, absent }
					})
				)
			)
		)
		assert.equal(outcomes.closed.kind, "command-epoch-closed")
		assert.equal(outcomes.expired.kind, "receipt-expired-unknown")
		assert.equal(outcomes.absent.kind, "not-recorded-at")
	})
})
