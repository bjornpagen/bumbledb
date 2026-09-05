import { AuthoringError } from "@bjornpagen/bumbledb"
import type { AnyRelation, Fact, ScalarExpr, Schema, SchemaRelations } from "@bjornpagen/bumbledb"
import type { MigrationIntentEntry } from "./types.ts"

/** Inert authoring metadata: no rows are consumed and no schema is lowered here. */
export interface MigrationIntent<Rels extends SchemaRelations> {
	readonly schema: Schema<Rels>
	readonly entries: readonly MigrationIntentEntry[]
}

function name(value: string): string {
	if (value.length === 0 || value.includes("\0")) {
		throw new AuthoringError({ message: "migration intent requires a nonempty schema name without NUL" })
	}
	return value
}

export function migrationIntent<Rels extends SchemaRelations>(
	schema: Schema<Rels>,
	entries: readonly MigrationIntentEntry[]
): MigrationIntent<Rels> {
	// Retain caller-owned input until generation. Do not synchronously copy or
	// consume an arbitrary intent/seed graph during a metadata constructor.
	return Object.freeze({ schema, entries })
}

export function renameRelation(from: string, to: AnyRelation): MigrationIntentEntry {
	return Object.freeze({ kind: "rename-relation", from: name(from), to: to.name })
}

export function renameField<R extends AnyRelation, K extends keyof Fact<R> & string>(
	relation: R, from: string, to: K
): MigrationIntentEntry {
	return Object.freeze({ kind: "rename-field", relation: relation.name, from: name(from), to })
}

/** The explicit data-loss acknowledgement is the constructor itself. */
export function dropRelation(relation: string): MigrationIntentEntry {
	return Object.freeze({ kind: "drop-relation", relation: name(relation) })
}

export function dropField(relation: AnyRelation | string, field: string): MigrationIntentEntry {
	return Object.freeze({ kind: "drop-field", relation: typeof relation === "string" ? name(relation) : relation.name, field: name(field) })
}

export function backfill<R extends AnyRelation, K extends keyof Fact<R> & string>(
	relation: R, field: K, expression: ScalarExpr<Fact<R>[K]>
): MigrationIntentEntry {
	return Object.freeze({ kind: "backfill", relation: relation.name, field, expression })
}

export function convert<R extends AnyRelation, K extends keyof Fact<R> & string>(
	relation: R, field: K, expression: ScalarExpr<Fact<R>[K]>
): MigrationIntentEntry {
	return Object.freeze({ kind: "convert", relation: relation.name, field, expression })
}

export function seed<R extends AnyRelation>(relation: R, rows: Iterable<Fact<R>>): MigrationIntentEntry {
	return Object.freeze({ kind: "seed", relation: relation.name, rows })
}
