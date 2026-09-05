/**
 * Sealed commands: `seal` retains the exact captured native change, yields
 * an immutable ref before dispatch, refuses malformed declared results at
 * its boundary; `encode`/`decode` are the one bounded native command codec
 * (decode checks the supplied schema; SharedArrayBuffer-backed input
 * refuses). A spent/closed command refuses reuse. Maps to API-01/API-04/
 * API-06 (layer side) and the chapter 35 command rows; OPS-006.
 */
import assert from "node:assert/strict"
import { describe, test } from "node:test"
import type { AnySchema } from "@bjornpagen/bumbledb"
import { Effect, Exit } from "effect"
import { makeLogMachine } from "#machine.ts"
import type { CommandInput } from "#surface.ts"
import {
	closeRegisteredChange,
	makeIntegration,
	makeWireDouble,
	provideRuntime,
	refWire,
	registerChange,
	work
} from "#test/double.ts"

const schema = { name: "TestSchema" } as unknown as AnySchema

type Double = ReturnType<typeof makeWireDouble>
type Machine = ReturnType<typeof makeLogMachine>

function input(overrides: Partial<Record<"result" | "requestId", unknown>> = {}) {
	const changes = { __changes: true }
	registerChange(changes, { native: "change" })
	return {
		scope: {
			databaseId: refWire.identity.databaseId,
			incarnationId: refWire.identity.incarnationId,
			schemaId: refWire.identity.schemaId
		},
		id: { receiptEpoch: 1n, requestId: overrides.requestId ?? refWire.requestId },
		changes,
		precondition: { kind: "blind" as const },
		result: overrides.result ?? { attempt: "6f".repeat(16), units: 1n, score: 0.5, pinned: true }
	} as unknown as CommandInput<typeof schema>
}

function plannedSeal(double: Double, machine: Machine, sealInput = input()) {
	double.plan("logCommandSeal", { result: { command: { __command: true }, ref: refWire } })
	return machine.Command.seal(sealInput, work)
}

describe("Command.seal", function suite() {
	test("seal retains the registered native change and yields the ref before dispatch", async function seal() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		const ref = await Effect.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const command = yield* plannedSeal(double, machine)
					return command.ref
				})
			)
		)
		assert.equal(ref.digest, refWire.digest)
		assert.equal(ref.id.requestId, refWire.requestId)
		const seal = double.calls.find((call) => call.verb === "logCommandSeal")
		assert.ok(seal !== undefined)
		const request = seal.request as { change: unknown; request: { precondition: { kind: string } } }
		// The exact registered native change crossed; no reconstruction.
		assert.deepEqual(request.change, { native: "change" })
		assert.equal(request.request.precondition.kind, "blind")
		// The scope closed the command.
		assert.equal(double.calls.filter((call) => call.verb === "logCommandClose").length, 1)
	})

	test("a non-scalar declared result refuses at the seal boundary, before native work", async function badResult() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		const exit = await Effect.runPromiseExit(
			Effect.scoped(machine.Command.seal(input({ result: { nested: { object: true } } }), work))
		)
		assert.ok(Exit.isFailure(exit))
		assert.ok(Exit.hasFails(exit))
		assert.equal(double.calls.length, 0)
	})

	test("a declared-result bigint outside the u64/i64 split refuses before dispatch", async function bigintRange() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		// The canonical cell splits at the sign: nonnegative must fit u64,
		// negative must fit i64 (the P01R CommandScalar mapping, confirmed
		// against the native result_cell_in). Out of range is caller misuse
		// refused here, never a mid-marshal native throw.
		for (const value of [1n << 64n, -(1n << 63n) - 1n]) {
			const exit = await Effect.runPromiseExit(
				Effect.scoped(machine.Command.seal(input({ result: { units: value } }), work))
			)
			assert.ok(Exit.isFailure(exit))
			const error = Exit.findErrorOption(exit)
			assert.ok(error._tag === "Some")
			assert.equal(error.value.code, "InvalidArgument")
		}
		// The exact boundary values still cross.
		double.plan("logCommandSeal", { result: { command: { __command: true }, ref: refWire } })
		await Effect.runPromise(
			Effect.scoped(machine.Command.seal(input({ result: { hi: (1n << 64n) - 1n, lo: -(1n << 63n) } }), work))
		)
		assert.equal(double.calls.filter((call) => call.verb === "logCommandSeal").length, 1)
	})

	test("an unregistered (foreign) change refuses as a typed failure", async function foreignChange() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		const foreign = {
			...input(),
			changes: { __someone: "else" }
		} as unknown as CommandInput<typeof schema>
		const exit = await Effect.runPromiseExit(Effect.scoped(machine.Command.seal(foreign, work)))
		assert.ok(Exit.isFailure(exit))
		assert.equal(double.calls.length, 0)
	})

	test("a closed (spent) ChangeSet refuses with ClosedHandle before dispatch", async function closedChange() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		const sealInput = input()
		closeRegisteredChange(sealInput.changes as unknown as object)
		const exit = await Effect.runPromiseExit(Effect.scoped(machine.Command.seal(sealInput, work)))
		assert.ok(Exit.isFailure(exit))
		const error = Exit.findErrorOption(exit)
		assert.ok(error._tag === "Some")
		assert.equal(error.value.code, "ClosedHandle")
		assert.equal(double.calls.length, 0)
	})

	test("a scope/ChangeSet schema mismatch refuses before dispatch", async function schemaMismatch() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		const changes = { __changes: true }
		registerChange(changes, { native: "change" }, { schemaId: "9e".repeat(32) })
		const mismatched = { ...input(), changes } as unknown as CommandInput<typeof schema>
		const exit = await Effect.runPromiseExit(Effect.scoped(machine.Command.seal(mismatched, work)))
		assert.ok(Exit.isFailure(exit))
		const error = Exit.findErrorOption(exit)
		assert.ok(error._tag === "Some")
		assert.equal(error.value.code, "InvalidArgument")
		assert.equal(double.calls.length, 0)
	})
})

describe("Command.encode / Command.decode", function suite() {
	test("encode yields the native canonical bytes; a closed command refuses", async function encode() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		const bytes = new Uint8Array([9, 9, 9])
		const outcome = await Effect.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const command = yield* plannedSeal(double, machine)
					double.plan("logCommandEncode", { result: bytes })
					const encoded = yield* machine.Command.encode(command, work)
					yield* command.close()
					const closedExit = yield* Effect.exit(machine.Command.encode(command, work))
					return { encoded, closedExit }
				})
			)
		)
		assert.deepEqual(outcome.encoded, bytes)
		assert.ok(Exit.isFailure(outcome.closedExit))
		const error = Exit.findErrorOption(outcome.closedExit)
		assert.ok(error._tag === "Some")
		assert.equal(error.value.code, "ClosedHandle")
	})

	test("decode passes the schema for the identity check and yields a usable command", async function decode() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		double.plan("logCommandDecode", { result: { command: { __command: 2 }, ref: refWire } })
		const ref = await Effect.runPromise(
			provideRuntime(
				Effect.scoped(
					Effect.gen(function* () {
						const command = yield* machine.Command.decode(new Uint8Array([1, 2]), schema, work)
						return command.ref
					})
				)
			)
		)
		assert.equal(ref.digest, refWire.digest)
		const call = double.calls.find((item) => item.verb === "logCommandDecode")
		assert.ok(call !== undefined)
		assert.deepEqual((call.request as { schema: unknown }).schema, schema)
	})

	test("decode refuses a schema mismatch with the native typed refusal", async function decodeMismatch() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		double.plan("logCommandDecode", {
			failure: { source: "protocol", reason: { _tag: "ForeignIdentity" } }
		})
		const exit = await Effect.runPromiseExit(
			provideRuntime(Effect.scoped(machine.Command.decode(new Uint8Array([1]), schema, work)))
		)
		assert.ok(Exit.isFailure(exit))
		const error = Exit.findErrorOption(exit)
		assert.ok(error._tag === "Some")
		assert.equal(error.value.code, "ForeignIdentity")
	})

	test("decode refuses SharedArrayBuffer-backed views before any dispatch", async function sharedInput() {
		const double = makeWireDouble()
		const machine = makeLogMachine(double.wire, makeIntegration())
		const shared = new Uint8Array(new SharedArrayBuffer(4))
		const exit = await Effect.runPromiseExit(provideRuntime(Effect.scoped(machine.Command.decode(shared, schema, work))))
		assert.ok(Exit.isFailure(exit))
		assert.equal(double.calls.length, 0)
	})
})
