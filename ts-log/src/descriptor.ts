/**
 * The parsed protocol descriptor: one thin parse of the engine's sealed
 * truth — relation ids, sealed field order, resolved closed rows,
 * materialized statements, and the real fingerprint — plus the braid
 * map and serial-at roster read off the log core's braids seat
 * (`internalLogBraidsOf`): one derivation of the braid partition,
 * projected here into the driver's branded shapes. Cached per theory
 * value.
 */

import type {
	AnySchema,
	FactValue,
	LogCodecHandle,
	SchemaSpec,
	SealedDescriptor,
	SealedStatement,
	ValueTypeSpec
} from "@bjornpagen/bumbledb"
import { internalDescriptor, internalLogBraidsOf, internalLogCodec, lower } from "@bjornpagen/bumbledb"
import * as errors from "@superbuilders/errors"
import { regex } from "arkregex"
import { fromHex } from "#bytes.ts"

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
	readonly rows: ReadonlyArray<readonly FactValue[]>
}

declare const braidBrand: unique symbol
type Braid = string & { readonly [braidBrand]: typeof braidBrand }

const BRAID_ID = regex("^c[0-9a-f]{8}$")

function braid(raw: string): Braid {
	if (!BRAID_ID.test(raw)) {
		throw errors.new(`not a braid id: ${raw}`)
	}
	return raw as Braid
}

function refuseShape(why: string): never {
	throw errors.new(`theory: ${why}`)
}

interface SerialStatement {
	readonly statement: number
	readonly braid: Braid
}

/** The braids seat names serial-at statements by id; the braid tag is a
 *  projection join through the statement's own relation. */
function serialAtFrom(
	ids: readonly number[],
	statements: readonly SealedStatement[],
	braidOfRelation: ReadonlyMap<number, Braid>
): SerialStatement[] {
	const statementById = new Map<number, SealedStatement>()
	for (const statement of statements) {
		statementById.set(statement.id, statement)
	}
	return ids.map(function joinBraid(id) {
		const statement = statementById.get(id)
		if (statement === undefined) {
			refuseShape(`serial-at statement ${id} is not in the sealed statements`)
		}
		if (statement.kind === "containment") {
			refuseShape(`serial-at statement ${id} is a containment`)
		}
		const relation = statement.kind === "functionality" ? statement.relation : statement.target.relation
		const braid = braidOfRelation.get(relation)
		if (braid === undefined) {
			refuseShape(`serial-at statement ${id} relation ${relation} is in no braid`)
		}
		return { statement: id, braid }
	})
}

interface Descriptor {
	readonly relations: readonly RelationInfo[]
	readonly relationByName: ReadonlyMap<string, RelationInfo>
	readonly statements: readonly SealedStatement[]
	/** Ordinary relation id → braid id (`c{smallest:08x}`). */
	readonly braidOfRelation: ReadonlyMap<number, Braid>
	/** Braid id → member relation ids, ascending. */
	readonly braidMembers: ReadonlyMap<Braid, readonly number[]>
	readonly serialAtStatements: readonly SerialStatement[]
	/** The sealed per-theory codec handle — the grammar's one reader,
	 *  minted once and cached with the parse. */
	readonly codec: LogCodecHandle
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

function fromSealed(spec: SchemaSpec): Descriptor {
	const sealed: SealedDescriptor = internalDescriptor(spec)

	/** Handle newtype (`{name}.id`, off each closed relation's sealed id slot) → roster name. */
	const ownerOfIdClass = new Map<string, string>()
	for (const relation of sealed.relations) {
		if (relation.extension === undefined) {
			continue
		}
		const idSlot = relation.fields.find(function idOf(field) {
			return field.name === "id"
		})
		const handleClass = idSlot?.newtype
		if (handleClass === undefined) {
			refuseShape(`closed relation ${relation.name} has no handle class on its sealed id slot`)
		}
		ownerOfIdClass.set(handleClass, relation.name)
	}

	const relations: RelationInfo[] = sealed.relations.map(function relationOf(relation) {
		const fields: FieldInfo[] = relation.fields.map(function fieldOf(field) {
			return {
				name: field.name,
				type: field.valueType,
				fresh: field.fresh,
				closedRef: field.newtype === undefined ? undefined : ownerOfIdClass.get(field.newtype)
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
					return [cell.name, cell.value]
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
			closed: relation.extension !== undefined,
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

	const statements = sealed.statements
	const braids = internalLogBraidsOf(sealed)
	const braidOfRelation = new Map<number, Braid>()
	const braidMembers = new Map<Braid, readonly number[]>()
	for (const component of braids.components) {
		const id = braidHex(component.braid)
		braidMembers.set(id, component.relations)
		for (const member of component.relations) {
			braidOfRelation.set(member, id)
		}
	}
	const serialAtStatements = serialAtFrom(braids.serialAt, statements, braidOfRelation)

	const fingerprint = sealed.fingerprint
	return {
		relations,
		relationByName: byName,
		statements,
		braidOfRelation,
		braidMembers,
		serialAtStatements,
		codec: internalLogCodec(sealed),
		fingerprint,
		fingerprintBytes: fromHex(fingerprint)
	}
}

export type { Braid, Descriptor, FieldInfo, RelationInfo, SerialStatement, Theory }
export { braid, braidHex, descriptorOf, serialAtFrom }
