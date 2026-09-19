/** Owned BEDC transport. Envelope recognition does not certify any map, face,
 * product, support or law; query preparation re-establishes those natively. */
import { Result } from "effect"
import { AuthoringError } from "#errors.ts"
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

const EventDescriptor = Object.freeze({ fromBytes, toBytes, isDescriptor })

export { descriptorLength, EventDescriptor, encodedDescriptor }
