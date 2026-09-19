import { type Event, encodedEvent, eventBytes } from "#event-value.ts"
import { parameterData, parameterResult } from "#parameter-operation.ts"
import { operands, roster } from "#parameter-source.ts"
import { encodedSource, type SourceValue, sourceBytes } from "#source-value.ts"
import { arrayValue, recordValue } from "#values.ts"

export interface FunctionPatch<F> {
	readonly region: Event
	readonly function: F
}
/** Native admission has proved exact agreement on overlaps and complete coverage
 * on parent. The total assembled function is zero outside parent. */
export interface FunctionCover<F> {
	readonly parent: Event
	readonly patches: readonly FunctionPatch<F>[]
	readonly function: F
}
type Kind = "function" | "familyFunction"
export function maskFunction<K extends Kind>(kind: K, fn: SourceValue<K>, region: Event) {
	return parameterResult(
		kind === "function" ? "cover.finiteMask" : "cover.familyMask",
		() => [sourceBytes(kind, fn), eventBytes(region)],
		(value) => encodedSource(kind, value)
	)
}
/** Inputs are replayed on the worker; a caller's plain cover object is never
 * accepted as a certificate. Empty and duplicate patches remain inspectable. */
export function glueFunctions<K extends Kind>(
	kind: K,
	parent: Event,
	patches: readonly FunctionPatch<SourceValue<K>>[]
) {
	return parameterData(
		kind === "function" ? "cover.finite" : "cover.family",
		() => {
			roster(patches, 8191)
			const out = operands()
			out.add(eventBytes(parent))
			arrayValue("Function patches", patches, (_, patch) => {
				const data = recordValue("FunctionPatch", patch, ["region", "function"])
				out.add(eventBytes(data.region as Event))
				out.add(sourceBytes(kind, data.function as SourceValue<K>))
			})
			return out.values
		},
		(input): FunctionCover<SourceValue<K>> => {
			const data = recordValue("Function cover", input, ["parent", "patches", "function"])
			return Object.freeze({
				parent: encodedEvent(data.parent),
				function: encodedSource(kind, data.function),
				patches: arrayValue("Function patches", data.patches, (_, patch) => {
					const data = recordValue("FunctionPatch", patch, ["region", "function"])
					return Object.freeze({ region: encodedEvent(data.region), function: encodedSource(kind, data.function) })
				})
			})
		}
	)
}
