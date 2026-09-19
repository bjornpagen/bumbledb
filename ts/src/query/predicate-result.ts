/** Exact truth partitions alongside their complete source-retaining derivations. */
import { SdkInvariantError } from "#errors.ts"
import { encodedParameter, type ParameterValue } from "#parameter-value.ts"
import { encodedPredicate, type ObservationPredicate } from "#predicate-value.ts"
import { recordValue } from "#values.ts"

const resultTag: unique symbol = Symbol("bumbledb.PredicateResult")
export interface PredicateResult {
	readonly kind: "predicate"
	readonly [resultTag]: true
}
export const predicateResult: PredicateResult = Object.freeze({ kind: "predicate", [resultTag]: true as const })
export type PredicateAnswer =
	| {
			readonly kind: "predicate"
			readonly predicate: ObservationPredicate
			readonly law: "fixed"
			readonly value: boolean | null
	  }
	| {
			readonly kind: "predicate"
			readonly predicate: ObservationPredicate
			readonly law: "parameter"
			readonly holds: ParameterValue<"region">
			readonly fails: ParameterValue<"region">
			readonly undefined: ParameterValue<"region">
			readonly domain: ParameterValue<"domain">
	  }
export interface PredicateWire {
	readonly kind: "predicate"
	readonly bytes: Uint8Array
	readonly law: "fixed" | "parameter"
	readonly value: boolean | null
	readonly holds: Uint8Array | null
	readonly fails: Uint8Array | null
	readonly undefined: Uint8Array | null
	readonly domain: Uint8Array | null
}
export function decodePredicate(input: unknown): PredicateAnswer {
	const data = recordValue("Query predicate", input, [
		"kind",
		"bytes",
		"law",
		"value",
		"holds",
		"fails",
		"undefined",
		"domain"
	])
	if (data.kind !== "predicate") throw new SdkInvariantError({ message: "Expected a query predicate" })
	const predicate = encodedPredicate(data.bytes)
	if (
		data.law === "fixed" &&
		(data.value === null || typeof data.value === "boolean") &&
		data.holds === null &&
		data.fails === null &&
		data.undefined === null &&
		data.domain === null
	)
		return Object.freeze({ kind: "predicate", predicate, law: "fixed", value: data.value })
	if (data.law === "parameter" && data.value === null)
		return Object.freeze({
			kind: "predicate",
			predicate,
			law: "parameter",
			holds: encodedParameter("region", data.holds),
			fails: encodedParameter("region", data.fails),
			undefined: encodedParameter("region", data.undefined),
			domain: encodedParameter("domain", data.domain)
		})
	throw new SdkInvariantError({ message: "Unexpected query predicate domain" })
}
