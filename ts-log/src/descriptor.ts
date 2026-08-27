/**
 * The parsed protocol descriptor: one thin parse of the engine's sealed
 * truth — relation ids, sealed field order, resolved closed rows,
 * materialized statements, and the real fingerprint — plus the braid
 * map the protocol derives from that truth. Cached per theory value.
 */

import type {
	AnySchema,
	FactValue,
	SchemaSpec,
	SealedDescriptor,
	SealedStatement,
	ValueTypeSpec
} from "@bjornpagen/bumbledb"
import { internalDescriptor, lower } from "@bjornpagen/bumbledb"
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

interface SerialStatement {
	readonly statement: number
	readonly braid: Braid
}

function serialAtOf(
	byId: ReadonlyMap<number, RelationInfo>,
	statements: readonly SealedStatement[],
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
	readonly statements: readonly SealedStatement[]
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

function deriveBraids(
	byId: ReadonlyMap<number, RelationInfo>,
	statements: readonly SealedStatement[]
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

export type { Braid, Descriptor, FieldInfo, RelationInfo, SerialStatement, Theory }
export { braid, braidHex, deriveBraids, descriptorOf, serialAtOf, withFingerprint }
