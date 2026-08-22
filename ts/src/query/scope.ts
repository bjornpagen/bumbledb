import * as errors from "@superbuilders/errors"
import type { AnyClosed } from "#closed.ts"
import { sealedFieldsOf } from "#closed.ts"
import type { AnyField, Infer } from "#fields.ts"
import { rosterOf } from "#fields.ts"
import type { ClassLookup, ClassRecordOf, SchemaClasses } from "#law.ts"
import type { QueryParam } from "#native.ts"
import type { AnyRelation, RelationFields } from "#relation.ts"

const term: unique symbol = Symbol("bumbledb.query.term")

const inferred: unique symbol = Symbol("bumbledb.query.inferred")

type MatchOwner = AnyRelation | AnyClosed

type MatchFields<R extends MatchOwner> = R extends AnyClosed
	? { readonly id: R["id"] } & R["columns"]
	: R extends AnyRelation
		? RelationFields<R>
		: never

interface Var<F extends AnyField = AnyField, RN extends string = string, K extends string = string> {
	readonly [term]: "var"
	readonly owner: MatchOwner & { readonly name: RN }
	readonly column: K
	readonly field: F
	readonly label: string
}

type AnyVar = Var

interface Param<Name extends string = string> {
	readonly [term]: "param"
	readonly name: Name
}

interface SetParam<Name extends string = string> {
	readonly [term]: "setParam"
	readonly name: Name
}

type AnyTerm = Var | Param | SetParam

function isTerm(value: unknown): value is AnyTerm {
	return typeof value === "object" && value !== null && term in value
}

type VarsOf<R extends MatchOwner> = {
	readonly [K in keyof MatchFields<R> & string]: Var<MatchFields<R>[K], R["name"], K>
}

/**
 * The exactness judgment of the six full-binding `match` forms (`#query/
 * lower.ts` intersects it into the `bindings` parameter): a mapped type
 * over the INFERRED record `B` requiring every entry to be a variable
 * whose mint COLUMN is the entry's own key restricted to `R`'s matchable
 * fields — for a column of `R` that is just the {@link VarsOf} entry's own
 * shape, and for a FOREIGN key the column type is `never`, which no
 * mintable variable inhabits. So an aliased or function-returned record
 * carrying an extra key (`{...v(Account), extra: otherVar }` — shapes
 * excess-property checking never sees, it covers inline literals only)
 * fails the intersection and falls to the general form's judgment: the
 * pre-0.16.0 compile refusal, restored. The identity record `v(rel)` still
 * unifies for GENERIC `R` — the judgment is intersections and index
 * constraints only, no deferred conditional anywhere, so the full-binding
 * law (50-generic-binding.md, "The ruling") stands untouched.
 */
type ExactVars<R extends MatchOwner, B> = {
	readonly [K in keyof B]: Var<AnyField, R["name"], K & keyof MatchFields<R> & string>
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
 * column names must work, the `closed` precedent). Every `v` call mints
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

function makeParam<const Name extends string>(name: Name): Param<Name> {
	const value: Param<Name> = { [term]: "param", name }
	return Object.freeze(value)
}

function makeSetParam<const Name extends string>(name: Name): SetParam<Name> {
	const value: SetParam<Name> = { [term]: "setParam", name }
	return Object.freeze(value)
}

interface ClassedField {
	readonly field: AnyField
	readonly class: string | undefined
}

type MintClassOf<Classes extends SchemaClasses, V> =
	V extends Var<AnyField, infer RN extends string, infer K extends string>
		? ClassLookup<ClassRecordOf<Classes, RN>, K>
		: never

type MintSlotOf<Classes extends SchemaClasses, V extends AnyVar> = {
	readonly field: V["field"]
	readonly class: MintClassOf<Classes, V>
}

type ParamsRecord = Readonly<Record<string, unknown>>

type Flatten<T> = { [K in keyof T]: T[K] }

type UnionToIntersection<U> = (U extends unknown ? (member: U) => void : never) extends (member: infer I) => void
	? I
	: never

type ShapeOf<U> = [U] extends [never] ? Record<never, never> : Flatten<UnionToIntersection<U>>

type WidthOf<F extends AnyField> = F extends { readonly width: infer W } ? W : undefined

type ElementOf<F extends AnyField> = F extends { readonly element: infer E } ? E : undefined

type RosterOf<F extends AnyField> = F extends {
	readonly closed: { readonly name: infer N extends string; readonly handles: readonly (infer H extends string)[] }
}
	? readonly [N, H]
	: undefined

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

type ParamValueAt<F extends AnyField> = Infer<F>

type InferredOf<T> = T extends { readonly [inferred]?: infer S } ? Exclude<S, undefined> : never

interface ParamEntry {
	readonly name: string
	readonly shape: "value" | "set"
	readonly anchor: AnyField | undefined
	readonly op: "binding" | "eq" | "ne" | "lt" | "le" | "gt" | "ge" | "pointIn" | "allen"
	readonly membership: QueryParam | undefined
}

export type {
	AnyTerm,
	AnyVar,
	ClassedField,
	ExactVars,
	Flatten,
	InferredOf,
	JoinOk,
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
export { fieldJoins, inferred, isTerm, makeParam, makeSetParam, renderFieldKind, term, v }
