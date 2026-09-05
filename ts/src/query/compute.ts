/**
 * Query-scoped scalar expressions (C05 `FindTerm::Compute`): the
 * variable-addressed leaf scope over the ONE shared operator roster in
 * `#scalar.ts`. Leaves are bound query variables that lower to `VarId`s.
 * Operators are the shared constructors — this module only admits known
 * schema kinds at query-var leaves.
 *
 * `ComputeExpr<K>` is a `ScalarNode<"query-var", K>`. Distinct I64 and U64
 * survive host inference; they do not collapse to plain `bigint`.
 */
import { AuthoringError } from "#errors.ts"
import type { AnyField } from "#fields.ts"
import { bool as boolField, f64 as f64Field, i64 as i64Field, rosterOf, u64 as u64Field } from "#fields.ts"
import type { ScalarKind, ScalarLiteral, ScalarNode } from "#scalar.ts"
import {
	MAX_SCALAR_DEPTH,
	checkBool,
	checkF64,
	checkI64,
	checkU64,
	isScalarNode,
	queryVarLeaf,
	queryVarsOf,
	scalarBinary,
	scalarCast,
	scalarFloatPredicate,
	scalarLiteral,
	scalarNegate
} from "#scalar.ts"
import type { AnyVar } from "#query/scope.ts"
import { isTerm, term } from "#query/scope.ts"

/** Query-var tree: the shared AST restricted to this leaf scope. */
type QueryNode = ScalarNode<"query-var">

/** Host value for a derived query compute kind. */
type ComputeValue<K extends ScalarKind> = K extends "f64" ? number : K extends "bool" ? boolean : bigint

type ComputeExpr<K extends ScalarKind> = ScalarNode<"query-var", K>

type AnyComputeExpr = ComputeExpr<"u64"> | ComputeExpr<"i64"> | ComputeExpr<"f64"> | ComputeExpr<"bool">

/** One operand as the runtime judgment sees it. */
type Operand = AnyVar | AnyComputeExpr

type NumericVarOperand = AnyVar & { readonly field: { readonly kind: "u64" | "i64" | "f64" } }
type ArithmeticOperand = ComputeExpr<"u64"> | ComputeExpr<"i64"> | ComputeExpr<"f64"> | NumericVarOperand
type FloatOperand = ComputeExpr<"f64"> | (AnyVar & { readonly field: { readonly kind: "f64" } })

type OperandKind<O> = O extends ComputeExpr<infer K extends ScalarKind>
	? K
	: O extends AnyVar
		? O["field"]["kind"] extends "u64" | "i64" | "f64" | "bool"
			? O["field"]["kind"]
			: never
		: never

function isComputeExpr(value: unknown): value is AnyComputeExpr {
	return isScalarNode(value) && value.scope === "query-var" && value.result !== "unresolved"
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
	return scalarLiteral("query-var", value) as ComputeExpr<K>
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
	right: R
): ComputeExpr<OperandKind<L> & OperandKind<R> & ("u64" | "i64" | "f64")> {
	return binary("add", left, right)
}

function subtract<L extends ArithmeticOperand, R extends ArithmeticOperand>(
	left: L,
	right: R
): ComputeExpr<OperandKind<L> & OperandKind<R> & ("u64" | "i64" | "f64")> {
	return binary("subtract", left, right)
}

function multiply<L extends ArithmeticOperand, R extends ArithmeticOperand>(
	left: L,
	right: R
): ComputeExpr<OperandKind<L> & OperandKind<R> & ("u64" | "i64" | "f64")> {
	return binary("multiply", left, right)
}

function divide<L extends ArithmeticOperand, R extends ArithmeticOperand>(
	left: L,
	right: R
): ComputeExpr<OperandKind<L> & OperandKind<R> & ("u64" | "i64" | "f64")> {
	return binary("divide", left, right)
}

function negate<O extends ArithmeticOperand>(operand: O): ComputeExpr<OperandKind<O> & ("i64" | "f64")> {
	const where = "Compute.negate"
	return scalarNegate(where, asQueryNode(where, operand)) as ComputeExpr<OperandKind<O> & ("i64" | "f64")>
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

function isNaN(operand: FloatOperand): ComputeExpr<"bool"> {
	const where = "Compute.isNaN"
	return scalarFloatPredicate(where, "isNaN", asQueryNode(where, operand))
}

function isFinite(operand: FloatOperand): ComputeExpr<"bool"> {
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
	toF64,
	toF64Exact,
	toI64Exact,
	toU64Exact,
	isNaN,
	isFinite
})

export type { AnyComputeExpr, ComputeExpr, ComputeValue, QueryNode }
export { Compute, computeFieldOf, computeVarsOf, isComputeExpr, MAX_SCALAR_DEPTH as MAX_COMPUTE_DEPTH }
