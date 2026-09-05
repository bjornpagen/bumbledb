/**
 * Pure declarative intent constructors (TS-MIG-10 tooling boundary): inert
 * frozen metadata, no I/O, no schema lowering, no seed ingestion at
 * construction — a metadata constructor never consumes an arbitrary graph.
 * Authoring misuse throws the core AuthoringError synchronously (pure
 * authoring failure, chapter 35), never an Effect.
 */
import assert from "node:assert/strict"
import { describe, test } from "node:test"
import { AuthoringError, relation, Scalar, str, u64 } from "@bjornpagen/bumbledb"
import {
	backfill,
	convert,
	dropField,
	dropRelation,
	migrationIntent,
	renameField,
	renameRelation,
	seed
} from "#migrations/intent.ts"
import { App1, Note1 } from "#test/migrations-example.ts"

const Tag = relation("Tag", { id: u64, name: str })

describe("intent constructors are inert typed metadata", function suite() {
	test("construction ingests nothing: the seed iterable is untouched", function inert() {
		let iterated = 0
		const rows = {
			*[Symbol.iterator]() {
				iterated += 1
				yield { id: 1n, name: "x" }
			}
		}
		const entry = seed(Tag, rows)
		assert.equal(iterated, 0, "seed() must not consume the caller iterable")
		assert.equal(entry.kind, "seed")
		assert.ok(Object.isFrozen(entry))
	})

	test("the exact core ScalarExpr value passes through by reference, never copied or evaluated", function exprReuse() {
		const expression = { kind: "literal", value: { bool: false } } as const
		const fill = backfill(Note1, "pinned", expression)
		assert.ok(fill.kind === "backfill")
		assert.equal(fill.expression, expression, "the core AST value is retained, not cloned")
		const conv = convert(Note1, "body", { kind: "field", name: "body" })
		assert.ok(conv.kind === "convert")
	})

	test("convert and backfill accept unresolved field arithmetic without a cast", function unresolved() {
		const increment = Scalar.add(Scalar.field("units"), Scalar.u64(1n))
		const Stock = relation("Stock", { id: u64, units: u64, next: u64 })
		const conv = convert(Stock, "units", increment)
		assert.ok(conv.kind === "convert")
		assert.equal(conv.expression, increment)
		const fill = backfill(Stock, "next", increment)
		assert.ok(fill.kind === "backfill")
		assert.equal(fill.expression, increment)
	})

	test("migrationIntent freezes its own entry list but retains caller-owned rows", function frozen() {
		const entries = [dropRelation("Old")]
		const intent = migrationIntent(App1, entries)
		entries.push(dropRelation("Older"))
		assert.equal(intent.entries.length, 1, "later caller mutation cannot grow the declared intent")
		assert.ok(Object.isFrozen(intent))
		assert.ok(Object.isFrozen(intent.entries))
		assert.equal(intent.schema, App1)
	})

	test("rename constructors bind the target through the typed relation value", function renames() {
		const rel = renameRelation("Old", Tag)
		assert.deepEqual(rel, { kind: "rename-relation", from: "Old", to: "Tag" })
		const field = renameField(Tag, "label", "name")
		assert.deepEqual(field, { kind: "rename-field", relation: "Tag", from: "label", to: "name" })
		assert.deepEqual(dropField(Tag, "name"), { kind: "drop-field", relation: "Tag", field: "name" })
	})

	test("authoring misuse throws the core AuthoringError synchronously", function misuse() {
		assert.throws(() => dropRelation(""), AuthoringError)
		assert.throws(() => dropRelation("a".repeat(256)), AuthoringError)
		assert.throws(() => dropField(Tag, "a\0b"), AuthoringError)
		assert.throws(() => migrationIntent(App1, {} as never), AuthoringError)
		assert.throws(() => seed(Tag, {} as never), AuthoringError)
	})
})
