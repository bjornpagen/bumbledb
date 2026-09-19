/** Pure authoring for explicit source-bound truth Events and exact transport. */
import { AuthoringError } from "#errors.ts"
import { type Event, encodedEvent, eventByteLength, eventBytes, eventValue } from "#event-value.ts"
import type { EventField } from "#fields.ts"
import type { GuardExprIr, GuardPlanIr } from "#native.ts"
import {
	asPredicateExpr,
	type PredicateExpr,
	type PredicateOperand,
	predicateFromIr,
	predicateIr,
	predicateShape,
	predicateVars
} from "#query/predicate.ts"
import { type AnyVar, isTerm, term } from "#query/scope.ts"
import { arrayValue, bytesValue } from "#values.ts"

const planTag: unique symbol = Symbol("bumbledb.GuardPlan")
const exprTag: unique symbol = Symbol("bumbledb.GuardExpression")
const contextTag: unique symbol = Symbol("bumbledb.GuardContext")
export interface GuardPlan {
	readonly [planTag]: true
}
interface Plan {
	readonly source: Event
	readonly identity: Uint8Array | null
}
const plans = new WeakMap<GuardPlan, Plan>()
export interface GuardContext {
	readonly [contextTag]: true
}
interface Context {
	readonly plan: GuardPlan
	readonly predicates: readonly PredicateExpr[]
}
const contexts = new WeakMap<GuardContext, Context>()
function common(plan: GuardPlan, predicates: readonly PredicateOperand[]): GuardContext {
	planData(plan)
	if (!Array.isArray(predicates) || predicates.length === 0 || predicates.length > 4094)
		return fail("Common guards require a nonempty bounded predicate roster")
	const owned = arrayValue("Common guards", predicates, (_context, p) => asPredicateExpr(p as PredicateOperand))
	let nodes = 1
	let bytes = eventByteLength(planData(plan).source) + (planData(plan).identity?.length ?? 0)
	for (const predicate of owned) {
		const shape = predicateShape(predicate)
		nodes += shape.nodes
		bytes += shape.bytes
		if (nodes > 4095 || shape.depth + 1 > 128 || bytes > 16 * 1024 * 1024)
			return fail("Common guards exceed combined shape or imported byte budget")
	}
	const value: GuardContext = Object.freeze({ [contextTag]: true as const })
	contexts.set(value, { plan, predicates: owned })
	return value
}
export interface GuardExpr {
	readonly kind: "guard"
	readonly [exprTag]: true
	readonly node: GuardExprIr<AnyVar, PredicateExpr, GuardPlan>
}
const expressions = new WeakSet<GuardExpr>()
type EventVar = AnyVar & { readonly field: EventField }
function fail(message: string): never {
	throw new AuthoringError({ message })
}
function plan(source: Event, identity: Uint8Array | null): GuardPlan {
	eventValue("GuardPlan source", source)
	const bytes = identity === null ? null : bytesValue("GuardPlan identity", identity, 32)
	if (bytes !== null && bytes.length !== 32) return fail("GuardPlan identity must have 32 bytes")
	const value: GuardPlan = Object.freeze({ [planTag]: true as const })
	plans.set(value, { source, identity: bytes })
	return value
}
export const GuardPlan = Object.freeze({
	existing: (source: Event) => plan(source, null),
	refine: (identity: Uint8Array, source: Event) => plan(source, identity)
})
function planData(value: GuardPlan): Plan {
	return plans.get(value) ?? fail("Expected an owned GuardPlan")
}
function own(
	predicate: PredicateOperand,
	captured: GuardPlan | GuardContext,
	operation: { kind: "holds" | "fails" | "undefined" } | { kind: "lift" | "descend"; input: EventVar }
): GuardExpr {
	const value = asPredicateExpr(predicate)
	const shape = predicateShape(value)
	const context = contexts.get(captured as GuardContext)
	const sourcePlan = context?.plan ?? (captured as GuardPlan)
	const p = planData(sourcePlan)
	const resolve = context?.predicates ?? []
	let nodes = shape.nodes + 1
	let depth = shape.depth + 1
	let bytes = shape.bytes + eventByteLength(p.source) + (p.identity?.length ?? 0)
	for (const companion of resolve) {
		const added = predicateShape(companion)
		nodes += added.nodes
		depth = Math.max(depth, added.depth + 1)
		bytes += added.bytes
	}
	if (nodes > 4096 || depth > 128 || bytes > 16 * 1024 * 1024)
		return fail("Guard expression exceeds combined shape or imported byte budget")
	if (
		"input" in operation &&
		(!isTerm(operation.input) || operation.input[term] !== "var" || operation.input.field.kind !== "event")
	)
		return fail("Guard transport requires an Event variable")
	const expr: GuardExpr = Object.freeze({
		kind: "guard",
		[exprTag]: true as const,
		node: Object.freeze({ ...operation, predicate: value, plan: sourcePlan, ...(resolve.length ? { resolve } : {}) })
	})
	expressions.add(expr)
	return expr
}
export const Guard = Object.freeze({
	common,
	holds: (p: PredicateOperand, g: GuardPlan | GuardContext) => own(p, g, { kind: "holds" }),
	fails: (p: PredicateOperand, g: GuardPlan | GuardContext) => own(p, g, { kind: "fails" }),
	undefined: (p: PredicateOperand, g: GuardPlan | GuardContext) => own(p, g, { kind: "undefined" }),
	lift: (p: PredicateOperand, g: GuardPlan | GuardContext, input: EventVar) => own(p, g, { kind: "lift", input }),
	descend: (p: PredicateOperand, g: GuardPlan | GuardContext, input: EventVar) => own(p, g, { kind: "descend", input })
})
export function isGuardExpr(value: unknown): value is GuardExpr {
	return typeof value === "object" && value !== null && expressions.has(value as GuardExpr)
}
export function snapshotGuardExpression(source: object, snapshot: object): void {
	if (expressions.has(source as GuardExpr)) expressions.add(snapshot as GuardExpr)
}
export function guardVars(value: GuardExpr): readonly AnyVar[] {
	return [
		...predicateVars(value.node.predicate),
		...(value.node.resolve ?? []).flatMap(predicateVars),
		...("input" in value.node ? [value.node.input] : [])
	]
}
export function guardIr(value: GuardExpr, variable: (v: AnyVar) => number): GuardExprIr {
	const node = value.node
	const p = planData(node.plan)
	const plan: GuardPlanIr =
		p.identity === null
			? { kind: "existing", source: eventBytes(p.source) }
			: { kind: "refine", source: eventBytes(p.source), identity: new Uint8Array(p.identity) }
	const predicate = predicateIr(node.predicate, variable)
	const resolve = node.resolve === undefined ? {} : { resolve: node.resolve.map((p) => predicateIr(p, variable)) }
	return "input" in node
		? { kind: node.kind, predicate, plan, ...resolve, input: variable(node.input) }
		: { kind: node.kind, predicate, plan, ...resolve }
}
export function guardFromIr(node: GuardExprIr, variableAt: (v: number) => AnyVar): GuardExpr {
	const g = plan(encodedEvent(node.plan.source), node.plan.kind === "refine" ? node.plan.identity : null)
	const predicate = predicateFromIr(node.predicate, variableAt)
	const context =
		node.resolve === undefined
			? g
			: common(
					g,
					node.resolve.map((p) => predicateFromIr(p, variableAt))
				)
	return "input" in node
		? own(predicate, context, { kind: node.kind, input: variableAt(node.input) as EventVar })
		: own(predicate, context, { kind: node.kind })
}
