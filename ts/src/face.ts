import type { AnyClosed } from "#closed.ts"
import { memberDescriptor, sealedFieldOf } from "#closed.ts"
import { AuthoringError } from "#errors.ts"
import { type AnyField, assertDeclarationRecord, type SignatureOf } from "#fields.ts"
import type { Same } from "#judgment.ts"
import type { AnyRelation, FieldsShape } from "#relation.ts"
import { type AnySelected, type FieldsOf, type SelectionBinding, selectionBindings } from "#selection.ts"
import { renderLiteralSet } from "#spec.ts"
import { arrayValue, recordValue } from "#values.ts"

const emptySelection: readonly SelectionBinding[] = Object.freeze([])

type OwnerOf<S extends FaceSource> = S extends AnySelected ? S["relation"] : S

function faceParts(source: FaceSource): {
	readonly owner: FaceOwner
	readonly selection: readonly SelectionBinding[]
} {
	assertDeclarationRecord("face source", source)
	if ("relation" in source) {
		recordValue("selected relation", source, ["relation", "selection"])
		return { owner: source.relation, selection: source.selection }
	}
	return { owner: source, selection: emptySelection }
}

type FaceOwner = AnyRelation | AnyClosed

interface Face<O extends FaceOwner = FaceOwner, P extends readonly string[] = readonly string[]> {
	readonly owner: O
	readonly projection: P
	readonly selection: readonly SelectionBinding[]
}

type AnyFace = Face<FaceOwner, readonly string[]>

type FaceSource = AnyRelation | AnyClosed | AnySelected

type FaceFields<S extends FaceSource> = keyof FieldsOf<OwnerOf<S>> & string

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

type ProjectedShape<S extends FaceSource, K extends string> = ShapeIn<FieldsOf<OwnerOf<S>>, K>

type ShapesOf<S extends FaceSource, P extends readonly string[]> = {
	readonly [I in keyof P]: ProjectedShape<S, P[I] & string>
}

type FaceShapes<F extends AnyFace> = ShapesOf<F["owner"], F["projection"]>

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

function on<S extends FaceSource, const F extends FaceFields<S>>(source: S, field: F): Face<OwnerOf<S>, readonly [F]>
function on<S extends FaceSource, const P extends readonly [FaceFields<S>, ...FaceFields<S>[]]>(
	source: S,
	fields: P
): Face<OwnerOf<S>, P>
function on(source: FaceSource, fields: string | readonly string[]): AnyFace {
	const parts = faceParts(source)
	return faceDescriptor({
		owner: parts.owner,
		projection: typeof fields === "string" ? [fields] : fields,
		selection: parts.selection
	})
}

/** Own and check a structural face. No constructor-only admission token. */
function faceDescriptor<F extends AnyFace>(input: F): F
function faceDescriptor(input: unknown): AnyFace
function faceDescriptor(raw: unknown): AnyFace {
	const input = recordValue("face", raw, ["owner", "projection", "selection"])
	const owner = memberDescriptor(input.owner)
	const projection = arrayValue("face projection", input.projection, (_, value) => {
		if (typeof value !== "string") throw new AuthoringError({ message: "face: expected a field name" })
		return value
	})
	if (projection.length === 0) {
		throw new AuthoringError({ message: "face: expected a nonempty projection" })
	}
	const projected = new Set<string>()
	for (const field of projection) {
		if (typeof field !== "string" || sealedFieldOf(owner, field) === undefined)
			throw new AuthoringError({ message: `face ${owner.name}: unknown field ${String(field)}` })
		if (projected.has(field)) throw new AuthoringError({ message: `face ${owner.name}: duplicate field ${field}` })
		projected.add(field)
	}
	const selection = selectionBindings(owner, input.selection)
	return Object.freeze({ owner, projection, selection })
}

function renderFace(face: AnyFace): string {
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
export { faceDescriptor, on, renderFace }
