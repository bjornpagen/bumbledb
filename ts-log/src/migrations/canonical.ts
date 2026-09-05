/**
 * Canonical value spelling and deterministic generated-text helpers.
 *
 * The native codec (P09) is the rendering and digest authority for plan,
 * manifest and schema-snapshot FILES: those bytes come back from the native
 * entrypoints (`schema_file::render`, `migration::plan::render_plan`,
 * `migration::manifest::render_manifest`) and are written verbatim, so JSON
 * formatting can never diverge between languages and no digest is computed
 * twice. This module spells only (a) the canonical VALUE arms
 * (`migration::json` grammar) used to hand plan data across the bridge,
 * (b) the small TS-owned derived files (generated `index.ts`, the runtime
 * contract) and (c) deterministic JSON text for requests/reports.
 *
 * Float cells use the canonical bit image — canonical quiet NaN
 * (`7ff8000000000000`), canonical +0 — exactly the core F64 quotient; the
 * native codec re-judges every value.
 */
import type { ManifestEntry, MigrationPlan, PlanExpression, PlanOperation, PlanValue, RuntimeContract } from "#migrations/types.ts"

// ---------------------------------------------------------------------------
// Canonical scalar spellings.
// ---------------------------------------------------------------------------

const F64_IMAGE = new DataView(new ArrayBuffer(8))
const CANONICAL_NAN = "7ff8000000000000"

/** Canonical binary64 bit image: quiet NaN, +0 for both zeros. */
export function f64Bits(value: number): string {
	if (Number.isNaN(value)) {
		return CANONICAL_NAN
	}
	F64_IMAGE.setFloat64(0, value === 0 ? 0 : value)
	return F64_IMAGE.getBigUint64(0).toString(16).padStart(16, "0")
}

const HEX = "0123456789abcdef"

export function bytesHex(bytes: Uint8Array): string {
	let out = ""
	for (const byte of bytes) {
		out += HEX[byte >> 4]
		out += HEX[byte & 0x0f]
	}
	return out
}

// ---------------------------------------------------------------------------
// Deterministic JSON text for TS-owned artifacts and native requests.
// Objects are constructed with one fixed key order throughout this package;
// the renderer preserves insertion order and ends with one newline.
// ---------------------------------------------------------------------------

export type JsonValue = null | boolean | number | string | readonly JsonValue[] | { readonly [key: string]: JsonValue }

export function renderJson(value: JsonValue): string {
	return `${JSON.stringify(value, null, "\t")}\n`
}

/** Compact single-line spelling for request bodies. */
export function compactJson(value: JsonValue): string {
	return JSON.stringify(value)
}

// ---------------------------------------------------------------------------
// Plan data → JSON tree (the native request carries this subtree; the native
// parser is `migration::plan::parse_plan`'s operation/expression grammar).
// PlanValue is already exactly the JSON value-arm spelling.
// ---------------------------------------------------------------------------

export function valueJson(value: PlanValue): JsonValue {
	if ("bool" in value) {
		return { bool: value.bool }
	}
	if ("u64" in value) {
		return { u64: value.u64 }
	}
	if ("i64" in value) {
		return { i64: value.i64 }
	}
	if ("$f64" in value) {
		return { $f64: value.$f64 }
	}
	if ("id128" in value) {
		return { id128: value.id128 }
	}
	if ("string" in value) {
		return { string: value.string }
	}
	if ("fixedBytes" in value) {
		return { fixedBytes: value.fixedBytes }
	}
	if ("intervalU64" in value) {
		return { intervalU64: value.intervalU64 }
	}
	if ("intervalI64" in value) {
		return { intervalI64: value.intervalI64 }
	}
	return { intervalF64: value.intervalF64 }
}

function expressionJson(expression: PlanExpression): JsonValue {
	switch (expression.kind) {
		case "field":
			return { kind: "field", name: expression.name }
		case "literal":
			return { kind: "literal", value: valueJson(expression.value) }
		case "negate":
		case "isNaN":
		case "isFinite":
			return { kind: expression.kind, expr: expressionJson(expression.expr) }
		case "add":
		case "subtract":
		case "multiply":
		case "divide":
			return { kind: expression.kind, left: expressionJson(expression.left), right: expressionJson(expression.right) }
		case "cast":
			return { kind: "cast", cast: expression.cast, expr: expressionJson(expression.expr) }
	}
}

function operationJson(operation: PlanOperation): JsonValue {
	switch (operation.kind) {
		case "map-relation":
			return {
				kind: "map-relation",
				source: operation.source,
				target: operation.target,
				fields: operation.fields.map((field) => ({
					target: field.target,
					expression: expressionJson(field.expression)
				}))
			}
		case "empty-relation":
			return { kind: "empty-relation", target: operation.target }
		case "drop-relation":
			return { kind: "drop-relation", source: operation.source }
		case "seed":
			return {
				kind: "seed",
				target: operation.target,
				rows: operation.rows.map((row) => row.map(valueJson))
			}
		case "validate-schema":
			return { kind: "validate-schema", schemaId: operation.schemaId }
	}
}

export function planJson(plan: MigrationPlan): JsonValue {
	const destructive: JsonValue[] = plan.destructive.map((loss) =>
		loss.field === undefined ? { relation: loss.relation } : { relation: loss.relation, field: loss.field }
	)
	return {
		planVersion: plan.planVersion,
		sequence: plan.sequence,
		id: plan.id,
		fromSchemaId: plan.fromSchemaId,
		toSchemaId: plan.toSchemaId,
		operations: plan.operations.map(operationJson),
		destructive
	}
}

// ---------------------------------------------------------------------------
// TS-owned derived files.
// ---------------------------------------------------------------------------

export function renderContract(contract: RuntimeContract): string {
	return renderJson({
		contractVersion: contract.contractVersion,
		schemaId: contract.schemaId,
		appliedPrefixDigest: contract.appliedPrefixDigest,
		steps: contract.steps
	})
}

/**
 * The generated static index: data imports only, no I/O, no logic. The app's
 * admin runner imports this module's default export as `GeneratedMigrations`.
 */
export function renderIndex(entries: readonly ManifestEntry[]): string {
	const lines: string[] = ["/** Generated by bumbledb-log. Static migration data imports only — do not edit. */"]
	entries.forEach((entry, ordinal) => {
		lines.push(`import plan${ordinal} from "./${entry.id}.plan.json" with { type: "json" }`)
	})
	lines.push(`import manifest from "./manifest.json" with { type: "json" }`)
	lines.push("")
	const roster = entries.map((_, ordinal) => `plan${ordinal}`).join(", ")
	lines.push(`const plans = Object.freeze([${roster}])`)
	lines.push("export default Object.freeze({ manifest, plans })")
	return `${lines.join("\n")}\n`
}
