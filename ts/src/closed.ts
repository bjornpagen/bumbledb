/**
 * Closed relations (`docs/architecture/10-data-model.md` § closed
 * relations): a vocabulary whose extension is declared in the schema — two
 * tiers, one function. The emission per closed relation mirrors the
 * macro's (host-enum analog): handle CONSTANTS on the value
 * (`Kind.Checking`, ids = declaration order, each a BARE `bigint` — no
 * brand), the `fromId` weld, an `id` field descriptor carrying the CLOSED
 * LINKAGE (the roster — pure structure, no declared domain: the laws type
 * the columns, and `schema()` names the id's generator class `"Kind.id"`)
 * for other relations' field blocks (`kind: Kind.id`), payload readback
 * (`Kind.axioms`), and the declared payload column descriptors
 * (`Kind.columns` — the runtime twin of the `Cols` type parameter, which
 * the face layer's structural wall reads). Bare tier: `closed("Kind", ["Checking",
 * "Savings"])`. Payload tier: `closed("Sev", { pages: bool }, { Critical:
 * { pages: true }, ... })` — one call, three arguments (the curried tier-2
 * spelling is DELETED — canonical utterance): the axioms record IS the
 * handle declaration, every handle carrying every column exactly once
 * (type-enforced). No fact type and no insert surface exist — closed
 * relations are unwritable by construction: the value simply lacks the
 * writable relation shape. Both tiers mint `match()` — exhaustive dispatch
 * over the handle union (arms typed by the mapped type, so a missing or
 * extra arm is a compile error; the payload tier's arm receives the typed
 * axiom row). The payload tier additionally mints `where()` — the
 * ψ-selection surface (`Kind.where({ mastered: true })` as a face source),
 * resolved through the ONE selection machine
 * (`relation.ts::resolveSelection`); the bare tier has no payload columns to
 * select on, so `.where` is absent there, at the type AND on the value.
 */

import * as errors from "@superbuilders/errors"
import type { OneOf } from "#face.ts"
import {
	type AnyField,
	assertDeclarationOrderKey,
	type ClosedIdField,
	type ClosedRoster,
	type Infer,
	literalOf
} from "#fields.ts"
import { resolveSelection, type SelectionBinding } from "#relation.ts"
import type { LiteralSpec } from "#spec.ts"

/**
 * The value-surface property names a handle may not shadow — the macro's
 * name-collision diagnostic, here over the closed value's own properties
 * (`relation`/`selection` are reserved so a closed value can never be
 * mistaken for a selected relation by `on()`'s discriminant; `where` is
 * reserved because the payload tier mints the ψ-selection method under
 * exactly that name; `match` because BOTH tiers mint the exhaustive
 * dispatch under exactly that name).
 */
const reservedHandleNames: readonly string[] = Object.freeze([
	"name",
	"id",
	"data",
	"axioms",
	"columns",
	"fromId",
	"where",
	"match",
	"relation",
	"selection"
])

/**
 * A payload column of a closed relation: any field descriptor except a
 * fresh-marked one (a vocabulary's rows are ground axioms, never minted).
 */
type PayloadField = Exclude<AnyField, { readonly fresh: true }>

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
	 * field block is the reference through which bare handle ids become
	 * legal in that relation's selections. Pure structure plus the roster —
	 * the referencing field's domain is law-born: `schema()` computes it
	 * from the declared containment (`"Kind.id"`, the generator class).
	 */
	readonly id: ClosedIdField
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
	/** The weld: declaration-order id back to its handle, or undefined beyond the roster. */
	fromId(id: bigint): Handles | undefined
}

/**
 * The `where()` argument of a closed relation: per PAYLOAD column, a bare
 * structural literal of that column's value type or an `oneOf(a, b, ...)`
 * literal set — the ordinary `where()`'s vocabulary exactly. The synthetic
 * `id` is deliberately absent: an id selection is spelled only as handle
 * literals on the REFERENCING side (the canonical-utterance law).
 */
type ClosedSelectionInput<Cols extends Record<string, PayloadField>> = {
	readonly [C in keyof Cols]?: Infer<Cols[C]> | OneOf<Infer<Cols[C]>>
}

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
 * Exhaustive dispatch over a BARE closed vocabulary: one arm per handle,
 * no literal types and no brands — the handle-name union on the value's
 * type IS the exhaustiveness proof (a missing arm is a missing-property
 * compile error; an extra arm an excess-property compile error). The bare
 * tier declares no payload, so an arm takes nothing. The arms record types
 * any `bigint` in — the runtime roster refuses an out-of-vocabulary id
 * with a throw, never a misdispatch.
 */
interface ClosedMatchBare<Handles extends string> {
	match<T>(id: bigint, arms: { readonly [H in Handles]: () => T }): T
}

/**
 * Exhaustive dispatch over a PAYLOAD-tier closed vocabulary: the same
 * mapped-type exhaustiveness as the bare tier, and each arm receives its
 * handle's typed axiom row (the declared columns, bare and structural —
 * the frozen readback row from `axioms`).
 */
interface ClosedMatchPayload<Handles extends string, Cols extends Record<string, PayloadField>> {
	match<T>(id: bigint, arms: { readonly [H in Handles]: (row: AxiomRow<Cols>) => T }): T
}

/**
 * A closed relation value: the core surface plus one BARE constant per
 * handle (`Kind.Checking: bigint`, ids = declaration order — the value is
 * structural; the roster judges out-of-vocabulary ids at construction and
 * the engine at commit), plus `match()` on BOTH tiers (bare arms take
 * nothing; payload arms receive the typed axiom row), plus — exactly when
 * payload columns exist — `where()` (the bare tier has nothing to select
 * on, so the method is ABSENT there, not merely uncallable).
 */
type Closed<Name extends string, Handles extends string, Cols extends Record<string, PayloadField>> = ClosedCore<
	Name,
	Handles,
	Cols
> & { readonly [H in Handles]: bigint } & ([keyof Cols] extends [never]
		? ClosedMatchBare<Handles>
		: ClosedSelectable<Name, Handles, Cols> & ClosedMatchPayload<Handles, Cols>)

/** Any closed relation value, whatever its roster and columns. */
interface AnyClosed {
	readonly name: string
	readonly id: ClosedIdField
	readonly data: ClosedData
	readonly axioms: Readonly<Record<string, object>>
	readonly columns: Readonly<Record<string, PayloadField>>
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
 * The trusted seam of the handle-constant mint: every handle reads back as
 * an own bigint — verified before the record is admitted at the constants
 * type (a "__proto__"-named handle riding the object-protocol accessor
 * would fail exactly this check).
 */
function constantsMinted<Handles extends string>(
	record: Readonly<Record<string, bigint>>,
	handles: readonly Handles[]
): record is Readonly<Record<string, bigint>> & { readonly [H in Handles]: bigint } {
	return handles.every(function constantMinted(handle) {
		return typeof record[handle] === "bigint"
	})
}

/**
 * Reads one handle's ground axiom row for lowering. The typed payload
 * surface makes absence unrepresentable ({@link Axioms} carries every
 * handle's row); the refusal below guards the one ill-typed path — payload
 * columns with the bare tier's absent axioms — which no public spelling
 * reaches.
 */
function groundRow<Handles extends string, Cols extends Record<string, PayloadField>>(
	name: string,
	axioms: Axioms<Handles, Cols> | undefined,
	handle: Handles
): Readonly<Record<string, unknown>> {
	if (axioms === undefined) {
		throw errors.new(`closed relation ${name}: payload columns declared without ground axioms`)
	}
	return axioms[handle]
}

/**
 * Mints the axiom-readback record: one own frozen row per handle (the bare
 * tier's rows are empty — it declares no columns), each row a fresh copy of
 * its ground axiom.
 */
function mintAxioms<Handles extends string, Cols extends Record<string, PayloadField>>(
	name: string,
	handles: readonly Handles[],
	cols: readonly ClosedColumn[],
	axioms: Axioms<Handles, Cols> | undefined
): Axioms<Handles, Cols> {
	const out: Record<string, object> = {}
	for (const handle of handles) {
		const row = axioms === undefined ? Object.freeze({}) : Object.freeze({ ...groundRow(name, axioms, handle) })
		Object.defineProperty(out, handle, { value: row, enumerable: true })
	}
	Object.freeze(out)
	if (!axiomsMinted<Handles, Cols>(out, handles, cols)) {
		throw errors.new(`closed relation ${name}: axiom-row minting incomplete`)
	}
	return out
}

/** Mints the handle constants: one own bigint per handle, ids = declaration order. */
function mintHandleConstants<Handles extends string>(
	name: string,
	handles: readonly Handles[]
): { readonly [H in Handles]: bigint } {
	const out: Record<string, bigint> = {}
	handles.forEach(function mintHandleConstant(handle, index) {
		Object.defineProperty(out, handle, { value: BigInt(index), enumerable: true })
	})
	Object.freeze(out)
	if (!constantsMinted(out, handles)) {
		throw errors.new(`closed relation ${name}: handle-constant minting incomplete`)
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
function closed<const Name extends string, const Cols extends Record<string, PayloadField>, Handles extends string>(
	name: Name,
	columns: Cols,
	axioms: Axioms<Handles, Cols>
): Closed<Name, Handles, Cols>

function closed<const Name extends string, const Cols extends Record<string, PayloadField>, Handles extends string>(
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

/** The bare tier's precisely-typed builder: no columns, no axioms. */
function closedBare<Name extends string, Handles extends string>(
	name: Name,
	handles: readonly [Handles, ...Handles[]]
): Closed<Name, Handles, Record<never, never>> {
	return mintClosed<Name, Handles, Record<never, never>>(name, handles, {}, undefined)
}

/**
 * The payload tier's precisely-typed builder: column names are judged
 * first (the macro-expansion analog), then the handle set is read off the
 * axioms record's own keys.
 */
function closedPayload<Name extends string, Handles extends string, Cols extends Record<string, PayloadField>>(
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
 * The trusted seam of the ergonomic-surface mint: `match` reads back as an
 * own function on BOTH tiers, and `where` exactly when payload columns
 * exist — the runtime twin of the {@link Closed} type's conditional arm
 * (`ClosedMatchBare` vs `ClosedSelectable & ClosedMatchPayload`), verified
 * before the minted value is admitted at the conditional type.
 */
function surfaceMinted<Name extends string, Handles extends string, Cols extends Record<string, PayloadField>>(
	value: ClosedCore<Name, Handles, Cols> & { readonly [H in Handles]: bigint },
	cols: readonly ClosedColumn[]
): value is Closed<Name, Handles, Cols> {
	const matchable = "match" in value && typeof value.match === "function"
	const selectable = "where" in value && typeof value.where === "function"
	return matchable && (cols.length > 0 ? selectable : !selectable)
}

/**
 * Mints one closed relation value — the shared seam of both tiers, HONESTLY
 * typed end to end (a wrong-shaped mint is a compile error here, not a
 * laundered `unknown`): roster checks, eager axiom lowering, the
 * roster-carrying `id` descriptor, the frozen `columns` carrier (the runtime
 * twin of the `Cols` type parameter), the handle constants, the exhaustive
 * `match()` on both tiers, and — on the payload tier only — the
 * ψ-selection `where()`.
 */
function mintClosed<Name extends string, Handles extends string, Cols extends Record<string, PayloadField>>(
	name: Name,
	handles: readonly Handles[],
	columns: Cols,
	axioms: Axioms<Handles, Cols> | undefined
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
		if (reservedHandleNames.includes(handle)) {
			throw errors.new(
				`closed relation ${name}: handle ${handle} collides with the closed value's own surface (${reservedHandleNames.join(", ")})`
			)
		}
	}
	const handleList: readonly Handles[] = Object.freeze([...handles])
	const roster: ClosedRoster = Object.freeze({ name, handles: handleList })
	const cols: ClosedColumn[] = []
	for (const [columnName, field] of Object.entries(columns)) {
		assertDeclarationOrderKey(`closed relation ${name} column`, columnName)
		cols.push(Object.freeze({ name: columnName, field }))
	}
	Object.freeze(cols)
	const rows: ClosedRow[] = handleList.map(function lowerRow(handle) {
		const values = cols.map(function lowerAxiomLiteral(column) {
			const row = groundRow(name, axioms, handle)
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
	const id: ClosedIdField = Object.freeze({ kind: "u64", closed: roster })
	/**
	 * Handle names are arbitrary identifiers, so rows and constants are
	 * minted with OWN-property definition (inside {@link mintAxioms} and
	 * {@link mintHandleConstants}), never assignment: a handle named
	 * "__proto__" would otherwise ride the Object.prototype accessor —
	 * silently swapping the record's prototype instead of creating the row,
	 * and no-oping the constant (a primitive through the setter) — minting a
	 * value whose type claims a bigint constant but reads back an object.
	 * (Object SPREAD is CreateDataProperty by spec, so the copies below are
	 * own-property safe for column names too.)
	 */
	const axiomsOut = mintAxioms<Handles, Cols>(name, handleList, cols, axioms)
	const constants = mintHandleConstants(name, handleList)
	const columnsOut: Cols = { ...columns }
	Object.freeze(columnsOut)
	function fromId(idValue: bigint): Handles | undefined {
		return handleList[Number(idValue)]
	}
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
	/**
	 * The exhaustive dispatch: the ROSTER judges the id (the structural type
	 * admits any bigint — a dishonest id is a THROW, never a misdispatch),
	 * and the chosen arm receives the handle's frozen axiom row (the bare
	 * tier's arms are typed to take nothing; the empty row rides along
	 * unread). One implementation serves both tiers — the conditional
	 * {@link Closed} arm claims the tier-exact arm signature and
	 * {@link surfaceMinted} is the trusted seam that admits it.
	 */
	function match<T>(idValue: bigint, arms: { readonly [H in Handles]: (row: AxiomRow<Cols>) => T }): T {
		const handle = fromId(idValue)
		if (handle === undefined) {
			throw errors.new(`closed relation ${name}: match on id ${idValue} misses the roster (${handleList.join(", ")})`)
		}
		return arms[handle](axiomsOut[handle])
	}
	const core = { name, id, data, axioms: axiomsOut, columns: columnsOut, fromId }
	const value: ClosedCore<Name, Handles, Cols> & { readonly [H in Handles]: bigint } =
		cols.length > 0
			? Object.freeze({ ...constants, ...core, where, match })
			: Object.freeze({ ...constants, ...core, match })
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
	ClosedSelectionInput,
	PayloadField,
	SelectedClosed
}
export { closed }
