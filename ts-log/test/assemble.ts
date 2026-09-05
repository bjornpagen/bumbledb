/**
 * The shadow sealer: assemble a descriptor from a SchemaSpec that is not
 * a theory — the conformance corpus pins the codec and the braid map on
 * shapes the engine seal refuses. Theories enter through descriptorOf;
 * this module serves only the corpus tests and never ships.
 */
import type {
	FactValue,
	LiteralSetSpec,
	LiteralSpec,
	SchemaSpec,
	SealedDescriptor,
	SealedHi,
	SealedSide,
	SealedStatement,
	SealedWeight,
	SideSpec,
	StatementSpec,
	ValueSpec
} from "@bjornpagen/bumbledb"
import { internalLogBraidsOf, internalLogCodec } from "@bjornpagen/bumbledb"
import { regex } from "arkregex"
import { fromHex } from "#bytes.ts"
import type { Braid, Descriptor, FieldInfo, RelationInfo } from "#descriptor.ts"
import { braidHex, serialAtFrom } from "#descriptor.ts"
import { LogInputError } from "#errors.ts"

const ID_CLASS = regex("^(.*)\\.id$")
function refuseShape(why: string): never {
	throw new LogInputError({ message: `theory: ${why}` })
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
function fieldNamed(
	relation: RelationInfo,
	name: string
): {
	readonly ordinal: number
	readonly field: FieldInfo
} {
	let ordinal = 0
	for (const field of relation.fields) {
		if (field.name === name) {
			return { ordinal, field }
		}
		ordinal += 1
	}
	refuseShape(`relation ${relation.name} has no field ${name}`)
}
function rawValue(value: ValueSpec): FactValue {
	switch (value.kind) {
		case "bool":
			return value.value
		case "u64":
		case "i64":
		case "f64":
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
	tables: Map<
		string,
		{
			closed: boolean
			handles: readonly string[]
		}
	>,
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
function resolveLiteral(tables: SpecTables, relation: RelationInfo, field: FieldInfo, literal: LiteralSpec): FactValue {
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
function literalSetOf(tables: SpecTables, relation: RelationInfo, field: FieldInfo, set: LiteralSetSpec): FactValue[] {
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
function specSideOf(tables: SpecTables, side: SideSpec): SealedSide {
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
function boundValue(
	context: string,
	bound: {
		readonly kind: string
	}
): bigint {
	if (bound.kind !== "lit") {
		refuseShape(`${context}: dependent floors are refused by the schema grammar`)
	}
	return (
		bound as {
			readonly kind: "lit"
			readonly value: bigint
		}
	).value
}
function capacityOf(
	tables: SpecTables,
	id: number,
	statement: Extract<
		StatementSpec,
		{
			kind: "capacity"
		}
	>
): SealedStatement {
	const target = specSideOf(tables, statement.target)
	const source = specSideOf(tables, statement.source)
	const sourceRelation = tables.byName.get(statement.source.relation)
	const targetRelation = tables.byName.get(statement.target.relation)
	if (sourceRelation === undefined || targetRelation === undefined) {
		refuseShape("capacity statement cites unknown relations")
	}
	let weight: SealedWeight
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
	let hi: SealedHi
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
	const prepass = new Map<
		string,
		{
			closed: boolean
			handles: readonly string[]
		}
	>()
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
			const values: FactValue[] = [BigInt(rowId)]
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
	const statements: SealedStatement[] = []
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
	const fingerprint = "00".repeat(32)
	// The shadow wire: exactly what the bridge's descriptor walker reads
	// — names, layouts, fresh flags, extension PRESENCE, the assembled
	// statements verbatim — so the one derivation and the one codec judge
	// the corpus shapes too.
	const wire: SealedDescriptor = {
		relations: relations.map(function relationWire(relation) {
			const fields = relation.fields.map(function fieldWire(field, ordinal) {
				return { name: field.name, id: ordinal, valueType: field.type, fresh: field.fresh }
			})
			return relation.closed
				? { name: relation.name, id: relation.id, fields, extension: [] }
				: { name: relation.name, id: relation.id, fields }
		}),
		statements,
		fingerprint
	}
	const braids = internalLogBraidsOf(wire)
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
	return {
		relations,
		relationByName: byName,
		statements,
		braidOfRelation,
		braidMembers,
		serialAtStatements,
		codec: internalLogCodec(wire),
		fingerprint,
		fingerprintBytes: fromHex(fingerprint)
	}
}

export { assembleFromSpec }
