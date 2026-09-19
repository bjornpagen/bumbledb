import { Effect } from "effect"
import { AuthoringError, SdkInvariantError } from "#errors.ts"
import { type Event, encodedEvent, eventBytes } from "#event-value.ts"
import { type ExactRational, encodedRational, rationalBytes } from "#exact-value.ts"
import type { ParameterFunction } from "#parameter-function.ts"
import { parameterBoolean, parameterData, parameterResult } from "#parameter-operation.ts"
import { ParameterDomain, ParameterRegion, type RealWitness } from "#parameter-region.ts"
import { encodedParameter, type ParameterValue, parameterBytes } from "#parameter-value.ts"
import { argumentError } from "#runtime-errors.ts"
import { encodedSource, type SourceValue, sourceBytes } from "#source-value.ts"
import { arrayValue, recordValue } from "#values.ts"

export interface ParameterGuard {
	readonly coordinate: bigint
	readonly region: ParameterRegion
}
export interface ParameterWorld {
	readonly parameter: RealWitness
	readonly outcomes: bigint
}
export interface ParameterSourceDescription {
	readonly domain: ParameterDomain
	readonly guards: readonly ParameterGuard[]
	readonly outcomeCoordinates: bigint
}
export type WorldCardinality = { readonly kind: "finite"; readonly count: bigint } | { readonly kind: "continuum" }
interface ParameterObservation {
	readonly given: Event
	readonly numerator: ParameterFunction
	readonly evidenceMass: ParameterFunction
	/** Owned partial conditional function. It can be nowhere defined. */
	readonly value: ParameterFunction
	readonly defined: ParameterRegion
}
export interface ParameterProbabilityObservation extends ParameterObservation {
	readonly kind: "probability"
	readonly event: Event
}
export interface ParameterExpectationObservation<A> extends ParameterObservation {
	readonly kind: "expectation"
	readonly function: A
}

export function unsigned(input: unknown, maximum = 0xffffffffffffffffn): bigint {
	if (typeof input !== "bigint" || input < 0n || input > maximum)
		throw new AuthoringError({ message: "Event parameter: invalid unsigned integer" })
	return input
}
export function outcomeBytes(input: unknown): Uint8Array {
	const bytes = new Uint8Array(8)
	new DataView(bytes.buffer).setBigUint64(0, unsigned(input), true)
	return bytes
}
export function operands() {
	let remaining = 16 * 1024 * 1024
	const values: Uint8Array[] = []
	return {
		values,
		add(bytes: Uint8Array) {
			if (bytes.length > remaining) throw new AuthoringError({ message: "Event parameter: exceeds 16 MiB input" })
			remaining -= bytes.length
			values.push(bytes)
		}
	}
}
export function roster(input: unknown, maximum: number): asserts input is readonly unknown[] {
	if (!Array.isArray(input) || input.length > maximum)
		throw new AuthoringError({ message: "Event parameter: exceeds input roster" })
}
export function realWitness(input: unknown): RealWitness {
	const data = recordValue("Parameter witness", input, ["kind", "value"])
	if (data.kind === "rational") return Object.freeze({ kind: "rational", value: encodedRational(data.value) })
	if (data.kind === "algebraic")
		return Object.freeze({ kind: "algebraic", value: encodedParameter("root", data.value) })
	throw new SdkInvariantError({ message: "Parameter witness: unexpected kind" })
}
/** The input must be full, unmeasured and not already parameterized. Guards
 * reinterpret existing coordinates deterministically; no random mass or prior. */
export function withParameters(space: Event, domain: ParameterDomain, guards: readonly ParameterGuard[]) {
	return parameterResult(
		"source.new",
		() => {
			roster(guards, 62)
			const out = operands()
			out.add(eventBytes(space))
			out.add(ParameterDomain.toBytes(domain))
			arrayValue("Parameter guards", guards, (_, guard) => {
				const data = recordValue("ParameterGuard", guard, ["coordinate", "region"])
				out.add(Uint8Array.of(Number(unsigned(data.coordinate, 61n))))
				out.add(ParameterRegion.toBytes(data.region as ParameterRegion))
			})
			return out.values
		},
		encodedEvent
	)
}
export function describeParameters(value: Event) {
	return parameterData(
		"source.describe",
		() => [eventBytes(value)],
		(input): ParameterSourceDescription => {
			const data = recordValue("Parameter source", input, ["domain", "guards", "outcomeCoordinates"])
			return Object.freeze({
				domain: encodedParameter("domain", data.domain),
				outcomeCoordinates: unsigned(data.outcomeCoordinates),
				guards: arrayValue("Parameter guards", data.guards, (_, guard) => {
					const data = recordValue("ParameterGuard", guard, ["coordinate", "region"])
					return Object.freeze({
						coordinate: unsigned(data.coordinate, 61n),
						region: encodedParameter("region", data.region)
					})
				})
			})
		}
	)
}
/** Represent the predicate in the existing guard presentation; a new cut
 * requires an explicit ParameterRefinement first. */
export function parameterEvent(context: Event, predicate: ParameterRegion) {
	return parameterResult("source.event", () => [eventBytes(context), ParameterRegion.toBytes(predicate)], encodedEvent)
}
export function parameterMass(value: Event) {
	return parameterResult(
		"source.mass",
		() => [eventBytes(value)],
		(input) => encodedSource("parameterFunction", input)
	)
}
function observation(input: unknown, kind: string) {
	const data = recordValue("Parameter observation", input, [
		"kind",
		"input",
		"given",
		"numerator",
		"evidenceMass",
		"value",
		"defined"
	])
	if (data.kind !== kind) throw new SdkInvariantError({ message: "Parameter observation: unexpected kind" })
	return {
		data,
		given: encodedEvent(data.given),
		numerator: encodedSource("parameterFunction", data.numerator),
		evidenceMass: encodedSource("parameterFunction", data.evidenceMass),
		value: encodedSource("parameterFunction", data.value),
		defined: encodedParameter("region", data.defined)
	}
}
export function parameterProbability(event: Event, given: Event) {
	return parameterData(
		"source.probability",
		() => [eventBytes(event), eventBytes(given)],
		(input): ParameterProbabilityObservation => {
			const { data, ...values } = observation(input, "probability")
			return Object.freeze({ kind: "probability", event: encodedEvent(data.input), ...values })
		}
	)
}
export function parameterExpectation(fn: SourceValue<"function">, given: Event) {
	return parameterData(
		"source.expectation",
		() => [sourceBytes("function", fn), eventBytes(given)],
		(input): ParameterExpectationObservation<SourceValue<"function">> => {
			const { data, ...values } = observation(input, "finiteExpectation")
			return Object.freeze({ kind: "expectation", function: encodedSource("function", data.input), ...values })
		}
	)
}
export function familyExpectation(fn: SourceValue<"familyFunction">, given: Event) {
	return parameterData(
		"family.expectation",
		() => [sourceBytes("familyFunction", fn), eventBytes(given)],
		(input): ParameterExpectationObservation<SourceValue<"familyFunction">> => {
			const { data, ...values } = observation(input, "familyExpectation")
			return Object.freeze({ kind: "expectation", function: encodedSource("familyFunction", data.input), ...values })
		}
	)
}
/** Domain/outcome violations refuse; false means a legal world outside this Event. */
export const containsParameter = Effect.fn("Event.containsParameter")(function* (value: Event, world: ParameterWorld) {
	const owned = yield* Effect.try({
		try: () => {
			const data = recordValue("ParameterWorld", world, ["parameter", "outcomes"])
			const point = recordValue("RealWitness", data.parameter, ["kind", "value"])
			if (point.kind !== "rational" && point.kind !== "algebraic")
				throw new AuthoringError({ message: "ParameterWorld: invalid witness kind" })
			return {
				op: point.kind === "rational" ? "source.containsRational" : "source.containsRoot",
				inputs: [
					eventBytes(value),
					point.kind === "rational"
						? rationalBytes(point.value as ExactRational)
						: parameterBytes("root", point.value as ParameterValue<"root">),
					outcomeBytes(data.outcomes)
				]
			}
		},
		catch: (cause) => argumentError("Event.containsParameter", cause)
	})
	return yield* parameterBoolean(owned.op, () => owned.inputs)
})
export function parameterWitness(value: Event) {
	return parameterData(
		"source.witness",
		() => [eventBytes(value)],
		(input): ParameterWorld | null => {
			const data = recordValue("Parameter world", input, ["world"])
			if (data.world === null) return null
			const world = recordValue("ParameterWorld", data.world, ["parameter", "outcomes"])
			return Object.freeze({ parameter: realWitness(world.parameter), outcomes: unsigned(world.outcomes) })
		}
	)
}
export function worldCardinality(value: Event) {
	return parameterData(
		"source.cardinality",
		() => [eventBytes(value)],
		(input): WorldCardinality => {
			const data = recordValue("World cardinality", input, ["kind", "count"])
			if (data.kind === "finite") return Object.freeze({ kind: "finite", count: unsigned(data.count) })
			if (data.kind === "continuum" && data.count === null) return Object.freeze({ kind: "continuum" })
			throw new SdkInvariantError({ message: "World cardinality: unexpected result" })
		}
	)
}
