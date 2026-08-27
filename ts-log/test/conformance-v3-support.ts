/**
 * Per-schema corpus assembly for the v:3 inventory walker. Loads one
 * fixture at a time so a closed-relation crash cannot take the rest
 * of the roster with it.
 */

import * as fs from "node:fs"
import * as path from "node:path"
import type { LiteralSpec, SchemaSpec, StatementSpec, ValueSpec, ValueTypeSpec } from "@bjornpagen/bumbledb"
import * as errors from "@superbuilders/errors"
import { fromHex } from "#bytes.ts"
import type { Descriptor } from "#descriptor.ts"
import { withFingerprint } from "#descriptor.ts"
import { assembleFromSpec } from "#test/assemble.ts"

const corpusRoot = path.resolve(import.meta.dirname, "../../crates/bumbledb-log/conformance/v3")

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

const schemasRaw = JSON.parse(fs.readFileSync(path.join(corpusRoot, "schemas.json"), "utf8")) as {
	schemas: Record<string, CorpusSchema>
}
const assembled = new Map<string, Descriptor>()

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
			let window: Extract<StatementSpec, { kind: "capacity" }>["window"]
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

function schemaNamed(name: string): Descriptor {
	const hit = assembled.get(name)
	if (hit !== undefined) {
		return hit
	}
	const corpus = schemasRaw.schemas[name]
	if (corpus === undefined) {
		throw errors.new(`fixture cites schema ${name}`)
	}
	const descriptor = assembleFromSpec(specOf(corpus))
	assembled.set(name, descriptor)
	return descriptor
}

function pinned(schema: string, fingerprint: string): Descriptor {
	return withFingerprint(schemaNamed(schema), fingerprint)
}

export { corpusRoot, pinned, schemaNamed }
