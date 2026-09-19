/** Named compilation of exact possibility memory. BEBA stores its checked
 * reconstruction recipe, not an imported graph or claimed policy certificate. */
import { Effect, Result } from "effect"
import { dbNative } from "#db-native.ts"
import { AuthoringError } from "#errors.ts"
import { type EventDescriptor, encodedDescriptor } from "#event-descriptor.ts"
import { DescriptorBudget } from "#event-descriptor-data.ts"
import { type EventMemory, encodedMemory, memoryBytes } from "#event-memory.ts"
import { type Event, encodedEvent, eventBytes, eventValue } from "#event-value.ts"
import { nativeOperationWith, runtimeHandle } from "#runtime.ts"
import { argumentError, type DbError } from "#runtime-errors.ts"
import { bytesValue, recordValue } from "#values.ts"

export interface EventMemoryIdentities {
	readonly states: Uint8Array
	readonly actions: Uint8Array
	readonly environment: Uint8Array
	readonly stateActions: Uint8Array
	readonly transitions: Uint8Array
}
export interface EventMemoryArenaDescription {
	readonly memory: EventMemory
	readonly identities: EventMemoryIdentities
}
export interface EventMemoryArenaInspection {
	readonly states: Event
	readonly choices: Event
	readonly initial: Event
	readonly actions: EventDescriptor
	readonly transition: EventDescriptor
	readonly stateCodes: readonly Event[]
	readonly actionCodes: readonly Event[]
}
/** Detached results of the native ranked solver. These do not import an executable
 * strategy certificate. Every retained action strictly decreases first-entry rank. */
export interface EventMemoryReachResult {
	readonly goal: Event
	readonly winning: Event
	readonly policy: EventDescriptor
	readonly ranks: readonly Event[]
}
export interface EventMemorySafetyResult {
	readonly invariant: Event
	readonly winning: Event
	readonly policy: EventDescriptor
}
const tag: unique symbol = Symbol("bumbledb.EventMemoryArena")
interface EventMemoryArena {
	readonly [tag]: true
}
const encodings = new WeakMap<EventMemoryArena, Uint8Array>()
function isArena(input: unknown): input is EventMemoryArena {
	return typeof input === "object" && input !== null && encodings.has(input as EventMemoryArena)
}
function encoded(input: unknown): EventMemoryArena {
	const bytes = bytesValue("Event memory arena", input, 16 * 1024 * 1024)
	if (bytes.length < 5 || bytes[0] !== 66 || bytes[1] !== 69 || bytes[2] !== 66 || bytes[3] !== 65 || bytes[4] !== 1)
		throw new AuthoringError({ message: "Event memory arena: expected a BEBA v1 envelope" })
	const value = Object.freeze({ [tag]: true as const })
	encodings.set(value, bytes)
	return value
}
function toBytes(value: EventMemoryArena) {
	const bytes = encodings.get(value)
	if (bytes === undefined) throw new AuthoringError({ message: "Event memory arena: expected owned BEBA" })
	return new Uint8Array(bytes)
}
function fromBytes(input: Uint8Array): Result.Result<EventMemoryArena, DbError> {
	return Result.try({ try: () => encoded(input), catch: (cause) => argumentError("EventMemoryArena.fromBytes", cause) })
}
const names = ["states", "actions", "environment", "stateActions", "transitions"] as const
function identities(input: unknown, budget: DescriptorBudget): EventMemoryIdentities {
	const data = recordValue("Event memory identities", input, names)
	return Object.freeze({
		states: budget.identity(data.states),
		actions: budget.identity(data.actions),
		environment: budget.identity(data.environment),
		stateActions: budget.identity(data.stateActions),
		transitions: budget.identity(data.transitions)
	})
}
function bounded(inputs: readonly Uint8Array[]) {
	const budget = new DescriptorBudget()
	for (const bytes of inputs) {
		budget.item()
		budget.length(bytes.length)
	}
	return inputs
}
const compileMemory = Effect.fn("EventMemory.compile")(function* (memory: EventMemory, ids: EventMemoryIdentities) {
	const runtime = yield* runtimeHandle()
	const inputs = yield* Effect.try({
		try: () => {
			const idsCopy = identities(ids, new DescriptorBudget())
			return bounded([memoryBytes(memory), ...names.map((name) => idsCopy[name])])
		},
		catch: (cause) => argumentError("EventMemory.compile", cause)
	})
	return yield* nativeOperationWith(
		"EventMemory.compile",
		(cb) => dbNative.runtimeEventMemoryArena(runtime, "compile", inputs, cb),
		dbNative.runtimeBytesTake,
		encoded
	)
})
function decoder() {
	const budget = new DescriptorBudget()
	budget.item()
	const bytes = (input: unknown) => {
		budget.item()
		const value = bytesValue("Event memory arena result", input, budget.bytes)
		budget.length(value.length)
		return value
	}
	const event = (input: unknown) => encodedEvent(bytes(input))
	return {
		budget,
		bytes,
		event,
		descriptor: (input: unknown) => encodedDescriptor(bytes(input)),
		events: (input: unknown) => {
			budget.item()
			return budget.list(input, event)
		}
	}
}
function decodeDescription(input: unknown): EventMemoryArenaDescription {
	const { budget, bytes } = decoder()
	const data = recordValue("Event memory arena description", input, ["memory", "identities"])
	return Object.freeze({
		memory: encodedMemory(bytes(data.memory)),
		identities: identities(data.identities, budget)
	})
}
function fields(input: unknown, kind: string, names: readonly string[]) {
	const data = recordValue("Event memory arena result", input, ["kind", ...names])
	if (data.kind !== kind) throw new AuthoringError({ message: "Event memory arena: unexpected result kind" })
	return data
}
function decodeInspection(input: unknown): EventMemoryArenaInspection {
	const { event, descriptor, events } = decoder()
	const data = fields(input, "arena", [
		"states",
		"choices",
		"initial",
		"actions",
		"transition",
		"stateCodes",
		"actionCodes"
	])
	return Object.freeze({
		states: event(data.states),
		choices: event(data.choices),
		initial: event(data.initial),
		actions: descriptor(data.actions),
		transition: descriptor(data.transition),
		stateCodes: events(data.stateCodes),
		actionCodes: events(data.actionCodes)
	})
}
function decodeReach(input: unknown): EventMemoryReachResult {
	const { event, descriptor, events } = decoder()
	const data = fields(input, "reach", ["goal", "winning", "policy", "ranks"])
	return Object.freeze({
		goal: event(data.goal),
		winning: event(data.winning),
		policy: descriptor(data.policy),
		ranks: events(data.ranks)
	})
}
function decodeSafe(input: unknown): EventMemorySafetyResult {
	const { event, descriptor } = decoder()
	const data = fields(input, "safe", ["invariant", "winning", "policy"])
	return Object.freeze({
		invariant: event(data.invariant),
		winning: event(data.winning),
		policy: descriptor(data.policy)
	})
}
function inspectWith<A>(op: "describe" | "inspect" | "reach" | "safe", decode: (input: unknown) => A) {
	return Effect.fn(`EventMemoryArena.${op}`)(function* (value: EventMemoryArena, objective?: Event) {
		const runtime = yield* runtimeHandle()
		const inputs = yield* Effect.try({
			try: () =>
				bounded(
					objective === undefined
						? [toBytes(value)]
						: [toBytes(value), eventBytes(eventValue("Event memory objective", objective))]
				),
			catch: (cause) => argumentError(`EventMemoryArena.${op}`, cause)
		})
		return yield* nativeOperationWith(
			`EventMemoryArena.${op}`,
			(cb) => dbNative.runtimeEventMemoryArena(runtime, op, inputs, cb),
			dbNative.runtimeEventMemoryArenaTake,
			decode
		)
	})
}
const describeWork = inspectWith("describe", decodeDescription)
function describe(value: EventMemoryArena) {
	return describeWork(value)
}
const inspectWork = inspectWith("inspect", decodeInspection)
function inspect(value: EventMemoryArena) {
	return inspectWork(value)
}
const reachWork = inspectWith("reach", decodeReach)
const safeWork = inspectWith("safe", decodeSafe)
/** Goal and invariant are Events on the compiled memory-state space. */
function reach(value: EventMemoryArena, goal: Event) {
	return reachWork(value, goal)
}
function safe(value: EventMemoryArena, invariant: Event) {
	return safeWork(value, invariant)
}
function knowledge(op: "known" | "possible") {
	return Effect.fn(`EventMemoryArena.${op}`)(function* (value: EventMemoryArena, hidden: Event) {
		const runtime = yield* runtimeHandle()
		const inputs = yield* Effect.try({
			try: () => bounded([toBytes(value), eventBytes(eventValue("Event memory hidden predicate", hidden))]),
			catch: (cause) => argumentError(`EventMemoryArena.${op}`, cause)
		})
		return yield* nativeOperationWith(
			`EventMemoryArena.${op}`,
			(cb) => dbNative.runtimeEventMemoryArena(runtime, op, inputs, cb),
			dbNative.runtimeBytesTake,
			encodedEvent
		)
	})
}
const EventMemoryArena = Object.freeze({
	fromBytes,
	toBytes,
	isArena,
	describe,
	inspect,
	known: knowledge("known"),
	possible: knowledge("possible"),
	reach,
	safe
})

export { compileMemory, EventMemoryArena }
