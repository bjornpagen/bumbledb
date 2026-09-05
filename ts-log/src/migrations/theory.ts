/**
 * Bounded parse of the ONE native schema-file grammar
 * (`crates/bumbledb-log/src/schema_file.rs`: `{relations, statements}`,
 * closed extensions on relations) into the small image the diff walks.
 * Snapshot files are the native `schema_file::render` text verbatim; this
 * reader extracts relation/field names, canonical type spellings and
 * closedness — statements never need TS interpretation because schema change
 * is decided by the native canonical SchemaId, not by re-deriving laws here.
 */
import type { TheoryField, TheoryRelation, TheorySnapshot } from "#migrations/types.ts"

const MAX_RELATIONS = 4096
const MAX_FIELDS = 4096
const MAX_NAME = 255

export type TheoryResult =
	| { readonly ok: true; readonly snapshot: TheorySnapshot }
	| { readonly ok: false; readonly detail: string }

function refuse(detail: string): TheoryResult {
	return { ok: false, detail }
}

function isRecord(value: unknown): value is Readonly<Record<string, unknown>> {
	return typeof value === "object" && value !== null && !Array.isArray(value)
}

function boundedName(value: unknown): string | null {
	return typeof value === "string" && value.length > 0 && value.length <= MAX_NAME && !value.includes("\0")
		? value
		: null
}

/** Canonical type spelling for exact comparison and messages. */
function typeKey(value: unknown): string | null {
	if (typeof value === "string") {
		return ["bool", "u64", "i64", "f64", "string", "id128"].includes(value) ? JSON.stringify(value) : null
	}
	if (!isRecord(value)) {
		return null
	}
	const keys = Object.keys(value)
	if (keys.length !== 1) {
		return null
	}
	if (keys[0] === "fixedBytes" && typeof value.fixedBytes === "number" && Number.isSafeInteger(value.fixedBytes)) {
		return `{"fixedBytes":${value.fixedBytes}}`
	}
	if (keys[0] === "interval" && typeof value.interval === "string" && ["u64", "i64", "f64"].includes(value.interval)) {
		return `{"interval":"${value.interval}"}`
	}
	if (keys[0] === "fixedInterval" && isRecord(value.fixedInterval)) {
		const element = value.fixedInterval.element
		const width = value.fixedInterval.width
		if (typeof element === "string" && ["u64", "i64"].includes(element) && typeof width === "string") {
			return `{"fixedInterval":{"element":"${element}","width":"${width}"}}`
		}
	}
	return null
}

/** Parse one snapshot text (already size-bounded by the file reader). */
export function parseTheory(raw: string): TheoryResult {
	let tree: unknown
	try {
		tree = JSON.parse(raw)
	} catch {
		return refuse("snapshot is not JSON")
	}
	if (!isRecord(tree) || !Array.isArray(tree.relations) || !Array.isArray(tree.statements)) {
		return refuse("snapshot must be the {relations, statements} theory grammar")
	}
	if (tree.relations.length > MAX_RELATIONS) {
		return refuse(`snapshot exceeds ${MAX_RELATIONS} relations`)
	}
	const relations: TheoryRelation[] = []
	const seen = new Set<string>()
	for (const entry of tree.relations) {
		if (!isRecord(entry)) {
			return refuse("relation entry")
		}
		const name = boundedName(entry.name)
		if (name === null) {
			return refuse("relation name")
		}
		if (seen.has(name)) {
			return refuse(`duplicate relation ${name}`)
		}
		seen.add(name)
		if (!Array.isArray(entry.fields) || entry.fields.length > MAX_FIELDS) {
			return refuse(`relation ${name} fields`)
		}
		const fields: TheoryField[] = []
		const fieldNames = new Set<string>()
		for (const field of entry.fields) {
			if (!isRecord(field)) {
				return refuse(`relation ${name} field entry`)
			}
			const fieldName = boundedName(field.name)
			if (fieldName === null || fieldNames.has(fieldName)) {
				return refuse(`relation ${name} field name`)
			}
			fieldNames.add(fieldName)
			const key = typeKey(field.type)
			if (key === null) {
				return refuse(`relation ${name} field ${fieldName} type`)
			}
			if (field.generation !== undefined) {
				return refuse("fresh generation is deleted")
			}
			fields.push({ name: fieldName, type: key })
		}
		const extension = entry.extension
		if (extension !== undefined && extension !== null && !Array.isArray(extension)) {
			return refuse(`relation ${name} extension`)
		}
		relations.push({
			name,
			fields,
			closed: Array.isArray(extension)
		})
	}
	return { ok: true, snapshot: { relations } }
}

export const EMPTY_THEORY: TheorySnapshot = Object.freeze({ relations: Object.freeze([]) })
