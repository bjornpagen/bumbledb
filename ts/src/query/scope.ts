/**
 * Query scope terms, STRUCTURAL edition: string-named variables and
 * parameters as plain frozen values. A `Var` is a NAME — it is typed by the
 * field it first binds (structurally, off the schema type, through the rule
 * builder's environment), reuse of the name within one rule IS the join,
 * and a domain-mismatched reuse is a compile error (the structural analog
 * of the old brand-equal join — now domain-equal, no value brands
 * anywhere). Params are query-global by name and typed BY USE: the field
 * position or comparison sibling that anchors a param types it, the
 * query's inferred `Params` object is exactly the params the rules use,
 * and a param value that no rule uses simply never registers — the query
 * executes under its own inferred type (the bug-hunt law). This module
 * also owns the environment/typing utilities the whole surface shares:
 * the env shape (var name → field descriptor), the domain-equality
 * judgment, and the record-folding helpers `Params` and `Row` inference
 * ride.
 */

import * as errors from "@superbuilders/errors"
import type { AnyField, Infer } from "#fields.ts"

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

/** The record `makeVars` mints: one own frozen `Var<Name>` per requested name, each typed exactly. */
type VarsRecord<Names extends string> = { readonly [N in Names]: Var<N> }

/**
 * The trusted seam of the vars mint: every requested name reads back as an
 * own var term of exactly that name — verified before the record is
 * admitted at the {@link VarsRecord} type (a name riding the
 * object-protocol accessor instead of an own definition would fail exactly
 * this check).
 */
function varsMinted<Names extends string>(
	record: Readonly<Record<string, unknown>>,
	names: readonly Names[]
): record is Readonly<Record<string, unknown>> & VarsRecord<Names> {
	return names.every(function varMinted(name) {
		if (!Object.hasOwn(record, name)) {
			return false
		}
		const value = record[name]
		return isTerm(value) && value[term] === "var" && value.name === name
	})
}

/**
 * Mints several variables at once — `const { service, w } = r.vars("service",
 * "w")`: tuple-to-object, each name typed exactly (`Var<"service">`),
 * inference identical to the one-at-a-time `r.var` spelling (one lowering,
 * two entry flavors). Each key is defined as an OWN property (a name like
 * `"__proto__"` is a record key like any other, never a prototype write),
 * and a duplicate name in one call is a construction error: each name mints
 * one variable — write it once and reuse the binding.
 */
function makeVars<const Names extends readonly string[]>(...names: Names): VarsRecord<Names[number]> {
	const out: Record<string, unknown> = {}
	const seen = new Set<string>()
	for (const name of names) {
		if (seen.has(name)) {
			throw errors.new(
				`vars: duplicate name ${name} — each name mints one variable; write it once and reuse the binding`
			)
		}
		seen.add(name)
		Object.defineProperty(out, name, { value: makeVar(name), enumerable: true })
	}
	Object.freeze(out)
	if (!varsMinted<Names[number]>(out, names)) {
		throw errors.new("vars: variable minting incomplete")
	}
	return out
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
 * A rule's typing environment: variable name → the field descriptor it
 * first bound. Purely a TYPE — the runtime twin is the rule's `varFields`
 * record, and the two are built by the same walk.
 */
type EnvShape = Record<string, AnyField>

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

/** Reads a field descriptor's domain label (S1: the label IS the domain check). */
type DomainOf<F extends AnyField> = F["domain"]

/** Reads a field descriptor's width label (`bytes<N>`, `interval<E, W>`); `undefined` when the kind carries none. */
type WidthOf<F extends AnyField> = F extends { readonly width: infer W } ? W : undefined

/** Reads an interval descriptor's element kind; `undefined` on scalar kinds. */
type ElementOf<F extends AnyField> = F extends { readonly element: infer E } ? E : undefined

/**
 * The structural join judgment: two field descriptors join iff kind,
 * domain label, width label, and interval element all agree — the
 * string-literal comparison of descriptor shapes that replaced the value
 * brand (design ruling 3).
 */
type JoinOk<A extends AnyField, B extends AnyField> = [A["kind"], DomainOf<A>, WidthOf<A>, ElementOf<A>] extends [
	B["kind"],
	DomainOf<B>,
	WidthOf<B>,
	ElementOf<B>
]
	? [B["kind"], DomainOf<B>, WidthOf<B>, ElementOf<B>] extends [A["kind"], DomainOf<A>, WidthOf<A>, ElementOf<A>]
		? true
		: false
	: false

/**
 * The runtime twin of {@link JoinOk}: two field descriptors join iff kind,
 * domain label, width label, and interval element all agree — the same
 * structural comparison the type tier makes, judged on the descriptor
 * VALUES (S1 descriptors are honest at runtime). The rule builders throw
 * through this on a domain-unequal variable reuse, so the wall holds for
 * untyped callers too, not only where the compiler can see.
 */
function fieldJoins(a: AnyField, b: AnyField): boolean {
	const widthA = "width" in a ? a.width : undefined
	const widthB = "width" in b ? b.width : undefined
	const elementA = "element" in a ? a.element : undefined
	const elementB = "element" in b ? b.element : undefined
	return a.kind === b.kind && a.domain === b.domain && widthA === widthB && elementA === elementB
}

/**
 * Renders one field descriptor for join-mismatch diagnostics — the schema
 * grammar's own spelling (`u64 as HolderId`, `interval<i64, 7> as Window`).
 */
function renderFieldKind(field: AnyField): string {
	let base: string = field.kind
	if (field.kind === "bytes") {
		base = `bytes<${field.width}>`
	}
	if (field.kind === "interval") {
		base = field.width === undefined ? `interval<${field.element}>` : `interval<${field.element}, ${field.width}>`
	}
	return field.domain === undefined ? base : `${base} as ${field.domain}`
}

/**
 * What a PARAM anchored at field `F` accepts at execution: the field's
 * bare value type, exactly. At an interval field the engine resolves the
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
 * that state typed.
 */
interface ParamEntry {
	readonly name: string
	readonly shape: "value" | "set" | "mask"
	readonly anchor: AnyField | "measure" | undefined
	readonly op: "binding" | "eq" | "ne" | "lt" | "le" | "gt" | "ge" | "pointIn" | "allen"
}

export type {
	AnyTerm,
	DomainOf,
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
	Var,
	VarsRecord
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
	makeVars,
	renderFieldKind,
	term
}
