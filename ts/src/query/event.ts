/** Pure query authoring. These owned expressions retain every operand and
 * lower to native Event programs; they perform no Event mathematics in JS. */
import { AuthoringError } from "#errors.ts"
import { descriptorLength, EventDescriptor, encodedDescriptor } from "#event-descriptor.ts"
import { type Event, encodedEvent, eventByteLength, eventBytes, eventValue } from "#event-value.ts"
import type { EventField, I64Field, U64Field } from "#fields.ts"
import type { EventExprIr, EventTestIr, FindTermIr, RelationExprIr } from "#native.ts"
import { eventTree, testTree } from "#query/event-tree.ts"
import { type AnyVar, isTerm, term } from "#query/scope.ts"

type EventVar = AnyVar & { readonly field: EventField }
const expressionTag: unique symbol = Symbol("bumbledb.EventExpression")
interface EventExpr {
	readonly kind: "event"
	readonly [expressionTag]: "event"
	readonly node: EventNode
}
interface EventTest {
	readonly kind: "test"
	readonly [expressionTag]: "test"
	readonly node: TestNode
}
type IntegerVar = AnyVar & { readonly field: I64Field | U64Field }
interface ExpectationExpr {
	readonly kind: "expectation"
	readonly [expressionTag]: "expectation"
	readonly node: { readonly value: IntegerVar; readonly when: EventVar; readonly given: EventVar }
}
interface ProbabilityExpr {
	readonly kind: "probability"
	readonly [expressionTag]: "probability"
	readonly node: { readonly event: EventNode; readonly given: EventNode }
}
interface RelationExpr {
	readonly kind: "relation"
	readonly [expressionTag]: "relation"
	readonly node: RelationNode
}
type EventOperand = EventVar | EventExpr
type EventFind = EventExpr | EventTest | ProbabilityExpr | ExpectationExpr
type EventNode = EventExprIr<EventVar, EventDescriptor, Event>
type TestNode = EventTestIr<EventVar, EventDescriptor, Event>
type RelationNode = RelationExprIr<EventVar, EventDescriptor, Event>
interface Extent {
	readonly nodes: number
	readonly depth: number
	readonly bytes: number
}
interface Owned<N> {
	readonly node: N
	readonly extent: Extent
}
const events = new WeakMap<EventExpr, Owned<EventNode>>()
const tests = new WeakMap<EventTest, Owned<TestNode>>()
const expectations = new WeakSet<ExpectationExpr>()
const probabilities = new WeakMap<ProbabilityExpr, Owned<ProbabilityExpr["node"]>>()
const relations = new WeakMap<RelationExpr, Owned<RelationNode>>()

function refused(message: string): never {
	throw new AuthoringError({ message })
}
function eventVar(input: AnyVar): EventVar {
	if (!isTerm(input) || input[term] !== "var" || input.field.kind !== "event")
		return refused("Event expression: expected an Event variable")
	return input as EventVar
}
function extent(
	children: readonly Extent[],
	descriptors: readonly EventDescriptor[] = [],
	test = false,
	scopeBytes = 0
): Extent {
	const nodes = (test ? 0 : 1) + children.reduce((sum, child) => sum + child.nodes, 0)
	const depth = (test ? 0 : 1) + children.reduce((maximum, child) => Math.max(maximum, child.depth), 0)
	const bytes =
		scopeBytes +
		descriptors.reduce((sum, descriptor) => sum + descriptorLength(descriptor), 0) +
		children.reduce((sum, child) => sum + child.bytes, 0)
	if (nodes > 4096 || depth > 128 || bytes > 16 * 1024 * 1024)
		return refused("Event expression exceeds shape or imported byte budget")
	return Object.freeze({ nodes, depth, bytes })
}
function ownEvent(node: EventNode, size: Extent): EventExpr {
	const value: EventExpr = Object.freeze({
		kind: "event",
		[expressionTag]: "event" as const,
		node: Object.freeze(node)
	})
	events.set(value, { node: value.node, extent: size })
	return value
}
function ownTest(node: TestNode, size: Extent): EventTest {
	const value: EventTest = Object.freeze({ kind: "test", [expressionTag]: "test" as const, node: Object.freeze(node) })
	tests.set(value, { node: value.node, extent: size })
	return value
}
function ownRelation(node: RelationNode, size: Extent): RelationExpr {
	const value: RelationExpr = Object.freeze({
		kind: "relation",
		[expressionTag]: "relation" as const,
		node: Object.freeze(node)
	})
	relations.set(value, { node: value.node, extent: size })
	return value
}
function isEventFind(input: unknown): input is EventFind {
	return (
		typeof input === "object" &&
		input !== null &&
		(expectations.has(input as ExpectationExpr) ||
			events.has(input as EventExpr) ||
			tests.has(input as EventTest) ||
			probabilities.has(input as ProbabilityExpr))
	)
}
function ownProbability(node: ProbabilityExpr["node"], size: Extent): ProbabilityExpr {
	const value: ProbabilityExpr = Object.freeze({
		kind: "probability",
		[expressionTag]: "probability" as const,
		node: Object.freeze(node)
	})
	probabilities.set(value, { node: value.node, extent: size })
	return value
}
/** Admit a complete evidence-relative payoff roster before exact contraction. */
function expectation(value: IntegerVar, when: EventVar, given: EventVar): ExpectationExpr {
	if (!isTerm(value) || value[term] !== "var" || (value.field.kind !== "i64" && value.field.kind !== "u64"))
		return refused("Expectation requires an exact integer variable")
	const result: ExpectationExpr = Object.freeze({
		kind: "expectation",
		[expressionTag]: "expectation" as const,
		node: Object.freeze({ value, when: eventVar(when), given: eventVar(given) })
	})
	expectations.add(result)
	return result
}
/** Observe two Event programs on the same source, retaining the exact law and evidence. */
function probability(event: EventOperand, given: EventOperand): ProbabilityExpr {
	const a = eventData(event)
	const b = eventData(given)
	return ownProbability({ event: a.node, given: b.node }, extent([a.extent, b.extent], [], true))
}
/** The query snapshot copies variable references and their uses in one graph.
 * Re-enroll only copies made from a checked expression, retaining its extent. */
function snapshotEventExpression(source: object, snapshot: object): void {
	if (expectations.has(source as ExpectationExpr)) expectations.add(snapshot as ExpectationExpr)
	const probability = probabilities.get(source as ProbabilityExpr)
	if (probability !== undefined) {
		const value = snapshot as ProbabilityExpr
		probabilities.set(value, { node: value.node, extent: probability.extent })
	}
	const event = events.get(source as EventExpr)
	if (event !== undefined) {
		const value = snapshot as EventExpr
		events.set(value, { node: value.node, extent: event.extent })
	}
	const test = tests.get(source as EventTest)
	if (test !== undefined) {
		const value = snapshot as EventTest
		tests.set(value, { node: value.node, extent: test.extent })
	}
}
function eventData(input: EventOperand): Owned<EventNode> {
	const owned = events.get(input as EventExpr)
	if (owned !== undefined) return owned
	return { node: Object.freeze({ kind: "var", var: eventVar(input as AnyVar) }), extent: extent([]) }
}
function relationData(input: RelationExpr): Owned<RelationNode> {
	return relations.get(input) ?? refused("Relation expression: expected an owned relation program")
}
function truthBits(bits: number): number {
	if (!Number.isInteger(bits) || bits < 0 || bits > 15) return refused("Event truth function must have four bits")
	return bits
}
function threshold(value: bigint): bigint {
	if (typeof value !== "bigint" || value < 0n || value > 0xffffffffffffffffn)
		return refused("Event cardinality: expected an unsigned 64-bit bound")
	return value
}
function variable(input: EventVar): EventExpr {
	return ownEvent({ kind: "var", var: eventVar(input) }, extent([]))
}
function empty(input: EventVar): EventExpr {
	return ownEvent({ kind: "empty", var: eventVar(input) }, extent([]))
}
function full(input: EventVar): EventExpr {
	return ownEvent({ kind: "full", var: eventVar(input) }, extent([]))
}
// Negative handles exist only during authoring. Closing a binder translates its
// own handle at the current lexical depth; outer handles remain pending. An
// escaped handle is rejected at lowering, so reuse cannot capture a new binder.
let nextPredicate = -1
function fixedPoint(op: "least" | "greatest", scope: Event, body: (predicate: EventExpr) => EventOperand): EventExpr {
	const context = eventValue("Fixed-point scope", scope)
	if (!Number.isSafeInteger(nextPredicate)) return refused("Predicate handle capacity exceeded")
	if (typeof body !== "function") return refused("Fixed point requires an authoring callback")
	const handle = nextPredicate--
	const predicate = ownEvent({ kind: "bound", depth: handle }, extent([]))
	const child = eventData(body(predicate))
	const expr = eventTree(child.node, {
		variable: (value: EventVar) => value,
		descriptor: (value: EventDescriptor) => value,
		scope: (value: Event) => value,
		bound: (value: number, nesting: number) => (value === handle ? nesting : value)
	})
	return ownEvent(
		{ kind: "fixed", op, scope: context, expr },
		extent([child.extent], [], false, eventByteLength(context))
	)
}
function closedPredicate(value: number, nesting: number): number {
	if (!Number.isInteger(value) || value < 0 || value >= nesting)
		return refused("Event predicate escaped its fixed-point binder")
	return value
}
function complement(input: EventOperand): EventExpr {
	const child = eventData(input)
	return ownEvent({ kind: "not", expr: child.node }, extent([child.extent]))
}
function apply(bits: number, left: EventOperand, right: EventOperand): EventExpr {
	const a = eventData(left)
	const b = eventData(right)
	return ownEvent({ kind: "apply", bits: truthBits(bits), left: a.node, right: b.node }, extent([a.extent, b.extent]))
}
function ite(condition: EventOperand, high: EventOperand, low: EventOperand): EventExpr {
	const c = eventData(condition)
	const h = eventData(high)
	const l = eventData(low)
	return ownEvent({ kind: "ite", condition: c.node, high: h.node, low: l.node }, extent([c.extent, h.extent, l.extent]))
}
function cardinality(
	minimum: bigint,
	maximum: bigint,
	...inputs: readonly [EventOperand, ...EventOperand[]]
): EventExpr {
	if (inputs.length === 0 || inputs.length >= 4096)
		return refused("Event cardinality needs 1..4095 context-bearing operands")
	const children = inputs.map(eventData)
	return ownEvent(
		{
			kind: "cardinality",
			minimum: threshold(minimum),
			maximum: threshold(maximum),
			events: Object.freeze(children.map((child) => child.node))
		},
		extent(children.map((child) => child.extent))
	)
}
type MapOp = Extract<EventNode, { kind: "map" }>["op"]
function map(op: MapOp, input: EventOperand, descriptor: EventDescriptor): EventExpr {
	const child = eventData(input)
	return ownEvent({ kind: "map", op, descriptor, expr: child.node }, extent([child.extent], [descriptor]))
}
function relationView(op: "region" | "domain" | "range", input: RelationExpr): EventExpr {
	const child = relationData(input)
	return ownEvent({ kind: "relation", op, relation: child.node }, extent([child.extent]))
}
function modal(op: "may" | "all" | "must" | "post", relation: RelationExpr, input: EventOperand): EventExpr {
	const r = relationData(relation)
	const e = eventData(input)
	return ownEvent({ kind: "modal", op, relation: r.node, expr: e.node }, extent([r.extent, e.extent]))
}
function unaryTest(kind: "isEmpty" | "isFull", input: EventOperand): EventTest {
	const child = eventData(input)
	return ownTest({ kind, expr: child.node }, extent([child.extent], [], true))
}
function binaryTest(
	kind: "subset" | "equal" | "disjoint" | "covers",
	left: EventOperand,
	right: EventOperand
): EventTest {
	const a = eventData(left)
	const b = eventData(right)
	return ownTest({ kind, left: a.node, right: b.node }, extent([a.extent, b.extent], [], true))
}
function bindRelation(kind: "bind" | "test", input: EventOperand, descriptor: EventDescriptor): RelationExpr {
	const child = eventData(input)
	return ownRelation({ kind, descriptor, expr: child.node }, extent([child.extent], [descriptor]))
}
function identity(descriptor: EventDescriptor): RelationExpr {
	return ownRelation({ kind: "identity", descriptor }, extent([], [descriptor]))
}
function unaryRelation(kind: "not" | "converse", input: RelationExpr): RelationExpr {
	const child = relationData(input)
	return ownRelation({ kind, relation: child.node }, extent([child.extent]))
}
function relationApply(bits: number, left: RelationExpr, right: RelationExpr): RelationExpr {
	const a = relationData(left)
	const b = relationData(right)
	return ownRelation(
		{ kind: "apply", bits: truthBits(bits), left: a.node, right: b.node },
		extent([a.extent, b.extent])
	)
}
function product(
	op: "compose" | "leftResidual" | "rightResidual",
	left: RelationExpr,
	right: RelationExpr,
	descriptor: EventDescriptor
): RelationExpr {
	const a = relationData(left)
	const b = relationData(right)
	return ownRelation(
		{ kind: "product", op, descriptor, left: a.node, right: b.node },
		extent([a.extent, b.extent], [descriptor])
	)
}
function star(input: RelationExpr, descriptor: EventDescriptor): RelationExpr {
	const child = relationData(input)
	return ownRelation({ kind: "star", descriptor, relation: child.node }, extent([child.extent], [descriptor]))
}

const EventExpr = Object.freeze({
	/** Build a finite monotone least fixed point; the callback runs only during authoring. */
	least: (scope: Event, body: (predicate: EventExpr) => EventOperand) => fixedPoint("least", scope, body),
	/** Build a finite monotone greatest fixed point over the original full context. */
	greatest: (scope: Event, body: (predicate: EventExpr) => EventOperand) => fixedPoint("greatest", scope, body),
	variable,
	empty,
	full,
	complement,
	apply,
	ite,
	cardinality,
	and: (a: EventOperand, b: EventOperand) => apply(8, a, b),
	or: (a: EventOperand, b: EventOperand) => apply(14, a, b),
	xor: (a: EventOperand, b: EventOperand) => apply(6, a, b),
	difference: (a: EventOperand, b: EventOperand) => apply(4, a, b),
	implies: (a: EventOperand, b: EventOperand) => apply(11, a, b),
	equivalence: (a: EventOperand, b: EventOperand) => apply(9, a, b),
	atLeast: (minimum: bigint, ...inputs: readonly [EventOperand, ...EventOperand[]]) =>
		cardinality(minimum, 0xffffffffffffffffn, ...inputs),
	atMost: (maximum: bigint, ...inputs: readonly [EventOperand, ...EventOperand[]]) =>
		cardinality(0n, maximum, ...inputs),
	exactly: (count: bigint, ...inputs: readonly [EventOperand, ...EventOperand[]]) =>
		cardinality(count, count, ...inputs),
	pullback: (input: EventOperand, descriptor: EventDescriptor) => map("pullback", input, descriptor),
	image: (input: EventOperand, descriptor: EventDescriptor) => map("image", input, descriptor),
	universalImage: (input: EventOperand, descriptor: EventDescriptor) => map("universalImage", input, descriptor),
	nonvacuousImage: (input: EventOperand, descriptor: EventDescriptor) => map("nonvacuousImage", input, descriptor),
	possible: (input: EventOperand, descriptor: EventDescriptor) => map("possible", input, descriptor),
	guaranteed: (input: EventOperand, descriptor: EventDescriptor) => map("guaranteed", input, descriptor),
	region: (relation: RelationExpr) => relationView("region", relation),
	domain: (relation: RelationExpr) => relationView("domain", relation),
	range: (relation: RelationExpr) => relationView("range", relation),
	may: (relation: RelationExpr, input: EventOperand) => modal("may", relation, input),
	all: (relation: RelationExpr, input: EventOperand) => modal("all", relation, input),
	must: (relation: RelationExpr, input: EventOperand) => modal("must", relation, input),
	post: (relation: RelationExpr, input: EventOperand) => modal("post", relation, input)
})
const EventTest = Object.freeze({
	isEmpty: (input: EventOperand) => unaryTest("isEmpty", input),
	isFull: (input: EventOperand) => unaryTest("isFull", input),
	subset: (left: EventOperand, right: EventOperand) => binaryTest("subset", left, right),
	equal: (left: EventOperand, right: EventOperand) => binaryTest("equal", left, right),
	disjoint: (left: EventOperand, right: EventOperand) => binaryTest("disjoint", left, right),
	covers: (left: EventOperand, right: EventOperand) => binaryTest("covers", left, right)
})
const RelationExpr = Object.freeze({
	bind: (input: EventOperand, descriptor: EventDescriptor) => bindRelation("bind", input, descriptor),
	test: (input: EventOperand, descriptor: EventDescriptor) => bindRelation("test", input, descriptor),
	identity,
	star,
	complement: (input: RelationExpr) => unaryRelation("not", input),
	converse: (input: RelationExpr) => unaryRelation("converse", input),
	apply: relationApply,
	and: (a: RelationExpr, b: RelationExpr) => relationApply(8, a, b),
	or: (a: RelationExpr, b: RelationExpr) => relationApply(14, a, b),
	xor: (a: RelationExpr, b: RelationExpr) => relationApply(6, a, b),
	difference: (a: RelationExpr, b: RelationExpr) => relationApply(4, a, b),
	implies: (a: RelationExpr, b: RelationExpr) => relationApply(11, a, b),
	equivalence: (a: RelationExpr, b: RelationExpr) => relationApply(9, a, b),
	compose: (a: RelationExpr, b: RelationExpr, descriptor: EventDescriptor) => product("compose", a, b, descriptor),
	leftResidual: (a: RelationExpr, b: RelationExpr, descriptor: EventDescriptor) =>
		product("leftResidual", a, b, descriptor),
	rightResidual: (a: RelationExpr, b: RelationExpr, descriptor: EventDescriptor) =>
		product("rightResidual", a, b, descriptor)
})

function eventFindIr(
	input: EventFind,
	variable: (ref: AnyVar) => number
): Extract<FindTermIr, { kind: "event" | "test" | "probability" | "expectation" }> {
	if (input.kind === "expectation") {
		if (!expectations.has(input)) return refused("Expected an owned expectation expression")
		return {
			kind: "expectation",
			value: variable(input.node.value),
			when: variable(input.node.when),
			given: variable(input.node.given)
		}
	}
	const map = { variable, descriptor: EventDescriptor.toBytes, scope: eventBytes, bound: closedPredicate }
	if (input.kind === "probability") {
		const data = probabilities.get(input) ?? refused("Expected an owned probability expression")
		return { kind: "probability", event: eventTree(data.node.event, map), given: eventTree(data.node.given, map) }
	}
	if (input.kind === "event") {
		const data = events.get(input) ?? refused("Expected an owned Event expression")
		return { kind: "event", expr: eventTree(data.node, map) }
	}
	const data = tests.get(input) ?? refused("Expected an owned Event test")
	return { kind: "test", expr: testTree(data.node, map) }
}
function eventFindVars(input: EventFind): readonly AnyVar[] {
	if (input.kind === "expectation") {
		if (!expectations.has(input)) return refused("Expected an owned expectation expression")
		return [input.node.value, input.node.when, input.node.given]
	}
	const variables = new Set<EventVar>()
	const map = {
		variable: (value: EventVar) => {
			variables.add(value)
			return value
		},
		descriptor: (value: EventDescriptor) => value,
		scope: (value: Event) => value,
		bound: closedPredicate
	}
	if (input.kind === "probability") {
		const data = probabilities.get(input) ?? refused("Expected an owned probability expression")
		eventTree(data.node.event, map)
		eventTree(data.node.given, map)
	} else if (input.kind === "event")
		eventTree((events.get(input) ?? refused("Expected an owned Event expression")).node, map)
	else testTree((tests.get(input) ?? refused("Expected an owned Event test")).node, map)
	return [...variables]
}
/** Called only after the shared wire parser has admitted shape. Mathematical
 * descriptor admission remains on the worker when this query is prepared. */
function eventFindFromIr(
	input: Extract<FindTermIr, { kind: "event" | "test" | "probability" | "expectation" }>,
	variable: (ordinal: number) => AnyVar
): EventFind {
	if (input.kind === "expectation")
		return expectation(
			variable(input.value) as IntegerVar,
			eventVar(variable(input.when)),
			eventVar(variable(input.given))
		)
	let nodes = 0
	let depth = 0
	let bytes = 0
	const map = {
		variable: (ordinal: number) => eventVar(variable(ordinal)),
		descriptor: (value: Uint8Array) => {
			const descriptor = encodedDescriptor(value)
			bytes += descriptorLength(descriptor)
			return descriptor
		},
		scope: (value: Uint8Array) => {
			const scope = encodedEvent(value)
			bytes += eventByteLength(scope)
			return scope
		},
		bound: closedPredicate,
		enter: (level: number) => {
			nodes++
			depth = Math.max(depth, level)
		}
	}
	if (input.kind === "probability") {
		const node = { event: eventTree(input.event, map), given: eventTree(input.given, map) }
		return ownProbability(node, extent([{ nodes, depth, bytes }], [], true))
	}
	if (input.kind === "event") {
		const node = eventTree(input.expr, map)
		return ownEvent(node, extent([{ nodes, depth, bytes }], [], true))
	}
	const node = testTree(input.expr, map)
	return ownTest(node, extent([{ nodes, depth, bytes }], [], true))
}

export type { EventFind, EventOperand, EventVar, ExpectationExpr, ProbabilityExpr }
export {
	EventExpr,
	EventTest,
	eventFindFromIr,
	eventFindIr,
	eventFindVars,
	expectation,
	isEventFind,
	probability,
	RelationExpr,
	snapshotEventExpression
}
