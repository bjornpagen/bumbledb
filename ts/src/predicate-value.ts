/** Owned BENP derivations. A pure envelope import grants no mathematical
 * certificate; query admission replays every source and operation on a worker. */
import { AuthoringError } from "#errors.ts"
import { bytesValue } from "#values.ts"

const predicateTag: unique symbol = Symbol("bumbledb.ObservationPredicate")
export interface ObservationPredicate {
	readonly [predicateTag]: true
}
const encodings = new WeakMap<ObservationPredicate, Uint8Array>()
export function isObservationPredicate(value: unknown): value is ObservationPredicate {
	return typeof value === "object" && value !== null && encodings.has(value as ObservationPredicate)
}
export function predicateBytes(value: ObservationPredicate): Uint8Array {
	const bytes = encodings.get(value)
	if (bytes === undefined) throw new AuthoringError({ message: "ObservationPredicate: expected an owned BENP value" })
	return new Uint8Array(bytes)
}
export function encodedPredicate(input: unknown): ObservationPredicate {
	const bytes = bytesValue("ObservationPredicate", input, 16 * 1024 * 1024)
	if (
		bytes.length < 5 ||
		bytes[0] !== 66 ||
		bytes[1] !== 69 ||
		bytes[2] !== 78 ||
		bytes[3] !== 80 ||
		(bytes[4] !== 1 && bytes[4] !== 2)
	)
		throw new AuthoringError({ message: "ObservationPredicate: expected a BENP v1/v2 envelope" })
	const value: ObservationPredicate = Object.freeze({ [predicateTag]: true as const })
	encodings.set(value, bytes)
	return value
}
