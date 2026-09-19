/** Query observations are result values, never stored schema fields. */
import { SdkInvariantError } from "#errors.ts"
import { encodedEvent } from "#event-value.ts"
import { encodedRational } from "#exact-value.ts"
import type { ParameterProbabilityObservation } from "#parameter-source.ts"
import { encodedParameter } from "#parameter-value.ts"
import type { CellValue } from "#rows.ts"
import type { ProbabilityObservation } from "#source-operation.ts"
import { encodedSource } from "#source-value.ts"
import { recordValue } from "#values.ts"

export type ProbabilityAnswer =
	| (ProbabilityObservation & { readonly law: "fixed" })
	| (ParameterProbabilityObservation & { readonly law: "parameter" })

const resultTag: unique symbol = Symbol("bumbledb.ProbabilityResult")
export interface ProbabilityResult {
	readonly kind: "probability"
	readonly [resultTag]: true
}
/** Expected output descriptor for queryFromDescription; not a schema field. */
export const probabilityResult: ProbabilityResult = Object.freeze({ kind: "probability", [resultTag]: true as const })

export interface ProbabilityWire {
	readonly kind: "probability"
	readonly law: "fixed" | "parameter"
	readonly event: Uint8Array
	readonly given: Uint8Array
	readonly numerator: Uint8Array
	readonly evidenceMass: Uint8Array
	readonly value: Uint8Array | null
	readonly defined: Uint8Array | null
}
export type AnswerCell = CellValue | ProbabilityWire

export function decodeProbability(input: unknown): ProbabilityAnswer {
	const data = recordValue("Query probability", input, [
		"kind",
		"law",
		"event",
		"given",
		"numerator",
		"evidenceMass",
		"value",
		"defined"
	])
	if (data.kind !== "probability") throw new SdkInvariantError({ message: "Expected a query probability observation" })
	const event = encodedEvent(data.event)
	const given = encodedEvent(data.given)
	if (data.law === "fixed" && data.defined === null)
		return Object.freeze({
			kind: "probability",
			law: "fixed",
			event,
			given,
			numerator: encodedRational(data.numerator),
			evidenceMass: encodedRational(data.evidenceMass),
			value: data.value === null ? null : encodedRational(data.value)
		})
	if (data.law === "parameter")
		return Object.freeze({
			kind: "probability",
			law: "parameter",
			event,
			given,
			numerator: encodedSource("parameterFunction", data.numerator),
			evidenceMass: encodedSource("parameterFunction", data.evidenceMass),
			value: encodedSource("parameterFunction", data.value),
			defined: encodedParameter("region", data.defined)
		})
	throw new SdkInvariantError({ message: "Unexpected query probability law" })
}
