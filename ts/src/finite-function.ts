import { Result } from "effect"
import { dbNative } from "#db-native.ts"
import { AuthoringError } from "#errors.ts"
import { EventDescriptor } from "#event-descriptor.ts"
import { type Event, encodedEvent, eventBytes, eventValue } from "#event-value.ts"
import { type ExactRational, encodedRational, rationalBytes } from "#exact-value.ts"
import { type FunctionPatch, glueFunctions, maskFunction } from "#function-cover.ts"
import { parameterExpectation } from "#parameter-source.ts"
import { argumentError, type DbError } from "#runtime-errors.ts"
import { expectation, sourceBoolean, sourceOperation, sourceResult } from "#source-operation.ts"
import { encodedSource, isSource, type SourceValue, sourceBytes } from "#source-value.ts"
import { arrayValue, recordValue } from "#values.ts"

type FiniteFunction = SourceValue<"function">
export interface FunctionPiece {
	readonly region: Event
	readonly value: ExactRational
}
export interface FiniteFunctionDescription {
	readonly space: Event
	readonly pieces: readonly FunctionPiece[]
}
const encode = (input: unknown) => encodedSource("function", input)
const toBytes = (input: FiniteFunction) => sourceBytes("function", input)
const isFiniteFunction = (input: unknown): input is FiniteFunction => isSource("function", input)
function fromBytes(input: Uint8Array): Result.Result<FiniteFunction, DbError> {
	return Result.try({ try: () => encode(input), catch: (cause) => argumentError("FiniteFunction.fromBytes", cause) })
}
/** Unspecified worlds have value zero. Every supplied piece, including a zero
 * piece, must be disjoint and belong to the full named context. */
function create(space: Event, pieces: readonly FunctionPiece[]) {
	return sourceResult(
		"new",
		() => {
			if (!Array.isArray(pieces) || pieces.length > 2047)
				throw new AuthoringError({ message: "FiniteFunction: exceeds operand roster" })
			let remaining = 16 * 1024 * 1024
			const own = (bytes: Uint8Array) => {
				if (bytes.length > remaining) throw new AuthoringError({ message: "FiniteFunction: exceeds 16 MiB input" })
				remaining -= bytes.length
				return bytes
			}
			const values: Uint8Array[] = [own(eventBytes(space))]
			arrayValue("FiniteFunction pieces", pieces, (_, input) => {
				const data = recordValue("FunctionPiece", input, ["region", "value"])
				values.push(
					own(eventBytes(eventValue("FunctionPiece.region", data.region))),
					own(rationalBytes(data.value as ExactRational))
				)
				return undefined
			})
			return values
		},
		encode
	)
}
function validate(value: FiniteFunction) {
	return sourceResult("validate", () => [toBytes(value)], encode)
}
function describe(value: FiniteFunction) {
	return sourceOperation(
		"describe",
		() => [toBytes(value)],
		0n,
		(operation) => dbNative.runtimeEventSourceTake(operation),
		(input): FiniteFunctionDescription => {
			const data = recordValue("FiniteFunction description", input, ["space", "pieces"])
			return Object.freeze({
				space: encodedEvent(data.space),
				pieces: arrayValue("Function pieces", data.pieces, (_, input) => {
					const piece = recordValue("FunctionPiece", input, ["region", "value"])
					return Object.freeze({ region: encodedEvent(piece.region), value: encodedRational(piece.value) })
				})
			})
		}
	)
}
function constant(space: Event, value: ExactRational) {
	return sourceResult("constant", () => [eventBytes(space), rationalBytes(value)], encode)
}
function density(space: Event) {
	return sourceResult("density", () => [eventBytes(space)], encode)
}
function add(a: FiniteFunction, b: FiniteFunction) {
	return sourceResult("add", () => [toBytes(a), toBytes(b)], encode)
}
function multiply(a: FiniteFunction, b: FiniteFunction) {
	return sourceResult("multiply", () => [toBytes(a), toBytes(b)], encode)
}
function alignTo(value: FiniteFunction, space: Event) {
	return sourceResult("align", () => [toBytes(value), eventBytes(space)], encode)
}
function pullback(value: FiniteFunction, map: EventDescriptor) {
	return sourceResult("pullback", () => [toBytes(value), EventDescriptor.toBytes(map)], encode)
}
function pushforward(value: FiniteFunction, map: EventDescriptor) {
	return sourceResult("pushforward", () => [toBytes(value), EventDescriptor.toBytes(map)], encode)
}
function isZero(value: FiniteFunction) {
	return sourceBoolean("isZero", () => [toBytes(value)])
}
function isNonnegative(value: FiniteFunction) {
	return sourceBoolean("isNonnegative", () => [toBytes(value)])
}
function equivalent(a: FiniteFunction, b: FiniteFunction) {
	return sourceBoolean("equivalent", () => [toBytes(a), toBytes(b)])
}
function at(value: FiniteFunction, world: bigint) {
	return sourceResult("at", () => [toBytes(value)], encodedRational, world)
}
/** Requires a nonnegative density with total mass exactly one; never rescales. */
function designate(value: FiniteFunction) {
	return sourceResult("designate", () => [toBytes(value)], encodedEvent)
}
/** Zero outside the supplied region without changing the source context. */
function mask(value: FiniteFunction, region: Event) {
	return maskFunction("function", value, region)
}
/** Glue local values that agree on overlaps and completely cover parent. */
function glue(parent: Event, patches: readonly FunctionPatch<FiniteFunction>[]) {
	return glueFunctions("function", parent, patches)
}
const FiniteFunction = Object.freeze({
	mask,
	glue,
	fromBytes,
	toBytes,
	isFiniteFunction,
	new: create,
	validate,
	describe,
	constant,
	density,
	add,
	multiply,
	alignTo,
	pullback,
	pushforward,
	isZero,
	isNonnegative,
	equivalent,
	at,
	designate,
	expectation,
	parameterExpectation
})

export { FiniteFunction }
