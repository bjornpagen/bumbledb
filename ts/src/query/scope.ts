/**
 * Query scope terms, LAW-TYPED edition: string-named variables and
 * parameters as plain frozen values. A `Var` is a NAME — it is typed by the
 * field slot it first binds (the descriptor AND the slot's law-computed
 * CLASS, read off the schema's class map through the rule builder's
 * environment), reuse of the name within one rule IS the join, and a
 * class-mismatched reuse is a compile error: a var joins only class-equal
 * slots, and BARE PAIRS ONLY WITH BARE (ruling 3 — a slot in no law has no
 * class and never joins a classed slot; the deliberate sum-domain pointer
 * stays legal against other bare slots). Params are query-global by name
 * and typed BY USE: the field position or comparison sibling that anchors
 * a param types it, the query's inferred `Params` object is exactly the
 * params the rules use, and a param value that no rule uses simply never
 * registers — the query executes under its own inferred type (the
 * bug-hunt law). This module also owns the environment/typing utilities
 * the whole surface shares: the env shape (var name → classed slot), the
 * class-equality judgment {@link JoinOk} with its runtime twin
 * {@link fieldJoins}, and the record-folding helpers `Params` and `Row`
 * inference ride.
 */

import type { AnyField, Infer } from "#fields.ts"
import { rosterOf } from "#fields.ts"

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
 * A query variable — a NAME. Its type comes from the field it first binds
 * in the rule (the builder's environment); reusing the name joins, and a
 * cross-domain reuse is a compile error. Identity is the name, strictly
 * rule-scoped: the same name in two rules names two unrelated variables
 * (exactly as the IR scopes `VarId`).
 */
interface Var<Name extends string = string> {
	readonly [term]: "var"
	readonly name: Name
}

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
 * select entry, and as the input of `sum`/`min`/`max`; every other position
 * is unwritable, exactly as the IR rejects it typed. A ray has no finite
 * measure — the engine's `MeasureOfRay` execution error; exclude rays first
 * (`allen` against a bounded window).
 */
interface Duration<Name extends string = string> {
	readonly [term]: "duration"
	readonly name: Name
}

/** Any scope term value. */
type AnyTerm = Var | Param | SetParam | MaskParam | Duration

/** Narrows an unknown position value to a scope term (vs a host literal). */
function isTerm(value: unknown): value is AnyTerm {
	return typeof value === "object" && value !== null && term in value
}

/** Builds one variable term. */
function makeVar<const Name extends string>(name: Name): Var<Name> {
	const value: Var<Name> = { [term]: "var", name }
	return Object.freeze(value)
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

/** Builds one measure term over an interval-typed variable's name. */
function makeDuration<const Name extends string>(name: Name): Duration<Name> {
	const value: Duration<Name> = { [term]: "duration", name }
	return Object.freeze(value)
}

/**
 * One bound field slot: the field's descriptor plus the slot's
 * law-computed CLASS (`undefined` = bare — the slot is in no law). The one
 * shape the rule environment carries per variable, at the TYPE level (env
 * entries hold the schema type's class-map lookups) and at RUNTIME alike
 * (the rule's `varFields` record holds exactly this shape, read off the
 * schema value's frozen class map) — one shape, two tiers, one walk.
 */
interface ClassedField {
	readonly field: AnyField
	readonly class: string | undefined
}

/**
 * A rule's typing environment: variable name → the classed slot it first
 * bound. Purely a TYPE — the runtime twin is the rule's `varFields`
 * record, and the two are built by the same walk.
 */
type EnvShape = Record<string, ClassedField>

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

/** Reads a closed reference's handle union; `undefined` on every non-closed kind (the roster IS descriptor structure). */
type RosterOf<F extends AnyField> = F extends {
	readonly closed: { readonly handles: readonly (infer H extends string)[] }
}
	? H
	: undefined

/**
 * The join judgment: two bound slots join iff their descriptors' structure
 * agrees (kind, width label, interval element, and the closed ROSTER — a
 * closed reference pairs only with the same vocabulary, never with a bare
 * u64: the roster keys every closed judgment downstream, so a join across
 * it would decode/order/translate incoherently by binding order) AND their
 * law-computed classes agree — same class name joins, and bare
 * (`undefined`) pairs only with bare (ruling 3: a field in no law has no
 * class; a bare↔classed pairing refuses). The class names come off the
 * SCHEMA type's class map — the statements are the typing; no descriptor
 * label beyond the roster exists to compare.
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
 * IDENTITY — vocabulary identity is value identity — and the schema
 * value's frozen class map). The rule builders throw through this on a
 * class-unequal variable reuse, so the wall holds for untyped callers too,
 * not only where the compiler can see.
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
 * vocabulary: the roster is part of the structure being compared) plus the
 * slot's law-computed class (`u64 in class Holder.id`; a lawless slot
 * renders `bare`).
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
 * at execute through the one roster-verification point
 * (`taggedHandleId`). At an interval field the engine resolves the
 * bivalent anchor to the INTERVAL reading (value equality) — the point
 * reading of a param is spelled `pointIn(r.param(...), w)`, whose sibling
 * anchors it element-typed.
 */
type ParamValueAt<F extends AnyField> = Infer<F>

/** Reads a value's inferred-types carrier (rules, queries, recs). */
type InferredOf<T> = T extends { readonly [inferred]?: infer S } ? Exclude<S, undefined> : never

/**
 * One registered parameter of a query, as the wire marshal reads it: the
 * name, the wire shape, the field descriptor (or the measure) that anchored
 * it, and the comparison op the anchor came from (`"binding"` for atom
 * positions) — the op keeps literal tagging op-aware at `pointIn`
 * (the bug-hunt fix, preserved). `anchor` is `undefined` only on a query
 * built but not yet anchored by any rule; lowering and the wire both refuse
 * that state typed. `members` is present exactly on a MEMBERSHIP-ARRAY
 * entry (a literal set at a closed field, folded into the program): the
 * SDK itself translates and supplies the set at every execute — the entry
 * is never read from, and never demanded of, the host's params object.
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
	ClassedField,
	Duration,
	EnvShape,
	Flatten,
	InferredOf,
	JoinOk,
	MaskParam,
	Param,
	ParamEntry,
	ParamsRecord,
	ParamValueAt,
	SetParam,
	ShapeOf,
	UnionToIntersection,
	Var
}
export {
	fieldJoins,
	inferred,
	isTerm,
	makeDuration,
	makeMaskParam,
	makeParam,
	makeSetParam,
	makeVar,
	renderFieldKind,
	term
}
