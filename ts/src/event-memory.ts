/** Owned BEBM possibility-memory transport. Envelope recognition is pure;
 * worker admission reconstructs reachable memory from the checked recipe. */
import { Effect, Result } from "effect"
import { dbNative } from "#db-native.ts"
import { AuthoringError } from "#errors.ts"
import { compileMemory, type EventMemoryIdentities } from "#event-memory-arena.ts"
import {
	decodeMemory,
	decodeMemoryInspection,
	type EventMemoryDescription,
	type EventMemoryInspectionWire,
	type EventMemoryWire,
	encodeMemory
} from "#event-memory-data.ts"
import { nativeOperationWith, runtimeHandle } from "#runtime.ts"
import { argumentError, type DbError } from "#runtime-errors.ts"
import { bytesValue } from "#values.ts"

const memoryTag: unique symbol = Symbol("bumbledb.EventMemory")
interface EventMemory {
	readonly [memoryTag]: true
}
const encodings = new WeakMap<EventMemory, Uint8Array>()
function isMemory(input: unknown): input is EventMemory {
	return typeof input === "object" && input !== null && encodings.has(input as EventMemory)
}
function toBytes(input: EventMemory): Uint8Array {
	const bytes = encodings.get(input)
	if (bytes === undefined) throw new AuthoringError({ message: "Event memory: expected an owned BEBM value" })
	return new Uint8Array(bytes)
}
function encodedMemory(input: unknown): EventMemory {
	const bytes = bytesValue("Event memory", input, 16 * 1024 * 1024)
	if (bytes.length < 5 || bytes[0] !== 66 || bytes[1] !== 69 || bytes[2] !== 66 || bytes[3] !== 77 || bytes[4] !== 1)
		throw new AuthoringError({ message: "Event memory: expected a BEBM v1 envelope" })
	const memory = Object.freeze({ [memoryTag]: true as const })
	encodings.set(memory, bytes)
	return memory
}
function fromBytes(input: Uint8Array): Result.Result<EventMemory, DbError> {
	return Result.try({
		try: () => encodedMemory(input),
		catch: (cause) => argumentError("EventMemory.fromBytes", cause)
	})
}
/** Replay checked native construction; action/observation indices retain order. */
const admit = Effect.fn("EventMemory.admit")(function* (description: EventMemoryDescription) {
	const runtime = yield* runtimeHandle()
	const input = yield* Effect.try({
		try: () => encodeMemory(description),
		catch: (cause) => argumentError("EventMemory.admit", cause)
	})
	return yield* nativeOperationWith(
		"EventMemory.admit",
		(callback) => dbNative.runtimeEventMemory(runtime, "admit", input, callback),
		dbNative.runtimeBytesTake,
		encodedMemory
	)
})
/** The retained recipe uses the first action's normalized product presentation. */
const describe = Effect.fn("EventMemory.describe")(function* (value: EventMemory) {
	const runtime = yield* runtimeHandle()
	const input = yield* Effect.try({
		try: () => toBytes(value),
		catch: (cause) => argumentError("EventMemory.describe", cause)
	})
	return yield* nativeOperationWith(
		"EventMemory.describe",
		(callback) => dbNative.runtimeEventMemory(runtime, "describe", input, callback),
		(operation) => dbNative.runtimeEventMemoryTake(operation) as EventMemoryWire,
		decodeMemory
	)
})
/** Detached graph data, reconstructed natively. No graph is trusted from bytes. */
const inspect = Effect.fn("EventMemory.inspect")(function* (value: EventMemory) {
	const runtime = yield* runtimeHandle()
	const input = yield* Effect.try({
		try: () => toBytes(value),
		catch: (cause) => argumentError("EventMemory.inspect", cause)
	})
	return yield* nativeOperationWith(
		"EventMemory.inspect",
		(callback) => dbNative.runtimeEventMemory(runtime, "inspect", input, callback),
		(operation) => dbNative.runtimeEventMemoryTake(operation) as EventMemoryInspectionWire,
		decodeMemoryInspection
	)
})
// Resolve the arena module only when invoked: either module may be imported
// first, including through the general action adapter.
function compile(memory: EventMemory, identities: EventMemoryIdentities) {
	return compileMemory(memory, identities)
}
const EventMemory = Object.freeze({ fromBytes, toBytes, isMemory, admit, describe, inspect, compile })

export { EventMemory, encodedMemory, toBytes as memoryBytes }
