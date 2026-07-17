/**
 * Query scope terms (PRD-08): typed variables and parameters, declared
 * inside a `query()` scope callback so identity is lexical. A `Var` is
 * typed by the field it is declared from and joins are brand-equal by
 * construction (the nominal join discipline); var identity is OBJECT
 * identity — two `$.var` calls are two variables even from the same field,
 * and no name-collision machinery exists. Params are query-global, carry a
 * mandatory literal name (the key of `execute`'s params object), and lower
 * to dense positional `ParamId`s in declaration order. This module also
 * owns the type-level params extraction: every atom and condition value
 * carries its contributed params object in a phantom, and the query's
 * `Params` type is the flattened intersection over the returned rules —
 * inference rides the return value, never a mutable type accumulator.
 */

import * as errors from "@superbuilders/errors"
import { phantom } from "#brand.ts"
import type { AnyClosed } from "#closed.ts"
import type { FieldData } from "#fields.ts"
import type { PredicateData } from "#query/predicate.ts"
import type { AnySchema, SchemaRelation } from "#schema.ts"

/**
 * The runtime discriminant of query term values. Host literals (bigints,
 * strings, interval objects, `oneOf` sets) never carry it, so "is this
 * binding a term or a literal" is one property probe, never a guess.
 */
const term: unique symbol = Symbol("bumbledb.query.term")

/**
 * A query variable, typed by the field it was declared from
 * (`$.var(Holder.fields.id)` → `Var<Brand<bigint, "HolderId">>`). Usable in
 * any atom position whose field carries the same brand; a brand-mismatched
 * placement is a TYPE error. Identity is object identity.
 */
interface Var<V> {
	readonly [term]: "var"
	readonly relation: string
	readonly field: string
	readonly data: FieldData
	readonly [phantom]?: V
}

/**
 * A scalar query parameter — `$.param("root", Holder.fields.id)`. The name
 * literal is the key of the typed params object `execute` takes; the value
 * is marshaled by the declaring field's structural type at bind.
 */
interface Param<Name extends string, V> {
	readonly [term]: "param"
	readonly name: Name
	readonly relation: string
	readonly field: string
	readonly data: FieldData
	readonly [phantom]?: V
}

/**
 * A set-valued query parameter (the IR's `ParamSet` term): bound at
 * execution to an ARRAY of values of the declaring field's type; a binding
 * position matches iff the field value is in the set. Legal in atom
 * bindings (positive and negated) and as the right side of `is` — nowhere
 * else, exactly as the IR rules it.
 */
interface ParamSet<Name extends string, V> {
	readonly [term]: "paramSet"
	readonly name: Name
	readonly relation: string
	readonly field: string
	readonly data: FieldData
	readonly [phantom]?: V
}

/**
 * An Allen-mask parameter (the IR's `MaskTerm::Param`): the temporal
 * relation as a bind-time argument — one prepared query answers any of the
 * mask questions per execution. Bound to a 13-bit mask number built from
 * the `ALLEN` constants.
 */
interface MaskParam<Name extends string> {
	readonly [term]: "maskParam"
	readonly name: Name
}

/** Any query variable, whatever its brand. */
type AnyVar = Var<unknown>

/** Any parameter term of the scope, whatever its name and brand. */
type AnyParamTerm = Param<string, unknown> | ParamSet<string, unknown> | MaskParam<string>

/** Any scope term value. */
type AnyTerm = AnyVar | AnyParamTerm

/** Narrows an unknown binding value to a scope term (vs a host literal). */
function isTerm(value: unknown): value is AnyTerm {
	return typeof value === "object" && value !== null && term in value
}

/** A params object type — what `execute` takes and the phantoms carry. */
type ParamsRecord = Readonly<Record<string, unknown>>

/** Flattens an intersection into one displayed object type (hover legibility). */
type Flatten<T> = { [K in keyof T]: T[K] }

/** The standard union-to-intersection fold (distributes over `U`). */
type UnionToIntersection<U> = (U extends unknown ? (member: U) => void : never) extends (member: infer I) => void
	? I
	: never

/**
 * One term's contribution to the query's params object type: a `Param`
 * contributes its value type under its name, a `ParamSet` the readonly
 * array of it, a `MaskParam` a mask number; everything else contributes
 * nothing.
 */
type TermContribution<T> = T extends {
	readonly [term]: "param"
	readonly name: infer N extends string
	readonly [phantom]?: infer V
}
	? { readonly [K in N]: Exclude<V, undefined> }
	: T extends {
				readonly [term]: "paramSet"
				readonly name: infer N extends string
				readonly [phantom]?: infer V
			}
		? { readonly [K in N]: readonly Exclude<V, undefined>[] }
		: T extends { readonly [term]: "maskParam"; readonly name: infer N extends string }
			? { readonly [K in N]: number }
			: Record<never, never>

/**
 * Folds a union of per-term/per-item params objects into the one flattened
 * params record (the query's `Params` type).
 */
type ParamsShape<U> = [U] extends [never] ? Record<never, never> : Flatten<UnionToIntersection<U>>

/** Reads an atom's/condition's contributed params object off its phantom. */
type ItemParams<T> = T extends { readonly [phantom]?: infer P } ? Exclude<P, undefined> : Record<never, never>

/** One registered parameter: name, wire shape, and the declaring field. */
interface ParamEntry {
	readonly name: string
	readonly shape: "value" | "set" | "mask"
	readonly data: FieldData | undefined
}

/**
 * The mutable build-time registry one `query()` scope owns: declared vars
 * (membership polices cross-scope smuggling), params in declaration order
 * (= dense `ParamId`s), and declared predicates in declaration order
 * (= dense `PredId`s; the output predicate is appended by lowering).
 */
interface QueryRegistry {
	readonly theory: AnySchema
	readonly vars: Set<AnyVar>
	readonly params: ParamEntry[]
	readonly paramIndex: Map<AnyTerm, number>
	readonly predicates: PredicateData[]
}

/** Creates one empty scope registry over the query's theory. */
function createRegistry(theory: AnySchema): QueryRegistry {
	return {
		theory,
		vars: new Set(),
		params: [],
		paramIndex: new Map(),
		predicates: []
	}
}

/**
 * The relation-kind discriminant: a closed relation's runtime description
 * carries its handle roster, an ordinary relation's never does.
 */
function isClosedMember(member: SchemaRelation): member is AnyClosed {
	return "handles" in member.data
}

/**
 * Resolves a field reference's runtime description through the schema —
 * the seam that types `$.var`/`$.param` declarations at runtime (the type
 * level already carries the brand; this recovers the structural type the
 * lowering and the param marshaler direct by).
 */
function resolveFieldData(theory: AnySchema, relationName: string, fieldName: string): FieldData {
	const member: SchemaRelation | undefined = theory.relations[relationName]
	if (member === undefined) {
		throw errors.new(`schema ${theory.name} has no relation ${relationName}`)
	}
	if (isClosedMember(member)) {
		if (fieldName === "id") {
			return member.id.data
		}
		const column = member.data.columns.find(function byName(candidate) {
			return candidate.name === fieldName
		})
		if (column === undefined) {
			throw errors.new(`closed relation ${relationName} has no column ${fieldName}`)
		}
		return column.field
	}
	const declared = member.data.fields.find(function byName(candidate) {
		return candidate.name === fieldName
	})
	if (declared === undefined) {
		throw errors.new(`relation ${relationName} has no field ${fieldName}`)
	}
	return declared.field
}

/** A field reference's runtime half, as the scope factories consume it. */
interface RefNames {
	readonly relation: string
	readonly field: string
}

/** Declares one variable in the scope (the `$.var` implementation). */
function scopeVar<V>(registry: QueryRegistry, ref: RefNames): Var<V> {
	const value: Var<V> = Object.freeze({
		[term]: "var" as const,
		relation: ref.relation,
		field: ref.field,
		data: resolveFieldData(registry.theory, ref.relation, ref.field)
	})
	registry.vars.add(value)
	return value
}

/** Rejects a second parameter under an already-taken name. */
function assertFreshParamName(registry: QueryRegistry, name: string): void {
	const taken = registry.params.some(function byName(entry) {
		return entry.name === name
	})
	if (taken) {
		throw errors.new(
			`query scope already declares a param named ${name} — param names key the execute params object, one declaration each`
		)
	}
}

/** Declares one scalar parameter (the `$.param` implementation). */
function scopeParam<Name extends string, V>(registry: QueryRegistry, name: Name, ref: RefNames): Param<Name, V> {
	assertFreshParamName(registry, name)
	const data = resolveFieldData(registry.theory, ref.relation, ref.field)
	const value: Param<Name, V> = Object.freeze({
		[term]: "param" as const,
		name,
		relation: ref.relation,
		field: ref.field,
		data
	})
	registry.paramIndex.set(value, registry.params.length)
	registry.params.push(Object.freeze({ name, shape: "value" as const, data }))
	return value
}

/** Declares one set parameter (the `$.paramSet` implementation). */
function scopeParamSet<Name extends string, V>(registry: QueryRegistry, name: Name, ref: RefNames): ParamSet<Name, V> {
	assertFreshParamName(registry, name)
	const data = resolveFieldData(registry.theory, ref.relation, ref.field)
	const value: ParamSet<Name, V> = Object.freeze({
		[term]: "paramSet" as const,
		name,
		relation: ref.relation,
		field: ref.field,
		data
	})
	registry.paramIndex.set(value, registry.params.length)
	registry.params.push(Object.freeze({ name, shape: "set" as const, data }))
	return value
}

/** Declares one Allen-mask parameter (the `$.allenParam` implementation). */
function scopeAllenParam<Name extends string>(registry: QueryRegistry, name: Name): MaskParam<Name> {
	assertFreshParamName(registry, name)
	const value: MaskParam<Name> = Object.freeze({ [term]: "maskParam" as const, name })
	registry.paramIndex.set(value, registry.params.length)
	registry.params.push(Object.freeze({ name, shape: "mask" as const, data: undefined }))
	return value
}

export type {
	AnyParamTerm,
	AnyTerm,
	AnyVar,
	Flatten,
	ItemParams,
	MaskParam,
	Param,
	ParamEntry,
	ParamSet,
	ParamsRecord,
	ParamsShape,
	QueryRegistry,
	TermContribution,
	UnionToIntersection,
	Var
}
export { createRegistry, isTerm, resolveFieldData, scopeAllenParam, scopeParam, scopeParamSet, scopeVar, term }
