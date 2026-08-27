import * as errors from "@superbuilders/errors"
import type { AnyClosed } from "#closed.ts"
import { sealedFieldsOf } from "#closed.ts"
import type { AnyField, Infer, SignatureOf } from "#fields.ts"
import { rosterOf, signaturesAgree } from "#fields.ts"
import type { Same, SameLen } from "#judgment.ts"
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

interface Var<F extends AnyField, RN extends string, K extends string> {
	readonly [term]: "var"
	readonly owner: MatchOwner & { readonly name: RN }
	readonly column: K
	readonly field: F
	readonly label: string
}

type AnyVar = Var<AnyField, string, string>

interface Param<Name extends string> {
	readonly [term]: "param"
	readonly name: Name
}

interface SetParam<Name extends string> {
	readonly [term]: "setParam"
	readonly name: Name
}

type AnyTerm = AnyVar | Param<string> | SetParam<string>

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

/**
 * A slot's identity for the positive-join judgment: the field's structural
 * signature ({@link SignatureOf} — the ONE interpreter, shared with the
 * face pairing wall) plus the slot's law class.
 */
type SlotSignature<S extends ClassedField> = readonly [SignatureOf<S["field"]>, S["class"]]

type JoinOk<A extends ClassedField, B extends ClassedField> = Same<SlotSignature<A>, SlotSignature<B>>

type U64Wire<F extends AnyField> = F extends { readonly kind: "u64" }
	? F extends { readonly closed: unknown }
		? false
		: true
	: false

type ClosedHandles<F extends AnyField> = F extends {
	readonly closed: { readonly handles: infer H extends readonly string[] }
}
	? H
	: never

/**
 * Two closed ids anti-join when their handle vectors carry the same Peano
 * length ({@link SameLen}: zero equals zero, successor recurses on
 * successor). A bare field has no vector and proves nothing.
 */
type ClosedIdOk<A extends AnyField, B extends AnyField> = [ClosedHandles<A>] extends [never]
	? false
	: [ClosedHandles<B>] extends [never]
		? false
		: SameLen<ClosedHandles<A>, ClosedHandles<B>>

/**
 * Anti-join class safety: class-equal slots, plus same-identity u64
 * (fresh mint, foreign-key copy, or bare u64) when one side is bare,
 * plus two closed-id fields whose handle tuples have the same length.
 * Two distinct generators stay refused.
 */
type AntiJoinOk<A extends ClassedField, B extends ClassedField> =
	JoinOk<A, B> extends true
		? true
		: ClosedIdOk<A["field"], B["field"]> extends true
			? true
			: U64Wire<A["field"]> extends true
				? U64Wire<B["field"]> extends true
					? [A["class"]] extends [undefined]
						? true
						: [B["class"]] extends [undefined]
							? true
							: false
					: false
				: false

function fieldJoins(a: ClassedField, b: ClassedField): boolean {
	return a.class === b.class && signaturesAgree(a.field, b.field)
}

function u64Wire(field: AnyField): boolean {
	return field.kind === "u64" && rosterOf(field) === undefined
}

function closedIdAntiJoins(a: AnyField, b: AnyField): boolean {
	const rosterA = rosterOf(a)
	const rosterB = rosterOf(b)
	return rosterA !== undefined && rosterB !== undefined && rosterA.handles.length === rosterB.handles.length
}

function u64BareWirePair(a: ClassedField, b: ClassedField): boolean {
	return u64Wire(a.field) && u64Wire(b.field) && (a.class === undefined || b.class === undefined)
}

function fieldAntiJoins(a: ClassedField, b: ClassedField): boolean {
	if (fieldJoins(a, b) || closedIdAntiJoins(a.field, b.field)) {
		return true
	}
	return u64BareWirePair(a, b)
}

/**
 * Union-head class safety, the anti-join law's head twin: class-equal
 * slots, plus same-identity u64 wire when one side is bare. A head cell
 * is a value that flows to the caller either way — the class is
 * provenance, not wire shape — and the merged head carries the meet:
 * a bare rule demotes the column's class claim to bare, so downstream
 * joins cannot inherit provenance a rule never proved.
 */
function headFieldJoins(a: ClassedField, b: ClassedField): boolean {
	return fieldJoins(a, b) || u64BareWirePair(a, b)
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
	AntiJoinOk,
	AnyVar,
	ClassedField,
	ExactVars,
	Flatten,
	InferredOf,
	JoinOk,
	MatchFields,
	MatchOwner,
	MintSlotOf,
	Param,
	ParamEntry,
	ParamsRecord,
	ParamValueAt,
	SetParam,
	ShapeOf,
	Var,
	VarsOf
}
export { fieldAntiJoins, fieldJoins, headFieldJoins, inferred, isTerm, makeParam, makeSetParam, renderFieldKind, term, v }
