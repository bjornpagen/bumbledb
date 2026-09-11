/**
 * Administrative certainty wrappers: mutating operations return
 * `AdminOutcome` in A with E = never and a stable operation reference
 * derived BEFORE dispatch; `not-started` proves this invocation performed
 * no authoritative mutation; interruption after dispatch is
 * `outcome-unknown` (or `completed` if the receipt already decoded) under
 * the original operationId — never a new ID. Read-only status/verification
 * has typed E.
 * Maps to OPS-006 (primary audit row), OPS-TEST-01 (status fixtures,
 * layer side), and ERASE-03 reporting shape.
 */
import assert from "node:assert/strict"
import { describe, test } from "node:test"
import { Effect, Exit, Fiber } from "effect"
import type { OperationId } from "#identity.ts"
import { makeLogMachine } from "#machine.ts"
import {
	identityWire,
	localBinding,
	makeIntegration,
	makeWireDouble,
	provideRuntime,
	stampWire,
	stateWire
} from "#test/double.ts"

const OPERATION = "4b4b4b4b-4b4b-4b4b-4b4b-4b4b4b4b4b4b" as OperationId
const adminOptions = { operationId: OPERATION }

type Double = ReturnType<typeof makeWireDouble>
type Machine = ReturnType<typeof makeLogMachine>

function make(): { double: Double; machine: Machine } {
	const double = makeWireDouble()
	return { double, machine: makeLogMachine(double.wire, makeIntegration()) }
}

describe("admin certainty", function suite() {
	test("checkpoint completes with its report and the pre-dispatch ref", async function checkpoint() {
		const { double, machine } = make()
		double.plan("logAdmin", {
			result: {
				certainty: "completed",

				value: { verb: "checkpoint", at: stampWire, state: stateWire, root: "root-1" }
			}
		})
		const outcome = await Effect.runPromise(provideRuntime(machine.admin.checkpoint(localBinding, adminOptions)))
		assert.equal(outcome.kind, "completed")
		assert.equal(outcome.ref.operation, OPERATION)
		assert.equal(outcome.ref.identity.databaseId, identityWire.databaseId)
		if (outcome.kind === "completed") {
			assert.equal(outcome.value.root, "root-1")
			assert.equal(outcome.value.at.seq, 7n)
			assert.equal("phase" in outcome, false)
		}
		// The operation id crossed the wire with the request (fixed before dispatch).
		const request = double.calls[0]?.request as { operationId: string }
		assert.equal(request.operationId, OPERATION)
	})

	test("registration refusal is not-started and E stays never", async function notStarted() {
		const { double, machine } = make()
		double.plan("logAdmin", { refuse: { source: "core", reason: { _tag: "QueueFull" } } })
		const exit = await Effect.runPromiseExit(provideRuntime(machine.admin.collectGarbage(localBinding, adminOptions)))
		assert.ok(Exit.isSuccess(exit))
		const outcome = Exit.getSuccess(exit)
		assert.ok(outcome._tag === "Some")
		assert.equal(outcome.value.kind, "not-started")
		if (outcome.value.kind === "not-started") {
			assert.equal(outcome.value.ref.operation, OPERATION)
			assert.equal(outcome.value.error.code, "QueueFull")
		}
	})

	test("a lost completion is outcome-unknown with the retained ref", async function unknown() {
		const { double, machine } = make()
		double.plan("logAdmin", { failure: { source: "protocol", reason: { _tag: "Backend" } } })
		const outcome = await Effect.runPromise(
			provideRuntime(
				machine.admin.backup(localBinding, {
					...adminOptions,
					destination: { kind: "filesystem", directory: "/tmp/bumbledb-backup" }
				})
			)
		)
		assert.equal(outcome.kind, "outcome-unknown")
		if (outcome.kind === "outcome-unknown") {
			assert.equal(outcome.ref.operation, OPERATION)
			assert.equal(outcome.error.code, "Backend")
			assert.equal("phase" in outcome, false)
		}
	})

	test("erase reports residual copies honestly", async function erase() {
		const { double, machine } = make()
		double.plan("logAdmin", {
			result: {
				certainty: "completed",

				value: {
					verb: "erase",
					tombstoned: true,
					retainedRoots: ["root-legal-hold"],
					residual: [{ kind: "backup", location: "s3://backups/t1" }]
				}
			}
		})
		const outcome = await Effect.runPromise(
			provideRuntime(machine.admin.erase(localBinding, { ...adminOptions, retainRoots: [] }))
		)
		assert.equal(outcome.kind, "completed")
		if (outcome.kind === "completed") {
			assert.equal(outcome.value.tombstoned, true)
			assert.equal(outcome.value.residual[0]?.kind, "backup")
		}
	})

	test("verifyBackup is read-only with typed E", async function verify() {
		const { double, machine } = make()
		double.plan("logAdmin", { failure: { source: "protocol", reason: { _tag: "UnsupportedArtifact" } } })
		const exit = await Effect.runPromiseExit(
			provideRuntime(machine.admin.verifyBackup({ kind: "filesystem", directory: "/tmp/x" }, {}))
		)
		assert.ok(Exit.isFailure(exit))
		const error = Exit.findErrorOption(exit)
		assert.ok(error._tag === "Some")
		assert.equal(error.value.code, "UnsupportedArtifact")
	})

	test("interrupting admin joins cancellation and retains the caller's operationId", async function interrupted() {
		const { double, machine } = make()
		double.plan("logAdmin", {
			hold: true,
			result: {
				certainty: "completed",

				value: { verb: "checkpoint", at: stampWire, state: stateWire, root: "root-2" }
			}
		})
		// The ref exists BEFORE dispatch: the app persisted operationId already.
		const fiber = Effect.runFork(provideRuntime(machine.admin.checkpoint(localBinding, adminOptions)))
		await new Promise((resolve) => setImmediate(resolve))
		await Effect.runPromise(Fiber.interrupt(fiber))
		const exit = await Effect.runPromise(Fiber.await(fiber))
		assert.ok(Exit.hasInterrupts(exit))
		assert.equal(adminOptions.operationId, OPERATION)
		assert.ok(double.cancelCount() >= 1)
	})
})
