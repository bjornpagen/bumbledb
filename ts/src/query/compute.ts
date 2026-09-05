/**
 * Computed find terms (C05 `FindTerm::Compute(ScalarExpr)`), TypeScript
 * authoring — the query-side scalar expression over the RULE'S OWN bound
 * VARIABLES. This is the variable-addressed sibling of the name-addressed
 * migration `ScalarExpr` (`#scalar.ts`): same operator roster, same engine
 * (`crates/bumbledb/src/scalar.rs` — one `NumericalGuard` per whole
 * operation, canonicalize-after-every-node, checked integer arithmetic),
 * but its leaves are query variables that lower to `VarId`s, exactly like
 * every other term position.
 *
 * The construction walls mirror the engine's `ScalarExpr::result_type`
 * EXACTLY (no mixed promotion — a refused tree here is the same tree the
 * engine would refuse at prepare):
 *
 *   - binary arithmetic takes two operands of ONE numeric kind
 *     (u64/i64/f64 — never mixed, never implicit),
 *   - `negate` takes i64 or f64 (u64 negation does not exist),
 *   - `isNaN`/`isFinite` take f64 and yield bool,
 *   - casts take any numeric kind and yield their target kind,
 *   - literals are EXPLICITLY tagged (`Compute.u64(5n)`, `Compute.i64(-3n)`,
 *     `Compute.f64(0.5)`, `Compute.bool(true)`) — a bare bigint is
 *     ambiguous between u64 and i64, so there is no untagged literal,
 *   - a CLOSED reference never enters arithmetic (declaration-id order is
 *     an accident, not semantics),
 *   - trees deeper than the engine's 128-node bound refuse at construction.
 *
 * A `ComputeExpr` is a legal find entry beside projected variables and
 * aggregates: `find({ score, scaled: Compute.multiply(score, Compute.f64(2)) })`.
 * It is a PROJECT column computed once per binding — not a fold, not part
 * of the group key — and the recursive head stays projection-only (compute
 * is refused there like every non-var entry).
 *
 * WIRE (agreed with P06R2, landed in `#native.ts`/`ts/crate`):
 * `FindTermIr` carries `{ kind: "compute", expr: ScalarExprIr }`,
 * `HeadTermIr` carries `{ kind: "compute" }`, and `ScalarExprIr` is the
 * ten-arm roster (`var`/`literal`/`negate`/`add`/`subtract`/`multiply`/
 * `divide`/`cast`/`isNaN`/`isFinite`) lowering 1:1 onto
 * `bumbledb::ScalarExpr` (`marshal.rs::scalar_expr_in`, depth-capped at
 * the same 128).
 */
import { AuthoringError } from "#errors.ts"
import type { AnyField, Infer } from "#fields.ts"
import { bool as boolField, f64 as f64Field, i64 as i64Field, rosterOf, u64 as u64Field } from "#fields.ts"
import type { NumericCast } from "#scalar.ts"
import type { ValueSpec } from "#spec.ts"
import type { AnyVar } from "#query/scope.ts"
import { isTerm, term } from "#query/scope.ts"

/** The engine's scalar result vocabulary (`I64 | U64 | F64 | Bool`). */
type ComputeKind = "u64" | "i64" | "f64" | "bool"

/** The tagged literal subset a scalar expression may carry. */
type ComputeLiteral = Extract<ValueSpec, { readonly kind: ComputeKind }>

/**
 * The inert authoring node data. `var` keeps the AUTHORING reference (the
 * minted variable object) — the rule lowering assigns its `VarId` exactly
 * like every atom/condition position; everything else is already
 * wire-shaped.
 */
type ComputeData =
	| { readonly kind: "var"; readonly ref: AnyVar }
	| { readonly kind: "literal"; readonly value: ComputeLiteral }
	| { readonly kind: "negate"; readonly expr: ComputeData }
	| { readonly kind: "isNaN"; readonly expr: ComputeData }
	| { readonly kind: "isFinite"; readonly expr: ComputeData }
	| { readonly kind: "add"; readonly left: ComputeData; readonly right: ComputeData }
	| { readonly kind: "subtract"; readonly left: ComputeData; readonly right: ComputeData }
	| { readonly kind: "multiply"; readonly left: ComputeData; readonly right: ComputeData }
	| { readonly kind: "divide"; readonly left: ComputeData; readonly right: ComputeData }
	| { readonly kind: "cast"; readonly cast: NumericCast; readonly expr: ComputeData }

/** The engine's tree bound (`scalar.rs` `TooDeep` at depth > 128). */
const MAX_COMPUTE_DEPTH = 128

const computeTag: unique symbol = Symbol("bumbledb.query.compute")

/** The phantom result slot: typing aid only, never runtime data. */
declare const computeResult: unique symbol

/**
 * A typed computed find entry: inert node data, the DERIVED result kind
 * (computed at construction with the engine's exact rules), the tree depth
 * (for the 128 wall), and a phantom host type. Frozen; identity carries no
 * rule scope — like a variable, it means something only where its
 * variables are bound.
 */
interface ComputeExpr<T> {
	readonly [computeTag]: "compute"
	readonly node: ComputeData
	readonly result: ComputeKind
	readonly depth: number
	readonly [computeResult]?: T
}

type AnyComputeExpr = ComputeExpr<bigint> | ComputeExpr<number> | ComputeExpr<boolean>

/** The host value a result kind decodes to (`Infer`'s scalar half). */
type ComputeValue<K extends ComputeKind> = K extends "f64" ? number : K extends "bool" ? boolean : bigint

/** One operand as the RUNTIME judgment sees it (the leaf lift). */
type Operand = AnyVar | AnyComputeExpr

/**
 * The compile-time operand tiers (the runtime `judge`/`varKindOf` walls'
 * type twins — best effort: u64-vs-i64 sameness and closedness stay
 * runtime/prepare judgments, the kinds here are the structural screen).
 */
type NumericVarOperand = AnyVar & { readonly field: { readonly kind: "u64" | "i64" | "f64" } }
type ArithmeticOperand = ComputeExpr<bigint> | ComputeExpr<number> | NumericVarOperand
type FloatOperand = ComputeExpr<number> | (AnyVar & { readonly field: { readonly kind: "f64" } })

/**
 * One operand's HOST value type: an expression's phantom, or the
 * variable's inferred field value screened to the scalar kinds (a closed
 * reference's handle union intersects to `never`, so it cannot type an
 * arithmetic result even before the runtime wall fires). Binary results
 * intersect both sides, so a bigint-vs-number pair yields `never` — the
 * compile-tier twin of the engine's no-mixed-promotion rule at the
 * bigint/number grain (u64-vs-i64 sameness stays runtime/prepare).

type OperandValue<O> = O extends ComputeExpr<infer T>
	? T
	: O extends AnyVar
		? Infer<O["field"]> & (bigint | number)
		: never

function isComputeExpr(value: unknown): value is AnyComputeExpr {
	return typeof value === "object" && value !== null && computeTag in value
}

/** A numeric- or bool-kinded variable's compute kind; everything else refuses. */
function varKindOf(where: string, ref: AnyVar): ComputeKind {
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

interface Judged {
	readonly node: ComputeData
	readonly kind: ComputeKind
	readonly depth: number
}

/** Lifts one operand to (node, derived kind, depth) — the leaf judgment. */
function judge(where: string, operand: Operand): Judged {
	if (isComputeExpr(operand)) {
		return { node: operand.node, kind: operand.result, depth: operand.depth }
	}
	if (isTerm(operand) && operand[term] === "var") {
		return {
			node: Object.freeze({ kind: "var" as const, ref: operand }),
			kind: varKindOf(where, operand),
			depth: 1
		}
	}
	throw new AuthoringError({
		message: `${where}: expected a query variable or a Compute expression — literals are tagged (Compute.u64/i64/f64/bool)`
	})
}

function admit<T>(node: ComputeData, result: ComputeKind, depth: number): ComputeExpr<T> {
	if (depth > MAX_COMPUTE_DEPTH) {
		throw new AuthoringError({
			message: `compute: the expression is deeper than ${MAX_COMPUTE_DEPTH} nodes (the engine's scalar depth bound)`
		})
	}
	return Object.freeze({ [computeTag]: "compute" as const, node: Object.freeze(node), result, depth }) as ComputeExpr<T>
}

function literal<T>(value: ComputeLiteral): ComputeExpr<T> {
	return admit<T>({ kind: "literal", value: Object.freeze(value) }, value.kind, 1)
}

const U64_MAX = (1n << 64n) - 1n
const I64_MIN = -(1n << 63n)
const I64_MAX = (1n << 63n) - 1n

/** An explicit u64 literal (bigint, `0..=2^64-1`). */
function u64(value: bigint): ComputeExpr<bigint> {
	if (typeof value !== "bigint" || value < 0n || value > U64_MAX) {
		throw new AuthoringError({ message: "Compute.u64: a u64 literal is a bigint in 0..=2^64-1" })
	}
	return literal({ kind: "u64", value })
}

/** An explicit i64 literal (bigint, `-2^63..=2^63-1`). */
function i64(value: bigint): ComputeExpr<bigint> {
	if (typeof value !== "bigint" || value < I64_MIN || value > I64_MAX) {
		throw new AuthoringError({ message: "Compute.i64: an i64 literal is a bigint in -2^63..=2^63-1" })
	}
	return literal({ kind: "i64", value })
}

/** An explicit f64 literal (the native boundary canonicalizes NaN and -0). */
function f64(value: number): ComputeExpr<number> {
	if (typeof value !== "number") {
		throw new AuthoringError({ message: "Compute.f64: an f64 literal is a number" })
	}
	return literal({ kind: "f64", value })
}

/** An explicit bool literal (a predicate result column's constant arm). */
function bool(value: boolean): ComputeExpr<boolean> {
	if (typeof value !== "boolean") {
		throw new AuthoringError({ message: "Compute.bool: a bool literal is a boolean" })
	}
	return literal({ kind: "bool", value })
}

function numericOnly(where: string, judged: Judged): void {
	if (judged.kind === "bool") {
		throw new AuthoringError({ message: `${where}: the operand is bool, not numeric (u64/i64/f64)` })
	}
}

type BinaryKind = "add" | "subtract" | "multiply" | "divide"

function binary<T>(op: BinaryKind, left: ArithmeticOperand, right: ArithmeticOperand): ComputeExpr<T> {
	const where = `Compute.${op}`
	const a = judge(where, left)
	const b = judge(where, right)
	numericOnly(where, a)
	numericOnly(where, b)
	if (a.kind !== b.kind) {
		throw new AuthoringError({
			message: `${where}: operand kinds differ (${a.kind} vs ${b.kind}) — the engine has no mixed promotion; cast explicitly (Compute.toF64/ toF64Exact/ toI64Exact/ toU64Exact)`
		})
	}
	return admit<T>({ kind: op, left: a.node, right: b.node }, a.kind, Math.max(a.depth, b.depth) + 1)
}

/** Checked same-kind addition (`i128/u128`-exact integers; binary64 floats). */
function add<L extends ArithmeticOperand, R extends ArithmeticOperand>(
	left: L,
	right: R
): ComputeExpr<OperandValue<L> & OperandValue<R>> {
	return binary("add", left, right)
}

function subtract<L extends ArithmeticOperand, R extends ArithmeticOperand>(
	left: L,
	right: R
): ComputeExpr<OperandValue<L> & OperandValue<R>> {
	return binary("subtract", left, right)
}

function multiply<L extends ArithmeticOperand, R extends ArithmeticOperand>(
	left: L,
	right: R
): ComputeExpr<OperandValue<L> & OperandValue<R>> {
	return binary("multiply", left, right)
}

/** Same-kind division; integer division by zero is the engine's typed error. */
function divide<L extends ArithmeticOperand, R extends ArithmeticOperand>(
	left: L,
	right: R
): ComputeExpr<OperandValue<L> & OperandValue<R>> {
	return binary("divide", left, right)
}

/** Negation over i64/f64 (u64 negation does not exist — cast first). */
function negate<O extends ArithmeticOperand>(operand: O): ComputeExpr<OperandValue<O>> {
	const judged = judge("Compute.negate", operand)
	if (judged.kind !== "i64" && judged.kind !== "f64") {
		throw new AuthoringError({
			message: `Compute.negate: the operand is ${judged.kind} — negation is defined over i64 and f64 only`
		})
	}
	return admit<OperandValue<O>>({ kind: "negate", expr: judged.node }, judged.kind, judged.depth + 1)
}

function cast(where: string, kind: NumericCast, result: ComputeKind, operand: ArithmeticOperand): Judged {
	const judged = judge(where, operand)
	numericOnly(where, judged)
	return {
		node: Object.freeze({ kind: "cast" as const, cast: kind, expr: judged.node }),
		kind: result,
		depth: judged.depth + 1
	}
}

/** Rounding cast to binary64 (round-to-nearest ties-to-even). */
function toF64(operand: ArithmeticOperand): ComputeExpr<number> {
	const judged = cast("Compute.toF64", "toF64", "f64", operand)
	return admit<number>(judged.node, judged.kind, judged.depth)
}

/** Exact cast to binary64: refuses at evaluation when rounding would occur. */
function toF64Exact(operand: ArithmeticOperand): ComputeExpr<number> {
	const judged = cast("Compute.toF64Exact", "toF64Exact", "f64", operand)
	return admit<number>(judged.node, judged.kind, judged.depth)
}

/** Exact cast to i64: refuses at evaluation on any fractional/overflow loss. */
function toI64Exact(operand: ArithmeticOperand): ComputeExpr<bigint> {
	const judged = cast("Compute.toI64Exact", "toI64Exact", "i64", operand)
	return admit<bigint>(judged.node, judged.kind, judged.depth)
}

/** Exact cast to u64: refuses at evaluation on any fractional/sign/overflow loss. */
function toU64Exact(operand: ArithmeticOperand): ComputeExpr<bigint> {
	const judged = cast("Compute.toU64Exact", "toU64Exact", "u64", operand)
	return admit<bigint>(judged.node, judged.kind, judged.depth)
}

function floatPredicate(op: "isNaN" | "isFinite", operand: FloatOperand): ComputeExpr<boolean> {
	const where = `Compute.${op}`
	const judged = judge(where, operand)
	if (judged.kind !== "f64") {
		throw new AuthoringError({ message: `${where}: the operand is ${judged.kind} — the float predicates read f64` })
	}
	return admit<boolean>({ kind: op, expr: judged.node }, "bool", judged.depth + 1)
}

/** The explicit NaN predicate (host `===` is not a database predicate). */
function isNaN(operand: FloatOperand): ComputeExpr<boolean> {
	return floatPredicate("isNaN", operand)
}

function isFinite(operand: FloatOperand): ComputeExpr<boolean> {
	return floatPredicate("isFinite", operand)
}

/** Every variable the expression reads (validation binds them like any term). */
function computeVarsOf(data: ComputeData): readonly AnyVar[] {
	const vars: AnyVar[] = []
	const pending: ComputeData[] = [data]
	while (pending.length > 0) {
		const node = pending.pop()
		if (node === undefined) {
			break
		}
		switch (node.kind) {
			case "var":
				vars.push(node.ref)
				break
			case "literal":
				break
			case "negate":
			case "isNaN":
			case "isFinite":
			case "cast":
				pending.push(node.expr)
				break
			case "add":
			case "subtract":
			case "multiply":
			case "divide":
				pending.push(node.left, node.right)
				break
		}
	}
	return vars
}

/** The derived output column's descriptor (bare class — a computed value
 * is not the carrier any law tracked). */
function computeFieldOf(kind: ComputeKind): AnyField {
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

/**
 * The one authoring namespace (mirror of `Scalar` for migrations):
 * `Compute.multiply(score, Compute.f64(2))` inside a rule's `find`.
 */
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

export type { AnyComputeExpr, ComputeData, ComputeExpr, ComputeKind, ComputeLiteral, ComputeValue }
export { Compute, computeFieldOf, computeVarsOf, isComputeExpr, MAX_COMPUTE_DEPTH }
