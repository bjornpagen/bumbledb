/**
 * Pure declarative schema-evolution intent. These constructors build inert
 * authoring metadata: no rows are consumed, no schema is lowered, hashed or
 * admitted, and no I/O happens here. Ambiguous rename/drop/backfill/convert
 * and business seed data are typed inputs to bounded generation, never
 * imperative migration files under another name. Expressions are the core's
 * own `ScalarExpr` values (C01) — the generator serializes them into
 * canonical plan data and the native executor evaluates them; there is no
 * migration-only expression interpreter.
 */
import { AuthoringError } from "@bjornpagen/bumbledb"
import type { AnyRelation, Fact, ScalarExpr, Schema, SchemaRelations } from "@bjornpagen/bumbledb"

export type MigrationIntentEntry =
	| { readonly kind: "rename-relation"; readonly from: string; readonly to: string }
	| { readonly kind: "rename-field"; readonly relation: string; readonly from: string; readonly to: string }
	| { readonly kind: "drop-relation"; readonly relation: string }
	| { readonly kind: "drop-field"; readonly relation: string; readonly field: string }
	| {
			readonly kind: "backfill"
			readonly relation: string
			readonly field: string
			readonly expression: ScalarExpr<unknown>
	  }
	| {
			readonly kind: "convert"
			readonly relation: string
			readonly field: string
			readonly expression: ScalarExpr<unknown>
	  }
	| {
			readonly kind: "seed"
			readonly relation: string
			readonly rows: Iterable<Readonly<Record<string, unknown>>>
	  }

/**
 * Inert authoring metadata: the schema value plus its evolution entries.
 * Seed iterables are retained caller-owned and are consumed exactly once,
 * inside bounded generation — constructing this value ingests nothing.
 */
export interface MigrationIntent<Rels extends SchemaRelations> {
	readonly schema: Schema<Rels>
	readonly entries: readonly MigrationIntentEntry[]
}

const MAX_NAME = 255

function name(context: string, value: string): string {
	if (typeof value !== "string" || value.length === 0 || value.length > MAX_NAME || value.includes("\0")) {
		throw new AuthoringError({
			message: `${context}: a schema name must be a nonempty string of at most ${MAX_NAME} characters without NUL`
		})
	}
	return value
}

export function migrationIntent<Rels extends SchemaRelations>(
	schema: Schema<Rels>,
	entries: readonly MigrationIntentEntry[]
): MigrationIntent<Rels> {
	if (!Array.isArray(entries)) {
		throw new AuthoringError({ message: "migrationIntent: entries must be an array of intent values" })
	}
	// Retain caller-owned input until generation. A metadata constructor never
	// synchronously copies or consumes an arbitrary intent/seed graph.
	return Object.freeze({ schema, entries: Object.freeze([...entries]) })
}

/** Identity intent: the relation previously named `from` is now `to`. */
export function renameRelation(from: string, to: AnyRelation): MigrationIntentEntry {
	return Object.freeze({
		kind: "rename-relation" as const,
		from: name("renameRelation", from),
		to: name("renameRelation", to.name)
	})
}

/** Identity intent: `relation`'s field previously named `from` is now `to`. */
export function renameField<R extends AnyRelation, K extends keyof Fact<R> & string>(
	relation: R,
	from: string,
	to: K
): MigrationIntentEntry {
	return Object.freeze({
		kind: "rename-field" as const,
		relation: name("renameField", relation.name),
		from: name("renameField", from),
		to: name("renameField", to)
	})
}

/** The explicit whole-relation data-loss acknowledgement IS this constructor. */
export function dropRelation(relation: string): MigrationIntentEntry {
	return Object.freeze({ kind: "drop-relation" as const, relation: name("dropRelation", relation) })
}

/** The explicit field data-loss acknowledgement. `relation` is the surviving relation. */
export function dropField(relation: AnyRelation | string, field: string): MigrationIntentEntry {
	return Object.freeze({
		kind: "drop-field" as const,
		relation: name("dropField", typeof relation === "string" ? relation : relation.name),
		field: name("dropField", field)
	})
}

/**
 * A typed value for a NEW required field, as a core `ScalarExpr` over the
 * source row's old fields. No fabricated zero/null and no callback.
 */
export function backfill<R extends AnyRelation, K extends keyof Fact<R> & string>(
	relation: R,
	field: K,
	expression: ScalarExpr<Fact<R>[K]>
): MigrationIntentEntry {
	return Object.freeze({
		kind: "backfill" as const,
		relation: name("backfill", relation.name),
		field: name("backfill", field),
		expression
	})
}

/**
 * An explicit checked conversion for an EXISTING field whose type or meaning
 * changes, as a core `ScalarExpr` over the source row's old fields.
 */
export function convert<R extends AnyRelation, K extends keyof Fact<R> & string>(
	relation: R,
	field: K,
	expression: ScalarExpr<Fact<R>[K]>
): MigrationIntentEntry {
	return Object.freeze({
		kind: "convert" as const,
		relation: name("convert", relation.name),
		field: name("convert", field),
		expression
	})
}

/**
 * Declarative fixed seed facts with explicitly supplied application IDs.
 * The iterable stays caller-owned and stable until generation settles; it is
 * read once, under the generation policy's cell/row/total budgets.
 */
export function seed<R extends AnyRelation>(relation: R, rows: Iterable<Fact<R>>): MigrationIntentEntry {
	if (rows === null || typeof rows !== "object" || !(Symbol.iterator in rows)) {
		throw new AuthoringError({ message: "seed: rows must be an iterable of typed facts" })
	}
	return Object.freeze({ kind: "seed" as const, relation: name("seed", relation.name), rows })
}
