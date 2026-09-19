import { Effect } from "effect"
import { dbNative } from "#db-native.ts"
import { AuthoringError, SdkInvariantError } from "#errors.ts"
import { type Event, encodedEvent, eventBytes } from "#event-value.ts"
import { type ExactRational, encodedRational } from "#exact-value.ts"
import { nativeOperationWith, runtimeHandle } from "#runtime.ts"
import { argumentError } from "#runtime-errors.ts"
import type { OperationHandle } from "#runtime-native.ts"
import { encodedSource, type SourceValue, sourceBytes } from "#source-value.ts"
import { recordValue } from "#values.ts"

export interface ProbabilityObservation {
	readonly kind: "probability"
	readonly event: Event
	readonly given: Event
	readonly numerator: ExactRational
	readonly evidenceMass: ExactRational
	/** null means zero evidence mass, even if the evidence is nonempty. */
	readonly value: ExactRational | null
}
export interface ExpectationObservation {
	readonly kind: "expectation"
	readonly function: SourceValue<"function">
	readonly given: Event
	readonly numerator: ExactRational
	readonly evidenceMass: ExactRational
	readonly value: ExactRational | null
}
export const sourceOperation = Effect.fn("EventSource.execute")(function* <A, W>(
	op: string,
	inputs: () => readonly Uint8Array[],
	argument: bigint,
	take: (operation: OperationHandle) => W,
	accept: (wire: W) => A
) {
	const runtime = yield* runtimeHandle()
	const owned = yield* Effect.try({
		try: () => {
			if (typeof argument !== "bigint" || argument < 0n || argument > 0xffffffffffffffffn)
				throw new AuthoringError({ message: "Event source: expected a u64 world" })
			return inputs()
		},
		catch: (cause) => argumentError(`EventSource.${op}`, cause)
	})
	return yield* nativeOperationWith(
		`EventSource.${op}`,
		(cb) => dbNative.runtimeEventSource(runtime, op, owned, argument, cb),
		take,
		accept
	)
})
export function sourceResult<A>(
	op: string,
	inputs: () => readonly Uint8Array[],
	accept: (wire: Uint8Array) => A,
	argument = 0n
) {
	return sourceOperation(op, inputs, argument, (operation) => dbNative.runtimeBytesTake(operation), accept)
}
export function sourceBoolean(op: string, inputs: () => readonly Uint8Array[]) {
	return sourceOperation(
		op,
		inputs,
		0n,
		(operation) => dbNative.runtimeRowsTake(operation),
		(rows) => {
			if (rows.length !== 1 || rows[0]?.length !== 1 || typeof rows[0][0] !== "boolean")
				throw new SdkInvariantError({ message: "Event source: expected Boolean result" })
			return rows[0][0]
		}
	)
}
function observation(input: unknown, kind: "probability" | "expectation") {
	const data = recordValue("Event observation", input, [
		"kind",
		kind === "probability" ? "event" : "function",
		"given",
		"numerator",
		"evidenceMass",
		"value"
	])
	if (data.kind !== kind) throw new SdkInvariantError({ message: "Event source: wrong observation kind" })
	return {
		data,
		given: encodedEvent(data.given),
		numerator: encodedRational(data.numerator),
		evidenceMass: encodedRational(data.evidenceMass),
		value: data.value === null ? null : encodedRational(data.value)
	}
}
export function mass(event: Event) {
	return sourceResult("mass", () => [eventBytes(event)], encodedRational)
}
export function probability(event: Event, given: Event) {
	return sourceOperation(
		"probability",
		() => [eventBytes(event), eventBytes(given)],
		0n,
		(operation) => dbNative.runtimeEventSourceTake(operation),
		(input): ProbabilityObservation => {
			const { data, ...values } = observation(input, "probability")
			return Object.freeze({ kind: "probability", event: encodedEvent(data.event), ...values })
		}
	)
}
export function expectation(fn: SourceValue<"function">, given: Event) {
	return sourceOperation(
		"expectation",
		() => [sourceBytes("function", fn), eventBytes(given)],
		0n,
		(operation) => dbNative.runtimeEventSourceTake(operation),
		(input): ExpectationObservation => {
			const { data, ...values } = observation(input, "expectation")
			return Object.freeze({ kind: "expectation", function: encodedSource("function", data.function), ...values })
		}
	)
}
