/** Owned BEPL transport. The header alone does not certify a polynomial. */
import { AuthoringError } from "#errors.ts"
import { bytesValue } from "#values.ts"

const polynomialTag: unique symbol = Symbol("bumbledb.ExactPolynomial")
export interface ExactPolynomial {
	readonly [polynomialTag]: true
}
const encodings = new WeakMap<ExactPolynomial, Uint8Array>()
export function isExactPolynomial(input: unknown): input is ExactPolynomial {
	return typeof input === "object" && input !== null && encodings.has(input as ExactPolynomial)
}
export function polynomialBytes(input: ExactPolynomial): Uint8Array {
	const bytes = encodings.get(input)
	if (bytes === undefined) throw new AuthoringError({ message: "ExactPolynomial: expected an owned BEPL value" })
	return new Uint8Array(bytes)
}
export function encodedPolynomial(input: unknown): ExactPolynomial {
	const bytes = bytesValue("ExactPolynomial", input, 16 * 1024 * 1024)
	if (bytes.length < 5 || bytes[0] !== 66 || bytes[1] !== 69 || bytes[2] !== 80 || bytes[3] !== 76 || bytes[4] !== 1)
		throw new AuthoringError({ message: "ExactPolynomial: expected a BEPL v1 envelope" })
	const value: ExactPolynomial = Object.freeze({ [polynomialTag]: true as const })
	encodings.set(value, bytes)
	return value
}
