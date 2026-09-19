import { Result } from "effect"
import { EventDescriptor } from "#event-descriptor.ts"
import { type Event, encodedEvent, eventBytes } from "#event-value.ts"
import { type ExactRational, encodedRational, rationalBytes } from "#exact-value.ts"
import type { ParameterFunction, ParameterFunctionPiece } from "#parameter-function.ts"
import { parameterBoolean, parameterData, parameterResult } from "#parameter-operation.ts"
import type { ParameterRefinement } from "#parameter-refinement.ts"
import { ParameterRegion } from "#parameter-region.ts"
import { familyExpectation, operands, outcomeBytes, roster } from "#parameter-source.ts"
import { encodedParameter } from "#parameter-value.ts"
import { type ExactPolynomial, encodedPolynomial, polynomialBytes } from "#polynomial-value.ts"
import { argumentError, type DbError } from "#runtime-errors.ts"
import { encodedSource, isSource, type SourceValue, sourceBytes } from "#source-value.ts"
import { arrayValue, recordValue } from "#values.ts"

type FamilyFunction = SourceValue<"familyFunction">
export interface FamilyFunctionPiece {
	readonly region: Event
	readonly value: ParameterFunctionPiece
}
export interface FamilyFunctionDescription {
	readonly space: Event
	readonly pieces: readonly FamilyFunctionPiece[]
}
const encode = (input: unknown) => encodedSource("familyFunction", input)
const toBytes = (value: FamilyFunction) => sourceBytes("familyFunction", value)
const isFamilyFunction = (input: unknown): input is FamilyFunction => isSource("familyFunction", input)
function fromBytes(input: Uint8Array): Result.Result<FamilyFunction, DbError> {
	return Result.try({ try: () => encode(input), catch: (cause) => argumentError("FamilyFunction.fromBytes", cause) })
}
/** Total signed value on every legal world, with zero on omitted worlds. Each
 * supplied value must be defined wherever its Event cell is active. */
function create(space: Event, pieces: readonly FamilyFunctionPiece[]) {
	return parameterResult(
		"family.new",
		() => {
			roster(pieces, 2046)
			const out = operands()
			out.add(eventBytes(space))
			arrayValue("FamilyFunction pieces", pieces, (_, piece) => {
				const data = recordValue("FamilyFunctionPiece", piece, ["region", "value"])
				const value = recordValue("Guarded value", data.value, ["numerator", "denominator", "defined"])
				out.add(eventBytes(data.region as Event))
				out.add(polynomialBytes(value.numerator as ExactPolynomial))
				out.add(polynomialBytes(value.denominator as ExactPolynomial))
				out.add(ParameterRegion.toBytes(value.defined as ParameterRegion))
			})
			return out.values
		},
		encode
	)
}
/** The parameter function must be total, with each piece representable by the
 * existing guards; refine explicitly when a piece introduces a new cut. */
function fromParameter(space: Event, value: ParameterFunction) {
	return parameterResult(
		"family.fromParameter",
		() => [eventBytes(space), sourceBytes("parameterFunction", value)],
		encode
	)
}
function fromFinite(value: SourceValue<"function">) {
	return parameterResult("family.fromFinite", () => [sourceBytes("function", value)], encode)
}
function density(space: Event) {
	return parameterResult("family.density", () => [eventBytes(space)], encode)
}
/** Requires a nonnegative function with total exactly one on every parameter
 * fibre. Never rescales or invents a prior over the parameter. */
function designate(value: FamilyFunction) {
	return parameterResult("family.designate", () => [toBytes(value)], encodedEvent)
}
function validate(value: FamilyFunction) {
	return parameterResult("family.validate", () => [toBytes(value)], encode)
}
function describe(value: FamilyFunction) {
	return parameterData(
		"family.describe",
		() => [toBytes(value)],
		(input): FamilyFunctionDescription => {
			const data = recordValue("FamilyFunction description", input, ["space", "pieces"])
			return Object.freeze({
				space: encodedEvent(data.space),
				pieces: arrayValue("FamilyFunction pieces", data.pieces, (_, piece) => {
					const data = recordValue("FamilyFunctionPiece", piece, ["region", "value"])
					const value = recordValue("Guarded value", data.value, ["numerator", "denominator", "defined"])
					return Object.freeze({
						region: encodedEvent(data.region),
						value: Object.freeze({
							numerator: encodedPolynomial(value.numerator),
							denominator: encodedPolynomial(value.denominator),
							defined: encodedParameter("region", value.defined)
						})
					})
				})
			})
		}
	)
}
/** null means the parameter is outside the ambient. Illegal outcome encodings
 * refuse; a legal world always has a value, including the zero default. */
function at(value: FamilyFunction, parameter: ExactRational, outcomes: bigint) {
	return parameterData(
		"family.at",
		() => [toBytes(value), rationalBytes(parameter), outcomeBytes(outcomes)],
		(input): ExactRational | null => {
			const data = recordValue("FamilyFunction value", input, ["value"])
			return data.value === null ? null : encodedRational(data.value)
		}
	)
}
function add(a: FamilyFunction, b: FamilyFunction) {
	return parameterResult("family.add", () => [toBytes(a), toBytes(b)], encode)
}
function subtract(a: FamilyFunction, b: FamilyFunction) {
	return parameterResult("family.subtract", () => [toBytes(a), toBytes(b)], encode)
}
function multiply(a: FamilyFunction, b: FamilyFunction) {
	return parameterResult("family.multiply", () => [toBytes(a), toBytes(b)], encode)
}
/** Total division: any legal zero divisor refuses, even at zero-probability worlds. */
function divide(a: FamilyFunction, b: FamilyFunction) {
	return parameterResult("family.divide", () => [toBytes(a), toBytes(b)], encode)
}
function equivalent(a: FamilyFunction, b: FamilyFunction) {
	return parameterBoolean("family.equivalent", () => [toBytes(a), toBytes(b)])
}
function isNonnegative(value: FamilyFunction) {
	return parameterBoolean("family.isNonnegative", () => [toBytes(value)])
}
/** Requires every sign boundary to be representable by the existing guards. */
function whereSign(value: FamilyFunction, signs: bigint) {
	return parameterResult("family.sign", () => [toBytes(value)], encodedEvent, signs)
}
function alignTo(value: FamilyFunction, space: Event) {
	return parameterResult("family.align", () => [toBytes(value), eventBytes(space)], encode)
}
function pullback(value: FamilyFunction, map: EventDescriptor) {
	return parameterResult("family.pullback", () => [toBytes(value), EventDescriptor.toBytes(map)], encode)
}
function pushforward(value: FamilyFunction, map: EventDescriptor) {
	return parameterResult("family.pushforward", () => [toBytes(value), EventDescriptor.toBytes(map)], encode)
}
/** A fibre mean is admitted only if its pullback equals the original everywhere. */
function descend(value: FamilyFunction, map: EventDescriptor) {
	return parameterResult("family.descend", () => [toBytes(value), EventDescriptor.toBytes(map)], encode)
}
function refine(value: FamilyFunction, refinement: ParameterRefinement) {
	return parameterResult(
		"family.refine",
		() => [toBytes(value), sourceBytes("parameterRefinement", refinement)],
		encode
	)
}
/** Sum finite outcome witnesses at the same parameter; no law required. */
function outcomeSum(value: FamilyFunction, given: Event) {
	return parameterResult(
		"family.sum",
		() => [toBytes(value), eventBytes(given)],
		(bytes) => encodedSource("parameterFunction", bytes)
	)
}
const FamilyFunction = Object.freeze({
	fromBytes,
	toBytes,
	isFamilyFunction,
	new: create,
	fromParameter,
	fromFinite,
	density,
	designate,
	validate,
	describe,
	at,
	add,
	subtract,
	multiply,
	divide,
	equivalent,
	isNonnegative,
	whereSign,
	alignTo,
	pullback,
	pushforward,
	descend,
	refine,
	outcomeSum,
	expectation: familyExpectation
})

export { FamilyFunction }
