/** Query-only expectations retain checked scalar rosters or local function covers. */
import { SdkInvariantError } from "#errors.ts"
import { type Event, encodedEvent } from "#event-value.ts"
import { type ExactRational, encodedRational } from "#exact-value.ts"
import type { FunctionPatch } from "#function-cover.ts"
import type { ParameterExpectationObservation } from "#parameter-source.ts"
import { encodedParameter } from "#parameter-value.ts"
import type { ExpectationObservation } from "#source-operation.ts"
import { encodedSource, type SourceValue } from "#source-value.ts"
import { arrayValue, recordValue } from "#values.ts"

type FinitePayoff =
	| {
			readonly payoffKind: "scalar"
			readonly payoffs: readonly { readonly region: Event; readonly value: ExactRational }[]
	  }
	| { readonly payoffKind: "finite"; readonly patches: readonly FunctionPatch<SourceValue<"function">>[] }
type FamilyPayoff = {
	readonly payoffKind: "family"
	readonly patches: readonly FunctionPatch<SourceValue<"familyFunction">>[]
}
export type ExpectationAnswer =
	| (FinitePayoff &
			(
				| (ExpectationObservation & { readonly law: "fixed" })
				| (ParameterExpectationObservation<SourceValue<"function">> & { readonly law: "parameter" })
			))
	| (FamilyPayoff & ParameterExpectationObservation<SourceValue<"familyFunction">> & { readonly law: "parameter" })
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
	readonly payoffKind: "scalar" | "finite" | "family"
	readonly function: Uint8Array
	readonly given: Uint8Array
	readonly payoffs?: readonly { readonly region: Uint8Array; readonly value: Uint8Array }[]
	readonly patches?: readonly { readonly region: Uint8Array; readonly function: Uint8Array }[]
	readonly numerator: Uint8Array
	readonly evidenceMass: Uint8Array
	readonly value: Uint8Array | null
	readonly defined: Uint8Array | null
}
function patches<K extends "function" | "familyFunction">(
	kind: K,
	input: unknown
): readonly FunctionPatch<SourceValue<K>>[] {
	return arrayValue("Query function patches", input, (_, input) => {
		const patch = recordValue("Query function patch", input, ["region", "function"])
		return Object.freeze({ region: encodedEvent(patch.region), function: encodedSource(kind, patch.function) })
	})
}
export function decodeExpectation(input: unknown): ExpectationAnswer {
	const payoffKind =
		typeof input === "object" && input !== null
			? Object.getOwnPropertyDescriptor(input, "payoffKind")?.value
			: undefined
	const data = recordValue("Query expectation", input, [
		"kind",
		"law",
		"payoffKind",
		"function",
		"given",
		payoffKind === "scalar" ? "payoffs" : "patches",
		"numerator",
		"evidenceMass",
		"value",
		"defined"
	])
	if (data.kind !== "expectation") throw new SdkInvariantError({ message: "Expected a query expectation observation" })
	const common = { kind: "expectation" as const, given: encodedEvent(data.given) }
	const parameter = () => ({
		...common,
		law: "parameter" as const,
		numerator: encodedSource("parameterFunction", data.numerator),
		evidenceMass: encodedSource("parameterFunction", data.evidenceMass),
		value: encodedSource("parameterFunction", data.value),
		defined: encodedParameter("region", data.defined)
	})
	if (data.payoffKind === "family" && data.law === "parameter")
		return Object.freeze({
			...parameter(),
			payoffKind: "family",
			function: encodedSource("familyFunction", data.function),
			patches: patches("familyFunction", data.patches)
		})
	let roster: FinitePayoff
	if (data.payoffKind === "scalar")
		roster = {
			payoffKind: "scalar",
			payoffs: arrayValue("Query payoffs", data.payoffs, (_, input) => {
				const piece = recordValue("Query payoff", input, ["region", "value"])
				return Object.freeze({ region: encodedEvent(piece.region), value: encodedRational(piece.value) })
			})
		}
	else if (data.payoffKind === "finite") roster = { payoffKind: "finite", patches: patches("function", data.patches) }
	else throw new SdkInvariantError({ message: "Unexpected query payoff kind" })
	const finite = { ...common, ...roster, function: encodedSource("function", data.function) }
	if (data.law === "fixed" && data.defined === null)
		return Object.freeze({
			...finite,
			law: "fixed",
			numerator: encodedRational(data.numerator),
			evidenceMass: encodedRational(data.evidenceMass),
			value: data.value === null ? null : encodedRational(data.value)
		})
	if (data.law === "parameter") return Object.freeze({ ...finite, ...parameter() })
	throw new SdkInvariantError({ message: "Unexpected query expectation law" })
}
