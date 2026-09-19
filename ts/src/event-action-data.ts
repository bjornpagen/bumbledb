/** Plain BEAC recipes. All mathematical validation and solving is native. */
import { AuthoringError } from "#errors.ts"
import { type EventDescriptor, encodedDescriptor } from "#event-descriptor.ts"
import { DescriptorBudget, fibreDescription } from "#event-descriptor-data.ts"
import { type Event, encodedEvent, eventByteLength, eventBytes, eventValue } from "#event-value.ts"
import { bytesValue, recordValue } from "#values.ts"

type Fibre<E> = ReturnType<typeof fibreDescription<E>>
interface Arena<E> {
	readonly actions: Fibre<E>
	readonly transition: { readonly product: Fibre<E>; readonly region: E }
}
type Description<E> =
	| { readonly kind: "arena"; readonly arena: Arena<E> }
	| { readonly kind: "reach"; readonly arena: Arena<E>; readonly goal: E; readonly policy: E | null }
	| { readonly kind: "safe"; readonly arena: Arena<E>; readonly invariant: E; readonly policy: E | null }
export type EventActionArenaDescription = Arena<Event>
/** Null policy requests every permitted choice. A supplied policy is a region
 * on this arena's named state/action product and must cover all required states. */
export type EventActionDescription = Description<Event>
export type EventActionWire = Description<Uint8Array>
interface ArenaInspection {
	readonly states: Event
	readonly choices: Event
	readonly outcomes: Event
	readonly actions: EventDescriptor
	readonly transition: EventDescriptor
}
export type EventActionInspection = ArenaInspection &
	(
		| { readonly kind: "arena" }
		| {
				readonly kind: "reach"
				readonly goal: Event
				readonly winning: Event
				readonly policy: EventDescriptor
				readonly ranks: readonly Event[]
		  }
		| { readonly kind: "safe"; readonly invariant: Event; readonly winning: Event; readonly policy: EventDescriptor }
	)
function kind(input: unknown) {
	if (typeof input !== "object" || input === null)
		throw new AuthoringError({ message: "Event action: expected an object" })
	return recordValue("Event action", input, Object.keys(input)).kind
}
function description<E>(input: unknown, event: (input: unknown, budget: DescriptorBudget) => E): Description<E> {
	const budget = new DescriptorBudget()
	const read = (input: unknown) => event(input, budget)
	const tag = kind(input)
	if (tag !== "arena" && tag !== "reach" && tag !== "safe")
		throw new AuthoringError({ message: "Event action: unknown kind" })
	budget.item()
	const data = recordValue(
		"Event action",
		input,
		tag === "arena" ? ["kind", "arena"] : ["kind", "arena", tag === "reach" ? "goal" : "invariant", "policy"]
	)
	budget.item()
	const raw = recordValue("Event action arena", data.arena, ["actions", "transition"])
	const actions = fibreDescription(raw.actions, budget, read)
	budget.item()
	const step = recordValue("Event action transition", raw.transition, ["product", "region"])
	const arena = Object.freeze({
		actions,
		transition: Object.freeze({ product: fibreDescription(step.product, budget, read), region: read(step.region) })
	})
	if (tag === "arena") return Object.freeze({ kind: tag, arena })
	const objective = read(tag === "reach" ? data.goal : data.invariant)
	const policy = data.policy === null ? null : read(data.policy)
	return tag === "reach"
		? Object.freeze({ kind: tag, arena, goal: objective, policy })
		: Object.freeze({ kind: tag, arena, invariant: objective, policy })
}
export function encodeAction(input: EventActionDescription): EventActionWire {
	return description(input, (value, budget) => {
		budget.item()
		const event = eventValue("Event action", value)
		budget.length(eventByteLength(event))
		return eventBytes(event)
	})
}
export function decodeAction(input: unknown): EventActionDescription {
	return description(input, (value, budget) => {
		budget.item()
		const event = encodedEvent(bytesValue("Event action result", value, budget.bytes))
		budget.length(eventByteLength(event))
		return event
	})
}
export function decodeActionInspection(input: unknown): EventActionInspection {
	const budget = new DescriptorBudget()
	budget.item()
	const bytes = (input: unknown) => {
		budget.item()
		const value = bytesValue("Event action inspection", input, budget.bytes)
		budget.length(value.length)
		return value
	}
	const event = (input: unknown) => encodedEvent(bytes(input))
	const descriptor = (input: unknown) => encodedDescriptor(bytes(input))
	const tag = kind(input)
	if (tag !== "arena" && tag !== "reach" && tag !== "safe")
		throw new AuthoringError({ message: "Event action inspection: unknown kind" })
	const fields = ["kind", "states", "choices", "outcomes", "actions", "transition"]
	if (tag === "reach") fields.push("goal", "winning", "policy", "ranks")
	if (tag === "safe") fields.push("invariant", "winning", "policy")
	const data = recordValue("Event action inspection", input, fields)
	const common = {
		states: event(data.states),
		choices: event(data.choices),
		outcomes: event(data.outcomes),
		actions: descriptor(data.actions),
		transition: descriptor(data.transition)
	}
	if (tag === "arena") return Object.freeze({ kind: tag, ...common })
	if (tag === "safe")
		return Object.freeze({
			kind: tag,
			...common,
			invariant: event(data.invariant),
			winning: event(data.winning),
			policy: descriptor(data.policy)
		})
	const goal = event(data.goal)
	const winning = event(data.winning)
	const policy = descriptor(data.policy)
	budget.item()
	return Object.freeze({ kind: tag, ...common, goal, winning, policy, ranks: budget.list(data.ranks, event) })
}
