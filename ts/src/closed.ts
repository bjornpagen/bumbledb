import { AuthoringError } from "#errors.ts"
import {
	type AnyField,
	assertDeclarationOrderKey,
	assertDeclarationRecord,
	type ClosedHandleTuple,
	type ClosedIdField,
	type Infer,
	ownHandles,
	signaturesAgree
} from "#fields.ts"
import { descriptorCache } from "#immutable.ts"
import {
	type AnyRelation,
	type FieldsShape,
	ownFields,
	type RelationField,
	relationDescriptor,
	relationFields
} from "#relation.ts"
import { fieldValue, recordValue } from "#values.ts"

type PayloadField = AnyField
type PayloadColumns = FieldsShape & { readonly id?: never }
type AxiomRow<Cols extends FieldsShape> = { readonly [C in keyof Cols]: Infer<Cols[C]> }
type Axioms<Handles extends string, Cols extends FieldsShape> = { readonly [H in Handles]: AxiomRow<Cols> }

/** Closed relations declare their finite roster and its complete, typed ground facts. */
interface Closed<
	Name extends string = string,
	Handles extends ClosedHandleTuple = ClosedHandleTuple,
	Cols extends FieldsShape = FieldsShape
> {
	readonly kind: "closed"
	readonly name: Name
	readonly handles: Handles
	readonly columns: Cols
	readonly axioms: Axioms<Handles[number], Cols>
}
type AnyClosed = Closed

function isClosedMember(member: AnyRelation | AnyClosed): member is AnyClosed {
	return member.kind === "closed"
}

/** The synthetic id is an ordinary field descriptor, derived from the declared roster. */
function closedId<C extends AnyClosed>(member: C): ClosedIdField<C["name"], C["handles"]> {
	recordValue("closed relation", member, ["kind", "name", "handles", "columns", "axioms"])
	assertDeclarationOrderKey("closed relation", member.name)
	return Object.freeze({
		kind: "u64",
		closed: Object.freeze({ name: member.name, handles: ownHandles(`closed relation ${member.name}`, member.handles) })
	})
}

function closedDescriptor<C extends AnyClosed>(input: C): C
function closedDescriptor(input: unknown): AnyClosed
function closedDescriptor(input: unknown): AnyClosed {
	return checkedClosed(input)
}

const checkedClosed = descriptorCache((input): AnyClosed => {
	const raw = recordValue("closed relation", input, ["kind", "name", "handles", "columns", "axioms"])
	if (raw.kind !== "closed" || typeof raw.name !== "string")
		throw new AuthoringError({ message: "closed relation: expected a closed declaration" })
	assertDeclarationOrderKey("closed relation", raw.name)
	const handles = ownHandles(`closed relation ${raw.name}`, raw.handles)
	const columns = ownFields(`closed relation ${raw.name} columns`, raw.columns)
	if (Object.hasOwn(columns, "id"))
		throw new AuthoringError({
			message: `closed relation ${raw.name}: payload column id collides with the synthetic id`
		})
	const axioms = recordValue(`closed relation ${raw.name} axioms`, raw.axioms, handles)
	const names = Object.keys(columns)
	const owned = Object.fromEntries(
		handles.map((handle) => {
			const row = recordValue(`closed relation ${raw.name}.${handle}`, axioms[handle], names)
			return [
				handle,
				Object.freeze(
					Object.fromEntries(
						Object.entries(columns).map(([name, field]) => [
							name,
							fieldValue(`closed relation ${raw.name}.${handle}.${name}`, field, row[name])
						])
					)
				)
			]
		})
	)
	return Object.freeze({ kind: "closed", name: raw.name, handles, columns, axioms: Object.freeze(owned) })
})

function closed<const Name extends string, const Handles extends ClosedHandleTuple>(
	name: Name,
	handles: Handles
): Closed<Name, Handles, Record<never, never>>
function closed<const Name extends string, const Handles extends ClosedHandleTuple, const Cols extends PayloadColumns>(
	name: Name,
	handles: Handles,
	columns: Cols,
	axioms: Axioms<Handles[number], Cols>
): Closed<Name, Handles, Cols>
function closed(
	name: string,
	handles: ClosedHandleTuple,
	columns?: PayloadColumns,
	axioms?: Axioms<string, FieldsShape>
): AnyClosed {
	if (columns === undefined && axioms === undefined) {
		const roster = ownHandles(`closed relation ${name}`, handles)
		return closedDescriptor({
			kind: "closed",
			name,
			handles: roster,
			columns: {},
			axioms: Object.fromEntries(roster.map((handle) => [handle, {}]))
		})
	}
	if (columns === undefined || axioms === undefined)
		throw new AuthoringError({
			message: `closed relation ${name}: payload columns and ground axioms must be supplied together`
		})
	return closedDescriptor({ kind: "closed", name, handles, columns, axioms })
}

function memberDescriptor<M extends AnyRelation | AnyClosed>(input: M): M
function memberDescriptor(input: unknown): AnyRelation | AnyClosed
function memberDescriptor(input: unknown): AnyRelation | AnyClosed {
	if (typeof input !== "object" || input === null)
		throw new AuthoringError({ message: "relation: expected a declaration record" })
	assertDeclarationRecord("relation", input)
	if ("kind" in input && input.kind === "closed") return closedDescriptor(input)
	return relationDescriptor(input)
}

function sealedFieldsOf(member: AnyRelation | AnyClosed): readonly RelationField[] {
	return isClosedMember(member)
		? [
				{ name: "id", field: closedId(member) },
				...Object.entries(member.columns).map(([name, field]) => ({ name, field }))
			]
		: relationFields(member)
}
function sealedFieldOf(member: AnyRelation | AnyClosed, name: string): AnyField | undefined {
	if (isClosedMember(member)) {
		if (name === "id") return closedId(member)
		return Object.hasOwn(member.columns, name) ? member.columns[name] : undefined
	}
	return Object.hasOwn(member.fields, name) ? member.fields[name] : undefined
}

function sameValue(a: unknown, b: unknown): boolean {
	if (Object.is(a, b)) return true
	if (a instanceof Uint8Array && b instanceof Uint8Array)
		return a.length === b.length && a.every((byte, i) => byte === b[i])
	if (
		typeof a === "object" &&
		a !== null &&
		typeof b === "object" &&
		b !== null &&
		"start" in a &&
		"end" in a &&
		"start" in b &&
		"end" in b
	)
		return Object.is(a.start, b.start) && Object.is(a.end, b.end)
	return false
}

/** Equal names must denote equal ordered declarations, never constructor identities. */
function membersAgree(left: AnyRelation | AnyClosed | undefined, right: AnyRelation | AnyClosed): boolean {
	if (left === undefined) return false
	const a = memberDescriptor(left)
	const b = memberDescriptor(right)
	if (a === b) return true
	if (a.kind !== b.kind || a.name !== b.name) return false
	const af = sealedFieldsOf(a)
	const bf = sealedFieldsOf(b)
	if (
		af.length !== bf.length ||
		!af.every((field, i) => {
			const other = bf[i]
			return other !== undefined && field.name === other.name && signaturesAgree(field.field, other.field)
		})
	)
		return false
	if (isClosedMember(a) && isClosedMember(b)) {
		return (
			a.handles.length === b.handles.length &&
			a.handles.every(
				(handle, i) =>
					handle === b.handles[i] &&
					Object.keys(a.columns).every((name) => sameValue(a.axioms[handle]?.[name], b.axioms[handle]?.[name]))
			)
		)
	}
	return true
}

export type { AnyClosed, AxiomRow, Axioms, Closed, PayloadField }
export {
	closed,
	closedDescriptor,
	closedId,
	isClosedMember,
	memberDescriptor,
	membersAgree,
	sealedFieldOf,
	sealedFieldsOf
}
