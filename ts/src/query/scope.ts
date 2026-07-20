/**
 * Query scope terms, REFERENCE-IDENTITY edition: a query variable is an
 * OBJECT, minted fresh by {@link v} over a relation's statically-known
 * columns. `v(relation)` returns a record of fresh variables — one per
 * column, each typed at mint by its column's descriptor AND the mint
 * coordinate (the owner relation name and the column name), so
 * destructuring preserves every literal and every class
 * (`const { id, holder } = v(Account)`). Variable IDENTITY is the object
 * reference: reusing the same var value across binding positions IS the
 * join, and a name-collision join is unrepresentable (two `v()` calls mint
 * two distinct batches, so two same-named vars are two variables). Params
 * stay STRING-named — their names are the execute() params object's runtime
 * keys, an honest load-bearing channel, not a lie.
 *
 * THE DESIGN THEOREM. {@link JoinOk} is an EQUALITY (kind, class, width,
 * element, roster), so judging every binding position against the
 * variable's MINT slot ({@link MintSlotOf}) makes all cross-binding joins
 * mutually class-equal by transitivity — the env/sibling checks the
 * name-keyed edition needed are subsumed, deleted rather than ported. The
 * one check representation cannot carry is BOUNDNESS (is this var positively
 * bound in this rule): TypeScript types cannot see object identity, so
 * boundness moves from the type tier to construction-time walls only — an
 * explicit essential-vs-accidental concession; every runtime twin is
 * preserved.
 *
 * This module also owns the environment/typing utilities the whole surface
 * shares: the join descriptor {@link ClassedField}, the mint-slot machinery
 * ({@link MintSlotOf}/{@link MintClassOf}), the class-equality judgment
 * {@link JoinOk} with its runtime twin {@link fieldJoins}, and the
 * record-folding helpers `Params` and `Row` inference ride.
 */

import * as errors from "@superbuilders/errors"
import type { AnyClosed } from "#closed.ts"
import { sealedFieldsOf } from "#closed.ts"
import type { AnyField, Infer } from "#fields.ts"
import { rosterOf } from "#fields.ts"
import type { ClassLookup, ClassRecordOf, SchemaClasses } from "#law.ts"
import type { AnyRelation, RelationFields } from "#relation.ts"

/**
 * The runtime discriminant of query term values. Host literals (bigints,
 * strings, interval objects) never carry it, so "is this position a term
 * or a literal" is one property probe, never a guess.
 */
const term: unique symbol = Symbol("bumbledb.query.term")

/**
 * The carrier of a value's INFERRED types (a rule's row/params, a query's
 * row/params, a rec's params). The property is never present at runtime —
 * it exists so inference rides plain values without any brand on any
 * field value.
 */
const inferred: unique symbol = Symbol("bumbledb.query.inferred")

/**
 * What a query atom matches over: an ordinary relation or a CLOSED
 * vocabulary (ψ query atoms — the engine folds a resolvable closed atom
 * into a plan-constant member set at prepare, or joins the L1-resident
 * virtual image when the shape does not fold; the SDK never pre-folds and
 * never knows which — transparency is the contract).
 */
type MatchOwner = AnyRelation | AnyClosed

/**
 * The matchable field block of an atom owner: a relation's declared
 * fields; a closed relation's SEALED shape — the synthetic `id` (the
 * value's OWN roster-carrying descriptor, at its precise type) first, then
 * the declared payload columns read through the typed `columns` carrier.
 * The runtime twin is `sealedFieldsOf` in `#closed.ts`.
 */
type MatchFields<R extends MatchOwner> = R extends AnyClosed
	? { readonly id: R["id"] } & R["columns"]
	: R extends AnyRelation
		? RelationFields<R>
		: never

/**
 * A query variable — an OBJECT minted by {@link v}. Identity is the object
 * reference: reuse of the same value across binding positions is the join,
 * strictly rule-scoped (each rule numbers its own dense `VarId`s). The type
 * carries the mint COORDINATE — `RN` the owner relation name literal, `K`
 * the column name literal — and `F` the mint descriptor, so the mint slot
 * (descriptor + law-computed class) is recoverable at every binding
 * position for the join judgment.
 */
interface Var<F extends AnyField = AnyField, RN extends string = string, K extends string = string> {
	readonly [term]: "var"
	readonly owner: MatchOwner & { readonly name: RN }
	readonly column: K
	readonly field: F
	readonly label: string
}

/** Any query variable, whatever its descriptor and mint coordinate. */
type AnyVar = Var

/**
 * A scalar query parameter — `r.param("root")`. The name is the key of the
 * typed params object `execute` takes; the type is the element type of the
 * position that anchors it (a field binding, or the bound-variable side of
 * a comparison).
 */
interface Param<Name extends string = string> {
	readonly [term]: "param"
	readonly name: Name
}

/**
 * A set-valued query parameter (the IR's `ParamSet` term) — `r.inSet("frontier")`:
 * bound at execution to a readonly ARRAY of values of the anchoring field's
 * type; a binding position matches iff the field value is in the set. Legal
 * in atom bindings (positive and negated) and as the right side of `eq` —
 * nowhere else, exactly as the IR rules it.
 */
interface SetParam<Name extends string = string> {
	readonly [term]: "setParam"
	readonly name: Name
}

/**
 * An Allen-mask parameter (the IR's `MaskTerm::Param`) — the temporal
 * relation as a bind-time argument: one prepared query answers any of the
 * mask questions per execution. Bound to a 13-bit mask number built from
 * the `ALLEN` constants.
 */
interface MaskParam<Name extends string = string> {
	readonly [term]: "maskParam"
	readonly name: Name
}

/**
 * The measure of an interval-typed variable (`ir::Term::Measure`):
 * `|[s, e)| = e − s`, u64 — legal as one side of an order comparison, as a
 * find entry, and as the input of `sum`/`min`/`max`; every other position
 * is unwritable, exactly as the IR rejects it typed. Carries the interval
 * variable it measures BY REFERENCE.
 */
interface Duration<V extends AnyVar = AnyVar> {
	readonly [term]: "duration"
	readonly over: V
}

/** Any scope term value. */
type AnyTerm = Var | Param | SetParam | MaskParam | Duration

/** Narrows an unknown position value to a scope term (vs a host literal). */
function isTerm(value: unknown): value is AnyTerm {
	return typeof value === "object" && value !== null && term in value
}

/**
 * The record of fresh variables `v(owner)` mints — one per statically-known
 * column, each typed by its column's descriptor and mint coordinate.
 */
type VarsOf<R extends MatchOwner> = {
	readonly [K in keyof MatchFields<R> & string]: Var<MatchFields<R>[K], R["name"], K>
}

/**
 * The trusted admission seam of the variable-record mint (the pattern's
 * home is `isTypedScope` in `#query/lower.ts`): the checkable fact — one own
 * enumerable variable per sealed column — is verified before the record is
 * admitted at its computed {@link VarsOf} type.
 */
function varsMinted<R extends MatchOwner>(owner: R, record: Readonly<Record<string, AnyVar>>): record is VarsOf<R> {
	return sealedFieldsOf(owner).every(function columnMinted(declared) {
		return Object.hasOwn(record, declared.name)
	})
}

/**
 * Mints a FRESH batch of query variables over an atom owner's
 * statically-known columns — one variable per sealed column
 * (`sealedFieldsOf`: a closed owner mints `id` first, then payload columns),
 * each frozen and each defined by OWN-property definition (object-protocol
 * column names must work, the `closed()` precedent). Every `v()` call mints
 * new objects, so two batches are two variables; property access within one
 * batch is stable by construction (the record is an eager frozen record,
 * never a Proxy). Variable identity is the object reference: destructure
 * what you need (`const { id, holder } = v(Account)`) and reuse a value
 * across binding positions to join.
 */
function v<R extends MatchOwner>(owner: R): VarsOf<R> {
	const record: Record<string, AnyVar> = {}
	for (const declared of sealedFieldsOf(owner)) {
		const variable: AnyVar = Object.freeze({
			[term]: "var" as const,
			owner,
			column: declared.name,
			field: declared.field,
			label: `${owner.name}.${declared.name}`
		})
		Object.defineProperty(record, declared.name, { value: variable, enumerable: true })
	}
	Object.freeze(record)
	if (!varsMinted(owner, record)) {
		throw errors.new(`v(${owner.name}): variable-record minting incomplete`)
	}
	return record
}

/** Builds one scalar-parameter term. */
function makeParam<const Name extends string>(name: Name): Param<Name> {
	const value: Param<Name> = { [term]: "param", name }
	return Object.freeze(value)
}

/** Builds one set-parameter term. */
function makeSetParam<const Name extends string>(name: Name): SetParam<Name> {
	const value: SetParam<Name> = { [term]: "setParam", name }
	return Object.freeze(value)
}

/** Builds one Allen-mask-parameter term. */
function makeMaskParam<const Name extends string>(name: Name): MaskParam<Name> {
	const value: MaskParam<Name> = { [term]: "maskParam", name }
	return Object.freeze(value)
}

/** Builds one measure term over an interval-typed variable reference. */
function makeDuration<const V extends AnyVar>(over: V): Duration<V> {
	const value: Duration<V> = { [term]: "duration", over }
	return Object.freeze(value)
}

/**
 * One bound field slot: the field's descriptor plus the slot's
 * law-computed CLASS (`undefined` = bare — the slot is in no law). The one
 * shape every join judgment compares, at the TYPE level (a variable's mint
 * slot, a binding position's slot) and at RUNTIME alike.
 */
interface ClassedField {
	readonly field: AnyField
	readonly class: string | undefined
}

/**
 * A variable's law-computed CLASS at the TYPE level: its column's class,
 * read off the schema type's class map through the mint coordinate the
 * variable carries (`RN.K`). `undefined` = bare.
 */
type MintClassOf<Classes extends SchemaClasses, V> =
	V extends Var<AnyField, infer RN extends string, infer K extends string>
		? ClassLookup<ClassRecordOf<Classes, RN>, K>
		: never

/**
 * A variable's MINT slot: the descriptor it was minted at plus its
 * law-computed class. The one slot every binding position judges against —
 * because {@link JoinOk} is an equality, judging each position against the
 * mint slot makes every cross-binding join transitively class-equal.
 */
type MintSlotOf<Classes extends SchemaClasses, V extends AnyVar> = {
	readonly field: V["field"]
	readonly class: MintClassOf<Classes, V>
}

/** A params object type — what `execute` takes and inference carries. */
type ParamsRecord = Readonly<Record<string, unknown>>

/** Flattens an intersection into one displayed object type (hover legibility). */
type Flatten<T> = { [K in keyof T]: T[K] }

/** The standard union-to-intersection fold (distributes over `U`). */
type UnionToIntersection<U> = (U extends unknown ? (member: U) => void : never) extends (member: infer I) => void
	? I
	: never

/**
 * Folds a union of per-position record fragments into one flattened record
 * (the machinery both `Params` and `Row` inference ride).
 */
type ShapeOf<U> = [U] extends [never] ? Record<never, never> : Flatten<UnionToIntersection<U>>

/** Reads a field descriptor's width label (`bytes<N>`, `interval<E, W>`); `undefined` when the kind carries none. */
type WidthOf<F extends AnyField> = F extends { readonly width: infer W } ? W : undefined

/** Reads an interval descriptor's element kind; `undefined` on scalar kinds. */
type ElementOf<F extends AnyField> = F extends { readonly element: infer E } ? E : undefined

/**
 * Reads a closed reference's handle union; `undefined` on every non-closed kind (the roster IS descriptor structure).
 */
type RosterOf<F extends AnyField> = F extends {
	readonly closed: { readonly handles: readonly (infer H extends string)[] }
}
	? H
	: undefined

/**
 * The join judgment: two bound slots join iff their descriptors' structure
 * agrees (kind, width label, interval element, and the closed ROSTER — a
 * closed reference pairs only with the same vocabulary, never with a bare
 * u64) AND their law-computed classes agree — same class name joins, and
 * bare (`undefined`) pairs only with bare (ruling 3). The class names come
 * off the SCHEMA type's class map; no descriptor label beyond the roster
 * exists to compare.
 */
type JoinOk<A extends ClassedField, B extends ClassedField> = [
	A["field"]["kind"],
	A["class"],
	WidthOf<A["field"]>,
	ElementOf<A["field"]>,
	RosterOf<A["field"]>
] extends [B["field"]["kind"], B["class"], WidthOf<B["field"]>, ElementOf<B["field"]>, RosterOf<B["field"]>]
	? [B["field"]["kind"], B["class"], WidthOf<B["field"]>, ElementOf<B["field"]>, RosterOf<B["field"]>] extends [
			A["field"]["kind"],
			A["class"],
			WidthOf<A["field"]>,
			ElementOf<A["field"]>,
			RosterOf<A["field"]>
		]
		? true
		: false
	: false

/**
 * The runtime twin of {@link JoinOk}: two bound slots join iff descriptor
 * structure and class agree — the same comparison the type tier makes,
 * judged on the honest runtime values (the descriptor, the roster by VALUE
 * IDENTITY, and the schema value's frozen class map). The rule builders
 * throw through this on a class-unequal reuse, so the wall holds for untyped
 * callers too.
 */
function fieldJoins(a: ClassedField, b: ClassedField): boolean {
	const widthA = "width" in a.field ? a.field.width : undefined
	const widthB = "width" in b.field ? b.field.width : undefined
	const elementA = "element" in a.field ? a.field.element : undefined
	const elementB = "element" in b.field ? b.field.element : undefined
	const rosterA = rosterOf(a.field)
	const rosterB = rosterOf(b.field)
	return (
		a.field.kind === b.field.kind &&
		a.class === b.class &&
		widthA === widthB &&
		elementA === elementB &&
		rosterA === rosterB
	)
}

/**
 * Renders one bound slot for join-mismatch diagnostics — the structural
 * kind in the schema grammar's spelling (a closed reference names its
 * vocabulary) plus the slot's law-computed class (`u64 in class Holder.id`;
 * a lawless slot renders `bare`).
 */
function renderFieldKind(slot: ClassedField): string {
	const field = slot.field
	let base: string = field.kind
	const roster = rosterOf(field)
	if (roster !== undefined) {
		base = `u64 referencing ${roster.name}`
	}
	if (field.kind === "bytes") {
		base = `bytes<${field.width}>`
	}
	if (field.kind === "interval") {
		base = field.width === undefined ? `interval<${field.element}>` : `interval<${field.element}, ${field.width}>`
	}
	return slot.class === undefined ? `${base} (bare)` : `${base} in class ${slot.class}`
}

/**
 * What a PARAM anchored at field `F` accepts at execution: the field's
 * bare value type, exactly — at a CLOSED-reference field that is the
 * handle-name union (`"DirectPass" | "Failed"`), translated name → row id
 * at execute through the one roster-verification point (`taggedHandleId`).
 */
type ParamValueAt<F extends AnyField> = Infer<F>

/** Reads a value's inferred-types carrier (rules, queries, recs). */
type InferredOf<T> = T extends { readonly [inferred]?: infer S } ? Exclude<S, undefined> : never

/**
 * One registered parameter of a query, as the wire marshal reads it: the
 * name, the wire shape, the field descriptor (or the measure) that anchored
 * it, and the comparison op the anchor came from (`"binding"` for atom
 * positions). `anchor` is `undefined` only on a query built but not yet
 * anchored by any rule. `members` is present exactly on a MEMBERSHIP-ARRAY
 * entry.
 */
interface ParamEntry {
	readonly name: string
	readonly shape: "value" | "set" | "mask"
	readonly anchor: AnyField | "measure" | undefined
	readonly op: "binding" | "eq" | "ne" | "lt" | "le" | "gt" | "ge" | "pointIn" | "allen"
	readonly members: readonly string[] | undefined
}

export type {
	AnyTerm,
	AnyVar,
	ClassedField,
	Duration,
	Flatten,
	InferredOf,
	JoinOk,
	MaskParam,
	MatchFields,
	MatchOwner,
	MintClassOf,
	MintSlotOf,
	Param,
	ParamEntry,
	ParamsRecord,
	ParamValueAt,
	SetParam,
	ShapeOf,
	UnionToIntersection,
	Var,
	VarsOf
}
export { fieldJoins, inferred, isTerm, makeDuration, makeMaskParam, makeParam, makeSetParam, renderFieldKind, term, v }
