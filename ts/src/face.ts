/**
 * Faces — the projection-with-selection value both containments and
 * windows consume: `on(Account, "holder")` the common single-field
 * position, `on(Booking, ["room", "during"])` the composite/pointwise
 * position (one spelling, arity-generic), `on(Account.where({ kind:
 * Kind.Savings }), "id")` the σ-carrying source, `on(Kind, "id")` a closed
 * relation's sealed shape opened through its synthetic `id`. Projection is
 * positional: tuple order is preserved in the type, and the statement
 * constructors pair the two sides' tuples by arity ({@link SameArity}) AND
 * by domain ({@link SameDomains}) — the domain wall of the structural
 * design: every projected field's domain LABEL is read off the schema type
 * (`F["domain"]`, the S1 kernel) and compared positionwise by
 * string-literal equality, never by any value brand.
 */

import type { AnyClosed } from "#closed.ts"
import type { AnyRelation, AnySelected, FieldsShape, RelationFields, SelectionBinding } from "#relation.ts"
import { renderLiteralSet } from "#spec.ts"

/** The empty σ of a selection-free face, shared by every bare projection. */
const emptySelection: readonly SelectionBinding[] = Object.freeze([])

/**
 * Splits a face source into its owner and σ: a selected relation carries
 * its own bindings (`relation` is the discriminant — the property exists on
 * no relation or closed value, and `closed()` reserves the name against
 * handle collisions); a bare relation or closed relation carries none.
 */
function faceParts(source: FaceSource): {
	readonly owner: FaceOwner
	readonly selection: readonly SelectionBinding[]
} {
	if ("relation" in source) {
		return { owner: source.relation, selection: source.selection }
	}
	return { owner: source, selection: emptySelection }
}

/**
 * A disjunctive literal set for a selection binding — `field == {A, B}`.
 * The signature of {@link oneOf} demands two leading literals, so the
 * one-element set (banned: it is the bare literal) and the empty set
 * (banned: it selects nothing) are unwritable.
 */
interface OneOf<V> {
	readonly literals: readonly [V, V, ...V[]]
}

/**
 * Constructs a literal set (read disjunctively) for a `where()` binding.
 * Two leading arguments by signature: the degenerate sets have no spelling
 * (the canonical-utterance law, `docs/architecture/70-api.md`).
 */
function oneOf<V>(first: V, second: V, ...rest: V[]): OneOf<V> {
	const literals: readonly [V, V, ...V[]] = [first, second, ...rest]
	Object.freeze(literals)
	return Object.freeze({ literals })
}

/** The relation a face projects from — ordinary or closed. */
type FaceOwner = AnyRelation | AnyClosed

/** A face's runtime description: owner, π (written order), σ (resolved bindings). */
interface FaceData {
	readonly owner: FaceOwner
	readonly projection: readonly string[]
	readonly selection: readonly SelectionBinding[]
}

/**
 * A face value. `S` is the source exactly as written (the relation,
 * selected relation, or closed relation `on()` was handed — the statement
 * constructors resolve each projected field's DOMAIN through it), and `P`
 * is the projection tuple as written (its length is the positional-pairing
 * arity). Both are honest runtime properties, not phantoms.
 */
interface Face<S extends FaceSource, P extends readonly string[]> {
	readonly source: S
	readonly projection: P
	readonly data: FaceData
}

/** Any face value, whatever its source and projection. */
type AnyFace = Face<FaceSource, readonly string[]>

/** What `on()` accepts: a relation, a selected relation, or a closed relation. */
type FaceSource = AnyRelation | AnyClosed | AnySelected

/**
 * The field names a face over `S` may project: a relation's declared
 * fields; a selected relation's underlying fields; a closed relation's
 * SEALED shape — the synthetic `id` plus its declared payload columns
 * (`docs/architecture/70-api.md`: statement field names address the sealed
 * shape).
 */
type FaceFields<S extends FaceSource> = S extends AnySelected
	? keyof S["relation"]["fields"] & string
	: S extends AnyRelation
		? keyof S["fields"] & string
		: S extends { readonly axioms: Readonly<Record<string, infer Row>> }
			? "id" | (keyof Row & string)
			: never

/** One field's domain label within a declared field block (`undefined` when the name is foreign). */
type DomainIn<Fields extends FieldsShape, K extends string> = K extends keyof Fields ? Fields[K]["domain"] : undefined

/**
 * The domain LABEL of one projected field, read structurally off the
 * source's schema type — an ordinary or selected relation's field carries
 * its S1 descriptor's `domain`; a closed relation's synthetic `id` carries
 * the handle domain (`"KindId"`). A closed relation's payload columns type
 * as `undefined`: the `Closed` TYPE deliberately erases its payload
 * descriptors (axioms carry bare values), so no label survives to compare —
 * the lowering still carries the runtime label to the engine, which stays
 * the final authority on any pairing the type layer cannot see.
 */
type ProjectedDomain<S extends FaceSource, K extends string> = S extends AnySelected
	? DomainIn<RelationFields<S["relation"]>, K>
	: S extends AnyRelation
		? DomainIn<RelationFields<S>, K>
		: S extends { readonly id: { readonly domain: infer D extends string } }
			? K extends "id"
				? D
				: undefined
			: undefined

/** The positionwise domain-label tuple of a projection over `S`. */
type DomainsOf<S extends FaceSource, P extends readonly string[]> = {
	readonly [I in keyof P]: ProjectedDomain<S, P[I] & string>
}

/** The domain-label tuple a face projects, positionwise — the comparand of {@link SameDomains}. */
type FaceDomains<F extends AnyFace> = F extends Face<infer S, infer P> ? DomainsOf<S, P> : never

/** The projection arity of a face. */
type Arity<F extends AnyFace> = F["projection"]["length"]

/**
 * The legible arity-mismatch verdict: when the two faces of a containment,
 * bijection, or window project different numbers of fields, this type is
 * intersected into the second face's parameter and names both arities.
 */
interface FaceArityMismatch<Left, Right> {
	readonly "face arity mismatch — positional pairing requires both sides to project equally many fields": readonly [
		Left,
		Right
	]
}

/**
 * Resolves to `unknown` (a no-op intersection) when the two faces project
 * equally many fields, and to {@link FaceArityMismatch} otherwise — the
 * named helper the statement constructors constrain with.
 */
type SameArity<A extends AnyFace, B extends AnyFace> =
	Arity<A> extends Arity<B>
		? Arity<B> extends Arity<A>
			? unknown
			: FaceArityMismatch<Arity<A>, Arity<B>>
		: FaceArityMismatch<Arity<A>, Arity<B>>

/**
 * The legible domain-mismatch verdict: when the two faces of a containment,
 * bijection, or window project different domain labels at any position,
 * this type is intersected into the second face's parameter and names both
 * label tuples — a cross-domain pair is a COMPILE error, achieved by
 * string-literal comparison of descriptor shapes (the structural design's
 * ratified check), never by a value brand.
 */
interface FaceDomainMismatch<Left, Right> {
	readonly "face domain mismatch — positionwise domain labels must be equal on both sides": readonly [Left, Right]
}

/**
 * Resolves to `unknown` (a no-op intersection) when the two faces project
 * positionwise-equal domain labels, and to {@link FaceDomainMismatch}
 * otherwise. Equality is mutual tuple assignability over string-literal
 * labels (`undefined` pairs only with `undefined` — an unlabeled field
 * links only unlabeled fields, mirroring Rust where `u64` and `u64 as
 * HolderId` are different host types).
 */
type SameDomains<A extends AnyFace, B extends AnyFace> =
	FaceDomains<A> extends FaceDomains<B>
		? FaceDomains<B> extends FaceDomains<A>
			? unknown
			: FaceDomainMismatch<FaceDomains<A>, FaceDomains<B>>
		: FaceDomainMismatch<FaceDomains<A>, FaceDomains<B>>

/**
 * Projects a face — one spelling, arity-generic: `on(Account, "holder")`
 * for the common single-field position, `on(Booking, ["room", "during"])`
 * for the composite/pointwise position (the interval-pointwise `==` and
 * coverage recipes), `on(Account.where({...}), "id")` for a σ-carrying
 * source. Field names are typechecked against the source (unknown field =
 * type error, names autocomplete); tuple order is preserved (positional
 * pairing with the other side, macro parity). The empty projection is
 * unwritable by signature — it has no meaning in the statement grammar.
 */
function on<S extends FaceSource, const F extends FaceFields<S>>(source: S, field: F): Face<S, readonly [F]>
function on<S extends FaceSource, const P extends readonly [FaceFields<S>, ...FaceFields<S>[]]>(
	source: S,
	fields: P
): Face<S, P>
function on(source: FaceSource, fields: string | readonly string[]): Face<FaceSource, readonly string[]> {
	const projection: readonly string[] = Object.freeze(typeof fields === "string" ? [fields] : [...fields])
	const parts = faceParts(source)
	const data: FaceData = Object.freeze({
		owner: parts.owner,
		projection,
		selection: parts.selection
	})
	return Object.freeze({ source, projection, data })
}

/**
 * Renders one face in the exact macro notation — `Name(p1, p2 | f == lit,
 * g == {a, b})`, the selection block only when σ is nonempty (the engine
 * renderer's own shape, `schema/render.rs`).
 */
function renderFace(face: FaceData): string {
	const projection = face.projection.join(", ")
	if (face.selection.length === 0) {
		return `${face.owner.name}(${projection})`
	}
	const bindings = face.selection
		.map(function renderBinding(binding) {
			return `${binding.field} == ${renderLiteralSet(binding.set)}`
		})
		.join(", ")
	return `${face.owner.name}(${projection} | ${bindings})`
}

export type {
	AnyFace,
	Arity,
	Face,
	FaceArityMismatch,
	FaceData,
	FaceDomainMismatch,
	FaceDomains,
	FaceFields,
	FaceOwner,
	FaceSource,
	OneOf,
	SameArity,
	SameDomains
}
export { on, oneOf, renderFace }
