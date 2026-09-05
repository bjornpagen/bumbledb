/**
 * Close reports and finalizer policy: a known receipt is durable evidence
 * that a later cleanup failure cannot revoke — the scope dies with a
 * structured `CloseFailure` defect while the receipt stays retained; an
 * incomplete close is reported, never counted as reclaimed. Maps to the
 * chapter 35 row "known receipt, then finalizer defect"; API-04/API-10,
 * OPS-TEST-02 (layer side), OPS-006. D18 close/finalizer discriminator.
 */
import assert from "node:assert/strict"
import { describe, test } from "node:test"
import type { AnySchema } from "@bjornpagen/bumbledb"
import { CloseFailure } from "@bjornpagen/bumbledb"
import { Effect, Exit } from "effect"
import { makeLogMachine } from "#machine.ts"
import type { TerminalReceipt } from "#outcome.ts"
import type { CommandInput } from "#surface.ts"
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

const outstanding = {
	phase: "closing" as const,
	queued: 0n,
	active: 1n,
	retained: 1n,
	owners: 1n,
	databases: 1n,
	inputBytes: 0n,
	workingBytes: 64n,
	scratchBytes: 0n,
	resultBytes: 0n
}

function plannedSeal(double: ReturnType<typeof makeWireDouble>, machine: ReturnType<typeof makeLogMachine>) {
	const changes = { __changes: true }
	registerChange(changes, { native: "change" })
	double.plan("logCommandSeal", { result: { command: { __command: true }, ref: refWire } })
	const input = {
		scope: {
			databaseId: refWire.identity.databaseId,
			incarnationId: refWire.identity.incarnationId,
			schemaId: refWire.identity.schemaId
		},
		id: { receiptEpoch: 1n, requestId: refWire.requestId },
		changes,
		precondition: { kind: "blind" as const },
		result: {}
	} as unknown as CommandInput<typeof schema>
	return machine.Command.seal(input, work)
}

describe("known receipt, then finalizer defect", function suite() {
	test("the scope dies with CloseFailure while the receipt stays retained", async function receiptThenDefect() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		const retained: TerminalReceipt[] = []

		// The owner's close fails AFTER the decided receipt was delivered.
		double.planClose("logHistoryClose", { kind: "failed" })

		const exit = await Effect.runPromiseExit(
			provideRuntime(
				Effect.scoped(
					Effect.gen(function* () {
						double.plan("logHistoryOpen", { result: handleWire() })
						const history = yield* machine.LocalHistory.open(localBinding, schema, work)
						const command = yield* plannedSeal(double, machine)
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
						const outcome = yield* history.submit(command, submitOptions)
						assert.equal(outcome.kind, "decided")
						if (outcome.kind === "decided") {
							// The app persists the receipt in its own job state BEFORE the
							// enclosing scope finishes: the final Exit is a separate fact.
							retained.push(outcome.receipt)
						}
						return outcome
					})
				)
			)
		)

		// The receipt was observed and retained…
		assert.equal(retained.length, 1)
		assert.equal(retained[0]?.command.digest, refWire.digest)
		// …and the scope reports the cleanup failure as a structured defect —
		// never success, never a rewritten submit outcome.
		assert.ok(Exit.isFailure(exit))
		assert.ok(Exit.hasDies(exit))
		const defect = Exit.findDefect(exit)
		assert.ok(defect._tag === "Success")
		assert.ok(defect.success instanceof CloseFailure)
	})

	test("incomplete close is reported and repeated close joins; nothing claims closed", async function incomplete() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		double.planClose("logHistoryClose", { kind: "incomplete", outstanding })
		double.planClose("logHistoryClose", { kind: "incomplete", outstanding })
		double.planClose("logHistoryClose", { kind: "incomplete", outstanding })

		const exit = await Effect.runPromiseExit(
			provideRuntime(
				Effect.scoped(
					Effect.gen(function* () {
						double.plan("logHistoryOpen", { result: handleWire() })
						const history = yield* machine.LocalHistory.open(localBinding, schema, work)
						const first = yield* history.close()
						assert.equal(first.kind, "incomplete")
						const second = yield* history.close()
						assert.equal(second.kind, "incomplete")
						return first
					})
				)
			)
		)
		// The finalizer surfaces the still-incomplete drain as a defect.
		assert.ok(Exit.isFailure(exit))
		assert.ok(Exit.hasDies(exit))
		// Three close transitions were joined (two explicit, one finalizer).
		assert.equal(double.calls.filter((call) => call.verb === "logHistoryClose").length, 3)
	})

	test("a snapshot's failed close is a finalizer defect too", async function snapshotClose() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		double.planClose("logSnapshotClose", { kind: "failed" })

		const exit = await Effect.runPromiseExit(
			provideRuntime(
				Effect.scoped(
					Effect.gen(function* () {
						double.plan("logHistoryOpen", { result: handleWire() })
						const history = yield* machine.LocalHistory.open(localBinding, schema, work)
						double.plan("logHistoryCall", {
							result: {
								verb: "snapshot",
								snapshot: { __snapshot: true },
								core: { __core: true },
								provenance: {
									identity: handleWire().meta.identity,
									decision: receiptWire.decisionAt,
									state: receiptWire.stateAt,
									freshness: { kind: "latest" }
								}
							}
						})
						const snapshot = yield* history.snapshot({ ...work, consistency: { kind: "latest" } })
						assert.equal(snapshot.freshness.kind, "latest")
						assert.equal(snapshot.decisionStamp.seq, 7n)
						return snapshot.stateStamp
					})
				)
			)
		)
		assert.ok(Exit.isFailure(exit))
		assert.ok(Exit.hasDies(exit))
	})
})
