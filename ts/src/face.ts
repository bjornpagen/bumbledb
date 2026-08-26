import * as errors from "@superbuilders/errors"
import type { AnyClosed, AnySelectedClosed, PayloadField } from "#closed.ts"
import type { AnyField, SignatureOf } from "#fields.ts"
import type { Same } from "#judgment.ts"
import type { AnyRelation, AnySelected, FieldsShape, RelationFields, SelectionBinding } from "#relation.ts"
import { renderLiteralSet } from "#spec.ts"

const emptySelection: readonly SelectionBinding[] = Object.freeze([])

type OwnerOf<S extends FaceSource> = S extends AnySelected | AnySelectedClosed ? S["relation"] : S

function faceParts(source: FaceSource): {
	readonly owner: FaceOwner
	readonly selection: readonly SelectionBinding[]
} {
	if ("relation" in source) {
		return { owner: source.relation, selection: source.selection }
	}
	return { owner: source, selection: emptySelection }
}

type FaceOwner = AnyRelation | AnyClosed

interface FaceData<O extends FaceOwner = FaceOwner, P extends readonly string[] = readonly string[]> {
	readonly owner: O
	readonly projection: P
	readonly selection: readonly SelectionBinding[]
}

interface Face<S extends FaceSource, P extends readonly string[]> {
	readonly source: S
	readonly projection: P
	readonly data: FaceData<OwnerOf<S>, P>
}

type AnyFace = Face<FaceSource, readonly string[]>

type FaceSource = AnyRelation | AnyClosed | AnySelected | AnySelectedClosed

type FaceFields<S extends FaceSource> = S extends AnySelected
	? keyof RelationFields<S["relation"]> & string
	: S extends AnySelectedClosed
		? "id" | (keyof S["relation"]["columns"] & string)
		: S extends AnyRelation
			? keyof RelationFields<S> & string
			: S extends { readonly axioms: Readonly<Record<string, infer Row>> }
				? "id" | (keyof Row & string)
				: never

/**
 * The wire shape a face projects: the field's {@link SignatureOf} with an
 * interval's width erased. A fixed-width interval pairs with a plain one —
 * width is a measure label, not wire structure. Bytes width IS wire
 * structure and stays. The roster stays the full handle vector.
 */
type ProjectedSignature<F extends AnyField> = F extends { readonly kind: "interval" }
	? readonly [F["kind"], undefined, F extends { readonly element: infer E } ? E : undefined, undefined]
	: SignatureOf<F>

type ShapeIn<Fields extends FieldsShape, K extends string> = K extends keyof Fields
	? ProjectedSignature<Fields[K]>
	: undefined

type ProjectedShape<S extends FaceSource, K extends string> = S extends AnySelected
	? ShapeIn<RelationFields<S["relation"]>, K>
	: S extends AnySelectedClosed
		? K extends "id"
			? ProjectedSignature<S["relation"]["id"]>
			: ShapeIn<S["relation"]["columns"], K>
		: S extends AnyRelation
			? ShapeIn<RelationFields<S>, K>
			: S extends {
						readonly id: infer Id extends AnyField
						readonly columns: infer Cols extends Record<string, PayloadField>
					}
				? K extends "id"
					? ProjectedSignature<Id>
					: ShapeIn<Cols, K>
				: undefined

type ShapesOf<S extends FaceSource, P extends readonly string[]> = {
	readonly [I in keyof P]: ProjectedShape<S, P[I] & string>
}

type FaceShapes<F extends AnyFace> = F extends Face<infer S, infer P> ? ShapesOf<S, P> : never

type Arity<F extends AnyFace> = F["projection"]["length"]

interface FaceArityMismatch<Left, Right> {
	readonly "face arity mismatch — positional pairing requires both sides to project equally many fields": readonly [
		Left,
		Right
	]
}

type SameArity<A extends AnyFace, B extends AnyFace> =
	Same<Arity<A>, Arity<B>> extends true ? unknown : FaceArityMismatch<Arity<A>, Arity<B>>

interface FaceShapeMismatch<Left, Right> {
	readonly "face shape mismatch — positionwise kind, width, element, and closed roster must be equal on both sides": readonly [
		Left,
		Right
	]
}

type SameShapes<A extends AnyFace, B extends AnyFace> =
	Same<FaceShapes<A>, FaceShapes<B>> extends true ? unknown : FaceShapeMismatch<FaceShapes<A>, FaceShapes<B>>

function on<S extends FaceSource, const F extends FaceFields<S>>(source: S, field: F): Face<S, readonly [F]>
function on<S extends FaceSource, const P extends readonly [FaceFields<S>, ...FaceFields<S>[]]>(
	source: S,
	fields: P
): Face<S, P>
function on<S extends FaceSource>(source: S, fields: string | readonly string[]): Face<S, readonly string[]> {
	const projection: readonly string[] = Object.freeze(typeof fields === "string" ? [fields] : [...fields])
	const parts = faceParts(source)
	const data: FaceData = Object.freeze({
		owner: parts.owner,
		projection,
		selection: parts.selection
	})
	const value = Object.freeze({ source, projection, data })
	if (!faceMinted<S, readonly string[]>(value, source, projection)) {
		throw errors.new(`face over ${parts.owner.name}: face construction incomplete`)
	}
	return value
}

/**
 * The trusted admission seam of the face mint (the pattern's home is
 * `isTypedScope` in query/lower.ts): the
 * checkable facts — the value carries exactly the source and projection it
 * was built from, and `data.owner` is exactly the owner {@link faceParts}
 * resolves for that source — are verified before the wide construction is
 * admitted at the exact {@link Face} type (whose `data` claims the owner at
 * its precise type, the carrier the schema-level law-typing reads).
 */
function faceMinted<S extends FaceSource, P extends readonly string[]>(
	value: { readonly source: FaceSource; readonly projection: readonly string[]; readonly data: FaceData },
	source: S,
	projection: P
): value is Face<S, P> {
	const owner = "relation" in source ? source.relation : source
	return (
		value.source === source &&
		value.projection === projection &&
		value.data.owner === owner &&
		value.data.projection === projection
	)
}

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
	OwnerOf,
	ProjectedShape,
	SameArity,
	SameShapes
}
export { on, renderFace }
