import { Result } from "effect"
import { type EventDescriptor, encodedDescriptor } from "#event-descriptor.ts"
import { type Event, encodedEvent, eventBytes } from "#event-value.ts"
import { parameterData, parameterName, parameterResult } from "#parameter-operation.ts"
import { ParameterRefinement } from "#parameter-refinement.ts"
import { ParameterRegion } from "#parameter-region.ts"
import { operands, roster } from "#parameter-source.ts"
import { argumentError, type DbError } from "#runtime-errors.ts"
import { encodedSource, isSource, type SourceValue, sourceBytes } from "#source-value.ts"
import { arrayValue, recordValue } from "#values.ts"

type ParameterRestriction = SourceValue<"parameterRestriction">
export interface ParameterRestrictionDescription {
	readonly prior: Event
	readonly refinedPrior: Event
	readonly space: Event
	readonly refinement: ParameterRefinement
	/** Restricted space → refinedPrior. This is generally not onto. */
	readonly inclusion: EventDescriptor
}
const encode = (input: unknown) => encodedSource("parameterRestriction", input)
const toBytes = (value: ParameterRestriction) => sourceBytes("parameterRestriction", value)
const isParameterRestriction = (input: unknown): input is ParameterRestriction =>
	isSource("parameterRestriction", input)
function fromBytes(input: Uint8Array): Result.Result<ParameterRestriction, DbError> {
	return Result.try({
		try: () => encode(input),
		catch: (cause) => argumentError("ParameterRestriction.fromBytes", cause)
	})
}
/** Intersect the ambient with a predicate, retaining its per-parameter law and
 * outcome fibres. Empty intersections refuse. Extra guards are explicit. */
function create(
	identity: Uint8Array,
	source: Event,
	predicate: ParameterRegion,
	additional: readonly ParameterRegion[] = []
) {
	return parameterResult(
		"restriction.new",
		() => {
			roster(additional, 62)
			const out = operands()
			out.add(parameterName(identity))
			out.add(eventBytes(source))
			out.add(ParameterRegion.toBytes(predicate))
			arrayValue("Restriction predicates", additional, (_, value) =>
				out.add(ParameterRegion.toBytes(value as ParameterRegion))
			)
			return out.values
		},
		encode
	)
}
/** The predicate must already be representable in this retained refinement. */
function fromRefinement(refinement: ParameterRefinement, predicate: ParameterRegion) {
	return parameterResult(
		"restriction.fromRefinement",
		() => [ParameterRefinement.toBytes(refinement), ParameterRegion.toBytes(predicate)],
		encode
	)
}
function validate(value: ParameterRestriction) {
	return parameterResult("restriction.validate", () => [toBytes(value)], encode)
}
function describe(value: ParameterRestriction) {
	return parameterData(
		"restriction.describe",
		() => [toBytes(value)],
		(input): ParameterRestrictionDescription => {
			const data = recordValue("Parameter restriction", input, [
				"prior",
				"refinedPrior",
				"space",
				"refinement",
				"inclusion"
			])
			return Object.freeze({
				prior: encodedEvent(data.prior),
				refinedPrior: encodedEvent(data.refinedPrior),
				space: encodedEvent(data.space),
				refinement: encodedSource("parameterRefinement", data.refinement),
				inclusion: encodedDescriptor(data.inclusion)
			})
		}
	)
}
/** Translate an Event from the original prior, composing refinement and inclusion. */
function pullback(value: ParameterRestriction, event: Event) {
	return parameterResult("restriction.pullback", () => [toBytes(value), eventBytes(event)], encodedEvent)
}
const ParameterRestriction = Object.freeze({
	fromBytes,
	toBytes,
	isParameterRestriction,
	new: create,
	fromRefinement,
	validate,
	describe,
	pullback
})

export { ParameterRestriction }
