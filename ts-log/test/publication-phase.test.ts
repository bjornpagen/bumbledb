/**
 * Publication phase crosses the native bridge explicitly (identities.rs
 * `publicationPhase`). Effect decoders preserve the phase from wire data —
 * never infer it from generic I/O error names. LOG-001: post-dispatch admin
 * failures wire `outcome-unknown` with `dispatchedUnresolved`, not
 * `not-started`.
 */
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
	submitOptions,
	work
} from "#test/double.ts"

const schema = { name: "TestSchema" } as unknown as AnySchema
const OPERATION = "4b".repeat(16) as OperationId
const adminOptions = { ...work, operationId: OPERATION }

describe("publication phase on the wire", function suite() {
	test("submit decided preserves confirmed phase from native wire", async function submitConfirmed() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		const outcome = await Effect.runPromise(
			provideRuntime(
				Effect.scoped(
					Effect.gen(function* () {
						double.plan("logHistoryOpen", { result: handleWire() })
						const history = yield* machine.LocalHistory.open(localBinding, schema, work)
						double.plan("logCommandSeal", {
							result: { command: { __command: true }, ref: refWire }
						})
						const command = yield* machine.Command.seal(
							{
								scope: {
									databaseId: refWire.identity.databaseId,
									incarnationId: refWire.identity.incarnationId,
									schemaId: refWire.identity.schemaId
								},
								id: { receiptEpoch: 1n, requestId: refWire.requestId },
								changes: { __changes: true },
								precondition: { kind: "blind" },
								result: { attempt: "6f".repeat(16) }
							} as never,
							work
						)
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
			assert.equal(outcome.phase, "confirmed")
		}
	})

	test("post-dispatch admin decode preserves dispatchedUnresolved (LOG-001)", async function adminPostDispatch() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		double.plan("logAdmin", {
			result: {
				certainty: "outcome-unknown",
				publicationPhase: "dispatchedUnresolved",
				error: { source: "protocol", reason: { _tag: "Backend" } }
			}
		})
		const outcome = await Effect.runPromise(
			provideRuntime(machine.admin.rotateReceiptEpoch(localBinding, adminOptions))
		)
		assert.equal(outcome.kind, "outcome-unknown")
		if (outcome.kind === "outcome-unknown") {
			assert.equal(outcome.phase, "dispatchedUnresolved")
			assert.equal(outcome.error.code, "Backend")
		}
	})

	test("proved nonpublication admin stays not-started with native phase", async function adminProvedNonpublication() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		double.plan("logAdmin", {
			result: {
				certainty: "not-started",
				publicationPhase: "provedNonpublication",
				error: { source: "protocol", reason: { _tag: "OperationConflict" } }
			}
		})
		const outcome = await Effect.runPromise(
			provideRuntime(machine.admin.checkpoint(localBinding, adminOptions))
		)
		assert.equal(outcome.kind, "not-started")
		if (outcome.kind === "not-started") {
			assert.equal(outcome.phase, "provedNonpublication")
		}
	})
})
