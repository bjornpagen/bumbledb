/**
 * The conflict algebra's footprint (15): a pure function of (descriptor,
 * ops). Keys are blake3 over RAW command values — state-independent, so
 * equal keys mean equal values across writers. Net disposition first
 * (the batch's last op per fact identity wins, the base-independent
 * over-approximation L6 wants), then per-class emission; closed-target
 * statements emit nothing; W child deltas merge per key as one signed
 * i64 sum, and every batch's effective delta at any reachable base is
 * the evaporation interval `[Δ − Σw(F+ children), Δ + Σw(F− children)]`.
 */

import { internalBlake3 } from "@bjornpagen/bumbledb"
import * as errors from "@superbuilders/errors"
import { ByteWriter, bytesCompare, toHex } from "#bytes.ts"
import type { LogDescriptor, LogTheory, RelationInfo, SideInfo, StatementInfo } from "#descriptor.ts"
import { descriptorOf } from "#descriptor.ts"
import type { LogValue } from "#value.ts"
import { checkAgainst, valuesEqual, writeTagged } from "#value.ts"

interface BatchOp {
	readonly op: "insert" | "delete"
	readonly relation: string
	readonly rows: ReadonlyArray<readonly LogValue[]>
}

type FactMode = "insert" | "delete"
type ContainmentMode = "need" | "support+" | "support-"
type CapacityMode = "child" | "parent+" | "parent-"

type FootprintEntry =
	| { readonly class: "F"; readonly key: Uint8Array; readonly mode: FactMode }
	| { readonly class: "K"; readonly statement: number; readonly key: Uint8Array }
	| { readonly class: "C"; readonly statement: number; readonly key: Uint8Array; readonly mode: ContainmentMode }
	| {
			readonly class: "W"
			readonly statement: number
			readonly key: Uint8Array
			readonly mode: "child"
			readonly delta: bigint
	  }
	| { readonly class: "W"; readonly statement: number; readonly key: Uint8Array; readonly mode: "parent+" | "parent-" }

interface CapacityInterval {
	readonly statement: number
	readonly key: Uint8Array
	readonly keyHex: string
	readonly delta: bigint
	readonly lo: bigint
	readonly hi: bigint
}

interface KeyProvenance {
	readonly statement: number | undefined
	readonly values: readonly LogValue[]
}

interface FootprintRich {
	readonly entries: readonly FootprintEntry[]
	readonly intervals: ReadonlyMap<string, CapacityInterval>
	readonly provenance: ReadonlyMap<string, KeyProvenance>
}

const CLASS_BYTE = { F: 1, K: 2, C: 3, W: 4 } as const
const FACT_MODE_BYTE = { insert: 1, delete: 2 } as const
const CONTAINMENT_MODE_BYTE = { need: 1, "support+": 2, "support-": 3 } as const
const CAPACITY_MODE_BYTE = { child: 1, "parent+": 2, "parent-": 3 } as const

function modeByteOf(entry: FootprintEntry): number {
	switch (entry.class) {
		case "F":
			return FACT_MODE_BYTE[entry.mode]
		case "K":
			return 0
		case "C":
			return CONTAINMENT_MODE_BYTE[entry.mode]
		case "W":
			return CAPACITY_MODE_BYTE[entry.mode]
	}
}

function statementOf(entry: FootprintEntry): number {
	return entry.class === "F" ? -1 : entry.statement
}

/** The wire sort tuple: (class, statement, key, mode); F's statement is absent. */
function compareEntries(a: FootprintEntry, b: FootprintEntry): number {
	const classDelta = CLASS_BYTE[a.class] - CLASS_BYTE[b.class]
	if (classDelta !== 0) {
		return classDelta
	}
	const statementDelta = statementOf(a) - statementOf(b)
	if (statementDelta !== 0) {
		return statementDelta
	}
	const keyDelta = bytesCompare(a.key, b.key)
	if (keyDelta !== 0) {
		return keyDelta
	}
	return modeByteOf(a) - modeByteOf(b)
}

function entryIdentity(entry: FootprintEntry): string {
	return `${entry.class}:${entry.class === "F" ? "" : entry.statement}:${toHex(entry.key)}:${modeByteOf(entry)}`
}

/** The shared-key identity: (class, statement, key) — mode-blind. */
function keyIdentity(entry: FootprintEntry): string {
	return `${entry.class}:${entry.class === "F" ? "" : entry.statement}:${toHex(entry.key)}`
}

function blake3(bytes: Uint8Array): Uint8Array {
	return new Uint8Array(internalBlake3(bytes))
}

function taggedRowBytes(relation: RelationInfo, row: readonly LogValue[]): Uint8Array {
	const out = new ByteWriter(64)
	relation.fields.forEach(function writeCell(field, ordinal) {
		const value = row[ordinal]
		if (value === undefined) {
			throw errors.new(`relation ${relation.name}: row cell ${ordinal} absent`)
		}
		writeTagged(out, field.type, value)
	})
	return out.finish()
}

/** `fid = blake3(relation_id_le ∥ tagged raw values of the full row)`. */
function fidOf(relation: RelationInfo, row: readonly LogValue[]): Uint8Array {
	const out = new ByteWriter(72)
	out.u32le(relation.id)
	out.bytes(taggedRowBytes(relation, row))
	return blake3(out.finish())
}

/** `fkey = blake3(statement_id_le ∥ tagged raw values of the projection)`. */
function fkeyOf(
	statement: number,
	relation: RelationInfo,
	projection: readonly number[],
	row: readonly LogValue[]
): Uint8Array {
	const out = new ByteWriter(48)
	out.u16le(statement)
	for (const ordinal of projection) {
		const field = relation.fields[ordinal]
		const value = row[ordinal]
		if (field === undefined || value === undefined) {
			throw errors.new(`relation ${relation.name}: projection cites absent field ordinal ${ordinal}`)
		}
		writeTagged(out, field.type, value)
	}
	return blake3(out.finish())
}

function projectedValues(projection: readonly number[], row: readonly LogValue[]): LogValue[] {
	return projection.map(function pick(ordinal) {
		const value = row[ordinal]
		if (value === undefined) {
			throw errors.new(`projection cites absent field ordinal ${ordinal}`)
		}
		return value
	})
}

function matchesSelection(side: SideInfo, row: readonly LogValue[]): boolean {
	return side.selection.every(function binding(selection) {
		const value = row[selection.field]
		if (value === undefined) {
			return false
		}
		return selection.values.some(function member(candidate) {
			return valuesEqual(candidate, value)
		})
	})
}

function weightOf(statement: Extract<StatementInfo, { kind: "capacity" }>, row: readonly LogValue[]): bigint {
	switch (statement.weight.kind) {
		case "unit":
			return 1n
		case "field": {
			const value = row[statement.weight.field]
			if (typeof value !== "bigint") {
				throw errors.new("capacity weight field is not a u64 cell")
			}
			return value
		}
		case "duration": {
			const value = row[statement.weight.field]
			if (typeof value !== "object" || value instanceof Uint8Array) {
				throw errors.new("capacity duration weight field is not an interval cell")
			}
			return value.end - value.start
		}
	}
}

interface NetRow {
	readonly relation: RelationInfo
	readonly row: readonly LogValue[]
	mode: FactMode
	readonly fid: Uint8Array
}

/** Validates ops against the descriptor and nets them per fact identity. */
function netRowsOf(descriptor: LogDescriptor, ops: readonly BatchOp[]): Map<string, NetRow> {
	const net = new Map<string, NetRow>()
	for (const op of ops) {
		const relation = descriptor.relationByName.get(op.relation)
		if (relation === undefined) {
			throw errors.new(`op cites unknown relation ${op.relation}`)
		}
		if (relation.closed) {
			throw errors.new(`op writes closed relation ${op.relation} — sealed rows never change`)
		}
		for (const row of op.rows) {
			if (row.length !== relation.fields.length) {
				throw errors.new(
					`relation ${relation.name}: row arity ${row.length} does not match ${relation.fields.length} sealed fields`
				)
			}
			relation.fields.forEach(function checkCell(field, ordinal) {
				const value = row[ordinal]
				if (value === undefined) {
					throw errors.new(`relation ${relation.name}: row cell ${ordinal} absent`)
				}
				checkAgainst(`relation ${relation.name} field ${field.name}`, field.type, value)
			})
			const fid = fidOf(relation, row)
			const identity = toHex(fid)
			const existing = net.get(identity)
			if (existing === undefined) {
				net.set(identity, { relation, row, mode: op.op, fid })
			} else {
				existing.mode = op.op
			}
		}
	}
	return net
}

function computeFootprint(descriptor: LogDescriptor, ops: readonly BatchOp[]): FootprintRich {
	const net = netRowsOf(descriptor, ops)
	const entries = new Map<string, FootprintEntry>()
	const provenance = new Map<string, KeyProvenance>()
	const childSums = new Map<
		string,
		{ statement: number; key: Uint8Array; delta: bigint; insertW: bigint; deleteW: bigint }
	>()

	function put(entry: FootprintEntry, values: readonly LogValue[]): void {
		const identity = entryIdentity(entry)
		entries.set(identity, entry)
		provenance.set(keyIdentity(entry), { statement: entry.class === "F" ? undefined : entry.statement, values })
	}

	for (const netRow of net.values()) {
		const { relation, row, mode, fid } = netRow
		put({ class: "F", key: fid, mode }, row)
		for (const statement of descriptor.statements) {
			switch (statement.kind) {
				case "functionality": {
					if (statement.relation !== relation.id) {
						break
					}
					const key = fkeyOf(statement.id, relation, statement.projection, row)
					put({ class: "K", statement: statement.id, key }, projectedValues(statement.projection, row))
					break
				}
				case "containment": {
					const target = descriptor.relations[statement.target.relation]
					if (target === undefined || target.closed) {
						break
					}
					if (
						statement.source.relation === relation.id &&
						matchesSelection(statement.source, row) &&
						mode === "insert"
					) {
						const key = fkeyOf(statement.id, relation, statement.source.projection, row)
						put(
							{ class: "C", statement: statement.id, key, mode: "need" },
							projectedValues(statement.source.projection, row)
						)
					}
					if (statement.target.relation === relation.id && matchesSelection(statement.target, row)) {
						const key = fkeyOf(statement.id, relation, statement.target.projection, row)
						put(
							{ class: "C", statement: statement.id, key, mode: mode === "insert" ? "support+" : "support-" },
							projectedValues(statement.target.projection, row)
						)
					}
					break
				}
				case "capacity": {
					const target = descriptor.relations[statement.target.relation]
					if (target === undefined || target.closed) {
						break
					}
					if (statement.source.relation === relation.id && matchesSelection(statement.source, row)) {
						const key = fkeyOf(statement.id, relation, statement.source.projection, row)
						const identity = `W:${statement.id}:${toHex(key)}`
						const weight = weightOf(statement, row)
						const sums = childSums.get(identity) ?? {
							statement: statement.id,
							key,
							delta: 0n,
							insertW: 0n,
							deleteW: 0n
						}
						if (mode === "insert") {
							sums.delta += weight
							sums.insertW += weight
						} else {
							sums.delta -= weight
							sums.deleteW += weight
						}
						childSums.set(identity, sums)
						provenance.set(`W:${statement.id}:${toHex(key)}`, {
							statement: statement.id,
							values: projectedValues(statement.source.projection, row)
						})
					}
					if (statement.target.relation === relation.id && matchesSelection(statement.target, row)) {
						const key = fkeyOf(statement.id, relation, statement.target.projection, row)
						put(
							{ class: "W", statement: statement.id, key, mode: mode === "insert" ? "parent+" : "parent-" },
							projectedValues(statement.target.projection, row)
						)
					}
					break
				}
			}
		}
	}

	const intervals = new Map<string, CapacityInterval>()
	for (const [identity, sums] of childSums) {
		const entry: FootprintEntry = {
			class: "W",
			statement: sums.statement,
			key: sums.key,
			mode: "child",
			delta: sums.delta
		}
		entries.set(entryIdentity(entry), entry)
		intervals.set(identity, {
			statement: sums.statement,
			key: sums.key,
			keyHex: toHex(sums.key),
			delta: sums.delta,
			lo: sums.delta - sums.insertW,
			hi: sums.delta + sums.deleteW
		})
	}

	const sorted = [...entries.values()].sort(compareEntries)
	return { entries: sorted, intervals, provenance }
}

/** The published footprint section: the pure recomputation every replica runs. */
function footprintOf(theory: LogTheory, ops: readonly BatchOp[]): readonly FootprintEntry[] {
	return computeFootprint(descriptorOf(theory), ops).entries
}

/**
 * The W evaporation intervals (15): per shared parent key, the batch's
 * effective delta at any reachable base — what the intersection's
 * quantitative test consumes.
 */
function capacityIntervalsOf(theory: LogTheory, ops: readonly BatchOp[]): readonly CapacityInterval[] {
	return [...computeFootprint(descriptorOf(theory), ops).intervals.values()]
}

export type {
	BatchOp,
	CapacityInterval,
	CapacityMode,
	ContainmentMode,
	FactMode,
	FootprintEntry,
	FootprintRich,
	KeyProvenance
}
export {
	CAPACITY_MODE_BYTE,
	CLASS_BYTE,
	CONTAINMENT_MODE_BYTE,
	capacityIntervalsOf,
	compareEntries,
	computeFootprint,
	entryIdentity,
	FACT_MODE_BYTE,
	fidOf,
	fkeyOf,
	footprintOf,
	keyIdentity
}
