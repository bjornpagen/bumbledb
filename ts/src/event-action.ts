/** Owned BEAC arena/strategy recipes. Envelope recognition is pure; native
 * admission replays the model and objective, then checks selected policy choices. */
import { Effect, Result } from "effect"
import { dbNative } from "#db-native.ts"
import { AuthoringError } from "#errors.ts"
import { decodeAction, decodeActionInspection, type EventActionDescription, encodeAction } from "#event-action-data.ts"
import { encodedDescriptor } from "#event-descriptor.ts"
import { DescriptorBudget } from "#event-descriptor-data.ts"
import { EventMemoryArena } from "#event-memory-arena.ts"
import { type Event, encodedEvent, eventBytes, eventValue } from "#event-value.ts"
import { nativeOperationWith, runtimeHandle } from "#runtime.ts"
import { argumentError, type DbError } from "#runtime-errors.ts"
import { bytesValue } from "#values.ts"

const tag: unique symbol = Symbol("bumbledb.EventAction")
interface EventAction {
	readonly [tag]: true
}
const encodings = new WeakMap<EventAction, Uint8Array>()
function isAction(input: unknown): input is EventAction {
	return typeof input === "object" && input !== null && encodings.has(input as EventAction)
}
function encoded(input: unknown): EventAction {
	const bytes = bytesValue("Event action", input, 16 * 1024 * 1024)
	if (bytes.length < 5 || bytes[0] !== 66 || bytes[1] !== 69 || bytes[2] !== 65 || bytes[3] !== 67 || bytes[4] !== 1)
		throw new AuthoringError({ message: "Event action: expected a BEAC v1 envelope" })
	const value = Object.freeze({ [tag]: true as const })
	encodings.set(value, bytes)
	return value
}
function toBytes(value: EventAction): Uint8Array {
	const bytes = encodings.get(value)
	if (bytes === undefined) throw new AuthoringError({ message: "Event action: expected owned BEAC" })
	return new Uint8Array(bytes)
}
function fromBytes(input: Uint8Array): Result.Result<EventAction, DbError> {
	return Result.try({ try: () => encoded(input), catch: (cause) => argumentError("EventAction.fromBytes", cause) })
}
const admit = Effect.fn("EventAction.admit")(function* (description: EventActionDescription) {
	const runtime = yield* runtimeHandle()
	const input = yield* Effect.try({
		try: () => encodeAction(description),
		catch: (cause) => argumentError("EventAction.admit", cause)
	})
	return yield* nativeOperationWith(
		"EventAction.admit",
		(cb) => dbNative.runtimeEventAction(runtime, "admit", input, [], cb),
		dbNative.runtimeBytesTake,
		encoded
	)
})
function inputs(value: EventAction, event?: Event) {
	const input = toBytes(value)
	const operands = event === undefined ? [] : [eventBytes(eventValue("Event action operand", event))]
	const budget = new DescriptorBudget()
	for (const bytes of [input, ...operands]) {
		budget.item()
		budget.length(bytes.length)
	}
	return { input, operands }
}
function inspection<A>(operation: "describe" | "inspect", decode: (input: unknown) => A) {
	return Effect.fn(`EventAction.${operation}`)(function* (value: EventAction) {
		const runtime = yield* runtimeHandle()
		const input = yield* Effect.try({
			try: () => toBytes(value),
			catch: (cause) => argumentError(`EventAction.${operation}`, cause)
		})
		return yield* nativeOperationWith(
			`EventAction.${operation}`,
			(cb) => dbNative.runtimeEventAction(runtime, operation, input, [], cb),
			dbNative.runtimeEventActionTake,
			decode
		)
	})
}
function operation<A>(
	op: "enabled" | "good" | "predecessor" | "reach" | "safe" | "restrict",
	decode: (input: unknown) => A
) {
	return Effect.fn(`EventAction.${op}`)(function* (value: EventAction, event?: Event) {
		const runtime = yield* runtimeHandle()
		const { input, operands } = yield* Effect.try({
			try: () => inputs(value, event),
			catch: (cause) => argumentError(`EventAction.${op}`, cause)
		})
		return yield* nativeOperationWith(
			`EventAction.${op}`,
			(cb) => dbNative.runtimeEventAction(runtime, op, input, operands, cb),
			dbNative.runtimeBytesTake,
			decode
		)
	})
}
const enabledWork = operation("enabled", encodedDescriptor)
const goodWork = operation("good", encodedDescriptor)
const predecessorWork = operation("predecessor", encodedEvent)
const reachWork = operation("reach", encoded)
const safeWork = operation("safe", encoded)
const restrictWork = operation("restrict", encoded)
/** Explicitly forget hidden interpretation after replaying BEBA's exact named
 * compilation. Retain the memory arena to continue interpreting hidden Events. */
const fromMemory = Effect.fn("EventAction.fromMemory")(function* (value: EventMemoryArena) {
	const runtime = yield* runtimeHandle()
	const input = yield* Effect.try({
		try: () => EventMemoryArena.toBytes(value),
		catch: (cause) => argumentError("EventAction.fromMemory", cause)
	})
	return yield* nativeOperationWith(
		"EventAction.fromMemory",
		(cb) => dbNative.runtimeEventAction(runtime, "fromMemory", input, [], cb),
		dbNative.runtimeBytesTake,
		encoded
	)
})
const EventAction = Object.freeze({
	fromBytes,
	toBytes,
	isAction,
	admit,
	describe: inspection("describe", decodeAction),
	inspect: inspection("inspect", decodeActionInspection),
	fromMemory,
	enabled: (value: EventAction) => enabledWork(value),
	good: (value: EventAction, outcome: Event) => goodWork(value, outcome),
	predecessor: (value: EventAction, outcome: Event) => predecessorWork(value, outcome),
	/** Objectives are Events on the arena's state space. */
	reach: (value: EventAction, goal: Event) => reachWork(value, goal),
	safe: (value: EventAction, invariant: Event) => safeWork(value, invariant),
	/** A region on this strategy's named state/action product. */
	withPolicy: (value: EventAction, policy: Event) => restrictWork(value, policy)
})

export { EventAction }
