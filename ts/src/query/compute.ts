/** Query scalar expressions use the same nodes that native preparation reads.
 * Distinct i64 and u64 kinds survive TypeScript's shared bigint carrier. */
import { AuthoringError } from "#errors.ts"
import type { AnyField } from "#fields.ts"
import { bool as boolField, f64 as f64Field, i64 as i64Field, rosterOf, u64 as u64Field } from "#fields.ts"
import type { AnyVar } from "#query/scope.ts"
import { isTerm, term } from "#query/scope.ts"
import type { Rounding, ScalarKind, ScalarLiteral, ScalarNode } from "#scalar.ts"
import {
	checkBool,
	checkF64,
	checkI64,
	checkU64,
	isScalarNode,
	MAX_SCALAR_DEPTH,
	queryVarLeaf,
	queryVarsOf,
	scalarBinary,
	scalarCast,
	scalarFloatPredicate,
	scalarLiteral,
	scalarMeasure,
	scalarMulDiv,
	scalarNegate
} from "#scalar.ts"

/** Query expression, including interval leaves consumed by measure. */
type QueryNode = ScalarNode

/** Host value for a derived query compute kind. */
type ComputeValue<K extends ScalarKind> = K extends "f64" ? number : K extends "bool" ? boolean : bigint

type ComputeExpr<K extends ScalarKind> = ScalarNode<K>

type AnyComputeExpr = ComputeExpr<"u64"> | ComputeExpr<"i64"> | ComputeExpr<"f64"> | ComputeExpr<"bool">

/** One operand as the runtime judgment sees it. */
type Operand = AnyVar | AnyComputeExpr

type NumericVarOperand = AnyVar & { readonly field: { readonly kind: "u64" | "i64" | "f64" } }
type ArithmeticOperand = ComputeExpr<"u64"> | ComputeExpr<"i64"> | ComputeExpr<"f64"> | NumericVarOperand
type SignedOperand =
	| ComputeExpr<"i64">
	| ComputeExpr<"f64">
	| (AnyVar & { readonly field: { readonly kind: "i64" | "f64" } })
type FloatOperand = ComputeExpr<"f64"> | (AnyVar & { readonly field: { readonly kind: "f64" } })

type OperandKind<O> =
	O extends ComputeExpr<infer K extends ScalarKind>
		? K
		: O extends AnyVar
			? O["field"]["kind"] extends "u64" | "i64" | "f64" | "bool"
				? O["field"]["kind"]
				: never
			: never

function isComputeExpr(value: unknown): value is AnyComputeExpr {
	return (
		isScalarNode(value) &&
		(value.result === "u64" || value.result === "i64" || value.result === "f64" || value.result === "bool")
	)
}

function varKindOf(where: string, ref: AnyVar): ScalarKind {
	const roster = rosterOf(ref.field)
	if (roster !== undefined) {
		throw new AuthoringError({
			message: `${where}: ${ref.label} is a ${roster.name} reference — declaration order is an accident, not semantics: a closed reference never enters arithmetic`
		})
	}
	const kind = ref.field.kind
	if (kind === "u64" || kind === "i64" || kind === "f64" || kind === "bool") {
		return kind
	}
	throw new AuthoringError({
		message: `${where}: ${ref.label} is ${kind} — a scalar expression reads u64/i64/f64/bool variables only`
	})
}

function asQueryNode(where: string, operand: Operand): QueryNode {
	if (isComputeExpr(operand)) {
		return operand
	}
	if (isTerm(operand) && operand[term] === "var") {
		return queryVarLeaf(operand, varKindOf(where, operand))
	}
	throw new AuthoringError({
		message: `${where}: expected a query variable or a Compute expression — literals are tagged (Compute.u64/i64/f64/bool)`
	})
}

function literal<K extends ScalarKind>(value: ScalarLiteral): ComputeExpr<K> {
	return scalarLiteral(value) as ComputeExpr<K>
}

function u64(value: bigint): ComputeExpr<"u64"> {
	checkU64(value, "Compute.u64")
	return literal({ u64: value })
}

function i64(value: bigint): ComputeExpr<"i64"> {
	checkI64(value, "Compute.i64")
	return literal({ i64: value })
}

function f64(value: number): ComputeExpr<"f64"> {
	checkF64(value, "Compute.f64")
	return literal({ f64: value })
}

function bool(value: boolean): ComputeExpr<"bool"> {
	checkBool(value, "Compute.bool")
	return literal({ bool: value })
}

type BinaryKind = "add" | "subtract" | "multiply" | "divide"

/** The right operand must have the left operand's kind, never an inferred union. */
type SameKind<L, R> = [OperandKind<R>] extends [OperandKind<L>]
	? [OperandKind<L>] extends [OperandKind<R>]
		? R
		: never
	: never

function binary<K extends "u64" | "i64" | "f64">(
	op: BinaryKind,
	left: ArithmeticOperand,
	right: ArithmeticOperand
): ComputeExpr<K> {
	const where = `Compute.${op}`
	return scalarBinary(where, op, asQueryNode(where, left), asQueryNode(where, right)) as ComputeExpr<K>
}

function add<L extends ArithmeticOperand, R extends ArithmeticOperand>(
	left: L,
	right: R & SameKind<NoInfer<L>, R>
): ComputeExpr<OperandKind<L> & OperandKind<R> & ("u64" | "i64" | "f64")> {
	return binary("add", left, right)
}

function subtract<L extends ArithmeticOperand, R extends ArithmeticOperand>(
	left: L,
	right: R & SameKind<NoInfer<L>, R>
): ComputeExpr<OperandKind<L> & OperandKind<R> & ("u64" | "i64" | "f64")> {
	return binary("subtract", left, right)
}

function multiply<L extends ArithmeticOperand, R extends ArithmeticOperand>(
	left: L,
	right: R & SameKind<NoInfer<L>, R>
): ComputeExpr<OperandKind<L> & OperandKind<R> & ("u64" | "i64" | "f64")> {
	return binary("multiply", left, right)
}

function divide<L extends ArithmeticOperand, R extends ArithmeticOperand>(
	left: L,
	right: R & SameKind<NoInfer<L>, R>
): ComputeExpr<OperandKind<L> & OperandKind<R> & ("u64" | "i64" | "f64")> {
	return binary("divide", left, right)
}

function negate<O extends SignedOperand>(operand: O): ComputeExpr<OperandKind<O> & ("i64" | "f64")> {
	const where = "Compute.negate"
	return scalarNegate(where, asQueryNode(where, operand)) as ComputeExpr<OperandKind<O> & ("i64" | "f64")>
}

type IntegerOperand =
	| ComputeExpr<"u64">
	| ComputeExpr<"i64">
	| (AnyVar & { readonly field: { readonly kind: "i64" | "u64" } })

function mulDiv<A extends IntegerOperand, B extends IntegerOperand, D extends IntegerOperand>(
	a: A,
	b: B & (OperandKind<A> extends OperandKind<B> ? unknown : never),
	divisor: D & (OperandKind<A> extends OperandKind<D> ? unknown : never),
	rounding: Rounding
): ComputeExpr<OperandKind<A>> {
	return scalarMulDiv(
		"Compute.mulDiv",
		asQueryNode("Compute.mulDiv", a),
		asQueryNode("Compute.mulDiv", b),
		asQueryNode("Compute.mulDiv", divisor),
		rounding
	) as ComputeExpr<OperandKind<A>>
}

function measure<V extends AnyVar & { readonly field: { readonly kind: "interval" } }>(
	interval: V
): ComputeExpr<V["field"] extends { readonly element: "f64" } ? "f64" : "u64"> {
	if (!isTerm(interval) || interval[term] !== "var" || interval.field.kind !== "interval")
		throw new AuthoringError({ message: "Compute.measure: expected an interval variable" })
	const kind = { u64: "intervalU64", i64: "intervalI64", f64: "intervalF64" } as const
	return scalarMeasure("Compute.measure", queryVarLeaf(interval, kind[interval.field.element])) as ComputeExpr<
		V["field"] extends { readonly element: "f64" } ? "f64" : "u64"
	>
}

function toF64(operand: ArithmeticOperand): ComputeExpr<"f64"> {
	const where = "Compute.toF64"
	return scalarCast(where, "toF64", "f64", asQueryNode(where, operand)) as ComputeExpr<"f64">
}

function toF64Exact(operand: ArithmeticOperand): ComputeExpr<"f64"> {
	const where = "Compute.toF64Exact"
	return scalarCast(where, "toF64Exact", "f64", asQueryNode(where, operand)) as ComputeExpr<"f64">
}

function toI64Exact(operand: ArithmeticOperand): ComputeExpr<"i64"> {
	const where = "Compute.toI64Exact"
	return scalarCast(where, "toI64Exact", "i64", asQueryNode(where, operand)) as ComputeExpr<"i64">
}

function toU64Exact(operand: ArithmeticOperand): ComputeExpr<"u64"> {
	const where = "Compute.toU64Exact"
	return scalarCast(where, "toU64Exact", "u64", asQueryNode(where, operand)) as ComputeExpr<"u64">
}

function isNaNExpr(operand: FloatOperand): ComputeExpr<"bool"> {
	const where = "Compute.isNaN"
	return scalarFloatPredicate(where, "isNaN", asQueryNode(where, operand))
}

function isFiniteExpr(operand: FloatOperand): ComputeExpr<"bool"> {
	const where = "Compute.isFinite"
	return scalarFloatPredicate(where, "isFinite", asQueryNode(where, operand))
}

/** Every variable the expression reads (validation binds them like any term). */
function computeVarsOf(data: QueryNode): readonly AnyVar[] {
	return queryVarsOf(data) as readonly AnyVar[]
}

function computeFieldOf(kind: ScalarKind): AnyField {
	switch (kind) {
		case "u64":
			return u64Field
		case "i64":
			return i64Field
		case "f64":
			return f64Field
		case "bool":
			return boolField
	}
}

const Compute = Object.freeze({
	u64,
	i64,
	f64,
	bool,
	negate,
	add,
	subtract,
	multiply,
	divide,
	mulDiv,
	measure,
	toF64,
	toF64Exact,
	toI64Exact,
	toU64Exact,
	isNaN: isNaNExpr,
	isFinite: isFiniteExpr
})

export type { AnyComputeExpr, ComputeExpr, ComputeValue, QueryNode }
export { Compute, computeFieldOf, computeVarsOf, isComputeExpr, MAX_SCALAR_DEPTH as MAX_COMPUTE_DEPTH }
