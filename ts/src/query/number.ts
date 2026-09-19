/** Numerical query programs are data, evaluated by the native exact algebra. */
import { AuthoringError } from "#errors.ts"
import { type ExactRational, encodedRational, rationalBytes } from "#exact-value.ts"
import type { NumberExprIr } from "#native.ts"
import { encodedNumber, numberBytes, type ObservationNumber } from "#number-value.ts"
import { encodedParameter, type ParameterValue, parameterBytes } from "#parameter-value.ts"
import { type AnyVar, isTerm, term } from "#query/scope.ts"
import { queryRosterOf } from "#query/value.ts"

const expressionTag: unique symbol = Symbol("bumbledb.NumberExpression")
type Node = NumberExprIr<AnyVar, ExactRational, ObservationNumber, ParameterValue<"domain">>
export interface NumberExpr {
	readonly kind: "number"
	readonly [expressionTag]: true
	readonly node: Node
}
type NumberVar = AnyVar & { readonly field: { readonly kind: "number" } }
type IntegerVar = AnyVar & { readonly field: { readonly kind: "u64" | "i64" } }
type ObservationVar = AnyVar & { readonly field: { readonly kind: "probability" | "expectation" } }
export type NumberOperand = NumberExpr | NumberVar
interface Owned {
	readonly node: Node
	readonly nodes: number
	readonly depth: number
	readonly bytes: number
}
const expressions = new WeakMap<NumberExpr, Owned>()
function refused(message: string): never {
	throw new AuthoringError({ message })
}
function variable(value: AnyVar, kinds: readonly string[]): AnyVar {
	if (!isTerm(value) || value[term] !== "var" || !kinds.includes(value.field.kind))
		return refused(`Number expression: expected a ${kinds.join("/")} variable`)
	if (queryRosterOf(value.field) !== undefined)
		return refused("Number expression: a closed reference is not an exact integer")
	return value
}
function own(node: Node, children: readonly Owned[] = [], bytes = 0): NumberExpr {
	const size = {
		nodes: 1 + children.reduce((n, c) => n + c.nodes, 0),
		depth: 1 + children.reduce((n, c) => Math.max(n, c.depth), 0),
		bytes: bytes + children.reduce((n, c) => n + c.bytes, 0)
	}
	if (size.nodes > 4096 || size.depth > 128 || size.bytes > 16 * 1024 * 1024)
		return refused("Number expression exceeds shape or imported byte budget")
	const value: NumberExpr = Object.freeze({ kind: "number", [expressionTag]: true as const, node: Object.freeze(node) })
	expressions.set(value, { node: value.node, ...size })
	return value
}
export function isNumberExpr(value: unknown): value is NumberExpr {
	return typeof value === "object" && value !== null && expressions.has(value as NumberExpr)
}
function data(value: NumberOperand): Owned {
	const found = expressions.get(value as NumberExpr)
	if (found !== undefined) return found
	const expr = own({ kind: "var", var: variable(value as NumberVar, ["number"]) })
	return expressions.get(expr) ?? refused("Number expression: missing owned program")
}
function binary(kind: Extract<Node, { left: Node }>["kind"], left: NumberOperand, right: NumberOperand): NumberExpr {
	const a = data(left)
	const b = data(right)
	return own({ kind, left: a.node, right: b.node }, [a, b])
}
function unary(kind: "abs" | "negate", input: NumberOperand): NumberExpr {
	const value = data(input)
	return own({ kind, value: value.node }, [value])
}
function component(input: ObservationVar, component: "value" | "numerator" | "evidenceMass"): NumberExpr {
	return own({ kind: "component", observation: variable(input, ["probability", "expectation"]), component })
}
export const NumberExpr = Object.freeze({
	value: (input: ObservationVar) => component(input, "value"),
	numerator: (input: ObservationVar) => component(input, "numerator"),
	evidenceMass: (input: ObservationVar) => component(input, "evidenceMass"),
	integer: (input: IntegerVar) => own({ kind: "integer", var: variable(input, ["u64", "i64"]) }),
	literal: (input: ExactRational) => own({ kind: "literal", bytes: input }, [], rationalBytes(input).length),
	imported: (input: ObservationNumber) => own({ kind: "imported", bytes: input }, [], numberBytes(input).length),
	add: (a: NumberOperand, b: NumberOperand) => binary("add", a, b),
	subtract: (a: NumberOperand, b: NumberOperand) => binary("subtract", a, b),
	multiply: (a: NumberOperand, b: NumberOperand) => binary("multiply", a, b),
	divide: (a: NumberOperand, b: NumberOperand) => binary("divide", a, b),
	min: (a: NumberOperand, b: NumberOperand) => binary("min", a, b),
	max: (a: NumberOperand, b: NumberOperand) => binary("max", a, b),
	negate: (value: NumberOperand) => unary("negate", value),
	abs: (value: NumberOperand) => unary("abs", value),
	pow: (input: NumberOperand, exponent: number): NumberExpr => {
		if (!Number.isInteger(exponent) || exponent < 0 || exponent > 0xffffffff)
			return refused("Number expression: power requires a natural u32 exponent")
		const value = data(input)
		return own({ kind: "pow", value: value.node, exponent }, [value])
	},
	onDomain: (input: NumberOperand, domain: ParameterValue<"domain">): NumberExpr => {
		const value = data(input)
		return own({ kind: "onDomain", value: value.node, domain }, [value], parameterBytes("domain", domain).length)
	}
})
export function snapshotNumberExpression(source: object, snapshot: object): void {
	const original = expressions.get(source as NumberExpr)
	if (original !== undefined) {
		const value = snapshot as NumberExpr
		expressions.set(value, { ...original, node: value.node })
	}
}
export function numberVars(input: NumberExpr): readonly AnyVar[] {
	const variables: AnyVar[] = []
	const pending = [data(input).node]
	while (pending.length) {
		const node = pending.pop()
		if (node === undefined) break
		switch (node.kind) {
			case "var":
			case "integer":
				variables.push(node.var)
				break
			case "component":
				variables.push(node.observation)
				break
			case "literal":
			case "imported":
				break
			case "negate":
			case "abs":
			case "pow":
			case "onDomain":
				pending.push(node.value)
				break
			default:
				pending.push(node.right, node.left)
		}
	}
	return variables
}
export function numberIr(input: NumberExpr, variable: (v: AnyVar) => number): NumberExprIr {
	function walk(node: Node): NumberExprIr {
		switch (node.kind) {
			case "var":
			case "integer":
				return { kind: node.kind, var: variable(node.var) }
			case "component":
				return { kind: node.kind, observation: variable(node.observation), component: node.component }
			case "literal":
				return { kind: node.kind, bytes: rationalBytes(node.bytes) }
			case "imported":
				return { kind: node.kind, bytes: numberBytes(node.bytes) }
			case "negate":
			case "abs":
				return { kind: node.kind, value: walk(node.value) }
			case "pow":
				return { kind: node.kind, value: walk(node.value), exponent: node.exponent }
			case "onDomain":
				return { kind: node.kind, value: walk(node.value), domain: parameterBytes("domain", node.domain) }
			default:
				return { kind: node.kind, left: walk(node.left), right: walk(node.right) }
		}
	}
	return walk(data(input).node)
}
export function numberFromIr(input: NumberExprIr, variableAt: (v: number) => AnyVar): NumberExpr {
	function walk(node: NumberExprIr): NumberExpr {
		switch (node.kind) {
			case "var":
				return own({ kind: "var", var: variable(variableAt(node.var), ["number"]) })
			case "integer":
				return NumberExpr.integer(variableAt(node.var) as IntegerVar)
			case "component":
				return component(variableAt(node.observation) as ObservationVar, node.component)
			case "literal":
				return NumberExpr.literal(encodedRational(node.bytes))
			case "imported":
				return NumberExpr.imported(encodedNumber(node.bytes))
			case "negate":
			case "abs":
				return unary(node.kind, walk(node.value))
			case "pow":
				return NumberExpr.pow(walk(node.value), node.exponent)
			case "onDomain":
				return NumberExpr.onDomain(walk(node.value), encodedParameter("domain", node.domain))
			default:
				return binary(node.kind, walk(node.left), walk(node.right))
		}
	}
	return walk(input)
}
