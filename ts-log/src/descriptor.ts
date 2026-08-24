/**
 * The parsed protocol descriptor: the SDK's lowered `SchemaSpec` resolved
 * once — names to dense ids, handles to row ids, statements materialized
 * in the engine's own order (fresh auto-keys first, closed auto-keys,
 * then declared statements with `==` split into two containments) — so
 * every pure function downstream reads ids, never names. Braid
 * derivation and the schema fingerprint mirror live on the same parse:
 * one boundary, parsed in full, cached per theory value.
 */

import type {
	AnySchema,
	LiteralSetSpec,
	LiteralSpec,
	SchemaSpec,
	SideSpec,
	StatementSpec,
	ValueSpec,
	ValueTypeSpec
} from "@bjornpagen/bumbledb"
import { internalBlake3, lower } from "@bjornpagen/bumbledb"
import * as errors from "@superbuilders/errors"
import { ByteWriter, fromHex, toHex, utf8Encoder } from "#bytes.ts"
import type { Value } from "#value.ts"
import { writeCanonicalLiteral } from "#value.ts"

interface FieldInfo {
	readonly name: string
	readonly type: ValueTypeSpec
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

interface SerialStatement {
	readonly statement: number
	readonly braid: string
}

interface Descriptor {
	readonly relations: readonly RelationInfo[]
	readonly relationByName: ReadonlyMap<string, RelationInfo>
	readonly statements: readonly StatementInfo[]
	/** Ordinary relation id → braid id string (`c{smallest:08x}`). */
	readonly braidOfRelation: ReadonlyMap<number, string>
	/** Braid id string → member relation ids, ascending. */
	readonly braidMembers: ReadonlyMap<string, readonly number[]>
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
	const parsed = spec === theory ? parseSpec(spec) : (cache.get(spec) ?? parseSpec(spec))
	cache.set(theory, parsed)
	cache.set(spec, parsed)
	return parsed
}

function braidHex(relationId: number): string {
	return `c${relationId.toString(16).padStart(8, "0")}`
}

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
	readonly relations: RelationInfo[]
	readonly byName: Map<string, RelationInfo>
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
		if (field.newtype?.endsWith(".id")) {
			const owner = field.newtype.slice(0, -3)
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
		throw errors.new(
			`handle literal ${literal.handle} on ${relation.name}.${field.name}, which references no closed roster`
		)
	}
	const target = tables.byName.get(field.closedRef)
	if (target === undefined) {
		throw errors.new(`handle literal ${literal.handle}: unknown closed relation ${field.closedRef}`)
	}
	const id = target.handles.indexOf(literal.handle)
	if (id === -1) {
		throw errors.new(`handle literal ${literal.handle} is not in the ${target.name} roster`)
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
	const ordinal = relation.fields.findIndex(function byName(field) {
		return field.name === name
	})
	if (ordinal === -1) {
		throw errors.new(`relation ${relation.name} has no field ${name}`)
	}
	return ordinal
}

function sideOf(tables: SpecTables, side: SideSpec): SideInfo {
	const relation = tables.byName.get(side.relation)
	if (relation === undefined) {
		throw errors.new(`statement cites unknown relation ${side.relation}`)
	}
	const projection = side.projection.map(function ordinalOf(name) {
		return fieldOrdinal(relation, name)
	})
	const selection = side.selection.map(function bindingOf(binding) {
		const ordinal = fieldOrdinal(relation, binding[0])
		const field = relation.fields[ordinal]
		if (field === undefined) {
			throw errors.new(`relation ${relation.name} has no field ordinal ${ordinal}`)
		}
		return { field: ordinal, values: literalSetOf(tables, relation, field, binding[1]) }
	})
	return { relation: relation.id, projection, selection }
}

function boundValue(context: string, bound: { readonly kind: string }): bigint {
	if (bound.kind !== "lit") {
		throw errors.new(`${context}: dependent floors are refused by the schema grammar`)
	}
	return (bound as { readonly kind: "lit"; readonly value: bigint }).value
}

function capacityOf(
	tables: SpecTables,
	id: number,
	statement: Extract<StatementSpec, { kind: "capacity" }>
): StatementInfo {
	const target = sideOf(tables, statement.target)
	const source = sideOf(tables, statement.source)
	const sourceRelation = tables.byName.get(statement.source.relation)
	const targetRelation = tables.byName.get(statement.target.relation)
	if (sourceRelation === undefined || targetRelation === undefined) {
		throw errors.new("capacity statement cites unknown relations")
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

/** The `exact {n}` window: lo = hi = n; a non-literal n is grammar-refused upstream. */

function parseSpec(spec: SchemaSpec): Descriptor {
	const prepass = new Map<string, { closed: boolean; handles: readonly string[] }>()
	for (const relation of spec.relations) {
		prepass.set(relation.name, {
			closed: relation.closed !== undefined,
			handles: relation.closed === undefined ? [] : relation.closed.rows.map((row) => row.handle)
		})
	}

	const relations: RelationInfo[] = []
	const byName = new Map<string, RelationInfo>()
	spec.relations.forEach(function buildRelation(relation, id) {
		const fields = fieldsOf(prepass, relation)
		const info: RelationInfo = {
			id,
			name: relation.name,
			closed: relation.closed !== undefined,
			handles: prepass.get(relation.name)?.handles ?? [],
			fields,
			rows: []
		}
		relations.push(info)
		byName.set(relation.name, info)
	})
	const tables: SpecTables = { spec, relations, byName }

	spec.relations.forEach(function resolveRows(relation, id) {
		if (relation.closed === undefined) {
			return
		}
		const info = relations[id]
		if (info === undefined) {
			throw errors.new(`relation ordinal ${id} missing`)
		}
		const rows = relation.closed.rows.map(function resolveRow(row, rowId) {
			const values: Value[] = [BigInt(rowId)]
			row.values.forEach(function resolveCell(literal, column) {
				const field = info.fields[column + 1]
				if (field === undefined) {
					throw errors.new(`closed row ${row.handle} of ${relation.name}: no field at column ${column}`)
				}
				values.push(resolveLiteral(tables, info, field, literal))
			})
			return values
		})
		relations[id] = { ...info, rows }
		byName.set(relation.name, relations[id])
	})

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
					throw errors.new(`key statement cites unknown relation ${statement.relation}`)
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
				const source = sideOf(tables, statement.source)
				const target = sideOf(tables, statement.target)
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

	const { braidOfRelation, braidMembers } = deriveBraids(relations, statements)

	const serialAtStatements: SerialStatement[] = []
	for (const statement of statements) {
		if (statement.kind === "functionality" && statement.projection.length === 0) {
			const braid = braidOfRelation.get(statement.relation)
			if (braid !== undefined) {
				serialAtStatements.push({ statement: statement.id, braid })
			}
		}
		if (statement.kind === "capacity" && statement.target.projection.length === 0) {
			const targetRelation = relations[statement.target.relation]
			if (targetRelation !== undefined && !targetRelation.closed) {
				const braid = braidOfRelation.get(statement.target.relation)
				if (braid !== undefined) {
					serialAtStatements.push({ statement: statement.id, braid })
				}
			}
		}
	}

	let hashed: { readonly hex: string; readonly bytes: Uint8Array } | undefined
	function fingerprintLazily(): { readonly hex: string; readonly bytes: Uint8Array } {
		if (hashed === undefined) {
			const bytes = fingerprintOf(relations, statements)
			hashed = { hex: toHex(bytes), bytes }
		}
		return hashed
	}

	const descriptor: Descriptor = {
		relations,
		relationByName: byName,
		statements,
		braidOfRelation,
		braidMembers,
		serialAtStatements,
		get fingerprint() {
			return fingerprintLazily().hex
		},
		get fingerprintBytes() {
			return fingerprintLazily().bytes
		}
	}
	return descriptor
}

/**
 * The same descriptor under a pinned fingerprint — for stores whose
 * identity is carried (a manifest, a conformance sidecar) rather than
 * recomputed, e.g. when the mirror cannot hash a closed relation's
 * interned string axioms.
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

function deriveBraids(
	relations: readonly RelationInfo[],
	statements: readonly StatementInfo[]
): { braidOfRelation: Map<number, string>; braidMembers: Map<string, readonly number[]> } {
	const parent = new Map<number, number>()
	for (const relation of relations) {
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
		const source = relations[statement.source.relation]
		const target = relations[statement.target.relation]
		if (source === undefined || target === undefined || source.closed || target.closed) {
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
	const braidOfRelation = new Map<number, string>()
	const braidMembers = new Map<string, readonly number[]>()
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

/** `bumbledb-schema-v5` canonical bytes, mirrored from the engine's own encoder. */
const FORMAT_VERSION_LABEL = "bumbledb-schema-v5"

const VALUE_TYPE_TAG = { bool: 0, u64: 2, i64: 3, string: 4, fixedBytes: 5, interval: 6, fixedInterval: 7 } as const

function putLen(out: ByteWriter, len: number): void {
	out.u32le(len)
}

function putBytes(out: ByteWriter, raw: Uint8Array): void {
	putLen(out, raw.length)
	out.bytes(raw)
}

function putString(out: ByteWriter, text: string): void {
	putBytes(out, utf8Encoder.encode(text))
}

function putValueType(out: ByteWriter, type: ValueTypeSpec): void {
	switch (type.kind) {
		case "bool": {
			out.u8(VALUE_TYPE_TAG.bool)
			return
		}
		case "u64": {
			out.u8(VALUE_TYPE_TAG.u64)
			return
		}
		case "i64": {
			out.u8(VALUE_TYPE_TAG.i64)
			return
		}
		case "string": {
			out.u8(VALUE_TYPE_TAG.string)
			return
		}
		case "fixedBytes": {
			out.u8(VALUE_TYPE_TAG.fixedBytes)
			out.u16le(type.len)
			return
		}
		case "interval": {
			if (type.width === undefined) {
				out.u8(VALUE_TYPE_TAG.interval)
				out.u8(type.element === "u64" ? 0 : 1)
				return
			}
			out.u8(VALUE_TYPE_TAG.fixedInterval)
			out.u8(type.element === "u64" ? 0 : 1)
			out.u64le(type.width)
			return
		}
	}
}

function putSide(out: ByteWriter, relations: readonly RelationInfo[], side: SideInfo): void {
	out.u32le(side.relation)
	putLen(out, side.projection.length)
	for (const field of side.projection) {
		out.u16le(field)
	}
	putLen(out, side.selection.length)
	const relation = relations[side.relation]
	if (relation === undefined) {
		throw errors.new(`side cites unknown relation id ${side.relation}`)
	}
	for (const binding of side.selection) {
		out.u16le(binding.field)
		const field = relation.fields[binding.field]
		if (field === undefined) {
			throw errors.new(`selection cites unknown field ordinal ${binding.field}`)
		}
		putLen(out, binding.values.length)
		for (const value of binding.values) {
			if (typeof value === "string") {
				putString(out, value)
			} else {
				writeCanonicalLiteral(out, field.type, value)
			}
		}
	}
}

function fingerprintOf(relations: readonly RelationInfo[], statements: readonly StatementInfo[]): Uint8Array {
	const out = new ByteWriter(1024)
	putString(out, FORMAT_VERSION_LABEL)
	putLen(out, relations.length)
	for (const relation of relations) {
		putString(out, relation.name)
		putLen(out, relation.fields.length)
		for (const field of relation.fields) {
			putString(out, field.name)
			putValueType(out, field.type)
			out.u8(field.fresh ? 1 : 0)
		}
		if (!relation.closed) {
			out.u8(0)
		} else {
			out.u8(1)
			putLen(out, relation.rows.length)
			relation.rows.forEach(function putRow(row, rowId) {
				const handle = relation.handles[rowId]
				if (handle === undefined) {
					throw errors.new(`closed relation ${relation.name}: no handle for row ${rowId}`)
				}
				putString(out, handle)
				const fact = new ByteWriter(64)
				row.forEach(function putCell(value, ordinal) {
					const field = relation.fields[ordinal]
					if (field === undefined) {
						throw errors.new(`closed relation ${relation.name}: no field at ordinal ${ordinal}`)
					}
					if (field.type.kind === "string") {
						throw errors.new(
							`closed relation ${relation.name} has a string ground axiom — the fingerprint mirror does not carry interned axiom columns`
						)
					}
					writeCanonicalLiteral(fact, field.type, value)
				})
				putBytes(out, fact.finish())
			})
		}
	}
	putLen(out, statements.length)
	for (const statement of statements) {
		switch (statement.kind) {
			case "functionality": {
				out.u8(0)
				out.u32le(statement.relation)
				putLen(out, statement.projection.length)
				for (const field of statement.projection) {
					out.u16le(field)
				}
				break
			}
			case "containment": {
				out.u8(1)
				putSide(out, relations, statement.source)
				putSide(out, relations, statement.target)
				break
			}
			case "capacity": {
				out.u8(4)
				putSide(out, relations, statement.target)
				switch (statement.weight.kind) {
					case "unit": {
						out.u8(0)
						break
					}
					case "field": {
						out.u8(1)
						out.u16le(statement.weight.field)
						break
					}
					case "duration": {
						out.u8(2)
						out.u16le(statement.weight.field)
						break
					}
				}
				out.u64le(statement.lo)
				switch (statement.hi.kind) {
					case "unbounded": {
						out.u8(0)
						break
					}
					case "lit": {
						out.u8(1)
						out.u8(0)
						out.u64le(statement.hi.value)
						break
					}
					case "targetField": {
						out.u8(1)
						out.u8(1)
						out.u16le(statement.hi.field)
						break
					}
					case "targetDuration": {
						out.u8(1)
						out.u8(2)
						out.u16le(statement.hi.field)
						break
					}
				}
				putSide(out, relations, statement.source)
				break
			}
		}
	}
	return new Uint8Array(internalBlake3(out.finish()))
}

export type {
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
export { braidHex, descriptorOf, withFingerprint }
