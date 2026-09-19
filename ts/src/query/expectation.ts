/** Query-only expectations retain the checked payoff partition on evidence. */
import { SdkInvariantError } from "#errors.ts"
import { type Event, encodedEvent } from "#event-value.ts"
import { type ExactRational, encodedRational } from "#exact-value.ts"
import type { ParameterExpectationObservation } from "#parameter-source.ts"
import { encodedParameter } from "#parameter-value.ts"
import type { ExpectationObservation } from "#source-operation.ts"
import { encodedSource, type SourceValue } from "#source-value.ts"
import { recordValue } from "#values.ts"

interface PayoffRoster {
	/** Complete on given; includes supplied zero values and empty buckets. */
	readonly payoffs: readonly { readonly region: Event; readonly value: ExactRational }[]
}
export type ExpectationAnswer = PayoffRoster &
	(
		| (ExpectationObservation & { readonly law: "fixed" })
		| (ParameterExpectationObservation<SourceValue<"function">> & { readonly law: "parameter" })
	)
const resultTag: unique symbol = Symbol("bumbledb.ExpectationResult")
export interface ExpectationResult {
	readonly kind: "expectation"
	readonly [resultTag]: true
}
/** Imported output descriptor; not a stored schema field. */
export const expectationResult: ExpectationResult = Object.freeze({ kind: "expectation", [resultTag]: true as const })
export interface ExpectationWire {
	readonly kind: "expectation"
	readonly law: "fixed" | "parameter"
	readonly function: Uint8Array
	readonly given: Uint8Array
	readonly payoffs: readonly { readonly region: Uint8Array; readonly value: Uint8Array }[]
	readonly numerator: Uint8Array
	readonly evidenceMass: Uint8Array
	readonly value: Uint8Array | null
	readonly defined: Uint8Array | null
}
export function decodeExpectation(input: unknown): ExpectationAnswer {
	const data = recordValue("Query expectation", input, [
		"kind",
		"law",
		"function",
		"given",
		"payoffs",
		"numerator",
		"evidenceMass",
		"value",
		"defined"
	])
	if (data.kind !== "expectation" || !Array.isArray(data.payoffs))
		throw new SdkInvariantError({ message: "Expected a query expectation observation" })
	const payoffs = Object.freeze(
		data.payoffs.map((input: unknown) => {
			const piece = recordValue("Query payoff", input, ["region", "value"])
			return Object.freeze({ region: encodedEvent(piece.region), value: encodedRational(piece.value) })
		})
	)
	const common = {
		kind: "expectation" as const,
		function: encodedSource("function", data.function),
		given: encodedEvent(data.given),
		payoffs
	}
	if (data.law === "fixed" && data.defined === null)
		return Object.freeze({
			...common,
			law: "fixed",
			numerator: encodedRational(data.numerator),
			evidenceMass: encodedRational(data.evidenceMass),
			value: data.value === null ? null : encodedRational(data.value)
		})
	if (data.law === "parameter")
		return Object.freeze({
			...common,
			law: "parameter",
			numerator: encodedSource("parameterFunction", data.numerator),
			evidenceMass: encodedSource("parameterFunction", data.evidenceMass),
			value: encodedSource("parameterFunction", data.value),
			defined: encodedParameter("region", data.defined)
		})
	throw new SdkInvariantError({ message: "Unexpected query expectation law" })
}
