/** Owned programs over exact truth partitions; no SDK numerical evaluator. */
import { AuthoringError } from "#errors.ts"
import type { PredicateExprIr, PredicateQuantifier } from "#native.ts"
import { encodedParameter, type ParameterValue, parameterBytes } from "#parameter-value.ts"
import { encodedPredicate, type ObservationPredicate, predicateBytes } from "#predicate-value.ts"
import {
	asNumberExpr,
	NumberExpr,
	type NumberOperand,
	numberFromIr,
	numberIr,
	numberShape,
	numberVars
} from "#query/number.ts"
import { type AnyVar, isTerm, term } from "#query/scope.ts"

const expressionTag: unique symbol = Symbol("bumbledb.PredicateExpression")
const testTag: unique symbol = Symbol("bumbledb.PredicateTest")
type Node = PredicateExprIr<
	AnyVar,
	NumberExpr,
	ObservationPredicate,
	ParameterValue<"domain">,
	ParameterValue<"region">
>
export interface PredicateExpr {
	readonly kind: "predicate"
	readonly [expressionTag]: true
	readonly node: Node
}
export interface PredicateTest {
	readonly kind: "predicateTest"
	readonly [testTag]: true
	readonly expression: PredicateExpr
	readonly quantifier: PredicateQuantifier
}
type PredicateVar = AnyVar & { readonly field: { readonly kind: "predicate" } }
export type PredicateOperand = PredicateExpr | PredicateVar
interface Shape {
	readonly nodes: number
	readonly depth: number
	readonly bytes: number
}
interface Owned extends Shape {
	readonly node: Node
}
const expressions = new WeakMap<PredicateExpr, Owned>()
const tests = new WeakSet<PredicateTest>()
function refused(message: string): never {
	throw new AuthoringError({ message })
}
function own(node: Node, children: readonly Shape[] = [], bytes = 0): PredicateExpr {
	const size = {
		nodes: 1 + children.reduce((n, c) => n + c.nodes, 0),
		depth: 1 + children.reduce((n, c) => Math.max(n, c.depth), 0),
		bytes: bytes + children.reduce((n, c) => n + c.bytes, 0)
	}
	if (size.nodes > 4096 || size.depth > 128 || size.bytes > 16 * 1024 * 1024)
		return refused("Predicate expression exceeds combined shape or imported byte budget")
	const value: PredicateExpr = Object.freeze({
		kind: "predicate",
		[expressionTag]: true as const,
		node: Object.freeze(node)
	})
	expressions.set(value, { ...size, node: value.node })
	return value
}
export function isPredicateExpr(value: unknown): value is PredicateExpr {
	return typeof value === "object" && value !== null && expressions.has(value as PredicateExpr)
}
export function isPredicateTest(value: unknown): value is PredicateTest {
	return typeof value === "object" && value !== null && tests.has(value as PredicateTest)
}
function expression(input: PredicateOperand): PredicateExpr {
	if (isPredicateExpr(input)) return input
	if (!isTerm(input) || input[term] !== "var" || input.field.kind !== "predicate")
		return refused("Predicate expression: expected a completed predicate variable")
	return own({ kind: "var", var: input })
}
function data(input: PredicateOperand): Owned {
	return expressions.get(expression(input)) ?? refused("Predicate expression: missing owned program")
}
function sign(input: NumberOperand, mask: number | bigint): PredicateExpr {
	const signs = typeof mask === "bigint" ? Number(mask) : mask
	if (!Number.isInteger(signs) || signs < 0 || signs > 7)
		return refused("Predicate expression: expected a three-bit sign mask")
	const number = asNumberExpr(input)
	return own({ kind: "sign", number, signs }, [numberShape(number)])
}
function apply(mask: number | bigint, left: PredicateOperand, right: PredicateOperand): PredicateExpr {
	const op = typeof mask === "bigint" ? Number(mask) : mask
	if (!Number.isInteger(op) || op < 0 || op > 15)
		return refused("Predicate expression: expected a four-bit truth table")
	const a = data(left)
	const b = data(right)
	return own({ kind: "apply", op, left: a.node, right: b.node }, [a, b])
}
function compare(a: NumberOperand, b: NumberOperand, signs: number | bigint): PredicateExpr {
	return sign(NumberExpr.subtract(a, b), signs)
}
export const PredicateExpr = Object.freeze({
	region: (domain: ParameterValue<"domain">, region: ParameterValue<"region">) =>
		own(
			{ kind: "region", domain, region },
			[],
			parameterBytes("domain", domain).length + parameterBytes("region", region).length
		),
	sign,
	compare,
	apply,
	greater: (a: NumberOperand, b: NumberOperand) => compare(a, b, 4),
	greaterEqual: (a: NumberOperand, b: NumberOperand) => compare(a, b, 6),
	less: (a: NumberOperand, b: NumberOperand) => compare(a, b, 1),
	lessEqual: (a: NumberOperand, b: NumberOperand) => compare(a, b, 3),
	equal: (a: NumberOperand, b: NumberOperand) => compare(a, b, 2),
	notEqual: (a: NumberOperand, b: NumberOperand) => compare(a, b, 5),
	and: (a: PredicateOperand, b: PredicateOperand) => apply(8, a, b),
	or: (a: PredicateOperand, b: PredicateOperand) => apply(14, a, b),
	xor: (a: PredicateOperand, b: PredicateOperand) => apply(6, a, b),
	not: (input: PredicateOperand) => {
		const value = data(input)
		return own({ kind: "negate", value: value.node }, [value])
	},
	imported: (input: ObservationPredicate) => own({ kind: "imported", bytes: input }, [], predicateBytes(input).length),
	onDomain: (input: PredicateOperand, domain: ParameterValue<"domain">) => {
		const value = data(input)
		return own({ kind: "onDomain", value: value.node, domain }, [value], parameterBytes("domain", domain).length)
	}
})
function test(input: PredicateOperand, quantifier: PredicateQuantifier): PredicateTest {
	const value: PredicateTest = Object.freeze({
		kind: "predicateTest",
		[testTag]: true as const,
		expression: expression(input),
		quantifier
	})
	tests.add(value)
	return value
}
export const PredicateTest = Object.freeze({
	possibly: (value: PredicateOperand) => test(value, "possibly"),
	always: (value: PredicateOperand) => test(value, "always"),
	isTotal: (value: PredicateOperand) => test(value, "isTotal")
})
export function snapshotPredicateExpression(source: object, snapshot: object): void {
	const original = expressions.get(source as PredicateExpr)
	if (original !== undefined) {
		const value = snapshot as PredicateExpr
		expressions.set(value, { ...original, node: value.node })
	}
	if (tests.has(source as PredicateTest)) tests.add(snapshot as PredicateTest)
}
export function predicateVars(input: PredicateExpr): readonly AnyVar[] {
	const variables: AnyVar[] = []
	const pending = [data(input).node]
	while (pending.length) {
		const node = pending.pop()
		if (node === undefined) break
		switch (node.kind) {
			case "var":
				variables.push(node.var)
				break
			case "sign":
				variables.push(...numberVars(node.number))
				break
			case "imported":
			case "region":
				break
			case "apply":
				pending.push(node.right, node.left)
				break
			default:
				pending.push(node.value)
		}
	}
	return variables
}
export function predicateIr(input: PredicateExpr, variable: (v: AnyVar) => number): PredicateExprIr {
	function walk(node: Node): PredicateExprIr {
		switch (node.kind) {
			case "var":
				return { kind: node.kind, var: variable(node.var) }
			case "region":
				return {
					kind: node.kind,
					domain: parameterBytes("domain", node.domain),
					region: parameterBytes("region", node.region)
				}
			case "sign":
				return { kind: node.kind, number: numberIr(node.number, variable), signs: node.signs }
			case "imported":
				return { kind: node.kind, bytes: predicateBytes(node.bytes) }
			case "negate":
				return { kind: node.kind, value: walk(node.value) }
			case "apply":
				return { kind: node.kind, op: node.op, left: walk(node.left), right: walk(node.right) }
			case "onDomain":
				return { kind: node.kind, value: walk(node.value), domain: parameterBytes("domain", node.domain) }
		}
	}
	return walk(data(input).node)
}
export function predicateFromIr(input: PredicateExprIr, variableAt: (v: number) => AnyVar): PredicateExpr {
	function walk(node: PredicateExprIr): PredicateExpr {
		switch (node.kind) {
			case "var":
				return expression(variableAt(node.var) as PredicateVar)
			case "region":
				return PredicateExpr.region(encodedParameter("domain", node.domain), encodedParameter("region", node.region))
			case "sign":
				return sign(numberFromIr(node.number, variableAt), node.signs)
			case "imported":
				return PredicateExpr.imported(encodedPredicate(node.bytes))
			case "negate":
				return PredicateExpr.not(walk(node.value))
			case "apply":
				return apply(node.op, walk(node.left), walk(node.right))
			case "onDomain":
				return PredicateExpr.onDomain(walk(node.value), encodedParameter("domain", node.domain))
		}
	}
	return walk(input)
}

export function asPredicateExpr(value: PredicateOperand): PredicateExpr {
	return expression(value)
}
export function predicateShape(value: PredicateExpr): Shape {
	return data(value)
}
