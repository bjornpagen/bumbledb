/**
 * The parsed protocol descriptor: one thin parse of the engine's sealed
 * truth — relation ids, sealed field order, resolved closed rows,
 * materialized statements, and the real fingerprint — plus the braid
 * map the protocol derives from that truth. Cached per theory value.
 */

import type {
	AnySchema,
	LiteralSetSpec,
	LiteralSpec,
	SchemaSpec,
	SealedDescriptor,
	SealedSide,
	SealedStatement,
	SideSpec,
	StatementSpec,
	ValueSpec
} from "@bjornpagen/bumbledb"
import { internalDescriptor, lower } from "@bjornpagen/bumbledb"
import * as errors from "@superbuilders/errors"
import { regex } from "arkregex"
import { fromHex } from "#bytes.ts"
import type { Value } from "#value.ts"

interface FieldInfo {
	readonly name: string
	readonly type: SchemaSpec["relations"][number]["fields"][number]["valueType"]
	readonly fresh: boolean
	/** Set when the field's newtype names a closed relation's id class. */
	readonly closedRef: string | undefined
}

interface RelationInfo {
	readonly id: number
	readonly name: string
	readonly closed: boolean
	readonly handles: readonly string[]
	/** Sealed order: a closed relation's synthetic u64 `id` at ordinal 0. */
	readonly fields: readonly FieldInfo[]
	/** Closed ground axioms in sealed order (id first), resolved values. */
	readonly rows: ReadonlyArray<readonly Value[]>
}

interface SideInfo {
	readonly relation: number
	readonly projection: readonly number[]
	readonly selection: ReadonlyArray<{ readonly field: number; readonly values: readonly Value[] }>
}

type WeightInfo =
	| { readonly kind: "unit" }
	| { readonly kind: "field"; readonly field: number }
	| { readonly kind: "duration"; readonly field: number }

type HiInfo =
	| { readonly kind: "unbounded" }
	| { readonly kind: "lit"; readonly value: bigint }
	| { readonly kind: "targetField"; readonly field: number }
	| { readonly kind: "targetDuration"; readonly field: number }

type StatementInfo =
	| {
			readonly id: number
			readonly kind: "functionality"
			readonly relation: number
			readonly projection: readonly number[]
	  }
	| { readonly id: number; readonly kind: "containment"; readonly source: SideInfo; readonly target: SideInfo }
	| {
			readonly id: number
			readonly kind: "capacity"
			readonly target: SideInfo
			readonly weight: WeightInfo
			readonly lo: bigint
			readonly hi: HiInfo
			readonly source: SideInfo
	  }

declare const braidBrand: unique symbol
type Braid = string & { readonly [braidBrand]: typeof braidBrand }

const BRAID_ID = regex("^c[0-9a-f]{8}$")
const ID_CLASS = regex("^(.*)\\.id$")

function braid(raw: string): Braid {
	if (!BRAID_ID.test(raw)) {
		throw errors.new(`not a braid id: ${raw}`)
	}
	return raw as Braid
}

function refuseShape(why: string): never {
	throw errors.new(`theory: ${why}`)
}

/** `{name}.id` — the closed relation's generator class. */
function idClassOwner(newtype: string): string | undefined {
	const match = ID_CLASS.exec(newtype)
	const owner = match?.[1]
	if (owner === undefined || owner.length === 0) {
		return undefined
	}
	return owner
}

function indexRelations(relations: readonly RelationInfo[]): Map<number, RelationInfo> {
	const byId = new Map<number, RelationInfo>()
	for (const relation of relations) {
		if (byId.has(relation.id)) {
			refuseShape(`duplicate relation id ${relation.id}`)
		}
		byId.set(relation.id, relation)
	}
	return byId
}

function relationOfId(byId: ReadonlyMap<number, RelationInfo>, id: number): RelationInfo {
	const relation = byId.get(id)
	if (relation === undefined) {
		refuseShape(`unknown relation id ${id}`)
	}
	return relation
}

function fieldNamed(relation: RelationInfo, name: string): { readonly ordinal: number; readonly field: FieldInfo } {
	let ordinal = 0
	for (const field of relation.fields) {
		if (field.name === name) {
			return { ordinal, field }
		}
		ordinal += 1
	}
	refuseShape(`relation ${relation.name} has no field ${name}`)
}

interface SerialStatement {
	readonly statement: number
	readonly braid: Braid
}

function serialAtOf(
	byId: ReadonlyMap<number, RelationInfo>,
	statements: readonly StatementInfo[],
	braidOfRelation: ReadonlyMap<number, Braid>
): SerialStatement[] {
	const serialAt: SerialStatement[] = []
	for (const statement of statements) {
		if (statement.kind === "functionality" && statement.projection.length === 0) {
			const relation = relationOfId(byId, statement.relation)
			if (relation.fields.length > 0) {
				const braid = braidOfRelation.get(statement.relation)
				if (braid !== undefined) {
					serialAt.push({ statement: statement.id, braid })
				}
			}
		}
		if (statement.kind === "capacity" && statement.target.projection.length === 0) {
			const targetRelation = relationOfId(byId, statement.target.relation)
			if (!targetRelation.closed) {
				const braid = braidOfRelation.get(statement.target.relation)
				if (braid === undefined) {
					refuseShape(`capacity ${statement.id} target ${targetRelation.name} is not in the derived braid set`)
				}
				serialAt.push({ statement: statement.id, braid })
			}
		}
	}
	return serialAt
}

interface Descriptor {
	readonly relations: readonly RelationInfo[]
	readonly relationByName: ReadonlyMap<string, RelationInfo>
	readonly statements: readonly StatementInfo[]
	/** Ordinary relation id → braid id (`c{smallest:08x}`). */
	readonly braidOfRelation: ReadonlyMap<number, Braid>
	/** Braid id → member relation ids, ascending. */
	readonly braidMembers: ReadonlyMap<Braid, readonly number[]>
	readonly serialAtStatements: readonly SerialStatement[]
	readonly fingerprint: string
	readonly fingerprintBytes: Uint8Array
}

/** The pure trio's input: the theory value, its lowered spec, or an already-parsed descriptor. */
type Theory = AnySchema | SchemaSpec | Descriptor

function isDescriptor(theory: Theory): theory is Descriptor {
	return "braidMembers" in theory
}

function isSpec(theory: Theory): theory is SchemaSpec {
	return Array.isArray((theory as SchemaSpec).relations)
}

const cache = new WeakMap<object, Descriptor>()

function descriptorOf(theory: Theory): Descriptor {
	if (isDescriptor(theory)) {
		return theory
	}
	const hit = cache.get(theory)
	if (hit !== undefined) {
		return hit
	}
	const spec = isSpec(theory) ? theory : lower(theory)
	const parsed = spec === theory ? fromSealed(spec) : (cache.get(spec) ?? fromSealed(spec))
	cache.set(theory, parsed)
	cache.set(spec, parsed)
	return parsed
}

function braidHex(relationId: number): Braid {
	return braid(`c${relationId.toString(16).padStart(8, "0")}`)
}

function asValue(raw: unknown): Value {
	if (typeof raw === "boolean" || typeof raw === "bigint" || typeof raw === "string" || raw instanceof Uint8Array) {
		return raw
	}
	if (typeof raw === "object" && raw !== null && "start" in raw && "end" in raw) {
		const interval = raw as { start: unknown; end: unknown }
		if (typeof interval.start === "bigint" && typeof interval.end === "bigint") {
			return { start: interval.start, end: interval.end }
		}
	}
	refuseShape("sealed value is not a raw fact value")
}

function sideOf(side: SealedSide): SideInfo {
	return {
		relation: side.relation,
		projection: side.projection,
		selection: side.selection.map(function valuesOf(binding) {
			return { field: binding.field, values: binding.values.map(asValue) }
		})
	}
}

function statementOf(statement: SealedStatement): StatementInfo {
	switch (statement.kind) {
		case "functionality":
			return statement
		case "containment":
			return {
				id: statement.id,
				kind: "containment",
				source: sideOf(statement.source),
				target: sideOf(statement.target)
			}
		case "capacity":
			return {
				id: statement.id,
				kind: "capacity",
				target: sideOf(statement.target),
				weight: statement.weight,
				lo: statement.lo,
				hi: statement.hi,
				source: sideOf(statement.source)
			}
	}
}

function fromSealed(spec: SchemaSpec): Descriptor {
	const sealed: SealedDescriptor = internalDescriptor(spec)
	const specByName = new Map<string, SchemaSpec["relations"][number]>()
	for (const relation of spec.relations) {
		if (specByName.has(relation.name)) {
			refuseShape(`duplicate relation ${relation.name}`)
		}
		specByName.set(relation.name, relation)
	}
	const closedOwners = new Set(
		spec.relations
			.filter(function closedOf(relation) {
				return relation.closed !== undefined
			})
			.map(function nameOf(relation) {
				return relation.name
			})
	)

	const relations: RelationInfo[] = sealed.relations.map(function relationOf(relation) {
		const declared = specByName.get(relation.name)
		if (declared === undefined) {
			refuseShape(`sealed relation ${relation.name} is not in the spec`)
		}
		const closed = relation.extension !== undefined
		const fields: FieldInfo[] = relation.fields.map(function fieldOf(field) {
			if (field.name === "id" && closed) {
				return { name: field.name, type: field.valueType, fresh: false, closedRef: relation.name }
			}
			const specField = declared.fields.find(function named(candidate) {
				return candidate.name === field.name
			})
			let closedRef: string | undefined
			const owner = specField?.newtype === undefined ? undefined : idClassOwner(specField.newtype)
			if (owner !== undefined && closedOwners.has(owner)) {
				closedRef = owner
			}
			return {
				name: field.name,
				type: field.valueType,
				fresh: specField?.fresh === true,
				closedRef
			}
		})
		const extension = relation.extension ?? []
		const named = new Map(
			relation.fields.map(function indexOf(field) {
				return [field.name, field]
			})
		)
		const rows = extension.map(function rowOf(row) {
			const cells = new Map(
				row.values.map(function cellOf(cell) {
					return [cell.name, asValue(cell.value)]
				})
			)
			return relation.fields.map(function cellValue(field) {
				const hit = cells.get(field.name)
				if (hit !== undefined) {
					return hit
				}
				if (field.name === "id" && named.has("id")) {
					return row.id
				}
				return refuseShape(`closed relation ${relation.name}: sealed row ${row.handle} missing field ${field.name}`)
			})
		})
		return {
			id: relation.id,
			name: relation.name,
			closed,
			handles: extension.map(function handleOf(row) {
				return row.handle
			}),
			fields,
			rows
		}
	})

	const byName = new Map<string, RelationInfo>()
	for (const relation of relations) {
		if (byName.has(relation.name)) {
			refuseShape(`duplicate relation ${relation.name}`)
		}
		byName.set(relation.name, relation)
	}

	const statements = sealed.statements.map(statementOf)
	const byId = indexRelations(relations)
	const { braidOfRelation, braidMembers } = deriveBraids(byId, statements)
	const serialAtStatements = serialAtOf(byId, statements, braidOfRelation)

	const fingerprint = sealed.fingerprint
	return {
		relations,
		relationByName: byName,
		statements,
		braidOfRelation,
		braidMembers,
		serialAtStatements,
		fingerprint,
		fingerprintBytes: fromHex(fingerprint)
	}
}

/**
 * The same descriptor under a pinned fingerprint — for stores whose
 * identity is carried (a manifest, a conformance sidecar) rather than
 * recomputed.
 */
function withFingerprint(descriptor: Descriptor, fingerprint: string): Descriptor {
	return {
		relations: descriptor.relations,
		relationByName: descriptor.relationByName,
		statements: descriptor.statements,
		braidOfRelation: descriptor.braidOfRelation,
		braidMembers: descriptor.braidMembers,
		serialAtStatements: descriptor.serialAtStatements,
		fingerprint,
		fingerprintBytes: fromHex(fingerprint)
	}
}

/**
 * Assemble a descriptor from a SchemaSpec that is not a theory — the
 * conformance corpus pins the codec and the braid map on shapes the
 * engine seal refuses. Theories enter through descriptorOf.
 */
function rawValue(value: ValueSpec): Value {
	switch (value.kind) {
		case "bool":
			return value.value
		case "u64":
		case "i64":
			return value.value
		case "string":
			return value.value
		case "fixedBytes":
			return value.value
		case "intervalU64":
		case "intervalI64":
			return { start: value.start, end: value.end }
	}
}

interface SpecTables {
	readonly spec: SchemaSpec
	readonly byName: Map<string, RelationInfo>
	readonly byId: Map<number, RelationInfo>
}

function zipClosedPayload(
	relation: RelationInfo,
	handle: string,
	literals: readonly LiteralSpec[]
): Array<readonly [FieldInfo, LiteralSpec]> {
	const pairs: Array<readonly [FieldInfo, LiteralSpec]> = []
	const pending = literals[Symbol.iterator]()
	let seenId = false
	for (const field of relation.fields) {
		if (!seenId) {
			if (field.name !== "id") {
				refuseShape(`closed relation ${relation.name} has no sealed id field`)
			}
			seenId = true
			continue
		}
		const next = pending.next()
		if (next.done) {
			refuseShape(`closed row ${handle} of ${relation.name}: fewer cells than payload fields`)
		}
		pairs.push([field, next.value])
	}
	if (!seenId) {
		refuseShape(`closed relation ${relation.name} has no sealed id field`)
	}
	if (!pending.next().done) {
		refuseShape(`closed row ${handle} of ${relation.name}: more cells than payload fields`)
	}
	return pairs
}

function fieldsOf(
	tables: Map<string, { closed: boolean; handles: readonly string[] }>,
	relation: SchemaSpec["relations"][number]
): FieldInfo[] {
	const fields: FieldInfo[] = []
	if (relation.closed !== undefined) {
		fields.push({
			name: "id",
			type: { kind: "u64" },
			fresh: false,
			closedRef: relation.name
		})
	}
	for (const field of relation.fields) {
		let closedRef: string | undefined
		const owner = field.newtype === undefined ? undefined : idClassOwner(field.newtype)
		if (owner !== undefined) {
			const target = tables.get(owner)
			if (target?.closed === true) {
				closedRef = owner
			}
		}
		fields.push({ name: field.name, type: field.valueType, fresh: field.fresh, closedRef })
	}
	return fields
}

function resolveLiteral(tables: SpecTables, relation: RelationInfo, field: FieldInfo, literal: LiteralSpec): Value {
	if (literal.kind === "value") {
		return rawValue(literal.value)
	}
	if (field.closedRef === undefined) {
		refuseShape(`handle literal ${literal.handle} on ${relation.name}.${field.name}, which references no closed roster`)
	}
	const target = tables.byName.get(field.closedRef)
	if (target === undefined) {
		refuseShape(`handle literal ${literal.handle}: unknown closed relation ${field.closedRef}`)
	}
	const id = target.handles.indexOf(literal.handle)
	if (id === -1) {
		refuseShape(`handle literal ${literal.handle} is not in the ${target.name} roster`)
	}
	return BigInt(id)
}

function literalSetOf(tables: SpecTables, relation: RelationInfo, field: FieldInfo, set: LiteralSetSpec): Value[] {
	if (set.kind === "one") {
		return [resolveLiteral(tables, relation, field, set.literal)]
	}
	return set.literals.map(function resolveEach(literal) {
		return resolveLiteral(tables, relation, field, literal)
	})
}

function fieldOrdinal(relation: RelationInfo, name: string): number {
	return fieldNamed(relation, name).ordinal
}

function specSideOf(tables: SpecTables, side: SideSpec): SideInfo {
	const relation = tables.byName.get(side.relation)
	if (relation === undefined) {
		refuseShape(`statement cites unknown relation ${side.relation}`)
	}
	const projection = side.projection.map(function ordinalOf(name) {
		return fieldOrdinal(relation, name)
	})
	const selection = side.selection.map(function bindingOf(binding) {
		const { ordinal, field } = fieldNamed(relation, binding[0])
		return { field: ordinal, values: literalSetOf(tables, relation, field, binding[1]) }
	})
	return { relation: relation.id, projection, selection }
}

function boundValue(context: string, bound: { readonly kind: string }): bigint {
	if (bound.kind !== "lit") {
		refuseShape(`${context}: dependent floors are refused by the schema grammar`)
	}
	return (bound as { readonly kind: "lit"; readonly value: bigint }).value
}

function capacityOf(
	tables: SpecTables,
	id: number,
	statement: Extract<StatementSpec, { kind: "capacity" }>
): StatementInfo {
	const target = specSideOf(tables, statement.target)
	const source = specSideOf(tables, statement.source)
	const sourceRelation = tables.byName.get(statement.source.relation)
	const targetRelation = tables.byName.get(statement.target.relation)
	if (sourceRelation === undefined || targetRelation === undefined) {
		refuseShape("capacity statement cites unknown relations")
	}
	let weight: WeightInfo
	switch (statement.weight.kind) {
		case "unit": {
			weight = { kind: "unit" }
			break
		}
		case "field": {
			weight = { kind: "field", field: fieldOrdinal(sourceRelation, statement.weight.field) }
			break
		}
		case "durationField": {
			weight = { kind: "duration", field: fieldOrdinal(sourceRelation, statement.weight.field) }
			break
		}
	}
	let lo: bigint
	let hi: HiInfo
	const window = statement.window
	switch (window.kind) {
		case "exact": {
			lo = boundValue("capacity exact bound", window.n)
			hi = { kind: "lit", value: lo }
			break
		}
		case "floor": {
			lo = boundValue("capacity floor", window.lo)
			hi = { kind: "unbounded" }
			break
		}
		case "range": {
			lo = boundValue("capacity floor", window.lo)
			switch (window.hi.kind) {
				case "lit": {
					hi = { kind: "lit", value: window.hi.value }
					break
				}
				case "field": {
					hi = { kind: "targetField", field: fieldOrdinal(targetRelation, window.hi.field) }
					break
				}
				case "durationField": {
					hi = { kind: "targetDuration", field: fieldOrdinal(targetRelation, window.hi.field) }
					break
				}
			}
			break
		}
	}
	return { id, kind: "capacity", target, weight, lo, hi, source }
}

function assembleFromSpec(spec: SchemaSpec): Descriptor {
	const prepass = new Map<string, { closed: boolean; handles: readonly string[] }>()
	for (const relation of spec.relations) {
		if (prepass.has(relation.name)) {
			refuseShape(`duplicate relation ${relation.name}`)
		}
		prepass.set(relation.name, {
			closed: relation.closed !== undefined,
			handles: relation.closed === undefined ? [] : relation.closed.rows.map((row) => row.handle)
		})
	}

	const byName = new Map<string, RelationInfo>()
	const byId = new Map<number, RelationInfo>()
	let nextId = 0
	for (const relation of spec.relations) {
		const table = prepass.get(relation.name)
		if (table === undefined) {
			refuseShape(`relation ${relation.name} missing from the closedness table`)
		}
		const info: RelationInfo = {
			id: nextId,
			name: relation.name,
			closed: table.closed,
			handles: table.handles,
			fields: fieldsOf(prepass, relation),
			rows: []
		}
		nextId += 1
		byName.set(relation.name, info)
		byId.set(info.id, info)
	}

	const tables: SpecTables = { spec, byName, byId }

	for (const relation of spec.relations) {
		if (relation.closed === undefined) {
			continue
		}
		const info = byName.get(relation.name)
		if (info === undefined) {
			refuseShape(`closed relation ${relation.name} missing`)
		}
		const rows = relation.closed.rows.map(function resolveRow(row, rowId) {
			const values: Value[] = [BigInt(rowId)]
			for (const [field, literal] of zipClosedPayload(info, row.handle, row.values)) {
				values.push(resolveLiteral(tables, info, field, literal))
			}
			return values
		})
		const updated = { ...info, rows }
		byName.set(relation.name, updated)
		byId.set(info.id, updated)
	}

	const relations = [...byId.values()]

	const statements: StatementInfo[] = []
	relations.forEach(function freshKeys(relation) {
		relation.fields.forEach(function freshKey(field, ordinal) {
			if (field.fresh) {
				statements.push({ id: statements.length, kind: "functionality", relation: relation.id, projection: [ordinal] })
			}
		})
	})
	relations.forEach(function closedKeys(relation) {
		if (relation.closed) {
			statements.push({ id: statements.length, kind: "functionality", relation: relation.id, projection: [0] })
		}
	})
	for (const statement of spec.statements) {
		switch (statement.kind) {
			case "fd": {
				const relation = byName.get(statement.relation)
				if (relation === undefined) {
					refuseShape(`key statement cites unknown relation ${statement.relation}`)
				}
				statements.push({
					id: statements.length,
					kind: "functionality",
					relation: relation.id,
					projection: statement.projection.map(function ordinalOf(name) {
						return fieldOrdinal(relation, name)
					})
				})
				break
			}
			case "containment": {
				const source = specSideOf(tables, statement.source)
				const target = specSideOf(tables, statement.target)
				statements.push({ id: statements.length, kind: "containment", source, target })
				if (statement.bidirectional) {
					statements.push({ id: statements.length, kind: "containment", source: target, target: source })
				}
				break
			}
			case "capacity": {
				statements.push(capacityOf(tables, statements.length, statement))
				break
			}
		}
	}

	const { braidOfRelation, braidMembers } = deriveBraids(byId, statements)
	const serialAtStatements = serialAtOf(byId, statements, braidOfRelation)

	const fingerprint = "00".repeat(32)
	return {
		relations,
		relationByName: byName,
		statements,
		braidOfRelation,
		braidMembers,
		serialAtStatements,
		fingerprint,
		fingerprintBytes: fromHex(fingerprint)
	}
}

function deriveBraids(
	byId: ReadonlyMap<number, RelationInfo>,
	statements: readonly StatementInfo[]
): { braidOfRelation: Map<number, Braid>; braidMembers: Map<Braid, readonly number[]> } {
	const parent = new Map<number, number>()
	for (const relation of byId.values()) {
		if (!relation.closed) {
			parent.set(relation.id, relation.id)
		}
	}
	function rootOf(id: number): number {
		let cursor = id
		for (;;) {
			const up = parent.get(cursor)
			if (up === undefined || up === cursor) {
				return cursor
			}
			cursor = up
		}
	}
	function union(a: number, b: number): void {
		const ra = rootOf(a)
		const rb = rootOf(b)
		if (ra !== rb) {
			parent.set(Math.max(ra, rb), Math.min(ra, rb))
		}
	}
	for (const statement of statements) {
		if (statement.kind === "functionality") {
			continue
		}
		const source = relationOfId(byId, statement.source.relation)
		const target = relationOfId(byId, statement.target.relation)
		if (source.closed || target.closed) {
			continue
		}
		union(source.id, target.id)
	}
	const members = new Map<number, number[]>()
	for (const id of parent.keys()) {
		const root = rootOf(id)
		const list = members.get(root)
		if (list === undefined) {
			members.set(root, [id])
		} else {
			list.push(id)
		}
	}
	const braidOfRelation = new Map<number, Braid>()
	const braidMembers = new Map<Braid, readonly number[]>()
	for (const [root, ids] of [...members.entries()].sort(function byRoot(a, b) {
		return a[0] - b[0]
	})) {
		const braid = braidHex(root)
		ids.sort(function ascending(a, b) {
			return a - b
		})
		braidMembers.set(braid, ids)
		for (const id of ids) {
			braidOfRelation.set(id, braid)
		}
	}
	return { braidOfRelation, braidMembers }
}

export type {
	Braid,
	Descriptor,
	FieldInfo,
	HiInfo,
	RelationInfo,
	SerialStatement,
	SideInfo,
	StatementInfo,
	Theory,
	WeightInfo
}
export { assembleFromSpec, braid, braidHex, descriptorOf, withFingerprint }
