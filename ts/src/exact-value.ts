/** Owned BERA transport; only native admission certifies exact arithmetic. */
import { AuthoringError } from "#errors.ts"
import { bytesValue } from "#values.ts"

const rationalTag: unique symbol = Symbol("bumbledb.ExactRational")
export interface ExactRational {
	readonly [rationalTag]: true
}
const encodings = new WeakMap<ExactRational, Uint8Array>()
export function isExactRational(value: unknown): value is ExactRational {
	return typeof value === "object" && value !== null && encodings.has(value as ExactRational)
}
export function rationalBytes(value: ExactRational): Uint8Array {
	const bytes = encodings.get(value)
	if (bytes === undefined) throw new AuthoringError({ message: "ExactRational: expected an owned BERA value" })
	return new Uint8Array(bytes)
}
export function encodedRational(input: unknown): ExactRational {
	const bytes = bytesValue("ExactRational", input, 16 * 1024 * 1024)
	if (bytes.length < 5 || bytes[0] !== 66 || bytes[1] !== 69 || bytes[2] !== 82 || bytes[3] !== 65 || bytes[4] !== 1)
		throw new AuthoringError({ message: "ExactRational: expected a BERA v1 envelope" })
	const value: ExactRational = Object.freeze({ [rationalTag]: true as const })
	encodings.set(value, bytes)
	return value
}
