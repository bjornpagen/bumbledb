/** Query scalar nodes, with cached kind and depth. Construction is constant
 * work; native preparation binds variables and evaluates the expressions. */
import { AuthoringError } from "#errors.ts"
import type { ScalarExprIr } from "#native.ts"
import type { ValueSpec } from "#spec.ts"

/** The engine's scalar result vocabulary — distinct at the type level. */
type ScalarKind = "u64" | "i64" | "f64" | "bool"

/** Cached result kind: known only when derivable; otherwise honestly unresolved. */
type IntervalKind = "intervalU64" | "intervalI64" | "intervalF64"
type ScalarInputKind = ScalarKind | IntervalKind
type ScalarResultKind = ScalarInputKind
type Rounding = "towardZero" | "nearestTiesAwayFromZero" | "nearestTiesToEven"

/** Host value for a derived scalar kind. */
type ScalarValue<K extends ScalarKind> = K extends "f64" ? number : K extends "bool" ? boolean : bigint

type NumericCast = "toF64" | "toF64Exact" | "toI64Exact" | "toU64Exact"

type NumericKind = "u64" | "i64" | "f64"

type ScalarLiteral =
	| { readonly bool: boolean }
	| { readonly u64: bigint }
	| { readonly i64: bigint }
	| { readonly f64: number }

type ScalarOpKind =
	| "literal"
	| "negate"
	| "isNaN"
	| "isFinite"
	| "add"
	| "subtract"
	| "multiply"
	| "divide"
	| "cast"
	| "mulDiv"
	| "measure"

/**
 * Structural query-variable leaf. The query layer supplies a bound variable
 * whose field kind is already a schema kind; this module does not import
 * query types (scope → atom → compute → scalar would cycle).
 */
interface ScalarQueryVar {
	readonly label: string
	readonly field: { readonly kind: string }
}

type QueryVarLeaf = { readonly kind: "var"; readonly ref: ScalarQueryVar }

type ScalarNodeBody =
	| { readonly kind: "measure"; readonly expr: ScalarNode }
	| {
			readonly kind: "mulDiv"
			readonly a: ScalarNode
			readonly b: ScalarNode
			readonly divisor: ScalarNode
			readonly rounding: Rounding
	  }
	| QueryVarLeaf
	| { readonly kind: "literal"; readonly value: ScalarLiteral }
	| { readonly kind: "negate"; readonly expr: ScalarNode }
	| { readonly kind: "isNaN"; readonly expr: ScalarNode }
	| { readonly kind: "isFinite"; readonly expr: ScalarNode }
	| { readonly kind: "add"; readonly left: ScalarNode; readonly right: ScalarNode }
	| { readonly kind: "subtract"; readonly left: ScalarNode; readonly right: ScalarNode }
	| { readonly kind: "multiply"; readonly left: ScalarNode; readonly right: ScalarNode }
	| { readonly kind: "divide"; readonly left: ScalarNode; readonly right: ScalarNode }
	| { readonly kind: "cast"; readonly cast: NumericCast; readonly expr: ScalarNode }

/** Cached summaries never replace the expression grammar. */
type ScalarNode<K extends ScalarResultKind = ScalarResultKind> = ScalarNodeBody & {
	readonly result: K
	readonly depth: number
}

const MAX_SCALAR_DEPTH = 128
const U64_MAX = (1n << 64n) - 1n
const I64_MIN = -(1n << 63n)
const I64_MAX = (1n << 63n) - 1n

/** Constructor admissions only — not descendant visits. D27 construction-work pin. */
let authoringWork = 0

function scalarAuthoringWork(): number {
	return authoringWork
}

function admit<const Node extends { readonly result: ScalarResultKind; readonly depth: number }>(
	where: string,
	node: Node
): Readonly<Node> {
	authoringWork += 1
	assertDepth(where, node.depth)
	return Object.freeze(node)
}

function checkU64(value: bigint, where: string): void {
	if (typeof value !== "bigint" || value < 0n || value > U64_MAX) {
		throw new AuthoringError({ message: `${where}: a u64 literal is a bigint in 0..=2^64-1` })
	}
}

function checkI64(value: bigint, where: string): void {
	if (typeof value !== "bigint" || value < I64_MIN || value > I64_MAX) {
		throw new AuthoringError({ message: `${where}: an i64 literal is a bigint in -2^63..=2^63-1` })
	}
}

function checkF64(value: number, where: string): void {
	if (typeof value !== "number") {
		throw new AuthoringError({ message: `${where}: an f64 literal is a number` })
	}
}

function checkBool(value: boolean, where: string): void {
	if (typeof value !== "boolean") {
		throw new AuthoringError({ message: `${where}: a bool literal is a boolean` })
	}
}

function literalValue(value: ScalarLiteral): ScalarLiteral {
	return Object.freeze(value)
}

/** Converts a shared one-arm literal to the query wire `ValueSpec` spelling. */
function literalWireOf(value: ScalarLiteral): ValueSpec {
	if ("bool" in value) {
		return { kind: "bool", value: value.bool }
	}
	if ("u64" in value) {
		return { kind: "u64", value: value.u64 }
	}
	if ("i64" in value) {
		return { kind: "i64", value: value.i64 }
	}
	return { kind: "f64", value: value.f64 }
}

function literalKindOf(value: ScalarLiteral): ScalarKind {
	if ("bool" in value) {
		return "bool"
	}
	if ("u64" in value) {
		return "u64"
	}
	if ("i64" in value) {
		return "i64"
	}
	return "f64"
}

function assertDepth(where: string, depth: number): void {
	if (depth > MAX_SCALAR_DEPTH) {
		throw new AuthoringError({
			message: `${where}: the expression is deeper than ${MAX_SCALAR_DEPTH} nodes (the engine's scalar depth bound)`
		})
	}
}

function assertNumeric(where: string, kind: ScalarInputKind): asserts kind is NumericKind {
	if (kind !== "u64" && kind !== "i64" && kind !== "f64") {
		throw new AuthoringError({ message: `${where}: the operand is bool, not numeric (u64/i64/f64)` })
	}
}

function assertSameKind(where: string, left: ScalarInputKind, right: ScalarInputKind): ScalarInputKind {
	if (left !== right) {
		throw new AuthoringError({
			message: `${where}: operand kinds differ (${left} vs ${right}) — the engine has no mixed promotion; cast explicitly (Compute.toF64/ toF64Exact/ toI64Exact/ toU64Exact)`
		})
	}
	return left
}

function combineNumeric(where: string, left: ScalarResultKind, right: ScalarResultKind): ScalarResultKind {
	assertNumeric(where, left)
	assertNumeric(where, right)
	return assertSameKind(where, left, right)
}

function negateKind(where: string, inner: ScalarResultKind): ScalarResultKind {
	if (inner !== "i64" && inner !== "f64") {
		throw new AuthoringError({
			message: `${where}: the operand is ${inner} — negation is defined over i64 and f64 only`
		})
	}
	return inner
}

function assertCastOperand(where: string, inner: ScalarResultKind): void {
	assertNumeric(where, inner)
}

function assertFloatOperand(where: string, inner: ScalarResultKind): void {
	if (inner !== "f64") {
		throw new AuthoringError({ message: `${where}: the operand is ${inner} — the float predicates read f64` })
	}
}

function isScalarNode(value: unknown): value is ScalarNode {
	return typeof value === "object" && value !== null && "kind" in value && "result" in value && "depth" in value
}

/** Query-var leaf — kind is the variable's schema kind, already known. */
function queryVarLeaf<K extends ScalarInputKind>(ref: ScalarQueryVar, kind: K): ScalarNode<K> {
	return admit("Compute.variable", {
		kind: "var",
		ref,
		result: kind,
		depth: 1
	})
}

function scalarLiteral(value: ScalarLiteral): ScalarNode<ScalarKind> {
	const kind = literalKindOf(value)
	return admit("Compute.literal", {
		kind: "literal",
		value: literalValue(value),
		result: kind,
		depth: 1
	}) as ScalarNode<ScalarKind>
}

type BinaryOp = "add" | "subtract" | "multiply" | "divide"

function scalarBinary(where: string, op: BinaryOp, left: ScalarNode, right: ScalarNode): ScalarNode {
	return admit(where, {
		kind: op,
		left,
		right,
		result: combineNumeric(where, left.result, right.result),
		depth: Math.max(left.depth, right.depth) + 1
	})
}

function roundingMode(value: unknown): Rounding {
	if (value !== "towardZero" && value !== "nearestTiesAwayFromZero" && value !== "nearestTiesToEven")
		throw new AuthoringError({ message: "unknown integer rounding mode" })
	return value
}

function scalarMulDiv(
	where: string,
	a: ScalarNode,
	b: ScalarNode,
	divisor: ScalarNode,
	rounding: Rounding
): ScalarNode {
	let known: "i64" | "u64" | undefined
	for (const operand of [a, b, divisor]) {
		if ((operand.result !== "i64" && operand.result !== "u64") || (known !== undefined && known !== operand.result))
			throw new AuthoringError({ message: `${where}: operands must have one matching integer kind` })
		known = operand.result
	}
	return admit(where, {
		kind: "mulDiv",
		a,
		b,
		divisor,
		rounding: roundingMode(rounding),
		result: a.result,
		depth: Math.max(a.depth, b.depth, divisor.depth) + 1
	})
}

function scalarMeasure(where: string, expr: ScalarNode): ScalarNode {
	const kind = expr.result
	if (kind !== "intervalU64" && kind !== "intervalI64" && kind !== "intervalF64")
		throw new AuthoringError({ message: `${where}: measure requires an interval` })
	return admit(where, {
		kind: "measure",
		expr,
		result: ({ intervalF64: "f64", intervalI64: "u64", intervalU64: "u64" } as const)[kind],
		depth: expr.depth + 1
	})
}

function scalarNegate(where: string, expr: ScalarNode): ScalarNode {
	return admit(where, {
		kind: "negate",
		expr,
		result: negateKind(where, expr.result),
		depth: expr.depth + 1
	})
}

function scalarCast(where: string, cast: NumericCast, result: ScalarKind, expr: ScalarNode): ScalarNode<ScalarKind> {
	assertCastOperand(where, expr.result)
	return admit(where, {
		kind: "cast",
		cast,
		expr,
		result,
		depth: expr.depth + 1
	})
}

function scalarFloatPredicate(where: string, op: "isNaN" | "isFinite", expr: ScalarNode): ScalarNode<"bool"> {
	assertFloatOperand(where, expr.result)
	return admit(where, {
		kind: op,
		expr,
		result: "bool",
		depth: expr.depth + 1
	})
}

/** Query-var leaves collected once at find-binding time — not during construction. */
function queryVarsOf(node: ScalarNode): readonly ScalarQueryVar[] {
	const vars: ScalarQueryVar[] = []
	const pending: ScalarNode[] = [node]
	while (pending.length > 0) {
		const next = pending.pop()
		if (next === undefined) {
			break
		}
		switch (next.kind) {
			case "var":
				vars.push(next.ref)
				break
			case "literal":
				break
			case "measure":
			case "negate":
			case "isNaN":
			case "isFinite":
			case "cast":
				pending.push(next.expr)
				break
			case "mulDiv":
				pending.push(next.a, next.b, next.divisor)
				break
			case "add":
			case "subtract":
			case "multiply":
			case "divide":
				pending.push(next.left, next.right)
				break
		}
	}
	return vars
}

/** Lower each query variable through its existing binding. */
function scalarWire(node: ScalarNode, leaf: (node: QueryVarLeaf) => number): ScalarExprIr {
	const walk = (expr: ScalarNode): ScalarExprIr => scalarWire(expr, leaf)
	switch (node.kind) {
		case "var":
			return { kind: "var", var: leaf(node) }
		case "literal":
			return { kind: "literal", value: literalWireOf(node.value) }
		case "measure":
		case "negate":
		case "isNaN":
		case "isFinite":
			return { kind: node.kind, expr: walk(node.expr) }
		case "cast":
			return { kind: node.kind, cast: node.cast, expr: walk(node.expr) }
		case "mulDiv":
			return { kind: node.kind, a: walk(node.a), b: walk(node.b), divisor: walk(node.divisor), rounding: node.rounding }
		case "add":
		case "subtract":
		case "multiply":
		case "divide":
			return { kind: node.kind, left: walk(node.left), right: walk(node.right) }
	}
}

export type {
	IntervalKind,
	NumericCast,
	NumericKind,
	QueryVarLeaf,
	Rounding,
	ScalarInputKind,
	ScalarKind,
	ScalarLiteral,
	ScalarNode,
	ScalarOpKind,
	ScalarQueryVar,
	ScalarResultKind,
	ScalarValue
}
export {
	assertDepth,
	assertNumeric,
	assertSameKind,
	checkBool,
	checkF64,
	checkI64,
	checkU64,
	isScalarNode,
	literalKindOf,
	literalValue,
	literalWireOf,
	MAX_SCALAR_DEPTH,
	queryVarLeaf,
	queryVarsOf,
	roundingMode,
	scalarAuthoringWork,
	scalarBinary,
	scalarCast,
	scalarFloatPredicate,
	scalarLiteral,
	scalarMeasure,
	scalarMulDiv,
	scalarNegate,
	scalarWire
}
