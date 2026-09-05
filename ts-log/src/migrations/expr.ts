/**
 * Core `ScalarExpr` → canonical plan expression data (exact reuse, C01/C11).
 * The generator SERIALIZES the core's typed scalar AST; it never evaluates
 * it — the native executor lowers the plan onto `bumbledb::ScalarExpr` and
 * the core `ScalarEvaluator` executes it with the core's exact numeric
 * semantics, and native plan compilation re-judges every node and type. The
 * walk below is a bounded structural check with the same depth fence as the
 * native codec (`MAX_EXPR_DEPTH = 128`); any node outside the frozen core
 * roster (closure, module path, unknown kind) is an unsupported-transform
 * refusal, never a fallback.
 *
 * Assumed C01 core TS `ScalarExpr` runtime data (recorded in
 * implementation/packets/P10.md until P07's export lands) — the spelling is
 * identical to the plan JSON grammar in
 * `crates/bumbledb-log/src/migration/plan.rs::parse_expr`:
 *   { kind: "field", name } | { kind: "literal", value: <one-arm value> }
 *   | { kind: "negate" | "isNaN" | "isFinite", expr }
 *   | { kind: "add" | "subtract" | "multiply" | "divide", left, right }
 *   | { kind: "cast", cast: "toF64" | "toF64Exact" | "toI64Exact" | "toU64Exact", expr }
 * Literal payloads accept BOTH the wire spelling (decimal strings/bit hex)
 * and the idiomatic host values (bigint/number/Uint8Array) the core SDK's
 * `literal` constructor may retain.
 */
import { bytesHex, f64Bits } from "#migrations/canonical.ts"
import type { PlanExpression, PlanValue } from "#migrations/types.ts"

export type ExprResult =
	| { readonly ok: true; readonly expression: PlanExpression; readonly fields: readonly string[] }
	| { readonly ok: false; readonly detail: string }

const MAX_DEPTH = 128
const MAX_NODES = 4096
const MAX_TEXT = 65536

function refuse(detail: string): ExprResult {
	return { ok: false, detail }
}

function isRecord(value: unknown): value is Readonly<Record<string, unknown>> {
	return typeof value === "object" && value !== null
}

function isCanonicalHex(text: string, length: number): boolean {
	if (text.length !== length) {
		return false
	}
	for (let index = 0; index < text.length; index += 1) {
		const code = text.charCodeAt(index)
		if (!((code >= 48 && code <= 57) || (code >= 97 && code <= 102))) {
			return false
		}
	}
	return true
}

const U64_MAX = 0xffffffffffffffffn
const I64_MIN = -0x8000000000000000n
const I64_MAX = 0x7fffffffffffffffn

function decimalU64(value: unknown): string | null {
	if (typeof value === "bigint" && value >= 0n && value <= U64_MAX) {
		return value.toString(10)
	}
	if (typeof value === "string" && value.length > 0 && value.length <= 20 && /^(0|[1-9][0-9]*)$/.test(value)) {
		return BigInt(value) <= U64_MAX ? value : null
	}
	return null
}

function decimalI64(value: unknown): string | null {
	if (typeof value === "bigint" && value >= I64_MIN && value <= I64_MAX) {
		return value.toString(10)
	}
	if (typeof value === "string" && value.length > 0 && value.length <= 21 && /^(0|-?[1-9][0-9]*)$/.test(value)) {
		const parsed = BigInt(value)
		return parsed >= I64_MIN && parsed <= I64_MAX ? value : null
	}
	return null
}

function bitsOf(value: unknown): string | null {
	if (typeof value === "number") {
		return f64Bits(value)
	}
	if (typeof value === "string" && isCanonicalHex(value, 16)) {
		return value
	}
	return null
}

function pairOf(value: unknown, one: (arm: unknown) => string | null): readonly [string, string] | null {
	if (!Array.isArray(value) || value.length !== 2) {
		return null
	}
	const start = one(value[0])
	const end = one(value[1])
	return start === null || end === null ? null : [start, end]
}

/**
 * Bounded structural check of one literal payload into the canonical
 * one-arm value spelling.
 */
export function planValueOf(value: unknown): PlanValue | string {
	if (!isRecord(value)) {
		return "a literal payload must be a one-arm tagged core value"
	}
	const keys = Object.keys(value)
	if (keys.length !== 1) {
		return "a literal payload must have exactly one value arm"
	}
	const arm = keys[0]
	const body: unknown = value[arm as keyof typeof value]
	switch (arm) {
		case "bool":
			return typeof body === "boolean" ? { bool: body } : "bool literal needs a boolean"
		case "u64": {
			const decimal = decimalU64(body)
			return decimal === null ? "u64 literal needs a canonical in-range integer" : { u64: decimal }
		}
		case "i64": {
			const decimal = decimalI64(body)
			return decimal === null ? "i64 literal needs a canonical in-range integer" : { i64: decimal }
		}
		case "$f64":
		case "f64": {
			const bits = bitsOf(body)
			return bits === null ? "f64 literal needs a number or 16 lowercase hex bits" : { $f64: bits }
		}
		case "id128":
			return typeof body === "string" && isCanonicalHex(body, 32)
				? { id128: body }
				: "id128 literal needs canonical 32-lowercase-hex"
		case "string":
			return typeof body === "string" && body.length <= MAX_TEXT && body.isWellFormed()
				? { string: body }
				: "string literal needs bounded well-formed text"
		case "fixedBytes": {
			if (body instanceof Uint8Array && body.length <= MAX_TEXT) {
				return { fixedBytes: bytesHex(body) }
			}
			if (typeof body === "string" && body.length <= 2 * MAX_TEXT && body.length % 2 === 0 && isCanonicalHex(body, body.length)) {
				return { fixedBytes: body }
			}
			return "fixedBytes literal needs a bounded Uint8Array or lowercase hex"
		}
		case "intervalU64": {
			const pair = pairOf(body, decimalU64)
			return pair === null ? "intervalU64 literal needs two canonical integers" : { intervalU64: pair }
		}
		case "intervalI64": {
			const pair = pairOf(body, decimalI64)
			return pair === null ? "intervalI64 literal needs two canonical integers" : { intervalI64: pair }
		}
		case "intervalF64": {
			const pair = pairOf(body, bitsOf)
			return pair === null ? "intervalF64 literal needs two float endpoints" : { intervalF64: pair }
		}
		default:
			return `unknown literal arm ${arm}`
	}
}

const CASTS = ["toF64", "toF64Exact", "toI64Exact", "toU64Exact"] as const

interface Budget {
	nodes: number
}

function walk(node: unknown, depth: number, budget: Budget, fields: Set<string>): PlanExpression | string {
	if (depth > MAX_DEPTH) {
		return `expression deeper than ${MAX_DEPTH}`
	}
	budget.nodes += 1
	if (budget.nodes > MAX_NODES) {
		return `expression larger than ${MAX_NODES} nodes`
	}
	if (!isRecord(node) || typeof node.kind !== "string") {
		return "an expression node must be a tagged core ScalarExpr value — functions, promises and plain hosts are not plan data"
	}
	switch (node.kind) {
		case "field": {
			if (typeof node.name !== "string" || node.name.length === 0 || node.name.length > 255) {
				return "a field reference needs a bounded source field name"
			}
			fields.add(node.name)
			return { kind: "field", name: node.name }
		}
		case "literal": {
			const value = planValueOf(node.value)
			return typeof value === "string" ? value : { kind: "literal", value }
		}
		case "negate":
		case "isNaN":
		case "isFinite": {
			const inner = walk(node.expr, depth + 1, budget, fields)
			if (typeof inner === "string") {
				return inner
			}
			return { kind: node.kind, expr: inner }
		}
		case "add":
		case "subtract":
		case "multiply":
		case "divide": {
			const left = walk(node.left, depth + 1, budget, fields)
			if (typeof left === "string") {
				return left
			}
			const right = walk(node.right, depth + 1, budget, fields)
			if (typeof right === "string") {
				return right
			}
			return { kind: node.kind, left, right }
		}
		case "cast": {
			const cast = CASTS.find((name) => name === node.cast)
			if (cast === undefined) {
				return `unknown cast ${String(node.cast)} — the checked roster is toF64/toF64Exact/toI64Exact/toU64Exact`
			}
			const inner = walk(node.expr, depth + 1, budget, fields)
			if (typeof inner === "string") {
				return inner
			}
			return { kind: "cast", cast, expr: inner }
		}
		default:
			return `unsupported expression node ${node.kind} — the finite supported grammar is field/literal/negate/add/subtract/multiply/divide/cast/isNaN/isFinite`
	}
}

/**
 * Serialize one core scalar expression into canonical plan data, collecting
 * the referenced source field names for loss accounting.
 */
export function planExpressionOf(expression: unknown): ExprResult {
	const fields = new Set<string>()
	const outcome = walk(expression, 0, { nodes: 0 }, fields)
	if (typeof outcome === "string") {
		return refuse(outcome)
	}
	return { ok: true, expression: outcome, fields: [...fields] }
}

/** The identity projection every carried field lowers to. */
export function fieldExpression(name: string): PlanExpression {
	return { kind: "field", name }
}
