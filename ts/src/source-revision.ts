import { Result } from "effect"
import { dbNative } from "#db-native.ts"
import { AuthoringError } from "#errors.ts"
import { type Event, eventBytes, eventValue } from "#event-value.ts"
import { type ExactRational, rationalBytes } from "#exact-value.ts"
import { FiniteFunction } from "#finite-function.ts"
import { argumentError, type DbError } from "#runtime-errors.ts"
import { sourceOperation, sourceResult } from "#source-operation.ts"
import { decodeRevision } from "#source-revision-data.ts"
import { encodedSource, isSource, type SourceValue, sourceBytes } from "#source-value.ts"
import { arrayValue, recordValue } from "#values.ts"

type SourceRevision = SourceValue<"revision">
export interface JeffreyTarget {
	readonly cell: Event
	readonly target: ExactRational
}
const encode = (input: unknown) => encodedSource("revision", input)
const toBytes = (input: SourceRevision) => sourceBytes("revision", input)
const isSourceRevision = (input: unknown): input is SourceRevision => isSource("revision", input)
function fromBytes(input: Uint8Array): Result.Result<SourceRevision, DbError> {
	return Result.try({ try: () => encode(input), catch: (cause) => argumentError("SourceRevision.fromBytes", cause) })
}
/** Reconstruct and replay the revision, including every receipt and outcome. */
function validate(value: SourceRevision) {
	return sourceResult("revision.validate", () => [toBytes(value)], encode)
}
function inspect(value: SourceRevision) {
	return sourceOperation(
		"revision.inspect",
		() => [toBytes(value)],
		0n,
		(operation) => dbNative.runtimeEventSourceTake(operation),
		decodeRevision
	)
}
function condition(prior: Event, evidence: Event) {
	return sourceResult("revision.condition", () => [eventBytes(prior), eventBytes(evidence)], encode)
}
/** Nonnegative likelihood factors, with their scale retained in the receipt.
 * These are not posterior target probabilities. */
function likelihood(prior: Event, value: FiniteFunction) {
	return sourceResult("revision.likelihood", () => [eventBytes(prior), FiniteFunction.toBytes(value)], encode)
}
/** Complete disjoint partition and nonnegative target masses summing exactly
 * to one. Empty cells retain their positions. No missing cells/mass are inferred. */
function jeffrey(prior: Event, targets: readonly JeffreyTarget[]) {
	return sourceResult(
		"revision.jeffrey",
		() => {
			if (!Array.isArray(targets) || targets.length > 2047)
				throw new AuthoringError({ message: "SourceRevision.jeffrey: exceeds operand roster" })
			let remaining = 16 * 1024 * 1024
			const own = (bytes: Uint8Array) => {
				if (bytes.length > remaining)
					throw new AuthoringError({ message: "SourceRevision.jeffrey: exceeds 16 MiB input" })
				remaining -= bytes.length
				return bytes
			}
			const values: Uint8Array[] = [own(eventBytes(prior))]
			arrayValue("Jeffrey targets", targets, (_, input) => {
				const data = recordValue("JeffreyTarget", input, ["cell", "target"])
				values.push(
					own(eventBytes(eventValue("JeffreyTarget.cell", data.cell))),
					own(rationalBytes(data.target as ExactRational))
				)
				return undefined
			})
			return values
		},
		encode
	)
}
const SourceRevision = Object.freeze({
	fromBytes,
	toBytes,
	isSourceRevision,
	validate,
	inspect,
	condition,
	likelihood,
	jeffrey
})

export { SourceRevision }
