/** Exact partial query numbers alongside their complete owned derivations. */
import { SdkInvariantError } from "#errors.ts"
import { type ExactRational, encodedRational } from "#exact-value.ts"
import { encodedNumber, type ObservationNumber } from "#number-value.ts"
import { encodedParameter, type ParameterValue } from "#parameter-value.ts"
import { encodedSource, type SourceValue } from "#source-value.ts"
import { recordValue } from "#values.ts"

const resultTag: unique symbol = Symbol("bumbledb.NumberResult")
export interface NumberResult {
	readonly kind: "number"
	readonly [resultTag]: true
}
export const numberResult: NumberResult = Object.freeze({ kind: "number", [resultTag]: true as const })
export type NumberAnswer =
	| {
			readonly kind: "number"
			readonly number: ObservationNumber
			readonly law: "fixed"
			readonly value: ExactRational | null
	  }
	| {
			readonly kind: "number"
			readonly number: ObservationNumber
			readonly law: "parameter"
			readonly value: SourceValue<"parameterFunction">
			readonly defined: ParameterValue<"region">
			readonly domain: ParameterValue<"domain">
	  }
export interface NumberWire {
	readonly kind: "number"
	readonly bytes: Uint8Array
	readonly law: "fixed" | "parameter"
	readonly value: Uint8Array | null
	readonly defined: Uint8Array | null
	readonly domain: Uint8Array | null
}
export function decodeNumber(input: unknown): NumberAnswer {
	const data = recordValue("Query number", input, ["kind", "bytes", "law", "value", "defined", "domain"])
	if (data.kind !== "number") throw new SdkInvariantError({ message: "Expected a query number" })
	const number = encodedNumber(data.bytes)
	if (data.law === "fixed" && data.defined === null && data.domain === null)
		return Object.freeze({
			kind: "number",
			number,
			law: "fixed",
			value: data.value === null ? null : encodedRational(data.value)
		})
	if (data.law === "parameter")
		return Object.freeze({
			kind: "number",
			number,
			law: "parameter",
			value: encodedSource("parameterFunction", data.value),
			defined: encodedParameter("region", data.defined),
			domain: encodedParameter("domain", data.domain)
		})
	throw new SdkInvariantError({ message: "Unexpected query number domain" })
}
