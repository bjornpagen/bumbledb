/** Returned certainty carries its evidence without an independently writable phase. */
import assert from "node:assert/strict"
import { describe, test } from "node:test"
import type { AnySchema } from "@bjornpagen/bumbledb"
import { Effect } from "effect"
import type { OperationId } from "#identity.ts"
import { makeLogMachine } from "#machine.ts"
import {
	handleWire,
	localBinding,
	makeIntegration,
	makeWireDouble,
	provideRuntime,
	receiptWire,
	refWire,
	registerChange,
	submitOptions
} from "#test/double.ts"

const schema = { name: "TestSchema" } as unknown as AnySchema
const OPERATION = "4b4b4b4b-4b4b-4b4b-4b4b-4b4b4b4b4b4b" as OperationId
const adminOptions = { operationId: OPERATION }

describe("returned certainty without duplicate phase", function suite() {
	test("submit decided retains its receipt without a phase field", async function submitConfirmed() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		const outcome = await Effect.runPromise(
			provideRuntime(
				Effect.scoped(
					Effect.gen(function* () {
						double.plan("logHistoryOpen", { result: handleWire() })
						const history = yield* machine.LocalHistory.open(localBinding, schema)
						double.plan("logCommandSeal", {
							result: { command: { __command: true }, ref: refWire }
						})
						const changes = { __changes: true }
						registerChange(changes, { native: "change" })
						const command = yield* machine.Command.seal({
							scope: {
								databaseId: refWire.identity.databaseId,
								incarnationId: refWire.identity.incarnationId,
								schemaId: refWire.identity.schemaId
							},
							id: { receiptEpoch: 1n, requestId: refWire.requestId },
							changes,
							precondition: { kind: "blind" },
							result: { attempt: "6f6f6f6f-6f6f-6f6f-6f6f-6f6f6f6f6f6f" }
						} as never)
						double.plan("logHistoryCall", {
							result: {
								verb: "submit",
								outcome: {
									kind: "decided",
									receipt: receiptWire,
									localHealth: { kind: "ready", at: receiptWire.decisionAt }
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
			assert.equal("phase" in outcome, false)
		}
	})

	test("post-dispatch admin decode retains outcome-unknown", async function adminPostDispatch() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		double.plan("logAdmin", {
			result: {
				certainty: "outcome-unknown",

				error: { source: "protocol", reason: { _tag: "Backend" } }
			}
		})
		const outcome = await Effect.runPromise(
			provideRuntime(machine.admin.rotateReceiptEpoch(localBinding, adminOptions))
		)
		assert.equal(outcome.kind, "outcome-unknown")
		if (outcome.kind === "outcome-unknown") {
			assert.equal("phase" in outcome, false)
			assert.equal(outcome.error.code, "Backend")
		}
	})

	test("native proved nonpublication stays not-started", async function adminProvedNonpublication() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		double.plan("logAdmin", {
			result: {
				certainty: "not-started",

				error: { source: "protocol", reason: { _tag: "OperationConflict" } }
			}
		})
		const outcome = await Effect.runPromise(provideRuntime(machine.admin.checkpoint(localBinding, adminOptions)))
		assert.equal(outcome.kind, "not-started")
		if (outcome.kind === "not-started") {
			assert.equal("phase" in outcome, false)
		}
	})
})
