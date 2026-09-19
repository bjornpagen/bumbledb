import { Effect, Result } from "effect"
import { dbNative } from "#db-native.ts"
import { AuthoringError, SdkInvariantError } from "#errors.ts"
import { type Event as EventValue, encodedEvent, eventBytes, eventValue, isEvent } from "#event-value.ts"
import { nativeOperationWith, runtimeHandle } from "#runtime.ts"
import { argumentError, type DbError } from "#runtime-errors.ts"
import { mass, probability } from "#source-operation.ts"
import { bytesValue } from "#values.ts"

/** Immutable portable Event value. Pure transport parsing does not certify a
 * graph or law; native construction/algebra always re-establishes those facts. */
type Event = EventValue

function fromBytes(input: Uint8Array): Result.Result<Event, DbError> {
	return Result.try({ try: () => encodedEvent(input), catch: (cause) => argumentError("Event.fromBytes", cause) })
}
function toBytes(value: Event): Uint8Array {
	return eventBytes(value)
}

const execute = Effect.fn("Event.execute")(function* (
	op: string,
	inputs: readonly Event[],
	argument = 0n,
	identity?: Uint8Array
) {
	const runtime = yield* runtimeHandle()
	const bytes = yield* Effect.try({
		try: () => {
			if (typeof argument !== "bigint" || argument < 0n || argument > 0xffffffffffffffffn)
				throw new AuthoringError({ message: `Event.${op}: expected an unsigned 64-bit argument` })
			if (identity !== undefined) {
				const name = bytesValue("Event source identity", identity, 32)
				if (name.length !== 32) throw new AuthoringError({ message: "Event source identity: expected 32 bytes" })
				return [name]
			}
			return inputs.map((value) => eventBytes(eventValue(`Event.${op}`, value)))
		},
		catch: (cause) => argumentError(`Event.${op}`, cause)
	})
	return yield* nativeOperationWith(
		`Event.${op}`,
		(callback) => dbNative.runtimeEvent(runtime, op, bytes, argument, callback),
		dbNative.runtimeRowsTake,
		(rows) => {
			if (rows.length !== 1 || rows[0]?.length !== 1)
				throw new SdkInvariantError({ message: "Event operation: malformed result" })
			return rows[0][0]
		}
	)
})
function eventResult(op: string, inputs: readonly Event[], argument = 0n, identity?: Uint8Array) {
	return Effect.map(execute(op, inputs, argument, identity), encodedEvent)
}
function boolResult(op: string, inputs: readonly Event[], argument = 0n) {
	return Effect.map(execute(op, inputs, argument), (value) => {
		if (typeof value !== "boolean") throw new SdkInvariantError({ message: "Event operation: expected Boolean result" })
		return value
	})
}
function integerResult(op: string, inputs: readonly Event[]) {
	return Effect.map(execute(op, inputs), (value) => {
		if (typeof value !== "bigint") throw new SdkInvariantError({ message: "Event operation: expected integer result" })
		return value
	})
}
/** Recheck a portable value using the shared canonical decoder. */
function validate(value: Event) {
	return eventResult("validate", [value])
}
/** Full unmeasured named space; the caller supplies identity, never a random source. */
function space(identity: Uint8Array, coordinates: bigint) {
	return eventResult("space", [], coordinates, identity)
}
function coordinate(context: Event, index: bigint) {
	return eventResult("coordinate", [context], index)
}
function full(context: Event) {
	return eventResult("full", [context])
}
function empty(context: Event) {
	return eventResult("empty", [context])
}
function complement(value: Event) {
	return eventResult("complement", [value])
}
/** Full support of a structural restriction. The resulting context is unmeasured. */
function restrict(context: Event, support: Event) {
	return eventResult("restrict", [context, support])
}
/** Bit ((a << 1) | b) of mask selects the result; mask is in 0..15. */
function apply(mask: bigint, left: Event, right: Event) {
	return eventResult("apply", [left, right], mask)
}
function and(left: Event, right: Event) {
	return apply(8n, left, right)
}
function or(left: Event, right: Event) {
	return apply(14n, left, right)
}
function xor(left: Event, right: Event) {
	return apply(6n, left, right)
}
function difference(left: Event, right: Event) {
	return apply(4n, left, right)
}
function implies(left: Event, right: Event) {
	return apply(11n, left, right)
}
function equivalence(left: Event, right: Event) {
	return apply(9n, left, right)
}
function ite(condition: Event, high: Event, low: Event) {
	return eventResult("ite", [condition, high, low])
}
/** Exact world count; an infinite parameter region refuses. */
function count(value: Event) {
	return integerResult("count", [value])
}
/** Finite cells of the sealed Event presentation, never probability weights. */
function atomCount(value: Event) {
	return integerResult("atomCount", [value])
}
function signature(left: Event, right: Event) {
	return integerResult("signature", [left, right])
}
function isEmpty(value: Event) {
	return boolResult("isEmpty", [value])
}
function isFull(value: Event) {
	return boolResult("isFull", [value])
}
function includes(left: Event, right: Event) {
	return boolResult("subset", [right, left])
}
function subset(left: Event, right: Event) {
	return boolResult("subset", [left, right])
}
function equal(left: Event, right: Event) {
	return boolResult("equal", [left, right])
}
function disjoint(left: Event, right: Event) {
	return boolResult("disjoint", [left, right])
}
function contains(value: Event, world: bigint) {
	return boolResult("contains", [value], world)
}

const Event = Object.freeze({
	fromBytes,
	toBytes,
	isEvent,
	validate,
	space,
	coordinate,
	full,
	empty,
	complement,
	restrict,
	apply,
	and,
	or,
	xor,
	difference,
	implies,
	equivalence,
	ite,
	count,
	atomCount,
	signature,
	isEmpty,
	isFull,
	includes,
	subset,
	equal,
	disjoint,
	contains,
	mass,
	probability
})

export { Event }
