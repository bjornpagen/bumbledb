import { AuthoringError } from "#errors.ts"
import type { IntervalElementKind, IntervalField } from "#fields.ts"
import { type AnyVar, isTerm, term } from "#query/scope.ts"
import { recordValue } from "#values.ts"

type IntervalVar<E extends IntervalElementKind = IntervalElementKind> = AnyVar & { readonly field: IntervalField<E> }
type SegmentOp = "intersection" | "difference"

/** A relational find output, using the native binary segment operator directly. */
interface Segments<E extends IntervalElementKind = IntervalElementKind> {
	readonly kind: "segments"
	readonly op: SegmentOp
	readonly left: IntervalVar<E>
	readonly right: IntervalVar<E>
}

function segments<E extends IntervalElementKind>(
	op: SegmentOp,
	left: IntervalVar<E>,
	right: IntervalVar<NoInfer<E>>
): Segments<E> {
	if (
		!isTerm(left) ||
		left[term] !== "var" ||
		!isTerm(right) ||
		right[term] !== "var" ||
		left.field.kind !== "interval" ||
		right.field.kind !== "interval" ||
		left.field.element !== right.field.element
	)
		throw new AuthoringError({ message: `${op}: expected interval variables of the same element kind` })
	return Object.freeze({ kind: "segments", op, left, right })
}

function intersection<E extends IntervalElementKind>(
	left: IntervalVar<E>,
	right: IntervalVar<NoInfer<E>>
): Segments<E> {
	return segments("intersection", left, right)
}

function difference<E extends IntervalElementKind>(left: IntervalVar<E>, right: IntervalVar<NoInfer<E>>): Segments<E> {
	return segments("difference", left, right)
}

function isSegments(input: unknown): input is Segments {
	if (typeof input !== "object" || input === null || !("kind" in input) || input.kind !== "segments") return false
	recordValue("segments", input, ["kind", "op", "left", "right"])
	const value = input as Segments
	if (value.op !== "intersection" && value.op !== "difference")
		throw new AuthoringError({ message: "unknown segment operator" })
	segments(value.op, value.left, value.right)
	return true
}

function segmentField(value: Segments): IntervalField<IntervalElementKind, undefined> {
	return Object.freeze({ kind: "interval", element: value.left.field.element, width: undefined })
}

export type { IntervalVar, SegmentOp, Segments }
export { difference, intersection, isSegments, segmentField }
