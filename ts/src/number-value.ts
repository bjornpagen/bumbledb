/** Owned BENO derivations. A pure envelope import grants no mathematical
 * certificate; query admission replays every source and operation on a worker. */
import { AuthoringError } from "#errors.ts"
import { bytesValue } from "#values.ts"

const numberTag: unique symbol = Symbol("bumbledb.ObservationNumber")
export interface ObservationNumber {
	readonly [numberTag]: true
}
const encodings = new WeakMap<ObservationNumber, Uint8Array>()
export function isObservationNumber(value: unknown): value is ObservationNumber {
	return typeof value === "object" && value !== null && encodings.has(value as ObservationNumber)
}
export function numberBytes(value: ObservationNumber): Uint8Array {
	const bytes = encodings.get(value)
	if (bytes === undefined) throw new AuthoringError({ message: "ObservationNumber: expected an owned BENO value" })
	return new Uint8Array(bytes)
}
export function encodedNumber(input: unknown): ObservationNumber {
	const bytes = bytesValue("ObservationNumber", input, 16 * 1024 * 1024)
	if (bytes.length < 5 || bytes[0] !== 66 || bytes[1] !== 69 || bytes[2] !== 78 || bytes[3] !== 79 || bytes[4] !== 1)
		throw new AuthoringError({ message: "ObservationNumber: expected a BENO v1 envelope" })
	const value: ObservationNumber = Object.freeze({ [numberTag]: true as const })
	encodings.set(value, bytes)
	return value
}
