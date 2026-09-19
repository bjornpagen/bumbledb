import { Result } from "effect"
import { type Event, encodedEvent, eventBytes, eventValue } from "#event-value.ts"
import { FamilyFunction } from "#family-function.ts"
import { decodeFamilyRevision } from "#family-revision-data.ts"
import { ParameterFunction } from "#parameter-function.ts"
import { parameterData, parameterName, parameterResult } from "#parameter-operation.ts"
import { operands, roster } from "#parameter-source.ts"
import { argumentError, type DbError } from "#runtime-errors.ts"
import { encodedSource, isSource, type SourceValue, sourceBytes } from "#source-value.ts"
import { arrayValue, recordValue } from "#values.ts"

type FamilyRevision = SourceValue<"familyRevision">
export interface FamilyJeffreyTarget {
	readonly cell: Event
	readonly target: ParameterFunction
}
const encode = (input: unknown) => encodedSource("familyRevision", input)
const toBytes = (value: FamilyRevision) => sourceBytes("familyRevision", value)
const isFamilyRevision = (input: unknown): input is FamilyRevision => isSource("familyRevision", input)
function fromBytes(input: Uint8Array): Result.Result<FamilyRevision, DbError> {
	return Result.try({ try: () => encode(input), catch: (cause) => argumentError("FamilyRevision.fromBytes", cause) })
}
/** Reconstruct the source and replay every claimed receipt/domain/posterior. */
function validate(value: FamilyRevision) {
	return parameterResult("revision.validate", () => [toBytes(value)], encode)
}
function inspect(value: FamilyRevision) {
	return parameterData("revision.inspect", () => [toBytes(value)], decodeFamilyRevision)
}
/** Condition only where evidence mass is positive. Zero-posterior outcomes
 * remain possible; an all-impossible request still owns a complete receipt. */
function condition(identity: Uint8Array, prior: Event, evidence: Event) {
	return parameterResult(
		"revision.condition",
		() => [parameterName(identity), eventBytes(prior), eventBytes(evidence)],
		encode
	)
}
/** Retain nonnegative factors and their scale. Reapplication multiplies them;
 * the caller is responsible for whether evidence should be used again. */
function likelihood(identity: Uint8Array, prior: Event, value: FamilyFunction) {
	return parameterResult(
		"revision.likelihood",
		() => [parameterName(identity), eventBytes(prior), FamilyFunction.toBytes(value)],
		encode
	)
}
/** Replace a complete ordered partition's masses. Targets must be total,
 * nonnegative and sum to one; unsupported parameter regions remain indexed. */
function jeffrey(identity: Uint8Array, prior: Event, targets: readonly FamilyJeffreyTarget[]) {
	return parameterResult(
		"revision.jeffrey",
		() => {
			roster(targets, 2047)
			const out = operands()
			out.add(parameterName(identity))
			out.add(eventBytes(prior))
			arrayValue("Family Jeffrey targets", targets, (_, input) => {
				const data = recordValue("FamilyJeffreyTarget", input, ["cell", "target"])
				out.add(eventBytes(eventValue("FamilyJeffreyTarget.cell", data.cell)))
				out.add(ParameterFunction.toBytes(data.target as ParameterFunction))
			})
			return out.values
		},
		encode
	)
}
/** Translate an original-prior Event through the retained refinement into the
 * posterior. Refuses an everywhere-impossible revision, which has no source. */
function pullback(value: FamilyRevision, event: Event) {
	return parameterResult("revision.pullback", () => [toBytes(value), eventBytes(event)], encodedEvent)
}
const FamilyRevision = Object.freeze({
	fromBytes,
	toBytes,
	isFamilyRevision,
	validate,
	inspect,
	condition,
	likelihood,
	jeffrey,
	pullback
})

export { FamilyRevision }
