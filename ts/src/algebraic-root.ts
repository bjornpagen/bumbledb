import { Result } from "effect"
import { type ExactRational, encodedRational, rationalBytes } from "#exact-value.ts"
import { parameterData, parameterName, parameterOrdering, parameterResult } from "#parameter-operation.ts"
import { encodedParameter, isParameter, type ParameterValue, parameterBytes } from "#parameter-value.ts"
import { type ExactPolynomial, encodedPolynomial, polynomialBytes } from "#polynomial-value.ts"
import { argumentError, type DbError } from "#runtime-errors.ts"
import { arrayValue, recordValue } from "#values.ts"

type AlgebraicRoot = ParameterValue<"root">
export interface AlgebraicRootDescription {
	/** Formal indeterminate of the canonical numerical polynomial, not a source name. */
	readonly parameter: Uint8Array
	readonly polynomial: ExactPolynomial
	readonly lower: ExactRational
	readonly upper: ExactRational
}
const encode = (input: unknown) => encodedParameter("root", input)
const toBytes = (value: AlgebraicRoot) => parameterBytes("root", value)
const isAlgebraicRoot = (input: unknown): input is AlgebraicRoot => isParameter("root", input)
function fromBytes(input: Uint8Array): Result.Result<AlgebraicRoot, DbError> {
	return Result.try({ try: () => encode(input), catch: (cause) => argumentError("AlgebraicRoot.fromBytes", cause) })
}
/** All distinct real roots, sorted numerically. The zero polynomial refuses. */
function isolate(parameter: Uint8Array, polynomial: ExactPolynomial) {
	return parameterData(
		"root.isolate",
		() => [parameterName(parameter), polynomialBytes(polynomial)],
		(input): readonly AlgebraicRoot[] => {
			const data = recordValue("Algebraic roots", input, ["roots"])
			return arrayValue("Algebraic roots", data.roots, (_, root) => encode(root))
		}
	)
}
/** Open bounds must isolate exactly one real root, with no root at an endpoint.
 * Equal bounds instead require that exact rational to be a root. */
function fromInterval(parameter: Uint8Array, polynomial: ExactPolynomial, lower: ExactRational, upper: ExactRational) {
	return parameterResult(
		"root.fromInterval",
		() => [parameterName(parameter), polynomialBytes(polynomial), rationalBytes(lower), rationalBytes(upper)],
		encode
	)
}
function validate(value: AlgebraicRoot) {
	return parameterResult("root.validate", () => [toBytes(value)], encode)
}
function describe(value: AlgebraicRoot) {
	return parameterData(
		"root.describe",
		() => [toBytes(value)],
		(input): AlgebraicRootDescription => {
			const data = recordValue("AlgebraicRoot description", input, ["parameter", "polynomial", "lower", "upper"])
			return Object.freeze({
				parameter: parameterName(data.parameter),
				polynomial: encodedPolynomial(data.polynomial),
				lower: encodedRational(data.lower),
				upper: encodedRational(data.upper)
			})
		}
	)
}
function compare(a: AlgebraicRoot, b: AlgebraicRoot) {
	return parameterOrdering("root.compare", () => [toBytes(a), toBytes(b)])
}
function compareRational(a: AlgebraicRoot, b: ExactRational) {
	return parameterOrdering("root.compareRational", () => [toBytes(a), rationalBytes(b)])
}
/** A strictly separating rational; requires a < b. */
function rationalBetween(a: AlgebraicRoot, b: AlgebraicRoot) {
	return parameterResult("root.rationalBetween", () => [toBytes(a), toBytes(b)], encodedRational)
}
/** Exact polynomial sign at this numerical root. The polynomial must use only
 * the formal parameter returned by describe(), or be constant. Explicit
 * ExactPolynomial.substitute can rename an authored variable into that name. */
function sign(value: AlgebraicRoot, polynomial: ExactPolynomial) {
	return parameterOrdering("root.sign", () => [toBytes(value), polynomialBytes(polynomial)])
}
const AlgebraicRoot = Object.freeze({
	fromBytes,
	toBytes,
	isAlgebraicRoot,
	isolate,
	fromInterval,
	validate,
	describe,
	compare,
	compareRational,
	rationalBetween,
	sign
})

export { AlgebraicRoot }
