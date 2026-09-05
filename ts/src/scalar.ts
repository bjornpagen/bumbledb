/**
 * The ONE leaf-scoped scalar AST (C1/C8): one tagged literal grammar and
 * operator roster, parameterized by {@link ScalarLeafScope}. TypeScript never
 * evaluates — the native compiler binds source fields and the engine
 * `ScalarEvaluator` executes. Construction is constant work against cached
 * depth/kind summaries; it does not re-walk the tree.
 *
 * Query-var leaves carry a known schema kind. Source-field leaves stay
 * `unresolved` until native snapshot binding. A result kind is recorded only
 * when it is derivable from known operands; otherwise it stays unresolved.
 * Known incompatible kinds refuse here. Unresolved trees are inert metadata,
 * not typechecked programs.
 *
 * Wire spellings (the grammar arms, not a second flattened interpretation):
 *   migration plan JSON — field/literal/operator nodes; one-arm literals (`{ u64: n }`)
 *   query IR — `{ kind: "literal", value: ValueSpec }` via `literalWireOf`
 */
import { AuthoringError } from "#errors.ts"
import type { ValueSpec } from "#spec.ts"

/** F0: leaf scope for the shared AST (C1). Query vars are typed; source fields stay unresolved. */
export type ScalarLeafScope = "query-var" | "source-field"

/** The engine's scalar result vocabulary — distinct at the type level. */
type ScalarKind = "u64" | "i64" | "f64" | "bool"

/** Cached result kind: known only when derivable; otherwise honestly unresolved. */
type ScalarResultKind = ScalarKind | "unresolved"

/** Host value for a derived scalar kind. */
type ScalarValue<K extends ScalarKind> = K extends "f64" ? number : K extends "bool" ? boolean : bigint

type NumericCast = "toF64" | "toF64Exact" | "toI64Exact" | "toU64Exact"

type NumericKind = "u64" | "i64" | "f64"

type NumericOrUnresolved = NumericKind | "unresolved"

/**
 * Combine known numeric kinds, or stay unresolved when a source-field leaf
 * still needs native binding. Distinct known kinds collapse to `never`
 * (static refusal) and throw at the constructor.
 */
type CombineNumeric<L, R> = L extends "unresolved"
	? R extends "bool"
		? never
		: "unresolved"
	: R extends "unresolved"
		? L extends "bool"
			? never
			: "unresolved"
		: L extends R
			? L
			: never

/** The tagged literal subset shared by every leaf scope. */
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

type SourceFieldLeaf = { readonly kind: "field"; readonly name: string }

type ScalarLeaf<S extends ScalarLeafScope> = S extends "query-var" ? QueryVarLeaf : SourceFieldLeaf

type ScalarNodeBody<S extends ScalarLeafScope> =
	| ScalarLeaf<S>
	| { readonly kind: "literal"; readonly value: ScalarLiteral }
	| { readonly kind: "negate"; readonly expr: ScalarNode<S> }
	| { readonly kind: "isNaN"; readonly expr: ScalarNode<S> }
	| { readonly kind: "isFinite"; readonly expr: ScalarNode<S> }
	| { readonly kind: "add"; readonly left: ScalarNode<S>; readonly right: ScalarNode<S> }
	| { readonly kind: "subtract"; readonly left: ScalarNode<S>; readonly right: ScalarNode<S> }
	| { readonly kind: "multiply"; readonly left: ScalarNode<S>; readonly right: ScalarNode<S> }
	| { readonly kind: "divide"; readonly left: ScalarNode<S>; readonly right: ScalarNode<S> }
	| { readonly kind: "cast"; readonly cast: NumericCast; readonly expr: ScalarNode<S> }

/**
 * One authored node: roster arms plus cached `scope` / `result` / `depth`.
 * Summaries are construction metadata; lowering and L17 serialization read
 * the grammar arms and ignore the summaries.
 */
type ScalarNode<S extends ScalarLeafScope = ScalarLeafScope, K extends ScalarResultKind = ScalarResultKind> =
	ScalarNodeBody<S> & {
		readonly scope: S
		readonly result: K
		readonly depth: number
	}

/**
 * Migration authoring expression (source-field scope). `K` is the cached
 * result kind. Callers that index by a host value (`unknown` / `bigint`)
 * receive the unresolved-capable source-field node — they cannot assert a
 * field kind.
 */
type ScalarExpr<K = unknown> = ScalarNode<"source-field", K extends ScalarResultKind ? K : ScalarResultKind>

/** A migration field read: unresolved until native snapshot binding. */
type ScalarFieldRef = ScalarNode<"source-field", "unresolved"> & SourceFieldLeaf

const MAX_SCALAR_DEPTH = 128
const U64_MAX = (1n << 64n) - 1n
const I64_MIN = -(1n << 63n)
const I64_MAX = (1n << 63n) - 1n

/** Constructor admissions only — not descendant visits. D27 construction-work pin. */
let authoringWork = 0

function scalarAuthoringWork(): number {
	return authoringWork
}

function admit<S extends ScalarLeafScope, K extends ScalarResultKind>(
	where: string,
	node: ScalarNodeBody<S> & { readonly scope: S; readonly result: K; readonly depth: number }
): ScalarNode<S, K> {
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

function assertNumeric(where: string, kind: ScalarKind): void {
	if (kind === "bool") {
		throw new AuthoringError({ message: `${where}: the operand is bool, not numeric (u64/i64/f64)` })
	}
}

function assertSameKind(where: string, left: ScalarKind, right: ScalarKind): ScalarKind {
	if (left !== right) {
		throw new AuthoringError({
			message: `${where}: operand kinds differ (${left} vs ${right}) — the engine has no mixed promotion; cast explicitly (Scalar.toF64/ toF64Exact/ toI64Exact/ toU64Exact)`
		})
	}
	return left
}

function assertScope<S extends ScalarLeafScope>(where: string, left: ScalarNode<S>, right: ScalarNode<S>): S {
	if (left.scope !== right.scope) {
		throw new AuthoringError({
			message: `${where}: leaf scopes differ (${left.scope} vs ${right.scope}) — query-var and source-field trees do not mix`
		})
	}
	return left.scope
}

function combineNumeric(where: string, left: ScalarResultKind, right: ScalarResultKind): ScalarResultKind {
	if (left === "unresolved" || right === "unresolved") {
		if (left !== "unresolved") {
			assertNumeric(where, left)
		}
		if (right !== "unresolved") {
			assertNumeric(where, right)
		}
		return "unresolved"
	}
	assertNumeric(where, left)
	assertNumeric(where, right)
	return assertSameKind(where, left, right)
}

function negateKind(where: string, inner: ScalarResultKind): ScalarResultKind {
	if (inner === "unresolved") {
		return "unresolved"
	}
	if (inner !== "i64" && inner !== "f64") {
		throw new AuthoringError({
			message: `${where}: the operand is ${inner} — negation is defined over i64 and f64 only`
		})
	}
	return inner
}

function assertCastOperand(where: string, inner: ScalarResultKind): void {
	if (inner === "unresolved") {
		return
	}
	assertNumeric(where, inner)
}

function assertFloatOperand(where: string, inner: ScalarResultKind): void {
	if (inner === "unresolved") {
		return
	}
	if (inner !== "f64") {
		throw new AuthoringError({ message: `${where}: the operand is ${inner} — the float predicates read f64` })
	}
}

function isScalarNode(value: unknown): value is ScalarNode {
	return (
		typeof value === "object" &&
		value !== null &&
		"kind" in value &&
		"scope" in value &&
		"result" in value &&
		"depth" in value
	)
}

function isUnresolvedScalar(node: ScalarNode): boolean {
	return node.result === "unresolved"
}

/** Source-field leaf — unresolved until L14 binds it against the verified snapshot. */
function sourceField(name: string): ScalarFieldRef {
	if (typeof name !== "string" || name.length === 0) {
		throw new AuthoringError({ message: "scalar field(...) names a nonempty source field" })
	}
	return admit("Scalar.field", {
		kind: "field",
		name,
		scope: "source-field",
		result: "unresolved",
		depth: 1
	})
}

/** Query-var leaf — kind is the variable's schema kind, already known. */
function queryVarLeaf<K extends ScalarKind>(ref: ScalarQueryVar, kind: K): ScalarNode<"query-var", K> {
	return admit("Scalar.queryVar", {
		kind: "var",
		ref,
		scope: "query-var",
		result: kind,
		depth: 1
	})
}

function scalarLiteral<S extends ScalarLeafScope>(scope: S, value: ScalarLiteral): ScalarNode<S, ScalarKind> {
	const kind = literalKindOf(value)
	return admit("Scalar.literal", {
		kind: "literal",
		value: literalValue(value),
		scope,
		result: kind,
		depth: 1
	}) as ScalarNode<S, ScalarKind>
}

type BinaryOp = "add" | "subtract" | "multiply" | "divide"

function scalarBinary<S extends ScalarLeafScope>(
	where: string,
	op: BinaryOp,
	left: ScalarNode<S>,
	right: ScalarNode<S>
): ScalarNode<S, ScalarResultKind> {
	const scope = assertScope(where, left, right)
	return admit(where, {
		kind: op,
		left,
		right,
		scope,
		result: combineNumeric(where, left.result, right.result),
		depth: Math.max(left.depth, right.depth) + 1
	})
}

function scalarNegate<S extends ScalarLeafScope>(where: string, expr: ScalarNode<S>): ScalarNode<S, ScalarResultKind> {
	return admit(where, {
		kind: "negate",
		expr,
		scope: expr.scope,
		result: negateKind(where, expr.result),
		depth: expr.depth + 1
	})
}

function scalarCast<S extends ScalarLeafScope>(
	where: string,
	cast: NumericCast,
	result: ScalarKind,
	expr: ScalarNode<S>
): ScalarNode<S, ScalarKind> {
	assertCastOperand(where, expr.result)
	return admit(where, {
		kind: "cast",
		cast,
		expr,
		scope: expr.scope,
		result,
		depth: expr.depth + 1
	})
}

function scalarFloatPredicate<S extends ScalarLeafScope>(
	where: string,
	op: "isNaN" | "isFinite",
	expr: ScalarNode<S>
): ScalarNode<S, "bool"> {
	assertFloatOperand(where, expr.result)
	return admit(where, {
		kind: op,
		expr,
		scope: expr.scope,
		result: "bool",
		depth: expr.depth + 1
	})
}

/** Query-var leaves collected once at find-binding time — not during construction. */
function queryVarsOf(node: ScalarNode<"query-var">): readonly ScalarQueryVar[] {
	const vars: ScalarQueryVar[] = []
	const pending: ScalarNode<"query-var">[] = [node]
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
			case "negate":
			case "isNaN":
			case "isFinite":
			case "cast":
				pending.push(next.expr)
				break
			case "add":
			case "subtract":
			case "multiply":
			case "divide":
				pending.push(next.left, next.right)
				break
			default:
				throw new AuthoringError({
					message: "queryVarsOf: a query-var tree cannot carry a source-field leaf"
				})
		}
	}
	return vars
}

function field(name: string): ScalarFieldRef {
	return sourceField(name)
}

function literal(value: ScalarLiteral): ScalarExpr<ScalarKind> {
	return scalarLiteral("source-field", value)
}

function u64(value: bigint): ScalarExpr<"u64"> {
	checkU64(value, "Scalar.u64")
	return scalarLiteral("source-field", { u64: value }) as ScalarExpr<"u64">
}

function i64(value: bigint): ScalarExpr<"i64"> {
	checkI64(value, "Scalar.i64")
	return scalarLiteral("source-field", { i64: value }) as ScalarExpr<"i64">
}

function f64(value: number): ScalarExpr<"f64"> {
	checkF64(value, "Scalar.f64")
	return scalarLiteral("source-field", { f64: value }) as ScalarExpr<"f64">
}

function bool(value: boolean): ScalarExpr<"bool"> {
	checkBool(value, "Scalar.bool")
	return scalarLiteral("source-field", { bool: value }) as ScalarExpr<"bool">
}

function negate<K extends "i64" | "f64" | "unresolved">(expr: ScalarExpr<K>): ScalarExpr<K> {
	return scalarNegate("Scalar.negate", expr) as ScalarExpr<K>
}

function add<
	L extends NumericOrUnresolved,
	R extends CombineNumeric<L, R> extends never ? never : NumericOrUnresolved
>(left: ScalarExpr<L>, right: ScalarExpr<R>): ScalarExpr<CombineNumeric<L, R>> {
	return scalarBinary("Scalar.add", "add", left, right) as ScalarExpr<CombineNumeric<L, R>>
}

function subtract<
	L extends NumericOrUnresolved,
	R extends CombineNumeric<L, R> extends never ? never : NumericOrUnresolved
>(left: ScalarExpr<L>, right: ScalarExpr<R>): ScalarExpr<CombineNumeric<L, R>> {
	return scalarBinary("Scalar.subtract", "subtract", left, right) as ScalarExpr<CombineNumeric<L, R>>
}

function multiply<
	L extends NumericOrUnresolved,
	R extends CombineNumeric<L, R> extends never ? never : NumericOrUnresolved
>(left: ScalarExpr<L>, right: ScalarExpr<R>): ScalarExpr<CombineNumeric<L, R>> {
	return scalarBinary("Scalar.multiply", "multiply", left, right) as ScalarExpr<CombineNumeric<L, R>>
}

function divide<
	L extends NumericOrUnresolved,
	R extends CombineNumeric<L, R> extends never ? never : NumericOrUnresolved
>(left: ScalarExpr<L>, right: ScalarExpr<R>): ScalarExpr<CombineNumeric<L, R>> {
	return scalarBinary("Scalar.divide", "divide", left, right) as ScalarExpr<CombineNumeric<L, R>>
}

function toF64(expr: ScalarExpr<NumericOrUnresolved>): ScalarExpr<"f64"> {
	return scalarCast("Scalar.toF64", "toF64", "f64", expr) as ScalarExpr<"f64">
}

function toF64Exact(expr: ScalarExpr<NumericOrUnresolved>): ScalarExpr<"f64"> {
	return scalarCast("Scalar.toF64Exact", "toF64Exact", "f64", expr) as ScalarExpr<"f64">
}

function toI64Exact(expr: ScalarExpr<NumericOrUnresolved>): ScalarExpr<"i64"> {
	return scalarCast("Scalar.toI64Exact", "toI64Exact", "i64", expr) as ScalarExpr<"i64">
}

function toU64Exact(expr: ScalarExpr<NumericOrUnresolved>): ScalarExpr<"u64"> {
	return scalarCast("Scalar.toU64Exact", "toU64Exact", "u64", expr) as ScalarExpr<"u64">
}

function isNaN(expr: ScalarExpr<"f64" | "unresolved">): ScalarExpr<"bool"> {
	return scalarFloatPredicate("Scalar.isNaN", "isNaN", expr)
}

function isFinite(expr: ScalarExpr<"f64" | "unresolved">): ScalarExpr<"bool"> {
	return scalarFloatPredicate("Scalar.isFinite", "isFinite", expr)
}

/** The authoring namespace: `Scalar.field("units")`, `Scalar.add(...)`. */
const Scalar = Object.freeze({
	field,
	literal,
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

export type {
	CombineNumeric,
	NumericCast,
	NumericKind,
	NumericOrUnresolved,
	QueryVarLeaf,
	ScalarExpr,
	ScalarFieldRef,
	ScalarKind,
	ScalarLiteral,
	ScalarNode,
	ScalarOpKind,
	ScalarQueryVar,
	ScalarResultKind,
	ScalarValue,
	SourceFieldLeaf
}
export {
	MAX_SCALAR_DEPTH,
	Scalar,
	assertDepth,
	assertNumeric,
	assertSameKind,
	checkBool,
	checkF64,
	checkI64,
	checkU64,
	isScalarNode,
	isUnresolvedScalar,
	literalKindOf,
	literalValue,
	literalWireOf,
	queryVarLeaf,
	queryVarsOf,
	scalarAuthoringWork,
	scalarBinary,
	scalarCast,
	scalarFloatPredicate,
	scalarLiteral,
	scalarNegate
}
