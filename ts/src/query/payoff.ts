/** Owned numerical query data; semantic admission occurs on the native worker. */
import { AuthoringError } from "#errors.ts"
import { type ExactRational, encodedRational, isExactRational, rationalBytes } from "#exact-value.ts"
import { encodedSource, isSource, type SourceValue, sourceBytes } from "#source-value.ts"

export type ImportedPayoff = ExactRational | SourceValue<"function"> | SourceValue<"familyFunction">
export function isPayoff(input: unknown): input is ImportedPayoff {
	return isExactRational(input) || isSource("function", input) || isSource("familyFunction", input)
}
export function payoffBytes(input: ImportedPayoff): Uint8Array {
	if (isExactRational(input)) return rationalBytes(input)
	if (isSource("function", input)) return sourceBytes("function", input)
	return sourceBytes("familyFunction", input)
}
export function payoffFromBytes(bytes: Uint8Array): ImportedPayoff {
	if (bytes[2] === 82) return encodedRational(bytes)
	if (bytes[4] === 1 && bytes[5] === 0) return encodedSource("function", bytes)
	if (bytes[4] === 2 && bytes[5] === 1) return encodedSource("familyFunction", bytes)
	throw new AuthoringError({ message: "Payoff import requires an exact rational or total finite/family function" })
}
