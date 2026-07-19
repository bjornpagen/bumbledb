/**
 * Closed relations (`docs/architecture/10-data-model.md` § closed
 * relations): a vocabulary whose extension is declared in the schema — two
 * tiers, one function. Bare tier: `closed("Kind", ["Checking",
 * "Savings"])`. Payload tier: `closed("Sev", { pages: bool }, { Critical:
 * { pages: true }, ... })` — one call, three arguments (the curried tier-2
 * spelling is DELETED — canonical utterance): the axioms record IS the
 * handle declaration, every handle carrying every column exactly once
 * (type-enforced). At the host surface a handle is its NAME — a string
 * literal of the roster's union (the drizzle law: translation, not
 * abstraction; dispatch over a vocabulary is native `switch` narrowing, so
 * no match operator is minted and no handle constants exist — the literal
 * `"Checking"` is the ONE spelling). Handles are pure DATA, not properties
 * of the value, so NO handle name is reserved: a vocabulary may legally
 * contain handles named `match`, `where`, or `id` — the axioms record and
 * the roster are their own namespaces. The value's whole surface: `name`;
 * `id` — the field descriptor carrying the CLOSED LINKAGE (the roster —
 * pure structure, no declared domain: the laws type the columns, and
 * `schema()` names the id's generator class `"Kind.id"`) for other
 * relations' field blocks (`kind: Kind.id`); `data` (the lowering
 * carrier); `axioms` (payload readback, `Kind.axioms`); `columns` (the
 * runtime twin of the `Cols` type parameter, which the face layer's
 * structural wall reads); and — exactly when payload columns exist —
 * `where()`, the ψ-selection surface (`Kind.where({ mastered: true })` as a
 * face source), resolved through the ONE selection machine
 * (`relation.ts::resolveSelection`); the bare tier has no payload columns
 * to select on, so `.where` is absent there, at the type AND on the value.
 * No fact type and no insert surface exist — closed relations are
 * unwritable by construction: the value simply lacks the writable relation
 * shape.
 */

import * as errors from "@superbuilders/errors"
import {
	type AnyField,
	assertDeclarationOrderKey,
	type ClosedIdField,
	type ClosedRoster,
	type Infer,
	literalOf
} from "#fields.ts"
import type { AnyRelation, RelationField } from "#relation.ts"
import { resolveSelection, type SelectionBinding, type SelectionInput } from "#relation.ts"
import type { LiteralSpec } from "#spec.ts"

/**
 * A payload column of a closed relation: any field descriptor except a
 * fresh-marked one (a vocabulary's rows are ground axioms, never minted).
 */
type PayloadField = Exclude<AnyField, { readonly fresh: true }>

/**
 * A declared payload column BLOCK: name → descriptor, with `id`
 * unspellable — the sealed shape mints the synthetic `id` itself (ordinal
 * 0 of the matchable fields), so a declared column named `id` would be
 * shadowed by the synthetic slot everywhere the shape resolves by name
 * (`sealedFieldsOf`, the projected face, `spec.rs`'s resolver). The wall is
 * typed here and judged again at construction in {@link mintClosed} — the
 * runtime twin for untyped callers, warmer and earlier than the engine's
 * `DuplicateFieldName` at `Db.create`.
 */
type PayloadColumns = Record<string, PayloadField> & { readonly id?: never }

/** One declared payload column: name plus its field descriptor. */
interface ClosedColumn {
	readonly name: string
	readonly field: PayloadField
}

/**
 * One ground axiom, already lowered: the handle plus one wire literal per
 * declared column in column-declaration order (row id = index). Lowered
 * EAGERLY at construction so axiom literals ride the same selection-literal
 * machine as `where()` bindings, with the same errors (the macro's rule).
 */
interface ClosedRow {
	readonly handle: string
	readonly values: readonly LiteralSpec[]
}

/** A closed relation's runtime description. */
interface ClosedData {
	readonly name: string
	readonly handles: readonly string[]
	readonly columns: readonly ClosedColumn[]
	readonly rows: readonly ClosedRow[]
}

/** One axiom row as the host writes and reads it: column name to bare structural value. */
type AxiomRow<Cols extends Record<string, PayloadField>> = { readonly [C in keyof Cols]: Infer<Cols[C]> }

/**
 * The whole axiom record: every handle exactly once, every column exactly
 * once per row — a missing or extra column is a TYPE error (each row is
 * contextually checked against the declared columns), and the handle set
 * IS the record's key set.
 */
type Axioms<Handles extends string, Cols extends Record<string, PayloadField>> = {
	readonly [H in Handles]: AxiomRow<Cols>
}

/**
 * The named surface of a closed relation value, minus the handle constants
 * (which {@link Closed} intersects in).
 */
interface ClosedCore<Name extends string, Handles extends string, Cols extends Record<string, PayloadField>> {
	readonly name: Name
	/**
	 * The closed reference descriptor: `kind: Kind.id` in another relation's
	 * field block is the reference through which handle literals become
	 * legal in that relation's selections. Pure structure plus the PRECISE
	 * roster (`ClosedIdField<Handles>` — the handle union is the field's
	 * value type under `Infer`); the referencing field's domain is law-born:
	 * `schema()` computes it from the declared containment (`"Kind.id"`, the
	 * generator class).
	 */
	readonly id: ClosedIdField<Handles>
	readonly data: ClosedData
	/** Payload readback: handle to its declared column values, bare and structural. */
	readonly axioms: Axioms<Handles, Cols>
	/**
	 * The declared payload columns, name → field descriptor — an HONEST
	 * frozen runtime record (the descriptors themselves, by identity), and
	 * the typed carrier a projected payload column's structural shape is
	 * recovered through off the schema type (the face layer's
	 * `ProjectedShape` reads it; `data.columns` carries the same
	 * descriptors in declaration order for the lowering).
	 */
	readonly columns: Cols
}

/**
 * The `where()` argument of a closed relation: EXACTLY the relation
 * surface's {@link SelectionInput}, over the declared payload columns — the
 * ONE selection vocabulary, so a spelling change there (H3's membership
 * arrays) flows through with no local change here. The synthetic `id` is
 * deliberately unspellable ({@link PayloadColumns} refuses an `id` column):
 * an id selection is spelled only as handle literals on the REFERENCING
 * side (the canonical-utterance law).
 */
type ClosedSelectionInput<Cols extends Record<string, PayloadField>> = SelectionInput<Cols>

/**
 * A closed relation with a ψ selection applied — what `on()` consumes as a
 * σ-carrying closed source (`on(Kind.where({ mastered: true }), "id")`).
 * Deliberately the SAME discriminant shape as the ordinary `Selected`
 * (`relation`/`selection` — `face.ts::faceParts` splits both by `"relation"
 * in source`), and structurally UNMISTAKABLE for one: an `AnyClosed` lacks
 * the relation shape (no `fields` record, no `RelationData`).
 */
interface SelectedClosed<Name extends string, Handles extends string, Cols extends Record<string, PayloadField>> {
	readonly relation: Closed<Name, Handles, Cols>
	readonly selection: readonly SelectionBinding[]
}

/** Any ψ-selected closed relation value. */
interface AnySelectedClosed {
	readonly relation: AnyClosed
	readonly selection: readonly SelectionBinding[]
}

/**
 * The ψ-selection surface of a payload-tier closed value. The selection is
 * resolved EAGERLY against the declared columns and lowered as-is — the SDK
 * never pre-folds ψ into an id set: pass-through lowering is what the macro
 * does, and the ENGINE folds against the sealed extension at validate
 * (`compile_member_set`).
 */
interface ClosedSelectable<Name extends string, Handles extends string, Cols extends Record<string, PayloadField>> {
	where(selection: ClosedSelectionInput<Cols>): SelectedClosed<Name, Handles, Cols>
}

/**
 * A closed relation value: the core surface plus — exactly when payload
 * columns exist — `where()` (the bare tier has nothing to select on, so
 * the method is ABSENT there, not merely uncallable). NOTHING else:
 * handles are data on the roster, never properties of the value (the
 * handle constants, the match operator, and the id-to-handle weld died
 * with the bigint era — dispatch is native `switch` narrowing over the
 * handle union).
 */
type Closed<Name extends string, Handles extends string, Cols extends Record<string, PayloadField>> = [
	keyof Cols
] extends [never]
	? ClosedCore<Name, Handles, Cols>
	: ClosedCore<Name, Handles, Cols> & ClosedSelectable<Name, Handles, Cols>

/** Any closed relation value, whatever its roster and columns. */
interface AnyClosed {
	readonly name: string
	readonly id: ClosedIdField
	readonly data: ClosedData
	readonly axioms: Readonly<Record<string, object>>
	readonly columns: Readonly<Record<string, PayloadField>>
}

/**
 * THE relation-kind discriminant — the ONE spelling of "is this schema
 * member closed?" (the type tier's twin is the `AnyClosed` conditional
 * arms). A closed relation's runtime description carries its handle
 * roster; an ordinary relation's never does. Every runtime closed/ordinary
 * fork in the SDK judges through this predicate — never a re-spelled
 * structural probe.
 */
function isClosedMember(member: AnyRelation | AnyClosed): member is AnyClosed {
	return "handles" in member.data
}

/**
 * The SEALED field list of a schema member — THE one reader of "what
 * fields does this owner expose": an ordinary relation's declared fields;
 * a closed relation's sealed shape — the synthetic `id` (the value's own
 * roster-carrying descriptor, by identity) at ordinal 0, then the declared
 * payload columns at declared index + 1 (the sealed shift, mirroring the
 * engine's `SchemaDescriptor::sealed_fields`). A `ClosedColumn` is
 * structurally a `RelationField`, so both kinds read uniformly.
 */
function sealedFieldsOf(member: AnyRelation | AnyClosed): readonly RelationField[] {
	if (isClosedMember(member)) {
		return Object.freeze([Object.freeze({ name: "id", field: member.id }), ...member.data.columns])
	}
	return member.data.fields
}

/**
 * One sealed field by name — derived from {@link sealedFieldsOf}, so the
 * closed synthetic `id` resolves everywhere a name is looked up (no reader
 * can silently lack the `id` arm). `undefined` when the name is foreign
 * (the type tiers make that unwritable; the engine re-judges at
 * `Db.create`).
 */
function sealedFieldOf(member: AnyRelation | AnyClosed, fieldName: string): AnyField | undefined {
	const declared = sealedFieldsOf(member).find(function byName(candidate) {
		return candidate.name === fieldName
	})
	return declared?.field
}

/** Narrows the two-tier second argument: a handle tuple (bare tier) or a column block (payload tier). */
function isHandleTuple(
	shape: readonly [string, ...string[]] | Record<string, PayloadField>
): shape is readonly [string, ...string[]] {
	return Array.isArray(shape)
}

/**
 * The trusted seam of the payload tier's handle enumeration: the axioms
 * record's own enumerable keys ARE its handle set (the type says so —
 * {@link Axioms} is keyed by the handles), and this guard verifies exactly
 * that checkable fact before the key list is admitted at the handle type.
 */
function handleKeysOwn<Handles extends string>(
	axioms: { readonly [H in Handles]: object },
	names: readonly string[]
): names is readonly Handles[] {
	return names.every(function ownHandle(name) {
		return Object.hasOwn(axioms, name)
	})
}

/**
 * The trusted seam of the axiom-readback mint: every handle carries an own
 * frozen row and every row carries every declared column as an own
 * property — verified before the record is admitted as the typed
 * {@link Axioms} (the `refsComplete` analog of `relation()`).
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

/**
 * Mints the axiom-readback record: one own frozen row per handle (the bare
 * tier's rows are empty — it declares no columns), each row a fresh copy of
 * its ground axiom. Both tiers supply a REAL axioms record ({@link
 * closedBare} mints its empty rows), so the columns-without-axioms state is
 * unrepresentable here — no undefined arm exists to guard.
 */
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

/** Bare tier: `closed("Kind", ["Checking", "Savings"])` — handles only. */
function closed<const Name extends string, const Handles extends readonly [string, ...string[]]>(
	name: Name,
	handles: Handles
): Closed<Name, Handles[number], Record<never, never>>

/**
 * Payload tier: declared columns AND ground axioms, one call — `closed(
 * "Grade", { mastered: bool }, { DirectPass: { mastered: true }, Failed:
 * { mastered: false } })`. The curried tier-2 spelling is DELETED
 * (canonical utterance): `Cols` infers from the column block, the handle
 * set from the axioms record's keys (reverse mapped-type inference), and
 * every row is contextually checked against the declared columns — a
 * wrong-typed value errors ON its property. The axioms record's keys ARE
 * the handles (declaration order = key order, integer-index names
 * rejected); every row carries every column exactly once (type-enforced by
 * {@link Axioms}).
 */
function closed<const Name extends string, const Cols extends PayloadColumns, Handles extends string>(
	name: Name,
	columns: Cols,
	axioms: Axioms<Handles, Cols>
): Closed<Name, Handles, Cols>

function closed<const Name extends string, const Cols extends PayloadColumns, Handles extends string>(
	name: Name,
	shape: readonly [string, ...string[]] | Cols,
	axioms?: Axioms<Handles, Cols>
): Closed<Name, string, Record<never, never>> | Closed<Name, Handles, Cols> {
	if (isHandleTuple(shape)) {
		if (axioms !== undefined) {
			throw errors.new(`closed relation ${name}: the bare tier declares no columns, so ground axioms are inadmissible`)
		}
		return closedBare(name, shape)
	}
	if (axioms === undefined) {
		throw errors.new(
			`closed relation ${name}: payload columns declared without ground axioms — the payload tier is spelled closed(name, columns, axioms) (the curried spelling is deleted)`
		)
	}
	return closedPayload(name, shape, axioms)
}

/**
 * The bare tier's precisely-typed builder: no columns, and the axioms
 * record is the EMPTY-ROW record over the handle roster (one own frozen
 * `{}` per handle, `__proto__`-safe own-property definition) — the same
 * representation the payload tier carries, so `mintClosed` never sees a
 * tier fork and the columns-without-axioms state stops being spellable.
 */
function closedBare<Name extends string, Handles extends string>(
	name: Name,
	handles: readonly [Handles, ...Handles[]]
): Closed<Name, Handles, Record<never, never>> {
	const empty: Record<string, object> = {}
	for (const handle of handles) {
		/** A duplicated name mints one row; the roster's own duplicate refusal in {@link mintClosed} stays the judge. */
		if (!Object.hasOwn(empty, handle)) {
			Object.defineProperty(empty, handle, { value: Object.freeze({}), enumerable: true })
		}
	}
	Object.freeze(empty)
	if (!axiomsMinted<Handles, Record<never, never>>(empty, handles, [])) {
		throw errors.new(`closed relation ${name}: bare-tier axiom-row minting incomplete`)
	}
	return mintClosed<Name, Handles, Record<never, never>>(name, handles, {}, empty)
}

/**
 * The payload tier's precisely-typed builder: column names are judged
 * first (the macro-expansion analog), then the handle set is read off the
 * axioms record's own keys.
 */
function closedPayload<Name extends string, Handles extends string, Cols extends PayloadColumns>(
	name: Name,
	columns: Cols,
	axioms: Axioms<Handles, Cols>
): Closed<Name, Handles, Cols> {
	for (const columnName of Object.keys(columns)) {
		assertDeclarationOrderKey(`closed relation ${name} column`, columnName)
	}
	const handles = Object.keys(axioms)
	for (const handle of handles) {
		assertDeclarationOrderKey(`closed relation ${name} handle`, handle)
	}
	if (!handleKeysOwn(axioms, handles)) {
		throw errors.new(`closed relation ${name}: handle enumeration incomplete`)
	}
	return mintClosed<Name, Handles, Cols>(name, handles, columns, axioms)
}

/**
 * The trusted seam of the ergonomic-surface mint: `where` reads back as an
 * own function exactly when payload columns exist, and is ABSENT otherwise
 * — the runtime twin of the {@link Closed} type's conditional arm
 * (`ClosedCore` alone vs `ClosedCore & ClosedSelectable`), verified before
 * the minted value is admitted at the conditional type.
 */
function surfaceMinted<Name extends string, Handles extends string, Cols extends Record<string, PayloadField>>(
	value: ClosedCore<Name, Handles, Cols>,
	cols: readonly ClosedColumn[]
): value is ClosedCore<Name, Handles, Cols> & Closed<Name, Handles, Cols> {
	const selectable = "where" in value && typeof value.where === "function"
	return cols.length > 0 ? selectable : !selectable
}

/**
 * Mints one closed relation value — the shared seam of both tiers, HONESTLY
 * typed end to end (a wrong-shaped mint is a compile error here, not a
 * laundered `unknown`): roster checks, eager axiom lowering, the
 * roster-carrying `id` descriptor, the frozen `columns` carrier (the runtime
 * twin of the `Cols` type parameter), and — on the payload tier only — the
 * ψ-selection `where()`.
 */
function mintClosed<Name extends string, Handles extends string, Cols extends Record<string, PayloadField>>(
	name: Name,
	handles: readonly Handles[],
	columns: Cols,
	axioms: Axioms<Handles, Cols>
): Closed<Name, Handles, Cols> {
	if (handles.length === 0) {
		throw errors.new(`closed relation ${name}: at least one handle is required (an empty vocabulary declares nothing)`)
	}
	const seen = new Set<string>()
	for (const handle of handles) {
		if (seen.has(handle)) {
			throw errors.new(`closed relation ${name}: duplicate handle ${handle}`)
		}
		seen.add(handle)
	}
	const handleList: readonly Handles[] = Object.freeze([...handles])
	const roster: ClosedRoster<Handles> = Object.freeze({ name, handles: handleList })
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
	const rows: ClosedRow[] = handleList.map(function lowerRow(handle) {
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
	const id: ClosedIdField<Handles> = Object.freeze({ kind: "u64", closed: roster })
	/**
	 * Handle names are arbitrary identifiers, so axiom rows are minted with
	 * OWN-property definition (inside {@link mintAxioms}), never assignment:
	 * a handle named "__proto__" would otherwise ride the Object.prototype
	 * accessor — silently swapping the record's prototype instead of
	 * creating the row. (Object SPREAD is CreateDataProperty by spec, so the
	 * copies below are own-property safe for column names too.)
	 */
	const axiomsOut = mintAxioms<Handles, Cols>(name, handleList, cols, axioms)
	const columnsOut: Cols = { ...columns }
	Object.freeze(columnsOut)
	const holder: { value: Closed<Name, Handles, Cols> | undefined } = { value: undefined }
	/**
	 * The ψ selection: resolved against the declared payload columns through
	 * the ONE selection machine (`relation.ts::resolveSelection` — a
	 * `ClosedColumn` is structurally a `RelationField`), never pre-folded
	 * into an id set (the engine folds at validate).
	 */
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
	if (!surfaceMinted<Name, Handles, Cols>(value, cols)) {
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
