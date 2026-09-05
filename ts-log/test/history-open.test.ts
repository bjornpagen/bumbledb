/**
 * Opening histories: open NEVER initializes (no hidden genesis — zero
 * create dispatches on any open path), create is explicit and requires the
 * checked initialization artifact, bindings are backend-discriminated, and
 * closed capabilities refuse before dispatch. Maps to the chapter 35 "no
 * hidden genesis" and "closed/foreign capability refusal" test rows;
 * API-04/API-08, PROTO-19 (layer side), OPS-006.
 */
import assert from "node:assert/strict"
import { describe, test } from "node:test"
import type { AnySchema } from "@bjornpagen/bumbledb"
import { DbError } from "@bjornpagen/bumbledb"
import { Effect, Exit } from "effect"
import { ProtocolError } from "#errors.ts"
import type { OperationId } from "#identity.ts"
import { makeLogMachine } from "#machine.ts"
import type { HostedBinding } from "#options.ts"
import {
	handleWire,
	identityWire,
	localBinding,
	makeIntegration,
	makeWireDouble,
	provideRuntime,
	work
} from "#test/double.ts"

const schema = { name: "TestSchema" } as unknown as AnySchema

describe("LocalHistory.open", function suite() {
	test("open dispatches mode=open and never a create", async function openOnly() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		double.plan("logHistoryOpen", { result: handleWire() })
		const identity = await Effect.runPromise(
			provideRuntime(
				Effect.scoped(
					Effect.gen(function* () {
						const history = yield* machine.LocalHistory.open(localBinding, schema, work)
						return history.identity
					})
				)
			)
		)
		assert.equal(identity.databaseId, identityWire.databaseId)
		const open = double.calls.filter((call) => call.verb === "logHistoryOpen")
		assert.equal(open.length, 1)
		const request = open[0]?.request as { mode: string; creation: unknown }
		assert.equal(request.mode, "open")
		assert.equal(request.creation, null)
		// Scope closed the owner exactly once.
		assert.equal(double.calls.filter((call) => call.verb === "logHistoryClose").length, 1)
	})

	test("a missing database is a typed refusal, never an empty replacement", async function missing() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		double.plan("logHistoryOpen", {
			failure: { source: "protocol", reason: { _tag: "DatabaseMissing" } }
		})
		const exit = await Effect.runPromiseExit(
			provideRuntime(Effect.scoped(machine.LocalHistory.open(localBinding, schema, work)))
		)
		assert.ok(Exit.isFailure(exit))
		const failures = double.calls.map((call) => call.verb)
		assert.ok(!failures.includes("logCommandSeal"))
		assert.equal(double.calls.filter((call) => call.verb === "logHistoryOpen").length, 1)
	})

	test("create requires the creation artifact and dispatches mode=create", async function create() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		double.plan("logHistoryOpen", { result: handleWire() })
		await Effect.runPromise(
			provideRuntime(
				Effect.scoped(
					machine.LocalHistory.create(localBinding, schema, {
						...work,
						creation: {
							operationId: "4b".repeat(16) as OperationId,
							artifact: new Uint8Array([1, 2, 3])
						}
					})
				)
			)
		)
		const request = double.calls[0]?.request as { mode: string; creation: { operationId: string } }
		assert.equal(request.mode, "create")
		assert.equal(request.creation.operationId, "4b".repeat(16))
	})

	test("create refuses existing authority with the native refusal, unchanged", async function exists() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		double.plan("logHistoryOpen", {
			failure: { source: "protocol", reason: { _tag: "AuthorityExists" } }
		})
		const exit = await Effect.runPromiseExit(
			provideRuntime(
				Effect.scoped(
					machine.LocalHistory.create(localBinding, schema, {
						...work,
						creation: { operationId: "4b".repeat(16) as OperationId, artifact: new Uint8Array([1]) }
					})
				)
			)
		)
		assert.ok(Exit.isFailure(exit))
		assert.ok(Exit.hasFails(exit))
	})

	test("a hosted binding refuses at the local constructor before dispatch", async function foreignBinding() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		const hosted = {
			kind: "hosted",
			directory: "/tmp/x",
			origin: { bucket: "b", prefix: "p" },
			identity: localBinding.identity
		} as unknown as Parameters<typeof machine.LocalHistory.open>[0]
		const exit = await Effect.runPromiseExit(
			provideRuntime(Effect.scoped(machine.LocalHistory.open(hosted, schema, work)))
		)
		assert.ok(Exit.isFailure(exit))
		assert.equal(double.calls.length, 0)
	})
})

describe("HostedHistory.open", function suite() {
	test("credentials default to the supported provider chain", async function providerChain() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		double.plan("logHistoryOpen", { result: handleWire() })
		const hosted: HostedBinding = {
			kind: "hosted",
			directory: "/tmp/bumbledb-hosted",
			origin: { bucket: "tenants", prefix: "app/t1" },
			identity: localBinding.identity
		}
		await Effect.runPromise(provideRuntime(Effect.scoped(machine.HostedHistory.open(hosted, schema, work))))
		const request = double.calls[0]?.request as {
			binding: { kind: string; credentials: { kind: string } }
		}
		assert.equal(request.binding.kind, "hosted")
		assert.equal(request.binding.credentials.kind, "provider-chain")
	})
})

describe("bounded inspect", function suite() {
	test("inspect decodes the bounded health snapshot (OPS-TEST-01 fixture shape)", async function inspect() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		double.plan("logHistoryOpen", { result: handleWire() })
		const report = await Effect.runPromise(
			provideRuntime(
				Effect.scoped(
					Effect.gen(function* () {
						const history = yield* machine.LocalHistory.open(localBinding, schema, work)
						double.plan("logHistoryCall", {
							result: {
								verb: "inspect",
								inspection: {
									identity: identityWire,
									accessMode: "frozen",
									headRevision: 41n,
									decision: { seq: 7n, hash: "3c".repeat(32) },
									state: { incarnation: identityWire.incarnationId, dataRevision: 4n },
									openEpoch: 2n,
									retiredThrough: 1n,
									tailCount: 12n,
									tailBytes: 4096n,
									unknownCount: 1n,
									unknownOldestMillis: 12000,
									rootCount: 3,
									rootCapacity: 64,
									gc: "marking",
									lastMaintenanceError: null,
									diskBytes: 1n << 20n,
									workingBytes: 512n,
									queued: 0n,
									active: 1n
								}
							}
						})
						return yield* history.inspect(work)
					})
				)
			)
		)
		assert.equal(report.accessMode, "frozen")
		assert.equal(report.receipts.openEpoch, 2n)
		assert.equal(report.receipts.retiredThrough, 1n)
		assert.equal(report.tail.count, 12n)
		assert.equal(report.unknownCommands.oldestMillis, 12000)
		assert.equal(report.roots.capacity, 64)
		assert.equal(report.gc, "marking")
		assert.equal(report.lastMaintenanceError, null)
	})
})

describe("closed capabilities", function suite() {
	test("resolve and inspect on a closed history refuse with ClosedHandle before dispatch", async function closedRefusal() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		double.plan("logHistoryOpen", { result: handleWire() })
		const outcome = await Effect.runPromise(
			provideRuntime(
				Effect.scoped(
					Effect.gen(function* () {
						const history = yield* machine.LocalHistory.open(localBinding, schema, work)
						yield* history.close()
						const calls = double.calls.length
						const exit = yield* Effect.exit(history.inspect(work))
						return { calls, exit, after: double.calls.length }
					})
				)
			)
		)
		assert.ok(Exit.isFailure(outcome.exit))
		assert.ok(Exit.hasFails(outcome.exit))
		const failure = Exit.findErrorOption(outcome.exit)
		assert.ok(failure._tag === "Some")
		const error = failure.value
		assert.ok(error instanceof DbError || error instanceof ProtocolError)
		assert.equal(error.code, "ClosedHandle")
		// No new native dispatch happened for the refused call.
		assert.equal(outcome.after, outcome.calls)
	})
})
