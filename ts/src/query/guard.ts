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
import { bytesValue } from "#values.ts"

const planTag: unique symbol = Symbol("bumbledb.GuardPlan")
const exprTag: unique symbol = Symbol("bumbledb.GuardExpression")
export interface GuardPlan {
	readonly [planTag]: true
}
interface Plan {
	readonly source: Event
	readonly identity: Uint8Array | null
}
const plans = new WeakMap<GuardPlan, Plan>()
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
	captured: GuardPlan,
	operation: { kind: "holds" | "fails" | "undefined" } | { kind: "lift" | "descend"; input: EventVar }
): GuardExpr {
	const value = asPredicateExpr(predicate)
	const shape = predicateShape(value)
	const p = planData(captured)
	if (
		shape.nodes + 1 > 4096 ||
		shape.depth + 1 > 128 ||
		shape.bytes + eventByteLength(p.source) + (p.identity?.length ?? 0) > 16 * 1024 * 1024
	)
		return fail("Guard expression exceeds combined shape or imported byte budget")
	if (
		"input" in operation &&
		(!isTerm(operation.input) || operation.input[term] !== "var" || operation.input.field.kind !== "event")
	)
		return fail("Guard transport requires an Event variable")
	const expr: GuardExpr = Object.freeze({
		kind: "guard",
		[exprTag]: true as const,
		node: Object.freeze({ ...operation, predicate: value, plan: captured })
	})
	expressions.add(expr)
	return expr
}
export const Guard = Object.freeze({
	holds: (p: PredicateOperand, g: GuardPlan) => own(p, g, { kind: "holds" }),
	fails: (p: PredicateOperand, g: GuardPlan) => own(p, g, { kind: "fails" }),
	undefined: (p: PredicateOperand, g: GuardPlan) => own(p, g, { kind: "undefined" }),
	lift: (p: PredicateOperand, g: GuardPlan, input: EventVar) => own(p, g, { kind: "lift", input }),
	descend: (p: PredicateOperand, g: GuardPlan, input: EventVar) => own(p, g, { kind: "descend", input })
})
export function isGuardExpr(value: unknown): value is GuardExpr {
	return typeof value === "object" && value !== null && expressions.has(value as GuardExpr)
}
export function snapshotGuardExpression(source: object, snapshot: object): void {
	if (expressions.has(source as GuardExpr)) expressions.add(snapshot as GuardExpr)
}
export function guardVars(value: GuardExpr): readonly AnyVar[] {
	return [...predicateVars(value.node.predicate), ...("input" in value.node ? [value.node.input] : [])]
}
export function guardIr(value: GuardExpr, variable: (v: AnyVar) => number): GuardExprIr {
	const node = value.node
	const p = planData(node.plan)
	const plan: GuardPlanIr =
		p.identity === null
			? { kind: "existing", source: eventBytes(p.source) }
			: { kind: "refine", source: eventBytes(p.source), identity: new Uint8Array(p.identity) }
	const predicate = predicateIr(node.predicate, variable)
	return "input" in node
		? { kind: node.kind, predicate, plan, input: variable(node.input) }
		: { kind: node.kind, predicate, plan }
}
export function guardFromIr(node: GuardExprIr, variableAt: (v: number) => AnyVar): GuardExpr {
	const g = plan(encodedEvent(node.plan.source), node.plan.kind === "refine" ? node.plan.identity : null)
	const predicate = predicateFromIr(node.predicate, variableAt)
	return "input" in node
		? own(predicate, g, { kind: node.kind, input: variableAt(node.input) as EventVar })
		: own(predicate, g, { kind: node.kind })
}
