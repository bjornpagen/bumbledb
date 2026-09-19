/** Pure authoring for explicit source-bound truth Events and exact transport. */
import { AuthoringError } from "#errors.ts"
import { type Event, encodedEvent, eventByteLength, eventBytes, eventValue } from "#event-value.ts"
import type { BytesField, EventField } from "#fields.ts"
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
type Plan =
	| { readonly kind: "captured"; readonly source: Event; readonly identity: Uint8Array | null }
	| { readonly kind: "bound"; readonly source: EventVar; readonly identity: IdentityVar | null }
function planBytes(plan: Plan): number {
	return plan.kind === "captured" ? eventByteLength(plan.source) + (plan.identity?.length ?? 0) : 0
}
const plans = new WeakMap<GuardPlan, Plan>()
export interface GuardContext {
	readonly [contextTag]: true
}
interface Context {
	readonly plan: GuardPlan
	readonly predicates: readonly PredicateExpr[]
	readonly sources: readonly EventVar[]
}
const contexts = new WeakMap<GuardContext, Context>()
function contextData(value: GuardPlan | GuardContext): Context {
	return contexts.get(value as GuardContext) ?? { plan: value as GuardPlan, predicates: [], sources: [] }
}
function context(data: Context): GuardContext {
	let nodes = 1 + data.sources.length
	let bytes = planBytes(planData(data.plan))
	for (const predicate of data.predicates) {
		const shape = predicateShape(predicate)
		nodes += shape.nodes
		bytes += shape.bytes
		if (shape.depth + 1 > 128) return fail("Common guards exceed combined shape budget")
	}
	if (nodes > 4095 || bytes > 16 * 1024 * 1024)
		return fail("Common guards exceed combined shape or imported byte budget")
	const value: GuardContext = Object.freeze({ [contextTag]: true as const })
	contexts.set(value, data)
	return value
}
function common(plan: GuardPlan | GuardContext, predicates: readonly PredicateOperand[]): GuardContext {
	if (!Array.isArray(predicates) || predicates.length === 0 || predicates.length > 4094)
		return fail("Common guards require a nonempty bounded predicate roster")
	const owned = arrayValue("Common guards", predicates, (_context, p) => asPredicateExpr(p as PredicateOperand))
	const data = contextData(plan)
	return context({ ...data, predicates: Object.freeze([...data.predicates, ...owned]) })
}
function commonSources(plan: GuardPlan | GuardContext, sources: readonly EventVar[]): GuardContext {
	if (!Array.isArray(sources) || sources.length === 0 || sources.length > 4094)
		return fail("Common sources require a nonempty bounded source roster")
	const owned = arrayValue("Common sources", sources, (_, value) => sourceVariable(value as EventVar))
	const data = contextData(plan)
	return context({ ...data, sources: Object.freeze([...data.sources, ...owned]) })
}
export interface GuardExpr {
	readonly kind: "guard"
	readonly [exprTag]: true
	readonly node: GuardExprIr<AnyVar, PredicateExpr, GuardPlan>
}
const expressions = new WeakSet<GuardExpr>()
type EventVar = AnyVar & { readonly field: EventField }
type IdentityVar = AnyVar & { readonly field: BytesField<32> }
function fail(message: string): never {
	throw new AuthoringError({ message })
}
function plan(source: Event, identity: Uint8Array | null): GuardPlan {
	eventValue("GuardPlan source", source)
	const bytes = identity === null ? null : bytesValue("GuardPlan identity", identity, 32)
	if (bytes !== null && bytes.length !== 32) return fail("GuardPlan identity must have 32 bytes")
	const value: GuardPlan = Object.freeze({ [planTag]: true as const })
	plans.set(value, { kind: "captured", source, identity: bytes })
	return value
}
function bound(source: EventVar, identity: IdentityVar | null): GuardPlan {
	sourceVariable(source)
	if (
		identity !== null &&
		(!isTerm(identity) || identity[term] !== "var" || identity.field.kind !== "bytes" || identity.field.width !== 32)
	)
		return fail("Bound guard identity requires a bytes<32> variable")
	const value: GuardPlan = Object.freeze({ [planTag]: true as const })
	plans.set(value, { kind: "bound", source, identity })
	return value
}
function sourceVariable(value: EventVar): EventVar {
	if (!isTerm(value) || value[term] !== "var" || value.field.kind !== "event")
		return fail("Guard source requires an Event variable")
	return value
}
export const GuardPlan = Object.freeze({
	bound: (source: EventVar) => bound(source, null),
	boundRefine: (identity: IdentityVar, source: EventVar) => bound(source, identity),
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
	const sources = context?.sources ?? []
	let nodes = shape.nodes + 1 + sources.length
	let depth = shape.depth + 1
	let bytes = shape.bytes + planBytes(p)
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
		node: Object.freeze({
			...operation,
			predicate: value,
			plan: sourcePlan,
			...(resolve.length ? { resolve } : {}),
			...(sources.length ? { sources } : {})
		})
	})
	expressions.add(expr)
	return expr
}
export const Guard = Object.freeze({
	common,
	commonSources,
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
	const plan = planData(value.node.plan)
	return [
		...(plan.kind === "bound" ? [plan.source, ...(plan.identity === null ? [] : [plan.identity])] : []),
		...predicateVars(value.node.predicate),
		...(value.node.resolve ?? []).flatMap(predicateVars),
		...(value.node.sources ?? []),
		...("input" in value.node ? [value.node.input] : [])
	]
}
export function guardIr(value: GuardExpr, variable: (v: AnyVar) => number): GuardExprIr {
	const node = value.node
	const p = planData(node.plan)
	let plan: GuardPlanIr
	if (p.kind === "bound") {
		plan =
			p.identity === null
				? { kind: "boundExisting", source: variable(p.source) }
				: { kind: "boundRefine", source: variable(p.source), identity: variable(p.identity) }
	} else {
		plan =
			p.identity === null
				? { kind: "existing", source: eventBytes(p.source) }
				: { kind: "refine", source: eventBytes(p.source), identity: new Uint8Array(p.identity) }
	}
	const predicate = predicateIr(node.predicate, variable)
	const resolve = node.resolve === undefined ? {} : { resolve: node.resolve.map((p) => predicateIr(p, variable)) }
	const sources = node.sources === undefined ? {} : { sources: node.sources.map(variable) }
	return "input" in node
		? { kind: node.kind, predicate, plan, ...resolve, ...sources, input: variable(node.input) }
		: { kind: node.kind, predicate, plan, ...resolve, ...sources }
}
export function guardFromIr(node: GuardExprIr, variableAt: (v: number) => AnyVar): GuardExpr {
	const sourcePlan = node.plan
	const g =
		sourcePlan.kind === "boundExisting" || sourcePlan.kind === "boundRefine"
			? bound(
					variableAt(sourcePlan.source) as EventVar,
					sourcePlan.kind === "boundRefine" ? (variableAt(sourcePlan.identity) as IdentityVar) : null
				)
			: plan(encodedEvent(sourcePlan.source), sourcePlan.kind === "refine" ? sourcePlan.identity : null)
	const predicate = predicateFromIr(node.predicate, variableAt)
	let context: GuardPlan | GuardContext =
		node.resolve === undefined
			? g
			: common(
					g,
					node.resolve.map((p) => predicateFromIr(p, variableAt))
				)
	if (node.sources !== undefined)
		context = commonSources(
			context,
			node.sources.map((v) => variableAt(v) as EventVar)
		)
	return "input" in node
		? own(predicate, context, { kind: node.kind, input: variableAt(node.input) as EventVar })
		: own(predicate, context, { kind: node.kind })
}
