/**
 * Faces — the projection-with-selection value both containments and
 * windows consume: `on(Account, "holder")` the common single-field
 * position, `on(Booking, ["room", "during"])` the composite/pointwise
 * position (one spelling, arity-generic), `on(Account.where({ kind:
 * Kind.Savings }), "id")` the σ-carrying source, `on(Kind, "id")` a closed
 * relation's sealed shape opened through its synthetic `id`. Projection is
 * positional: tuple order is preserved in the type, and the statement
 * constructors pair the two sides' tuples by arity ({@link SameArity}) AND
 * by structural shape ({@link SameShapes}) — every projected field's
 * kind/width/element triple is read off the schema type (the minimal
 * kernel: descriptors are pure structure) and compared positionwise. There
 * is no domain to compare at construction — domains are LAW-BORN: the
 * statements themselves define the equivalence classes, and `schema()` is
 * where they aggregate and get judged (the one-generator-per-class wall).
 */

import type { AnyClosed, PayloadField } from "#closed.ts"
import type { AnyField } from "#fields.ts"
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

/**
 * One descriptor's structural comparand: the kind/width/element triple —
 * exactly the structure the minimal kernel carries (`width` on bytes and
 * intervals, `element` on intervals, `undefined` where a kind has no such
 * label).
 */
type ShapeOf<F extends AnyField> = readonly [
	F["kind"],
	F extends { readonly width: infer W } ? W : undefined,
	F extends { readonly element: infer E } ? E : undefined
]

/** One field's structural shape within a declared field block (`undefined` when the name is foreign). */
type ShapeIn<Fields extends FieldsShape, K extends string> = K extends keyof Fields ? ShapeOf<Fields[K]> : undefined

/**
 * The structural SHAPE of one projected field, read off the source's
 * schema type — an ordinary or selected relation's field contributes its
 * descriptor's triple; a closed relation's synthetic `id` is a u64, and
 * its payload columns contribute their declared descriptors' triples
 * through the closed value's typed `columns` carrier (whose runtime twin
 * is the frozen `columns` record the mint carries). The engine stays the
 * final authority at `Db.create`/`Db.open`.
 */
type ProjectedShape<S extends FaceSource, K extends string> = S extends AnySelected
	? ShapeIn<RelationFields<S["relation"]>, K>
	: S extends AnyRelation
		? ShapeIn<RelationFields<S>, K>
		: S extends {
					readonly id: infer Id extends AnyField
					readonly columns: infer Cols extends Record<string, PayloadField>
				}
			? K extends "id"
				? ShapeOf<Id>
				: ShapeIn<Cols, K>
			: undefined

/** The positionwise structural-shape tuple of a projection over `S`. */
type ShapesOf<S extends FaceSource, P extends readonly string[]> = {
	readonly [I in keyof P]: ProjectedShape<S, P[I] & string>
}

/** The shape tuple a face projects, positionwise — the comparand of {@link SameShapes}. */
type FaceShapes<F extends AnyFace> = F extends Face<infer S, infer P> ? ShapesOf<S, P> : never

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
 * The legible shape-mismatch verdict: when the two faces of a containment,
 * bijection, or window project structurally incompatible fields at any
 * position, this type is intersected into the second face's parameter and
 * names both shape tuples — a u64 face against a str face, a bytes width
 * mismatch, or an interval element mismatch is a COMPILE error.
 */
interface FaceShapeMismatch<Left, Right> {
	readonly "face shape mismatch — positionwise kind, width, and element must be equal on both sides": readonly [
		Left,
		Right
	]
}

/**
 * Resolves to `unknown` (a no-op intersection) when the two faces project
 * positionwise-equal structural shapes, and to {@link FaceShapeMismatch}
 * otherwise. Equality is mutual tuple assignability over the
 * kind/width/element triples. This is the whole construction-time wall —
 * deliberately: there is no domain to compare here. The domain wall lives
 * where domains are BORN: `schema()` computes every field's class from the
 * statement list and holds the one-generator-per-class law, and query
 * joins compare class names off the schema type.
 */
type SameShapes<A extends AnyFace, B extends AnyFace> =
	FaceShapes<A> extends FaceShapes<B>
		? FaceShapes<B> extends FaceShapes<A>
			? unknown
			: FaceShapeMismatch<FaceShapes<A>, FaceShapes<B>>
		: FaceShapeMismatch<FaceShapes<A>, FaceShapes<B>>

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
	FaceFields,
	FaceOwner,
	FaceShapeMismatch,
	FaceShapes,
	FaceSource,
	OneOf,
	SameArity,
	SameShapes
}
export { on, oneOf, renderFace }
