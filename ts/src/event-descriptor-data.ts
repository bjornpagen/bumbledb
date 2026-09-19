/** Plain structural data, with no JavaScript mathematical evaluator. */
import { AuthoringError } from "#errors.ts"
import { type EventDescriptor, encodedDescriptor } from "#event-descriptor.ts"
import { type Event, encodedEvent, eventByteLength, eventBytes, eventValue } from "#event-value.ts"
import { arrayValue, bytesValue, recordValue } from "#values.ts"

interface MapData<E> {
	readonly source: E
	readonly target: E
	readonly readouts: readonly E[]
}
interface FibreData<E> {
	readonly identity: Uint8Array
	readonly left: MapData<E>
	readonly right: MapData<E>
	readonly reversed: boolean
}
type DescriptorData<E> =
	| { readonly kind: "map" | "surjective"; readonly map: MapData<E> }
	| { readonly kind: "faces"; readonly identity: Uint8Array; readonly environments: readonly MapData<E>[] }
	| { readonly kind: "fibre"; readonly product: FibreData<E> }
	| { readonly kind: "relation"; readonly product: FibreData<E>; readonly region: E }
	| {
			readonly kind: "composition"
			readonly identity: Uint8Array
			readonly st: FibreData<E>
			readonly tu: FibreData<E>
			readonly su: FibreData<E>
	  }
	| { readonly kind: "square"; readonly product: FibreData<E>; readonly left: MapData<E>; readonly right: MapData<E> }

/** Source and target must be full space markers. Readouts are Events on source,
 * in target semantic-coordinate order. No map or onto claim is trusted. */
export type EventMapDescription = MapData<Event>
/** Stored left/right maps retain original coordinate order; reversed exchanges
 * endpoint roles without renaming or permuting the stored coordinates. */
export type EventFibreDescription = FibreData<Event>
export type EventDescriptorDescription = DescriptorData<Event>
export type EventDescriptorWire = DescriptorData<Uint8Array>

type InspectionData<E, D> =
	| ({ readonly kind: "map" | "surjective" } & MapData<E>)
	| {
			readonly kind: "faces"
			readonly space: E
			readonly projections: readonly D[]
			readonly environments: readonly D[]
	  }
	| {
			readonly kind: "fibre"
			readonly space: E
			readonly left: D
			readonly right: D
			readonly leftEnvironment: D
			readonly rightEnvironment: D
	  }
	| { readonly kind: "relation"; readonly region: E; readonly product: D; readonly input: E; readonly output: E }
	| {
			readonly kind: "composition"
			readonly workspace: E
			readonly products: readonly [D, D, D]
			readonly projections: readonly [D, D, D]
	  }
	| { readonly kind: "square"; readonly product: D; readonly joint: D; readonly left: D; readonly right: D }

/** Derived descriptors are portable owned values. Composition tuples use ST,
 * TU, SU order. All projections and environment maps are certified onto. */
export type EventDescriptorInspection = InspectionData<Event, EventDescriptor>
export type EventDescriptorInspectionWire = InspectionData<Uint8Array, Uint8Array>

function invalid(message: string): never {
	throw new AuthoringError({ message: `Event descriptor: ${message}` })
}
function kind(input: unknown): unknown {
	if (typeof input !== "object" || input === null) return invalid("expected a plain description")
	return recordValue("Event descriptor", input, Object.keys(input)).kind
}
class Budget {
	bytes = 16 * 1024 * 1024
	items = 4096
	item() {
		if (--this.items < 0) invalid("exceeds 4096 items")
	}
	length(length: number) {
		if (length > this.bytes) invalid("exceeds 16 MiB")
		this.bytes -= length
	}
	identity(input: unknown): Uint8Array {
		const bytes = bytesValue("Event descriptor identity", input, 32)
		if (bytes.length !== 32) invalid("identity needs 32 bytes")
		this.length(bytes.length)
		return bytes
	}
	list<T>(input: unknown, parse: (input: unknown) => T): readonly T[] {
		if (!Array.isArray(input) || input.length > this.items) invalid("expected a bounded array")
		return arrayValue("Event descriptor list", input, (_, value) => parse(value))
	}
}

function description<E>(input: unknown, budget: Budget, event: (input: unknown) => E): DescriptorData<E> {
	const map = (input: unknown): MapData<E> => {
		budget.item()
		const value = recordValue("Event map", input, ["source", "target", "readouts"])
		return Object.freeze({
			source: event(value.source),
			target: event(value.target),
			readouts: budget.list(value.readouts, event)
		})
	}
	const fibre = (input: unknown): FibreData<E> => {
		budget.item()
		const value = recordValue("Event fibre", input, ["identity", "left", "right", "reversed"])
		if (typeof value.reversed !== "boolean") invalid("reversed must be Boolean")
		return Object.freeze({
			identity: budget.identity(value.identity),
			left: map(value.left),
			right: map(value.right),
			reversed: value.reversed
		})
	}
	budget.item()
	const tag = kind(input)
	switch (tag) {
		case "map":
		case "surjective": {
			const value = recordValue("Event descriptor", input, ["kind", "map"])
			return Object.freeze({ kind: tag, map: map(value.map) })
		}
		case "faces": {
			const value = recordValue("Event descriptor", input, ["kind", "identity", "environments"])
			return Object.freeze({
				kind: tag,
				identity: budget.identity(value.identity),
				environments: budget.list(value.environments, map)
			})
		}
		case "fibre": {
			const value = recordValue("Event descriptor", input, ["kind", "product"])
			return Object.freeze({ kind: tag, product: fibre(value.product) })
		}
		case "relation": {
			const value = recordValue("Event descriptor", input, ["kind", "product", "region"])
			return Object.freeze({ kind: tag, product: fibre(value.product), region: event(value.region) })
		}
		case "composition": {
			const value = recordValue("Event descriptor", input, ["kind", "identity", "st", "tu", "su"])
			return Object.freeze({
				kind: tag,
				identity: budget.identity(value.identity),
				st: fibre(value.st),
				tu: fibre(value.tu),
				su: fibre(value.su)
			})
		}
		case "square": {
			const value = recordValue("Event descriptor", input, ["kind", "product", "left", "right"])
			return Object.freeze({ kind: tag, product: fibre(value.product), left: map(value.left), right: map(value.right) })
		}
		default:
			return invalid("unknown kind")
	}
}

export function encodeDescription(input: EventDescriptorDescription): EventDescriptorWire {
	const budget = new Budget()
	return description(input, budget, (value) => {
		budget.item()
		const event = eventValue("Event descriptor", value)
		budget.length(eventByteLength(event))
		return eventBytes(event)
	})
}
export function decodeDescription(input: EventDescriptorWire): EventDescriptorDescription {
	const budget = new Budget()
	return description(input, budget, (value) => {
		budget.item()
		const event = encodedEvent(value)
		budget.length(eventByteLength(event))
		return event
	})
}

export function decodeInspection(input: EventDescriptorInspectionWire): EventDescriptorInspection {
	const budget = new Budget()
	const bytes = (value: unknown) => {
		budget.item()
		const owned = bytesValue("Event inspection", value, budget.bytes)
		budget.length(owned.length)
		return owned
	}
	const event = (value: unknown) => encodedEvent(bytes(value))
	const descriptor = (value: unknown) => encodedDescriptor(bytes(value))
	const list = <T>(value: unknown, parse: (input: unknown) => T) => {
		budget.item()
		return budget.list(value, parse)
	}
	const triple = (value: unknown): readonly [EventDescriptor, EventDescriptor, EventDescriptor] => {
		const values = list(value, descriptor)
		const [a, b, c] = values
		if (values.length !== 3 || a === undefined || b === undefined || c === undefined)
			invalid("composition requires three roles")
		return Object.freeze([a, b, c])
	}
	budget.item()
	const tag = kind(input)
	switch (tag) {
		case "map":
		case "surjective": {
			const value = recordValue("Event inspection", input, ["kind", "source", "target", "readouts"])
			return Object.freeze({
				kind: tag,
				source: event(value.source),
				target: event(value.target),
				readouts: list(value.readouts, event)
			})
		}
		case "faces": {
			const value = recordValue("Event inspection", input, ["kind", "space", "projections", "environments"])
			return Object.freeze({
				kind: tag,
				space: event(value.space),
				projections: list(value.projections, descriptor),
				environments: list(value.environments, descriptor)
			})
		}
		case "fibre": {
			const value = recordValue("Event inspection", input, [
				"kind",
				"space",
				"left",
				"right",
				"leftEnvironment",
				"rightEnvironment"
			])
			return Object.freeze({
				kind: tag,
				space: event(value.space),
				left: descriptor(value.left),
				right: descriptor(value.right),
				leftEnvironment: descriptor(value.leftEnvironment),
				rightEnvironment: descriptor(value.rightEnvironment)
			})
		}
		case "relation": {
			const value = recordValue("Event inspection", input, ["kind", "region", "product", "input", "output"])
			return Object.freeze({
				kind: tag,
				region: event(value.region),
				product: descriptor(value.product),
				input: event(value.input),
				output: event(value.output)
			})
		}
		case "composition": {
			const value = recordValue("Event inspection", input, ["kind", "workspace", "products", "projections"])
			return Object.freeze({
				kind: tag,
				workspace: event(value.workspace),
				products: triple(value.products),
				projections: triple(value.projections)
			})
		}
		case "square": {
			const value = recordValue("Event inspection", input, ["kind", "product", "joint", "left", "right"])
			return Object.freeze({
				kind: tag,
				product: descriptor(value.product),
				joint: descriptor(value.joint),
				left: descriptor(value.left),
				right: descriptor(value.right)
			})
		}
		default:
			return invalid("unknown inspection kind")
	}
}
