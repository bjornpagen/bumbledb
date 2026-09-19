/** Owned BEDC transport. Envelope recognition does not certify any map, face,
 * product, support or law; query preparation re-establishes those natively. */
import { Effect, Result } from "effect"
import { dbNative } from "#db-native.ts"
import { AuthoringError } from "#errors.ts"
import {
	decodeDescription,
	decodeInspection,
	type EventDescriptorDescription,
	type EventDescriptorInspectionWire,
	type EventDescriptorWire,
	encodeDescription
} from "#event-descriptor-data.ts"
import { nativeOperationWith, runtimeHandle } from "#runtime.ts"
import { argumentError, type DbError } from "#runtime-errors.ts"
import { bytesValue } from "#values.ts"

const descriptorTag: unique symbol = Symbol("bumbledb.EventDescriptor")
interface EventDescriptor {
	readonly [descriptorTag]: true
}
const encodings = new WeakMap<EventDescriptor, Uint8Array>()

function isDescriptor(input: unknown): input is EventDescriptor {
	return typeof input === "object" && input !== null && encodings.has(input as EventDescriptor)
}
function encoding(input: EventDescriptor): Uint8Array {
	const bytes = encodings.get(input)
	if (bytes === undefined) throw new AuthoringError({ message: "Event descriptor: expected an owned BEDC value" })
	return bytes
}
function descriptorLength(input: EventDescriptor): number {
	return encoding(input).byteLength
}
function toBytes(input: EventDescriptor): Uint8Array {
	return new Uint8Array(encoding(input))
}
function encodedDescriptor(input: unknown): EventDescriptor {
	const bytes = bytesValue("Event descriptor", input, 16 * 1024 * 1024)
	if (bytes.length < 5 || bytes[0] !== 66 || bytes[1] !== 69 || bytes[2] !== 68 || bytes[3] !== 67 || bytes[4] !== 1)
		throw new AuthoringError({ message: "Event descriptor: expected a BEDC v1 envelope" })
	const value: EventDescriptor = Object.freeze({ [descriptorTag]: true as const })
	encodings.set(value, bytes)
	return value
}
function fromBytes(input: Uint8Array): Result.Result<EventDescriptor, DbError> {
	return Result.try({
		try: () => encodedDescriptor(input),
		catch: (cause) => argumentError("EventDescriptor.fromBytes", cause)
	})
}

/** Copy plain data at Effect execution, then reconstruct every certificate on
 * the cancellable worker. Success returns canonical owned BEDC transport. */
const admit = Effect.fn("EventDescriptor.admit")(function* (description: EventDescriptorDescription) {
	const runtime = yield* runtimeHandle()
	const input = yield* Effect.try({
		try: () => encodeDescription(description),
		catch: (cause) => argumentError("EventDescriptor.admit", cause)
	})
	return yield* nativeOperationWith(
		"EventDescriptor.admit",
		(callback) => dbNative.runtimeEventDescriptor(runtime, "admit", input, callback),
		dbNative.runtimeBytesTake,
		encodedDescriptor
	)
})
const describe = Effect.fn("EventDescriptor.describe")(function* (value: EventDescriptor) {
	const runtime = yield* runtimeHandle()
	const input = yield* Effect.try({
		try: () => toBytes(value),
		catch: (cause) => argumentError("EventDescriptor.describe", cause)
	})
	return yield* nativeOperationWith(
		"EventDescriptor.describe",
		(callback) => dbNative.runtimeEventDescriptor(runtime, "describe", input, callback),
		(operation) => dbNative.runtimeEventDescriptorTake(operation) as EventDescriptorWire,
		decodeDescription
	)
})
const inspect = Effect.fn("EventDescriptor.inspect")(function* (value: EventDescriptor) {
	const runtime = yield* runtimeHandle()
	const input = yield* Effect.try({
		try: () => toBytes(value),
		catch: (cause) => argumentError("EventDescriptor.inspect", cause)
	})
	return yield* nativeOperationWith(
		"EventDescriptor.inspect",
		(callback) => dbNative.runtimeEventDescriptor(runtime, "inspect", input, callback),
		(operation) => dbNative.runtimeEventDescriptorTake(operation) as EventDescriptorInspectionWire,
		decodeInspection
	)
})

const EventDescriptor = Object.freeze({ fromBytes, toBytes, isDescriptor, admit, describe, inspect })

export { descriptorLength, EventDescriptor, encodedDescriptor }
