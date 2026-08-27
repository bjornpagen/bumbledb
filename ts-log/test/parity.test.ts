import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as path from "node:path"
import { describe, test } from "node:test"
import type { LiteralSpec, SchemaSpec, StatementSpec, ValueSpec, ValueTypeSpec } from "@bjornpagen/bumbledb"
import { internalBlake3 } from "@bjornpagen/bumbledb"
import * as errors from "@superbuilders/errors"
import { digest32, digest32FromHex, fromHex, toHex } from "#bytes.ts"
import type { BatchHeader, Op } from "#codec.ts"
import { decodeBatch, encodeBatch, verifyChain } from "#codec.ts"
import type { Descriptor } from "#descriptor.ts"
import { braid, withFingerprint } from "#descriptor.ts"
import { chainMismatchOf, ErrChainMismatch, ErrRefused, refusalOf } from "#errors.ts"
import { generation } from "#keys.ts"
import { assembleFromSpec } from "#test/assemble.ts"
import type { Value } from "#value.ts"

/**
 * Lane 7's cross-language goldens: the corpus checked in beside the Rust
 * driver is decoded, re-encoded, and compared byte for byte; refusals
 * must carry the same cross-implementation identity. Skipped with a
 * reason when the corpus is not present in the working tree.
 */
const corpusRoot = path.resolve(import.meta.dirname, "../../crates/bumbledb-log/conformance/v3")
const present = fs.existsSync(path.join(corpusRoot, "schemas.json"))

type CorpusValue = Record<string, unknown>

interface CorpusField {
	readonly name: string
	readonly type: unknown
	readonly generation?: string
}

interface CorpusRelation {
	readonly name: string
	readonly fields: readonly CorpusField[]
	readonly extension?: ReadonlyArray<{ readonly handle: string; readonly values: readonly CorpusValue[] }>
}

interface CorpusSide {
	readonly relation: number
	readonly projection: readonly number[]
	readonly selection?: ReadonlyArray<readonly [number, readonly CorpusValue[]]>
}

interface CorpusSchema {
	readonly relations: readonly CorpusRelation[]
	readonly statements: readonly Record<string, unknown>[]
}

function typeOf(raw: unknown): ValueTypeSpec {
	if (raw === "bool" || raw === "u64" || raw === "i64" || raw === "string") {
		return { kind: raw }
	}
	const record = raw as Record<string, unknown>
	if (typeof record.fixedBytes === "number") {
		return { kind: "fixedBytes", len: record.fixedBytes }
	}
	if (record.interval === "u64" || record.interval === "i64") {
		return { kind: "interval", element: record.interval, width: undefined }
	}
	const fixed = record.fixedInterval as { element: "u64" | "i64"; width: string } | undefined
	if (fixed !== undefined) {
		return { kind: "interval", element: fixed.element, width: BigInt(fixed.width) }
	}
	throw errors.new(`corpus type unreadable: ${JSON.stringify(raw)}`)
}

function valueSpecOf(raw: CorpusValue): ValueSpec {
	if (typeof raw.bool === "boolean") {
		return { kind: "bool", value: raw.bool }
	}
	if (typeof raw.u64 === "string") {
		return { kind: "u64", value: BigInt(raw.u64) }
	}
	if (typeof raw.i64 === "string") {
		return { kind: "i64", value: BigInt(raw.i64) }
	}
	if (typeof raw.string === "string") {
		return { kind: "string", value: raw.string }
	}
	if (typeof raw.fixedBytes === "string") {
		return { kind: "fixedBytes", value: fromHex(raw.fixedBytes) }
	}
	const u = raw.intervalU64 as readonly [string, string] | undefined
	if (u !== undefined) {
		return { kind: "intervalU64", start: BigInt(u[0]), end: BigInt(u[1]) }
	}
	const i = raw.intervalI64 as readonly [string, string] | undefined
	if (i !== undefined) {
		return { kind: "intervalI64", start: BigInt(i[0]), end: BigInt(i[1]) }
	}
	throw errors.new(`corpus value unreadable: ${JSON.stringify(raw)}`)
}

function rawValue(raw: CorpusValue): Value {
	const spec = valueSpecOf(raw)
	switch (spec.kind) {
		case "bool":
		case "u64":
		case "i64":
		case "string":
			return spec.value
		case "fixedBytes":
			return spec.value
		case "intervalU64":
		case "intervalI64":
			return { start: spec.start, end: spec.end }
	}
}

function specOf(corpus: CorpusSchema): SchemaSpec {
	function sealedName(relation: CorpusRelation, ordinal: number): string {
		const sealed = relation.extension === undefined ? relation.fields : [{ name: "id" }, ...relation.fields]
		const field = sealed[ordinal]
		if (field === undefined) {
			throw errors.new(`corpus relation ${relation.name} has no sealed field ${ordinal}`)
		}
		return field.name
	}
	function relationName(id: number): CorpusRelation {
		const relation = corpus.relations[id]
		if (relation === undefined) {
			throw errors.new(`corpus cites unknown relation ${id}`)
		}
		return relation
	}
	function sideOf(raw: CorpusSide) {
		const relation = relationName(raw.relation)
		return {
			relation: relation.name,
			projection: raw.projection.map(function nameOf(ordinal) {
				return sealedName(relation, ordinal)
			}),
			selection: (raw.selection ?? []).map(function bindingOf(binding) {
				const literals = binding[1].map(function literalOf(value): LiteralSpec {
					return { kind: "value", value: valueSpecOf(value) }
				})
				const first = literals[0]
				if (literals.length === 1 && first !== undefined) {
					return [sealedName(relation, binding[0]), { kind: "one", literal: first }] as const
				}
				return [sealedName(relation, binding[0]), { kind: "many", literals }] as const
			})
		}
	}
	const statements: StatementSpec[] = corpus.statements.map(function statementOf(raw): StatementSpec {
		const fd = raw.functionality as { relation: number; projection: readonly number[] } | undefined
		if (fd !== undefined) {
			const relation = relationName(fd.relation)
			return {
				kind: "fd",
				relation: relation.name,
				projection: fd.projection.map(function nameOf(ordinal) {
					return sealedName(relation, ordinal)
				})
			}
		}
		const containment = raw.containment as { source: CorpusSide; target: CorpusSide } | undefined
		if (containment !== undefined) {
			return {
				kind: "containment",
				source: sideOf(containment.source),
				target: sideOf(containment.target),
				bidirectional: false
			}
		}
		const capacity = raw.capacity as
			| {
					target: CorpusSide
					weight: unknown
					lo: string
					hi?: Record<string, unknown>
					source: CorpusSide
			  }
			| undefined
		if (capacity !== undefined) {
			const sourceRelation = relationName(capacity.source.relation)
			const targetRelation = relationName(capacity.target.relation)
			let weight: { kind: "unit" } | { kind: "field"; field: string } | { kind: "durationField"; field: string } = {
				kind: "unit"
			}
			if (typeof capacity.weight === "object" && capacity.weight !== null) {
				const record = capacity.weight as Record<string, number>
				if (typeof record.field === "number") {
					weight = { kind: "field", field: sealedName(sourceRelation, record.field) }
				}
				if (typeof record.durationOf === "number") {
					weight = { kind: "durationField", field: sealedName(sourceRelation, record.durationOf) }
				}
			}
			const lo = { kind: "lit", value: BigInt(capacity.lo) } as const
			let window: SchemaSpec["statements"][number] extends never
				? never
				: Extract<StatementSpec, { kind: "capacity" }>["window"]
			if (capacity.hi === undefined) {
				window = { kind: "floor", lo }
			} else if (typeof capacity.hi.lit === "string") {
				window = { kind: "range", lo, hi: { kind: "lit", value: BigInt(capacity.hi.lit) } }
			} else if (typeof capacity.hi.targetField === "number") {
				window = {
					kind: "range",
					lo,
					hi: { kind: "field", field: sealedName(targetRelation, capacity.hi.targetField) }
				}
			} else if (typeof capacity.hi.targetDuration === "number") {
				window = {
					kind: "range",
					lo,
					hi: { kind: "durationField", field: sealedName(targetRelation, capacity.hi.targetDuration) }
				}
			} else {
				throw errors.new(`corpus capacity hi unreadable: ${JSON.stringify(capacity.hi)}`)
			}
			return { kind: "capacity", target: sideOf(capacity.target), weight, window, source: sideOf(capacity.source) }
		}
		throw errors.new(`corpus statement unreadable: ${JSON.stringify(raw)}`)
	})
	return {
		relations: corpus.relations.map(function relationSpecOf(relation) {
			return {
				name: relation.name,
				fields: relation.fields.map(function fieldSpecOf(field) {
					return {
						name: field.name,
						valueType: typeOf(field.type),
						newtype: undefined,
						fresh: field.generation === "fresh"
					}
				}),
				closed:
					relation.extension === undefined
						? undefined
						: {
								newtype: `${relation.name}.id`,
								rows: relation.extension.map(function rowOf(row) {
									return {
										handle: row.handle,
										values: row.values.map(function literalOf(value): LiteralSpec {
											return { kind: "value", value: valueSpecOf(value) }
										})
									}
								})
							}
			}
		}),
		statements
	}
}

interface BatchFixture {
	readonly expect: "ok" | "refusal" | "encode-refusal"
	readonly schema: string
	readonly fingerprint: string
	readonly refusal?: string
	readonly header?: {
		readonly braid: string
		readonly braidGen: string
		readonly prev: string
		readonly timestamp: string
		readonly writer: string
	}
	readonly ops?: ReadonlyArray<{
		readonly kind: "insert" | "delete"
		readonly relation: number
		readonly rows: readonly (readonly CorpusValue[])[]
	}>
}

function digestField(hex: string): BatchHeader["prev"] {
	const bytes = fromHex(hex)
	if (bytes.length === 32) {
		return digest32(bytes)
	}
	return bytes as BatchHeader["prev"]
}

interface ChainFixture {
	readonly schema: string
	readonly fingerprint: string
	readonly braid: string
	readonly slot: string
	readonly chain: { readonly g: string; readonly prev: string; readonly ts: string }
	readonly expect: "ok" | "chainMismatch"
	readonly cause?: "prev" | "slot" | "timestamp"
	readonly writer?: string
}

if (!present) {
	describe("parity goldens", function suite() {
		test("skipped: crates/bumbledb-log/conformance/v3 is not in the tree", { skip: true }, function absent() {})
	})
} else {
	const schemasRaw = JSON.parse(fs.readFileSync(path.join(corpusRoot, "schemas.json"), "utf8")) as {
		schemas: Record<string, CorpusSchema>
	}
	const descriptors = new Map<string, Descriptor>()
	for (const [name, corpus] of Object.entries(schemasRaw.schemas)) {
		descriptors.set(name, assembleFromSpec(specOf(corpus)))
	}

	function pinned(fixture: BatchFixture): Descriptor {
		const descriptor = descriptors.get(fixture.schema)
		assert.ok(descriptor !== undefined, `fixture cites schema ${fixture.schema}`)
		return withFingerprint(descriptor, fixture.fingerprint)
	}

	describe("parity goldens: the synthetic corpus fingerprints", function suite() {
		test("every carried fingerprint is the corpus's own blake3(label + schema name)", function synthetic() {
			for (const file of fs.readdirSync(path.join(corpusRoot, "batch"))) {
				if (!file.endsWith(".json")) {
					continue
				}
				const fixture = JSON.parse(fs.readFileSync(path.join(corpusRoot, "batch", file), "utf8")) as BatchFixture
				const label = new TextEncoder().encode(`bumbledb-log corpus fingerprint: ${fixture.schema}`)
				assert.deepEqual(digest32(new Uint8Array(internalBlake3(label))), digest32FromHex(fixture.fingerprint), file)
			}
		})
	})

	describe("parity goldens: braids", function suite() {
		for (const file of fs.readdirSync(path.join(corpusRoot, "braids"))) {
			test(`braids/${file}`, function golden() {
				const fixture = JSON.parse(fs.readFileSync(path.join(corpusRoot, "braids", file), "utf8")) as {
					schema: string
					braids: Record<string, readonly number[]>
					serialAt: readonly number[]
				}
				const descriptor = descriptors.get(fixture.schema)
				assert.ok(descriptor !== undefined)
				const derived: Record<string, readonly number[]> = {}
				for (const [braid, members] of descriptor.braidMembers) {
					derived[braid] = members
				}
				assert.deepEqual(derived, fixture.braids)
				assert.deepEqual(
					descriptor.serialAtStatements.map(function idOf(entry) {
						return entry.statement
					}),
					fixture.serialAt
				)
			})
		}
	})

	describe("parity goldens: the batch corpus, byte for byte", function suite() {
		for (const file of fs.readdirSync(path.join(corpusRoot, "batch"))) {
			if (!file.endsWith(".json")) {
				continue
			}
			const stem = file.slice(0, -5)
			test(`batch/${stem}`, function golden() {
				const fixture = JSON.parse(fs.readFileSync(path.join(corpusRoot, "batch", file), "utf8")) as BatchFixture
				const descriptor = pinned(fixture)
				if (fixture.expect === "encode-refusal") {
					assert.ok(fixture.header !== undefined)
					const header = fixture.header
					const caught = errors.trySync(function encodeIt() {
						return encodeBatch(
							descriptor,
							{
								fingerprint: digest32FromHex(fixture.fingerprint),
								braid: braid(header.braid),
								braidGen: generation(BigInt(header.braidGen)),
								prev: digestField(header.prev),
								writer: BigInt(header.writer),
								timestamp: BigInt(header.timestamp)
							},
							[]
						)
					})
					assert.ok(caught.error, `${stem}: expected an encode refusal`)
					assert.ok(errors.is(caught.error, ErrRefused), `${stem}: expected ErrRefused`)
					assert.equal(refusalOf(caught.error)?.kind, fixture.refusal, `${stem}: encode refusal identity`)
					return
				}
				const bytes = new Uint8Array(fs.readFileSync(path.join(corpusRoot, "batch", `${stem}.bin`)))
				if (fixture.expect === "refusal") {
					const caught = errors.trySync(function decodeIt() {
						return decodeBatch(descriptor, bytes)
					})
					assert.ok(caught.error, `${stem}: expected a refusal`)
					assert.ok(errors.is(caught.error, ErrRefused), `${stem}: expected ErrRefused, got ${caught.error.message}`)
					assert.equal(refusalOf(caught.error)?.kind, fixture.refusal, `${stem}: refusal identity`)
					return
				}
				const decoded = decodeBatch(descriptor, bytes)
				assert.ok(fixture.header !== undefined && fixture.ops !== undefined)
				const header: BatchHeader = {
					fingerprint: digest32FromHex(fixture.fingerprint),
					braid: braid(fixture.header.braid),
					braidGen: generation(BigInt(fixture.header.braidGen)),
					prev: digest32FromHex(fixture.header.prev),
					writer: BigInt(fixture.header.writer),
					timestamp: BigInt(fixture.header.timestamp)
				}
				assert.deepEqual(decoded.header, header, `${stem}: header`)
				const ops: Op[] = fixture.ops.map(function opOf(op) {
					const relation = descriptor.relations[op.relation]
					assert.ok(relation !== undefined)
					return {
						op: op.kind,
						relation: relation.name,
						rows: op.rows.map(function rowOf(row) {
							return row.map(rawValue)
						})
					}
				})
				assert.deepEqual(decoded.ops, ops, `${stem}: ops`)
				const encoded = encodeBatch(descriptor, header, ops)
				assert.equal(toHex(encoded), toHex(bytes), `${stem}: byte-exact re-encode`)
			})
		}
	})

	describe("parity goldens: the chain corpus", function suite() {
		for (const file of fs.readdirSync(path.join(corpusRoot, "chain"))) {
			if (!file.endsWith(".json")) {
				continue
			}
			const stem = file.slice(0, -5)
			test(`chain/${stem}`, function golden() {
				const fixture = JSON.parse(fs.readFileSync(path.join(corpusRoot, "chain", file), "utf8")) as ChainFixture
				const bytes = new Uint8Array(fs.readFileSync(path.join(corpusRoot, "chain", `${stem}.bin`)))
				const descriptor = descriptors.get(fixture.schema)
				assert.ok(descriptor !== undefined, `fixture cites schema ${fixture.schema}`)
				const decoded = decodeBatch(withFingerprint(descriptor, fixture.fingerprint), bytes)
				const position = {
					g: generation(BigInt(fixture.chain.g)),
					prev: digest32FromHex(fixture.chain.prev),
					ts: BigInt(fixture.chain.ts)
				}
				const run = errors.trySync(function checkIt() {
					verifyChain(decoded.header, braid(fixture.braid), generation(BigInt(fixture.slot)), position)
				})
				if (fixture.expect === "ok") {
					assert.equal(run.error, undefined, `${stem}: the clean chain passes`)
					return
				}
				assert.ok(run.error, `${stem}: expected a chain mismatch`)
				assert.ok(errors.is(run.error, ErrChainMismatch), `${stem}: expected ErrChainMismatch`)
				const data = chainMismatchOf(run.error)
				assert.equal(data?.cause, fixture.cause, `${stem}: cause`)
				assert.equal(data?.braid, fixture.braid, `${stem}: fetched braid`)
				assert.equal(data?.slot, BigInt(fixture.slot), `${stem}: slot`)
				assert.ok(fixture.writer !== undefined && fixture.writer.length > 0, `${stem}: writer is present`)
				assert.equal(data?.writer, BigInt(fixture.writer), `${stem}: writer`)
			})
		}
	})
}
