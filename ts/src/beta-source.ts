import { Result } from "effect"
import { type Event, encodedEvent, eventBytes } from "#event-value.ts"
import { type ExactRational, encodedRational, rationalBytes } from "#exact-value.ts"
import type { FamilyFunction } from "#family-function.ts"
import type { FiniteFunction } from "#finite-function.ts"
import type { ParameterFunction } from "#parameter-function.ts"
import { parameterData, parameterResult } from "#parameter-operation.ts"
import type { ParameterRegion } from "#parameter-region.ts"
import {
	observation,
	operands,
	type ParameterExpectationObservation,
	type ParameterProbabilityObservation
} from "#parameter-source.ts"
import { encodedParameter } from "#parameter-value.ts"
import { type ExactPolynomial, encodedPolynomial } from "#polynomial-value.ts"
import { argumentError, type DbError } from "#runtime-errors.ts"
import { encodedSource, isSource, type SourceValue, sourceBytes } from "#source-value.ts"
import { recordValue } from "#values.ts"

type BetaSource = SourceValue<"betaSource">
export interface BetaSourceDescription {
	readonly space: Event
	readonly alpha: ExactRational
	readonly beta: ExactRational
}
/** A total function agrees with this polynomial outside the certified finite
 * exception set. Exceptions remain part of the original function. */
export interface BetaIntegral {
	readonly source: BetaSource
	readonly function: ParameterFunction
	readonly polynomial: ExactPolynomial
	readonly exceptions: ParameterRegion
	readonly value: ExactRational
}
export interface BetaObservation<T> {
	readonly source: BetaSource
	readonly original: T
	readonly numerator: BetaIntegral
	readonly evidenceMass: BetaIntegral
	/** null means the integrated evidence has zero mass. */
	readonly value: ExactRational | null
}
export type BetaProbabilityObservation = BetaObservation<ParameterProbabilityObservation>
export type BetaExpectationObservation<F = FiniteFunction> = BetaObservation<ParameterExpectationObservation<F>>
const encode = (input: unknown) => encodedSource("betaSource", input)
const toBytes = (source: BetaSource) => sourceBytes("betaSource", source)
const isBetaSource = (input: unknown): input is BetaSource => isSource("betaSource", input)
function fromBytes(input: Uint8Array): Result.Result<BetaSource, DbError> {
	return Result.try({ try: () => encode(input), catch: (cause) => argumentError("BetaSource.fromBytes", cause) })
}
function inputs(...values: Uint8Array[]) {
	const out = operands()
	for (const value of values) out.add(value)
	return out.values
}
/** Add an explicit positive Beta prior over the source's full [0,1] parameter.
 * The existing fibre law and actual worlds remain unchanged. */
function create(space: Event, alpha: ExactRational, beta: ExactRational) {
	return parameterResult(
		"prior.new",
		() => inputs(eventBytes(space), rationalBytes(alpha), rationalBytes(beta)),
		encode
	)
}
function validate(source: BetaSource) {
	return parameterResult("prior.validate", () => [toBytes(source)], encode)
}
function describe(source: BetaSource) {
	return parameterData(
		"prior.describe",
		() => [toBytes(source)],
		(input): BetaSourceDescription => {
			const data = recordValue("BetaSource description", input, ["space", "alpha", "beta"])
			return Object.freeze({
				space: encodedEvent(data.space),
				alpha: encodedRational(data.alpha),
				beta: encodedRational(data.beta)
			})
		}
	)
}
function integral(input: unknown, source: BetaSource): BetaIntegral {
	const data = recordValue("Beta integral", input, ["function", "polynomial", "exceptions", "value"])
	return Object.freeze({
		source,
		function: encodedSource("parameterFunction", data.function),
		polynomial: encodedPolynomial(data.polynomial),
		exceptions: encodedParameter("region", data.exceptions),
		value: encodedRational(data.value)
	})
}
/** Exact polynomial integration with retained finite point exceptions. Undefined
 * points and nonpolynomial open sectors refuse, including removable holes. */
function integrate(source: BetaSource, fn: ParameterFunction) {
	return parameterData(
		"prior.integrate",
		() => inputs(toBytes(source), sourceBytes("parameterFunction", fn)),
		(input): BetaIntegral => {
			const data = recordValue("Beta integral result", input, ["source", "integral"])
			return integral(data.integral, encode(data.source))
		}
	)
}
function result<T>(input: unknown, original: (input: unknown) => T): BetaObservation<T> {
	const data = recordValue("Beta observation", input, ["source", "original", "numerator", "evidenceMass", "value"])
	const source = encode(data.source)
	return Object.freeze({
		source,
		original: original(data.original),
		numerator: integral(data.numerator, source),
		evidenceMass: integral(data.evidenceMass, source),
		value: data.value === null ? null : encodedRational(data.value)
	})
}
/** Integrate joint mass and evidence mass before taking their ratio. Shared draws
 * remain correlated; the original conditional function is never averaged. */
function probability(source: BetaSource, event: Event, given: Event) {
	return parameterData(
		"prior.probability",
		() => inputs(toBytes(source), eventBytes(event), eventBytes(given)),
		(input): BetaProbabilityObservation =>
			result(input, (original): ParameterProbabilityObservation => {
				const { data, ...values } = observation(original, "probability")
				return Object.freeze({ kind: "probability", event: encodedEvent(data.input), ...values })
			})
	)
}
function expectation(source: BetaSource, fn: FiniteFunction, given: Event) {
	return parameterData(
		"prior.expectation",
		() => inputs(toBytes(source), sourceBytes("function", fn), eventBytes(given)),
		(input): BetaExpectationObservation =>
			result(input, (original): ParameterExpectationObservation<FiniteFunction> => {
				const { data, ...values } = observation(original, "finiteExpectation")
				return Object.freeze({ kind: "expectation", function: encodedSource("function", data.input), ...values })
			})
	)
}
function familyExpectation(source: BetaSource, fn: FamilyFunction, given: Event) {
	return parameterData(
		"prior.familyExpectation",
		() => inputs(toBytes(source), sourceBytes("familyFunction", fn), eventBytes(given)),
		(input): BetaExpectationObservation<FamilyFunction> =>
			result(input, (original): ParameterExpectationObservation<FamilyFunction> => {
				const { data, ...values } = observation(original, "familyExpectation")
				return Object.freeze({ kind: "expectation", function: encodedSource("familyFunction", data.input), ...values })
			})
	)
}
const BetaSource = Object.freeze({
	fromBytes,
	toBytes,
	isBetaSource,
	new: create,
	validate,
	describe,
	integrate,
	probability,
	expectation,
	familyExpectation
})

export { BetaSource }
