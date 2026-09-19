import { Result } from "effect"
import { type Event, encodedEvent, eventBytes } from "#event-value.ts"
import { parameterData, parameterName, parameterResult } from "#parameter-operation.ts"
import { ParameterRegion } from "#parameter-region.ts"
import { operands, roster } from "#parameter-source.ts"
import { argumentError, type DbError } from "#runtime-errors.ts"
import { encodedSource, isSource, type SourceValue, sourceBytes } from "#source-value.ts"
import { arrayValue, recordValue } from "#values.ts"

type ParameterRefinement = SourceValue<"parameterRefinement">
export interface ParameterRefinementDescription {
	readonly source: Event
	readonly refined: Event
}
const encode = (input: unknown) => encodedSource("parameterRefinement", input)
const toBytes = (value: ParameterRefinement) => sourceBytes("parameterRefinement", value)
const isParameterRefinement = (input: unknown): input is ParameterRefinement => isSource("parameterRefinement", input)
function fromBytes(input: Uint8Array): Result.Result<ParameterRefinement, DbError> {
	return Result.try({
		try: () => encode(input),
		catch: (cause) => argumentError("ParameterRefinement.fromBytes", cause)
	})
}
/** Append predicates to the sealed presentation, preserving actual worlds and
 * the designated law. Identity names the resulting presentation explicitly. */
function create(identity: Uint8Array, source: Event, predicates: readonly ParameterRegion[]) {
	return parameterResult(
		"refinement.new",
		() => {
			roster(predicates, 62)
			const out = operands()
			out.add(parameterName(identity))
			out.add(eventBytes(source))
			arrayValue("Refinement predicates", predicates, (_, region) =>
				out.add(ParameterRegion.toBytes(region as ParameterRegion))
			)
			return out.values
		},
		encode
	)
}
function validate(value: ParameterRefinement) {
	return parameterResult("refinement.validate", () => [toBytes(value)], encode)
}
function describe(value: ParameterRefinement) {
	return parameterData(
		"refinement.describe",
		() => [toBytes(value)],
		(input): ParameterRefinementDescription => {
			const data = recordValue("Parameter refinement", input, ["source", "refined"])
			return Object.freeze({ source: encodedEvent(data.source), refined: encodedEvent(data.refined) })
		}
	)
}
function lift(value: ParameterRefinement, event: Event) {
	return parameterResult("refinement.lift", () => [toBytes(value), eventBytes(event)], encodedEvent)
}
/** Refuses when an added predicate is essential to the Event. */
function descend(value: ParameterRefinement, event: Event) {
	return parameterResult("refinement.descend", () => [toBytes(value), eventBytes(event)], encodedEvent)
}
const ParameterRefinement = Object.freeze({
	fromBytes,
	toBytes,
	isParameterRefinement,
	new: create,
	validate,
	describe,
	lift,
	descend
})

export { ParameterRefinement }
