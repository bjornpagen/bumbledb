import * as errors from "@superbuilders/errors"
import {
	type AnyClosedIdField,
	type AnyField,
	assertDeclarationOrderKey,
	assertDeclarationRecord,
	type ClosedHandleTuple,
	type ClosedIdField,
	type ClosedRoster,
	type Infer,
	literalOf
} from "#fields.ts"
import type { AnyRelation, RelationField } from "#relation.ts"
import { resolveSelection, type SelectionBinding, type SelectionInput } from "#relation.ts"
import type { LiteralSpec } from "#spec.ts"

type PayloadField = Exclude<AnyField, { readonly fresh: true }>

type PayloadColumns = Record<string, PayloadField> & { readonly id?: never }

interface ClosedColumn {
	readonly name: string
	readonly field: PayloadField
}

interface ClosedRow {
	readonly handle: string
	readonly values: readonly LiteralSpec[]
}

interface ClosedData {
	readonly name: string
	readonly handles: readonly string[]
	readonly columns: readonly ClosedColumn[]
	readonly rows: readonly ClosedRow[]
}

type AxiomRow<Cols extends Record<string, PayloadField>> = { readonly [C in keyof Cols]: Infer<Cols[C]> }

type Axioms<Handles extends string, Cols extends Record<string, PayloadField>> = {
	readonly [H in Handles]: AxiomRow<Cols>
}

interface ClosedCore<
	Name extends string,
	Handles extends ClosedHandleTuple,
	Cols extends Record<string, PayloadField>
> {
	readonly name: Name

	readonly id: ClosedIdField<Name, Handles>
	readonly data: ClosedData

	readonly axioms: Axioms<Handles[number], Cols>

	readonly columns: Cols
}

type ClosedSelectionInput<Cols extends Record<string, PayloadField>> = SelectionInput<Cols>

interface SelectedClosed<
	Name extends string,
	Handles extends ClosedHandleTuple,
	Cols extends Record<string, PayloadField>
> {
	readonly relation: Closed<Name, Handles, Cols>
	readonly selection: readonly SelectionBinding[]
}

interface AnySelectedClosed {
	readonly relation: AnyClosed
	readonly selection: readonly SelectionBinding[]
}

interface ClosedSelectable<
	Name extends string,
	Handles extends ClosedHandleTuple,
	Cols extends Record<string, PayloadField>
> {
	where(selection: ClosedSelectionInput<Cols>): SelectedClosed<Name, Handles, Cols>
}

type Closed<Name extends string, Handles extends ClosedHandleTuple, Cols extends Record<string, PayloadField>> = [
	keyof Cols
] extends [never]
	? ClosedCore<Name, Handles, Cols>
	: ClosedCore<Name, Handles, Cols> & ClosedSelectable<Name, Handles, Cols>

interface AnyClosed {
	readonly name: string
	readonly id: AnyClosedIdField
	readonly data: ClosedData
	readonly axioms: Readonly<Record<string, object>>
	readonly columns: Readonly<Record<string, PayloadField>>
}

function isClosedMember(member: AnyRelation | AnyClosed): member is AnyClosed {
	return "handles" in member.data
}

function sealedFieldsOf(member: AnyRelation | AnyClosed): readonly RelationField[] {
	if (isClosedMember(member)) {
		return Object.freeze([Object.freeze({ name: "id", field: member.id }), ...member.data.columns])
	}
	return member.data.fields
}

function sealedFieldOf(member: AnyRelation | AnyClosed, fieldName: string): AnyField | undefined {
	const declared = sealedFieldsOf(member).find(function byName(candidate) {
		return candidate.name === fieldName
	})
	return declared?.field
}

function isHandleTuple(shape: unknown): shape is ClosedHandleTuple {
	return (
		Array.isArray(shape) &&
		shape.length > 0 &&
		shape.every(function stringHandle(handle) {
			return typeof handle === "string"
		})
	)
}

/**
 * The trusted seam of the payload tier: the handle tuple and the axioms
 * record are the same finite set — every tuple name is an own key, and
 * every own key is in the tuple.
 */
function handleKeysOwn<Handles extends string>(
	axioms: { readonly [H in Handles]: object },
	names: readonly Handles[]
): boolean {
	const listed = new Set<string>(names)
	return (
		names.every(function ownHandle(name) {
			return Object.hasOwn(axioms, name)
		}) &&
		Object.keys(axioms).every(function listedHandle(name) {
			return listed.has(name)
		})
	)
}

/**
 * The trusted seam of the axiom-readback mint: every handle carries an own
 * frozen row and every row carries every declared column as an own
 * property — verified before the record is admitted as the typed
 * {@link Axioms} (the trusted-admission-seam pattern — its home is
 * `isTypedScope` in query/lower.ts).
 */
function axiomsMinted<Handles extends string, Cols extends Record<string, PayloadField>>(
	record: Readonly<Record<string, object>>,
	handles: readonly Handles[],
	cols: readonly ClosedColumn[]
): record is Axioms<Handles, Cols> & Readonly<Record<string, object>> {
	return handles.every(function rowMinted(handle) {
		const row = record[handle]
		return (
			row !== undefined &&
			cols.every(function columnMinted(column) {
				return Object.hasOwn(row, column.name)
			})
		)
	})
}

function mintAxioms<Handles extends string, Cols extends Record<string, PayloadField>>(
	name: string,
	handles: readonly Handles[],
	cols: readonly ClosedColumn[],
	axioms: Axioms<Handles, Cols>
): Axioms<Handles, Cols> {
	const out: Record<string, object> = {}
	for (const handle of handles) {
		const row = Object.freeze({ ...axioms[handle] })
		Object.defineProperty(out, handle, { value: row, enumerable: true })
	}
	Object.freeze(out)
	if (!axiomsMinted<Handles, Cols>(out, handles, cols)) {
		throw errors.new(`closed relation ${name}: axiom-row minting incomplete`)
	}
	return out
}

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

function closed<Name extends string, Handles extends ClosedHandleTuple, Cols extends PayloadColumns>(
	name: Name,
	handles: Handles,
	columns?: Cols,
	axioms?: Axioms<Handles[number], Cols>
): Closed<Name, Handles, Record<never, never>> | Closed<Name, Handles, Cols> {
	if (!Array.isArray(handles)) {
		throw errors.new(
			`closed relation ${name}: payload columns declared without ground axioms — the payload tier is spelled closed(name, handles, columns, axioms)`
		)
	}
	if (!isHandleTuple(handles)) {
		throw errors.new(`closed relation ${name}: at least one handle is required (an empty vocabulary declares nothing)`)
	}
	if (columns === undefined) {
		if (axioms !== undefined) {
			throw errors.new(`closed relation ${name}: the bare tier declares no columns, so ground axioms are inadmissible`)
		}
		return closedBare(name, handles)
	}
	if (axioms === undefined) {
		throw errors.new(
			`closed relation ${name}: payload columns declared without ground axioms — the payload tier is spelled closed(name, handles, columns, axioms)`
		)
	}
	return closedPayload(name, handles, columns, axioms)
}

function closedBare<Name extends string, const Hs extends ClosedHandleTuple>(
	name: Name,
	handles: Hs
): Closed<Name, Hs, Record<never, never>> {
	const empty: Record<string, object> = {}
	for (const handle of handles) {
		/** A duplicated name mints one row; the roster's own duplicate refusal in {@link mintClosed} stays the judge. */
		if (!Object.hasOwn(empty, handle)) {
			Object.defineProperty(empty, handle, { value: Object.freeze({}), enumerable: true })
		}
	}
	Object.freeze(empty)
	if (!axiomsMinted<Hs[number], Record<never, never>>(empty, handles, [])) {
		throw errors.new(`closed relation ${name}: bare-tier axiom-row minting incomplete`)
	}
	return mintClosed(name, handles, {}, empty)
}

function closedPayload<Name extends string, Handles extends ClosedHandleTuple, Cols extends PayloadColumns>(
	name: Name,
	handles: Handles,
	columns: Cols,
	axioms: Axioms<Handles[number], Cols>
): Closed<Name, Handles, Cols> {
	assertDeclarationRecord(`closed relation ${name} columns`, columns)
	for (const columnName of Object.keys(columns)) {
		assertDeclarationOrderKey(`closed relation ${name} column`, columnName)
	}
	assertDeclarationRecord(`closed relation ${name} axioms`, axioms)
	for (const handle of handles) {
		assertDeclarationOrderKey(`closed relation ${name} handle`, handle)
	}
	if (!handleKeysOwn(axioms, handles)) {
		throw errors.new(`closed relation ${name}: handle enumeration incomplete`)
	}
	return mintClosed(name, handles, columns, axioms)
}

function surfaceMinted<
	Name extends string,
	Handles extends ClosedHandleTuple,
	Cols extends Record<string, PayloadField>
>(
	value: ClosedCore<Name, Handles, Cols>,
	cols: readonly ClosedColumn[]
): value is ClosedCore<Name, Handles, Cols> & Closed<Name, Handles, Cols> {
	const selectable = "where" in value && typeof value.where === "function"
	return cols.length > 0 ? selectable : !selectable
}

function mintClosed<Name extends string, Handles extends ClosedHandleTuple, Cols extends Record<string, PayloadField>>(
	name: Name,
	handles: Handles,
	columns: Cols,
	axioms: Axioms<Handles[number], Cols>
): Closed<Name, Handles, Cols> {
	assertDeclarationOrderKey("closed relation", name)
	const seen = new Set<string>()
	for (const handle of handles) {
		if (seen.has(handle)) {
			throw errors.new(`closed relation ${name}: duplicate handle ${handle}`)
		}
		seen.add(handle)
	}
	const handleList = Object.freeze(handles)
	const roster: ClosedRoster<Name, Handles> = Object.freeze({ name, handles: handleList })
	const cols: ClosedColumn[] = []
	for (const [columnName, field] of Object.entries(columns)) {
		assertDeclarationOrderKey(`closed relation ${name} column`, columnName)
		if (columnName === "id") {
			throw errors.new(
				`closed relation ${name}: the payload column id collides with the sealed shape's synthetic id (the relation mints its own id at ordinal 0; name the column something else)`
			)
		}
		cols.push(Object.freeze({ name: columnName, field }))
	}
	Object.freeze(cols)
	const rows: ClosedRow[] = handleList.map(function lowerRow(handle: Handles[number]) {
		const row: Readonly<Record<string, unknown>> = axioms[handle]
		const values = cols.map(function lowerAxiomLiteral(column) {
			return Object.freeze(literalOf(column.field, row[column.name]))
		})
		return Object.freeze({ handle, values: Object.freeze(values) })
	})
	const data: ClosedData = Object.freeze({
		name,
		handles: roster.handles,
		columns: cols,
		rows: Object.freeze(rows)
	})
	const id: ClosedIdField<Name, Handles> = Object.freeze({ kind: "u64", closed: roster })

	const axiomsOut = mintAxioms<Handles[number], Cols>(name, handleList, cols, axioms)
	const columnsOut: Cols = { ...columns }
	Object.freeze(columnsOut)
	const holder: { value: Closed<Name, Handles, Cols> | undefined } = { value: undefined }

	function where(selection: ClosedSelectionInput<Cols>): SelectedClosed<Name, Handles, Cols> {
		const owner = holder.value
		if (owner === undefined) {
			throw errors.new(`closed relation ${name}: self-reference read before construction completed`)
		}
		return Object.freeze({
			relation: owner,
			selection: resolveSelection(name, cols, Object.entries(selection))
		})
	}
	const core = { name, id, data, axioms: axiomsOut, columns: columnsOut }
	const value: ClosedCore<Name, Handles, Cols> =
		cols.length > 0 ? Object.freeze({ ...core, where }) : Object.freeze(core)
	if (!surfaceMinted(value, cols)) {
		throw errors.new(`closed relation ${name}: ergonomic-surface minting incomplete`)
	}
	holder.value = value
	return value
}

export type {
	AnyClosed,
	AnySelectedClosed,
	AxiomRow,
	Axioms,
	Closed,
	ClosedColumn,
	ClosedCore,
	ClosedData,
	ClosedRow,
	ClosedSelectable,
	ClosedSelectionInput,
	PayloadField,
	SelectedClosed
}
export { closed, isClosedMember, sealedFieldOf, sealedFieldsOf }
