/**
 * Faces — the projection-with-selection value both containments and
 * windows consume: `on(Account, "holder")`, `on(Account.where({ kind:
 * Kind.Savings }), "id")`, `on(Kind, "id")` (a closed relation's sealed
 * shape opens with its synthetic `id`). Projection is positional: tuple
 * order is preserved in the type, and the statement constructors pair the
 * two sides' tuples by arity (`SameArity`).
 */

import type { AnyClosed } from "#closed.ts"
import type { AnyRelation, AnySelected, SelectionBinding } from "#relation.ts"
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
 * A face value. `P` is the projection tuple as written — the statement
 * constructors read its length for positional-arity pairing.
 */
interface Face<P extends readonly string[]> {
	readonly projection: P
	readonly data: FaceData
}

/** Any face value, whatever its projection. */
type AnyFace = Face<readonly string[]>

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
 * Projects a face: `on(Account, "holder")`, `on(Account.where({...}),
 * "id")`. Field names are typechecked against the source; tuple order is
 * preserved (positional pairing with the other side, macro parity). At
 * least one field by signature — an empty projection has no meaning in the
 * statement grammar.
 */
function on<S extends FaceSource, const P extends readonly [FaceFields<S>, ...FaceFields<S>[]]>(
	source: S,
	...projection: P
): Face<P> {
	const parts = faceParts(source)
	Object.freeze(projection)
	const data: FaceData = Object.freeze({
		owner: parts.owner,
		projection,
		selection: parts.selection
	})
	return Object.freeze({ projection, data })
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

export type { AnyFace, Arity, Face, FaceArityMismatch, FaceData, FaceFields, FaceOwner, FaceSource, OneOf, SameArity }
export { on, oneOf, renderFace }
