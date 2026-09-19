import { Result } from "effect"
import { AuthoringError } from "#errors.ts"
import { type ExactRational, encodedRational, rationalBytes } from "#exact-value.ts"
import { parameterBoolean, parameterData, parameterResult } from "#parameter-operation.ts"
import { ParameterDomain, ParameterRegion } from "#parameter-region.ts"
import { encodedParameter } from "#parameter-value.ts"
import { type ExactPolynomial, encodedPolynomial, polynomialBytes } from "#polynomial-value.ts"
import { argumentError, type DbError } from "#runtime-errors.ts"
import { encodedSource, isSource, type SourceValue, sourceBytes } from "#source-value.ts"
import { arrayValue, recordValue } from "#values.ts"

type ParameterFunction = SourceValue<"parameterFunction">
export interface ParameterFunctionPiece {
	readonly numerator: ExactPolynomial
	readonly denominator: ExactPolynomial
	readonly defined: ParameterRegion
}
export interface ParameterFunctionDescription {
	readonly ambient: ParameterDomain
	readonly defined: ParameterRegion
	readonly pieces: readonly ParameterFunctionPiece[]
}
const encode = (input: unknown) => encodedSource("parameterFunction", input)
const toBytes = (value: ParameterFunction) => sourceBytes("parameterFunction", value)
const isParameterFunction = (input: unknown): input is ParameterFunction => isSource("parameterFunction", input)
function fromBytes(input: Uint8Array): Result.Result<ParameterFunction, DbError> {
	return Result.try({ try: () => encode(input), catch: (cause) => argumentError("ParameterFunction.fromBytes", cause) })
}
/** Defined on ambient where denominator != 0. No cancellation fills holes. */
function ratio(ambient: ParameterDomain, numerator: ExactPolynomial, denominator: ExactPolynomial) {
	return parameterResult(
		"function.ratio",
		() => [ParameterDomain.toBytes(ambient), polynomialBytes(numerator), polynomialBytes(denominator)],
		encode
	)
}
/** Disjoint pieces with an explicit inhabited ambient. Uncovered points are
 * undefined. Each piece is additionally restricted to denominator != 0. */
function pieces(ambient: ParameterDomain, pieces: readonly ParameterFunctionPiece[]) {
	return parameterResult(
		"function.pieces",
		() => {
			if (!Array.isArray(pieces) || pieces.length > 4093)
				throw new AuthoringError({ message: "ParameterFunction: exceeds piece roster" })
			let remaining = 16 * 1024 * 1024
			const own = (bytes: Uint8Array) => {
				if (bytes.length > remaining) throw new AuthoringError({ message: "ParameterFunction: exceeds 16 MiB input" })
				remaining -= bytes.length
				return bytes
			}
			const inputs = [own(ParameterDomain.toBytes(ambient))]
			arrayValue("ParameterFunction pieces", pieces, (_, piece) => {
				const data = recordValue("ParameterFunctionPiece", piece, ["numerator", "denominator", "defined"])
				inputs.push(
					own(polynomialBytes(data.numerator as ExactPolynomial)),
					own(polynomialBytes(data.denominator as ExactPolynomial)),
					own(ParameterRegion.toBytes(data.defined as ParameterRegion))
				)
			})
			return inputs
		},
		encode
	)
}
function validate(value: ParameterFunction) {
	return parameterResult("function.validate", () => [toBytes(value)], encode)
}
function describe(value: ParameterFunction) {
	return parameterData(
		"function.describe",
		() => [toBytes(value)],
		(input): ParameterFunctionDescription => {
			const data = recordValue("ParameterFunction description", input, ["ambient", "defined", "pieces"])
			return Object.freeze({
				ambient: encodedParameter("domain", data.ambient),
				defined: encodedParameter("region", data.defined),
				pieces: arrayValue("ParameterFunction pieces", data.pieces, (_, piece) => {
					const data = recordValue("ParameterFunctionPiece", piece, ["numerator", "denominator", "defined"])
					return Object.freeze({
						numerator: encodedPolynomial(data.numerator),
						denominator: encodedPolynomial(data.denominator),
						defined: encodedParameter("region", data.defined)
					})
				})
			})
		}
	)
}
function domain(value: ParameterFunction) {
	return parameterResult(
		"function.domain",
		() => [toBytes(value)],
		(bytes) => encodedParameter("domain", bytes)
	)
}
function definedOn(value: ParameterFunction) {
	return parameterResult(
		"function.defined",
		() => [toBytes(value)],
		(bytes) => encodedParameter("region", bytes)
	)
}
/** null is mathematically undefined (including outside ambient); failures refuse. */
function at(value: ParameterFunction, point: ExactRational) {
	return parameterData(
		"function.at",
		() => [toBytes(value), rationalBytes(point)],
		(input): ExactRational | null => {
			const data = recordValue("ParameterFunction value", input, ["value"])
			return data.value === null ? null : encodedRational(data.value)
		}
	)
}
/** All sign masks exclude undefined points, including PolynomialSigns.any. */
function whereSign(value: ParameterFunction, signs: bigint) {
	return parameterResult(
		"function.sign",
		() => [toBytes(value)],
		(bytes) => encodedParameter("region", bytes),
		signs
	)
}
function isNowhereDefined(value: ParameterFunction) {
	return parameterBoolean("function.isNowhere", () => [toBytes(value)])
}
/** Preserve the ambient and restrict defined points. */
function restrict(value: ParameterFunction, region: ParameterRegion) {
	return parameterResult("function.restrict", () => [toBytes(value), ParameterRegion.toBytes(region)], encode)
}
/** Narrow to an inhabited subdomain; cannot extend the ambient. */
function onDomain(value: ParameterFunction, domain: ParameterDomain) {
	return parameterResult("function.onDomain", () => [toBytes(value), ParameterDomain.toBytes(domain)], encode)
}
function add(a: ParameterFunction, b: ParameterFunction) {
	return parameterResult("function.add", () => [toBytes(a), toBytes(b)], encode)
}
function subtract(a: ParameterFunction, b: ParameterFunction) {
	return parameterResult("function.subtract", () => [toBytes(a), toBytes(b)], encode)
}
function multiply(a: ParameterFunction, b: ParameterFunction) {
	return parameterResult("function.multiply", () => [toBytes(a), toBytes(b)], encode)
}
function divide(a: ParameterFunction, b: ParameterFunction) {
	return parameterResult("function.divide", () => [toBytes(a), toBytes(b)], encode)
}
/** Equality includes defined regions. Incompatible ambient domains refuse. */
function equivalent(a: ParameterFunction, b: ParameterFunction) {
	return parameterBoolean("function.equivalent", () => [toBytes(a), toBytes(b)])
}
const ParameterFunction = Object.freeze({
	fromBytes,
	toBytes,
	isParameterFunction,
	ratio,
	pieces,
	validate,
	describe,
	domain,
	definedOn,
	at,
	whereSign,
	isNowhereDefined,
	restrict,
	onDomain,
	add,
	subtract,
	multiply,
	divide,
	equivalent
})

export { ParameterFunction }
