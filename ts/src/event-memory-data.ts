/** Plain indexed recipes. Native constructors supply all mathematical checks. */
import { AuthoringError } from "#errors.ts"
import { DescriptorBudget, type EventFibreDescription, fibreDescription } from "#event-descriptor-data.ts"
import { type Event, encodedEvent, eventByteLength, eventBytes, eventValue } from "#event-value.ts"
import { recordValue } from "#values.ts"

interface MemoryData<E, F> {
	readonly source: E
	readonly given: E
	readonly observations: readonly E[]
	readonly actions: readonly { readonly product: F; readonly region: E }[]
}
type FibreWire = ReturnType<typeof fibreDescription<Uint8Array>>
export type EventMemoryDescription = MemoryData<Event, EventFibreDescription>
export type EventMemoryWire = MemoryData<Uint8Array, FibreWire>
export interface EventMemoryTransition {
	readonly action: number
	readonly observation: number
	readonly target: number
}
interface Inspection<E> {
	readonly initial: readonly (number | null)[]
	readonly states: readonly {
		readonly possible: E
		readonly observation: number
		readonly transitions: readonly EventMemoryTransition[]
	}[]
}
/** Indices belong to this memory's authored action/observation rosters. */
export type EventMemoryInspection = Inspection<Event>
export type EventMemoryInspectionWire = Inspection<Uint8Array>

function description<E>(input: unknown, event: (input: unknown, budget: DescriptorBudget) => E) {
	const budget = new DescriptorBudget()
	const read = (input: unknown) => event(input, budget)
	budget.item()
	const data = recordValue("Event memory", input, ["source", "given", "observations", "actions"])
	return Object.freeze({
		source: read(data.source),
		given: read(data.given),
		observations: budget.list(data.observations, read),
		actions: budget.list(data.actions, (input) => {
			budget.item()
			const action = recordValue("Event memory action", input, ["product", "region"])
			return Object.freeze({ product: fibreDescription(action.product, budget, read), region: read(action.region) })
		})
	})
}
export function encodeMemory(input: EventMemoryDescription): EventMemoryWire {
	return description(input, (value, budget) => {
		budget.item()
		const event = eventValue("Event memory", value)
		budget.length(eventByteLength(event))
		return eventBytes(event)
	})
}
export function decodeMemory(input: EventMemoryWire): EventMemoryDescription {
	return description(input, (value, budget) => {
		budget.item()
		const event = encodedEvent(value)
		budget.length(eventByteLength(event))
		return event
	})
}
export function decodeMemoryInspection(input: EventMemoryInspectionWire): EventMemoryInspection {
	const budget = new DescriptorBudget()
	const index = (input: unknown) => {
		if (typeof input !== "number" || !Number.isSafeInteger(input) || input < 0)
			throw new AuthoringError({ message: "Event memory: invalid roster index" })
		return input
	}
	budget.item()
	const data = recordValue("Event memory inspection", input, ["initial", "states"])
	return Object.freeze({
		initial: budget.list(data.initial, (value) => {
			budget.item()
			budget.length(8)
			return value === null ? null : index(value)
		}),
		states: budget.list(data.states, (input) => {
			budget.item()
			const state = recordValue("Event memory state", input, ["possible", "observation", "transitions"])
			const possible = encodedEvent(state.possible)
			budget.item()
			budget.length(eventByteLength(possible))
			return Object.freeze({
				possible,
				observation: index(state.observation),
				transitions: budget.list(state.transitions, (input) => {
					budget.item()
					budget.length(24)
					const edge = recordValue("Event memory transition", input, ["action", "observation", "target"])
					return Object.freeze({
						action: index(edge.action),
						observation: index(edge.observation),
						target: index(edge.target)
					})
				})
			})
		})
	})
}
