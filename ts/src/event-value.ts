/** Immutable owned BEVT carrier. An SDK value is transport data; the native
 * decoder remains the mathematical admission authority on every crossing. */
import { AuthoringError } from "#errors.ts"
import { bytesValue } from "#values.ts"

const eventTag: unique symbol = Symbol("bumbledb.Event")
interface Event {
	readonly [eventTag]: true
}
const encodings = new WeakMap<Event, Uint8Array>()
const MAX_EVENT_BYTES = 16 * 1024 * 1024

function isEvent(value: unknown): value is Event {
	return typeof value === "object" && value !== null && encodings.has(value as Event)
}
function eventValue(context: string, value: unknown): Event {
	if (!isEvent(value)) throw new AuthoringError({ message: `${context}: expected an owned Event value` })
	return value
}
function eventBytes(value: Event): Uint8Array {
	const owned = encodings.get(eventValue("Event bytes", value))
	if (owned === undefined) throw new AuthoringError({ message: "Event bytes: missing encoding" })
	return new Uint8Array(owned)
}
function eventByteLength(value: Event): number {
	const owned = encodings.get(eventValue("Event bytes", value))
	if (owned === undefined) throw new AuthoringError({ message: "Event bytes: missing encoding" })
	return owned.byteLength
}
/** Own an encoded envelope; this is not an admission certificate. */
function encodedEvent(input: unknown): Event {
	const bytes = bytesValue("Event encoding", input, MAX_EVENT_BYTES)
	if (
		bytes.length < 5 ||
		bytes[0] !== 66 ||
		bytes[1] !== 69 ||
		bytes[2] !== 86 ||
		bytes[3] !== 84 ||
		(bytes[4] !== 1 && bytes[4] !== 2 && bytes[4] !== 3)
	)
		throw new AuthoringError({ message: "Event encoding: expected a BEVT v1, v2 or v3 envelope" })
	const value: Event = Object.freeze({ [eventTag]: true as const })
	encodings.set(value, bytes)
	return value
}

export type { Event }
export { encodedEvent, eventByteLength, eventBytes, eventValue, isEvent, MAX_EVENT_BYTES }
