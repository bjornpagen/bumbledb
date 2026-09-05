/**
 * The pure schema diff (TS-MIG-04): inference happens ONLY where one safe
 * interpretation exists; ambiguous rename/drop/backfill/type/ID changes
 * refuse with the complete finite requirement list; coverage over ordinary
 * relations is total; every actual loss carries exactly one acknowledgement;
 * closed relations are exempt from data operations. Pure data in, inert plan
 * data out — no I/O, no native work, no guessing.
 */
import assert from "node:assert/strict"
import { describe, test } from "node:test"
import { diffSchemas } from "#migrations/diff.ts"
import type { MigrationIntentEntry } from "#migrations/intent.ts"
import type { TheoryRelation, TheorySnapshot } from "#migrations/types.ts"

function rel(name: string, fields: readonly (readonly [string, string])[], closed = false): TheoryRelation {
	return {
		name,
		fields: fields.map(([field, type]) => ({ name: field, type: JSON.stringify(type) })),
		closed
	}
}

function snap(...relations: TheoryRelation[]): TheorySnapshot {
	return { relations }
}

const LIT_FALSE = { kind: "literal", value: { bool: false } } as const

describe("structural inference", function suite() {
	test("unchanged relations are preserved automatically with identity projections", function unchanged() {
		const prev = snap(rel("Note", [["id", "u64"], ["body", "string"]]))
		const diff = diffSchemas(prev, prev, [])
		assert.deepEqual(diff.requirements, [])
		assert.deepEqual(diff.destructive, [])
		assert.deepEqual(diff.operations, [
			{
				kind: "map-relation",
				source: "Note",
				target: "Note",
				fields: [
					{ target: "id", expression: { kind: "field", name: "id" } },
					{ target: "body", expression: { kind: "field", name: "body" } }
				]
			}
		])
	})

	test("a new relation is created empty; nothing else is touched", function fresh() {
		const prev = snap(rel("Note", [["id", "u64"]]))
		const next = snap(rel("Note", [["id", "u64"]]), rel("Tag", [["id", "u64"]]))
		const diff = diffSchemas(prev, next, [])
		assert.deepEqual(diff.requirements, [])
		assert.deepEqual(
			diff.operations.map((op) => op.kind),
			["map-relation", "empty-relation"]
		)
	})

	test("closed relations are sealed schema axioms, exempt from data coverage", function closed() {
		const prev = snap(rel("Kind", [["label", "string"]], true), rel("Note", [["id", "u64"]]))
		const next = snap(rel("Note", [["id", "u64"]]))
		// Removing a closed relation needs no destructive data intent.
		const diff = diffSchemas(prev, next, [])
		assert.deepEqual(diff.requirements, [])
		assert.deepEqual(
			diff.operations.map((op) => op.kind),
			["map-relation"]
		)
		// Flipping closedness is not a supported transform.
		const flipped = diffSchemas(snap(rel("Kind", [["label", "string"]], true)), snap(rel("Kind", [["label", "string"]])), [])
		assert.equal(flipped.requirements[0]?.code, "unsupported")
	})
})

describe("ambiguity and loss refuse without typed intent", function suite() {
	test("a new required field needs a backfill; no zero/null is fabricated", function missing() {
		const prev = snap(rel("Note", [["id", "u64"]]))
		const next = snap(rel("Note", [["id", "u64"], ["pinned", "bool"]]))
		const diff = diffSchemas(prev, next, [])
		assert.deepEqual(
			diff.requirements.map((entry) => [entry.code, entry.relation, entry.field]),
			[["missing-backfill", "Note", "pinned"]]
		)
		const filled = diffSchemas(prev, next, [
			{ kind: "backfill", relation: "Note", field: "pinned", expression: LIT_FALSE } as MigrationIntentEntry
		])
		assert.deepEqual(filled.requirements, [])
		const map = filled.operations[0]
		assert.ok(map !== undefined && map.kind === "map-relation")
		assert.deepEqual(map.fields[1], { target: "pinned", expression: { kind: "literal", value: { bool: false } } })
	})

	test("a removed relation refuses without dropRelation and hints candidate renames", function removed() {
		const prev = snap(rel("Old", [["id", "u64"]]))
		const next = snap(rel("New", [["id", "u64"]]))
		const diff = diffSchemas(prev, next, [])
		assert.equal(diff.requirements[0]?.code, "destructive")
		assert.ok(diff.requirements[0]?.detail.includes('renameRelation("Old", New)'))
		const dropped = diffSchemas(prev, next, [{ kind: "drop-relation", relation: "Old" }])
		assert.deepEqual(dropped.requirements, [])
		assert.deepEqual(
			dropped.operations.map((op) => op.kind),
			["empty-relation", "drop-relation"]
		)
		assert.deepEqual(dropped.destructive, [{ relation: "Old" }])
	})

	test("a renamed relation with explicit intent maps old rows to the new name", function renamed() {
		const prev = snap(rel("Old", [["id", "u64"]]))
		const next = snap(rel("New", [["id", "u64"]]))
		const diff = diffSchemas(prev, next, [{ kind: "rename-relation", from: "Old", to: "New" }])
		assert.deepEqual(diff.requirements, [])
		assert.deepEqual(diff.operations, [
			{
				kind: "map-relation",
				source: "Old",
				target: "New",
				fields: [{ target: "id", expression: { kind: "field", name: "id" } }]
			}
		])
		assert.deepEqual(diff.destructive, [])
	})

	test("a removed field refuses without dropField; acknowledged loss is recorded once", function removedField() {
		const prev = snap(rel("Note", [["id", "u64"], ["draft", "bool"]]))
		const next = snap(rel("Note", [["id", "u64"]]))
		const diff = diffSchemas(prev, next, [])
		assert.deepEqual(
			diff.requirements.map((entry) => [entry.code, entry.relation, entry.field]),
			[["destructive", "Note", "draft"]]
		)
		const dropped = diffSchemas(prev, next, [{ kind: "drop-field", relation: "Note", field: "draft" }])
		assert.deepEqual(dropped.requirements, [])
		assert.deepEqual(dropped.destructive, [{ relation: "Note", field: "draft" }])
	})

	test("a type change requires an explicit checked conversion", function typeChange() {
		const prev = snap(rel("M", [["x", "i64"]]))
		const next = snap(rel("M", [["x", "f64"]]))
		const diff = diffSchemas(prev, next, [])
		assert.deepEqual(
			diff.requirements.map((entry) => [entry.code, entry.relation, entry.field]),
			[["type-change", "M", "x"]]
		)
		const converted = diffSchemas(prev, next, [
			{
				kind: "convert",
				relation: "M",
				field: "x",
				expression: { kind: "cast", cast: "toF64", expr: { kind: "field", name: "x" } }
			} as MigrationIntentEntry
		])
		assert.deepEqual(converted.requirements, [])
		const map = converted.operations[0]
		assert.ok(map !== undefined && map.kind === "map-relation")
		assert.deepEqual(map.fields, [
			{ target: "x", expression: { kind: "cast", cast: "toF64", expr: { kind: "field", name: "x" } } }
		])
		// The old value is referenced by the cast, so nothing is lost.
		assert.deepEqual(converted.destructive, [])
	})

	test("a convert that ignores the old value is an acknowledged loss, recorded exactly once", function convertLoss() {
		const prev = snap(rel("M", [["x", "i64"]]))
		const next = snap(rel("M", [["x", "bool"]]))
		const converted = diffSchemas(prev, next, [
			{ kind: "convert", relation: "M", field: "x", expression: LIT_FALSE } as MigrationIntentEntry
		])
		assert.deepEqual(converted.requirements, [])
		assert.deepEqual(converted.destructive, [{ relation: "M", field: "x" }])
	})

	test("backfill on an existing field and convert on a new field are conflicts", function misuse() {
		const prev = snap(rel("M", [["x", "i64"]]))
		const next = snap(rel("M", [["x", "i64"], ["y", "i64"]]))
		const wrongFill = diffSchemas(prev, next, [
			{ kind: "backfill", relation: "M", field: "x", expression: LIT_FALSE } as MigrationIntentEntry,
			{ kind: "backfill", relation: "M", field: "y", expression: { kind: "field", name: "x" } } as MigrationIntentEntry
		])
		assert.deepEqual(
			wrongFill.requirements.map((entry) => [entry.code, entry.field]),
			[["conflicting-intent", "x"]]
		)
		const wrongConvert = diffSchemas(prev, next, [
			{ kind: "convert", relation: "M", field: "y", expression: LIT_FALSE } as MigrationIntentEntry
		])
		assert.ok(wrongConvert.requirements.some((entry) => entry.code === "conflicting-intent" && entry.field === "y"))
	})

	test("backfill referencing unknown source fields is unsupported, not guessed", function unknownRef() {
		const prev = snap(rel("M", [["x", "i64"]]))
		const next = snap(rel("M", [["x", "i64"], ["y", "i64"]]))
		const diff = diffSchemas(prev, next, [
			{ kind: "backfill", relation: "M", field: "y", expression: { kind: "field", name: "ghost" } } as MigrationIntentEntry
		])
		assert.deepEqual(
			diff.requirements.map((entry) => entry.code),
			["unsupported"]
		)
	})

	test("field renames validate both endpoints and conflicts", function fieldRename() {
		const prev = snap(rel("Note", [["id", "u64"], ["body", "string"]]))
		const next = snap(rel("Note", [["id", "u64"], ["text", "string"]]))
		const renamed = diffSchemas(prev, next, [{ kind: "rename-field", relation: "Note", from: "body", to: "text" }])
		assert.deepEqual(renamed.requirements, [])
		const map = renamed.operations[0]
		assert.ok(map !== undefined && map.kind === "map-relation")
		assert.deepEqual(map.fields[1], { target: "text", expression: { kind: "field", name: "body" } })
		assert.deepEqual(renamed.destructive, [])
		// Stale endpoints refuse.
		const stale = diffSchemas(prev, next, [{ kind: "rename-field", relation: "Note", from: "ghost", to: "text" }])
		assert.ok(stale.requirements.some((entry) => entry.code === "stale-intent"))
	})

	test("stale intent matching no change is refused, never silently ignored", function stale() {
		const prev = snap(rel("Note", [["id", "u64"]]))
		const diff = diffSchemas(prev, prev, [{ kind: "drop-relation", relation: "Note" }])
		// Note still exists: the drop is not consumed by any removal.
		assert.deepEqual(
			diff.requirements.map((entry) => entry.code),
			["stale-intent"]
		)
	})

	test("seed on a closed relation is unsupported; seed order follows the target schema", function seeds() {
		const prev = snap(rel("Kind", [["label", "string"]], true), rel("A", [["id", "u64"]]), rel("B", [["id", "u64"]]))
		const closedSeed = diffSchemas(prev, prev, [{ kind: "seed", relation: "Kind", rows: [] }])
		assert.deepEqual(
			closedSeed.requirements.map((entry) => entry.code),
			["unsupported"]
		)
		const ordered = diffSchemas(prev, prev, [
			{ kind: "seed", relation: "B", rows: [] },
			{ kind: "seed", relation: "A", rows: [] }
		])
		assert.deepEqual(ordered.requirements, [])
		assert.deepEqual(ordered.seedRelations, ["A", "B"])
	})

	test("conflicting rename intents refuse instead of picking a winner", function conflicts() {
		const prev = snap(rel("Old", [["id", "u64"]]), rel("Old2", [["id", "u64"]]))
		const next = snap(rel("New", [["id", "u64"]]))
		const diff = diffSchemas(prev, next, [
			{ kind: "rename-relation", from: "Old", to: "New" },
			{ kind: "rename-relation", from: "Old2", to: "New" }
		])
		assert.ok(diff.requirements.some((entry) => entry.code === "conflicting-intent"))
	})
})
