/**
 * The frozen core scalar-expression roster (C01), TypeScript spelling —
 * the ONE typed scalar AST shared by the core engine, the migration
 * generator (`@bjornpagen/bumbledb-log/schema` imports it literally) and
 * the native plan codec. The roster mirrors `bumbledb::scalar::ScalarExpr`
 * exactly: field reads, literals, negation, the four binary arithmetic
 * operators, the four explicit numeric casts, and the two float
 * predicates. Nothing else — no closures, no module paths, no host
 * evaluation: a `ScalarExpr` is inert owned data that the NATIVE
 * `ScalarEvaluator` executes with the engine's exact deterministic
 * binary64/integer semantics (canonicalize-after-every-node, guarded
 * integer overflow). TypeScript never evaluates one.
 *
 * The runtime spelling is the canonical plan-expression grammar
 * (`crates/bumbledb-log/src/migration/plan.rs::parse_expr`):
 *
 *   { kind: "field", name }
 *   { kind: "literal", value }
 *   { kind: "negate" | "isNaN" | "isFinite", expr }
 *   { kind: "add" | "subtract" | "multiply" | "divide", left, right }
 *   { kind: "cast", cast: "toF64" | "toF64Exact" | "toI64Exact" | "toU64Exact", expr }
 *
 * The Rust `Var(FieldId)` arm crosses as the NAME-addressed `field` arm:
 * ordinal resolution happens at native plan compilation against the sealed
 * relation roster, exactly like every other name-to-id lowering.
 */
import { AuthoringError } from "#errors.ts"
import type { CellValue } from "#rows.ts"

/** The phantom result slot: typing aid only, never runtime data. */
declare const scalarResult: unique symbol

type NumericCast = "toF64" | "toF64Exact" | "toI64Exact" | "toU64Exact"

type ScalarNode =
	| { readonly kind: "field"; readonly name: string }
	| { readonly kind: "literal"; readonly value: CellValue }
	| { readonly kind: "negate"; readonly expr: ScalarNode }
	| { readonly kind: "isNaN"; readonly expr: ScalarNode }
	| { readonly kind: "isFinite"; readonly expr: ScalarNode }
	| { readonly kind: "add"; readonly left: ScalarNode; readonly right: ScalarNode }
	| { readonly kind: "subtract"; readonly left: ScalarNode; readonly right: ScalarNode }
	| { readonly kind: "multiply"; readonly left: ScalarNode; readonly right: ScalarNode }
	| { readonly kind: "divide"; readonly left: ScalarNode; readonly right: ScalarNode }
	| { readonly kind: "cast"; readonly cast: NumericCast; readonly expr: ScalarNode }

/**
 * A typed scalar expression: the tagged node data plus a phantom result
 * type. The phantom is OPTIONAL structure — plain roster-shaped data is
 * assignable, and the native codec re-judges every node regardless.
 */
type ScalarExpr<T> = ScalarNode & { readonly [scalarResult]?: T }

function admit<T>(node: ScalarNode): ScalarExpr<T> {
	return Object.freeze(node) as ScalarExpr<T>
}

/** Read the named field of the transform's source row. */
function field<T = unknown>(name: string): ScalarExpr<T> {
	if (typeof name !== "string" || name.length === 0) {
		throw new AuthoringError({ message: "scalar field(...) names a nonempty source field" })
	}
	return admit({ kind: "field", name })
}

/** An inert literal value (host spelling; the native codec canonicalizes). */
function literal<const T extends CellValue>(value: T): ScalarExpr<T> {
	return admit({ kind: "literal", value })
}

function negate<T extends bigint | number>(expr: ScalarExpr<T>): ScalarExpr<T> {
	return admit({ kind: "negate", expr })
}

function add<T extends bigint | number>(left: ScalarExpr<T>, right: ScalarExpr<T>): ScalarExpr<T> {
	return admit({ kind: "add", left, right })
}

function subtract<T extends bigint | number>(left: ScalarExpr<T>, right: ScalarExpr<T>): ScalarExpr<T> {
	return admit({ kind: "subtract", left, right })
}

function multiply<T extends bigint | number>(left: ScalarExpr<T>, right: ScalarExpr<T>): ScalarExpr<T> {
	return admit({ kind: "multiply", left, right })
}

function divide<T extends bigint | number>(left: ScalarExpr<T>, right: ScalarExpr<T>): ScalarExpr<T> {
	return admit({ kind: "divide", left, right })
}

/** Rounding cast to binary64 (round-to-nearest ties-to-even). */
function toF64(expr: ScalarExpr<bigint> | ScalarExpr<number>): ScalarExpr<number> {
	return admit({ kind: "cast", cast: "toF64", expr })
}

/** Exact cast to binary64: refuses at evaluation when rounding would occur. */
function toF64Exact(expr: ScalarExpr<bigint> | ScalarExpr<number>): ScalarExpr<number> {
	return admit({ kind: "cast", cast: "toF64Exact", expr })
}

/** Exact cast to i64: refuses at evaluation on any fractional/overflow loss. */
function toI64Exact(expr: ScalarExpr<bigint> | ScalarExpr<number>): ScalarExpr<bigint> {
	return admit({ kind: "cast", cast: "toI64Exact", expr })
}

/** Exact cast to u64: refuses at evaluation on any fractional/sign/overflow loss. */
function toU64Exact(expr: ScalarExpr<bigint> | ScalarExpr<number>): ScalarExpr<bigint> {
	return admit({ kind: "cast", cast: "toU64Exact", expr })
}

/** The explicit NaN predicate (host `===` is not a database predicate). */
function isNaN(expr: ScalarExpr<number>): ScalarExpr<boolean> {
	return admit({ kind: "isNaN", expr })
}

function isFinite(expr: ScalarExpr<number>): ScalarExpr<boolean> {
	return admit({ kind: "isFinite", expr })
}

/** The one authoring namespace: `Scalar.field("units")`, `Scalar.add(...)`. */
const Scalar = Object.freeze({
	field,
	literal,
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

export type { NumericCast, ScalarExpr, ScalarNode }
export { Scalar }
