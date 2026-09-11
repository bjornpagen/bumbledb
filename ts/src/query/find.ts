import type { BoolField, F64Field, I64Field, Infer, IntervalField, U64Field } from "#fields.ts"
import type { SchemaClasses } from "#law.ts"
import type { IntervalVarOk, NumericVarOk } from "#query/atom.ts"
import type { AnyComputeExpr, ComputeExpr, ComputeValue } from "#query/compute.ts"
import type { AnyVar, MintSlotOf } from "#query/scope.ts"
import type { Segments } from "#query/segments.ts"
import type { ScalarKind } from "#scalar.ts"

type FoldOpName = "sum" | "mean" | "min" | "max" | "pack"

interface CountAgg {
	readonly agg: "count"
}

interface Agg<Op extends FoldOpName, Over extends AnyVar> {
	readonly agg: Op
	readonly over: Over
}

type AnyAgg = CountAgg | Agg<FoldOpName, AnyVar>

type FindEntry = AnyVar | AnyAgg | AnyComputeExpr | Segments

type FindShape = Readonly<Record<string, FindEntry>>

function aggregate<Op extends FoldOpName, Over extends AnyVar>(op: Op, over: Over): Agg<Op, Over> {
	return Object.freeze({ agg: op, over })
}

function count(): CountAgg {
	return Object.freeze({ agg: "count" })
}

/**
 * Exact checked sum over a NUMERIC (u64/i64/f64) variable — wide accumulator,
 * one finalize range check; overflow is the engine's typed runtime error —
 * of a measured interval in a following stage. Bool stays refused: a
 * quantifier is not an addition (R3).
 */
function sum<const O extends AnyVar>(over: O): Agg<"sum", O> {
	return aggregate("sum", over)
}

/** Exact sum divided by the binding count, rounded once. F64 inputs only. */
function mean<const O extends AnyVar>(over: O): Agg<"mean", O> {
	return aggregate("mean", over)
}

function min<const O extends AnyVar>(over: O): Agg<"min", O> {
	return aggregate("min", over)
}

function max<const O extends AnyVar>(over: O): Agg<"max", O> {
	return aggregate("max", over)
}

function pack<const V extends AnyVar>(over: V): Agg<"pack", V> {
	return aggregate("pack", over)
}

type FindEntryOk<E> = E extends AnyVar
	? true
	: E extends CountAgg
		? true
		: E extends Agg<"mean", infer O extends AnyVar>
			? O["field"]["kind"] extends "f64"
				? true
				: false
			: E extends Agg<"sum" | "min" | "max", infer O extends AnyVar>
				? NumericVarOk<O>
				: E extends Agg<"pack", infer V extends AnyVar>
					? IntervalVarOk<V>
					: E extends AnyComputeExpr | Segments
						? true
						: false

type CheckFind<F extends FindShape> = {
	readonly [K in keyof F]: FindEntryOk<F[K]> extends true ? F[K] : never
}

type CheckRecFind<F extends FindShape> = {
	readonly [K in keyof F]: F[K] extends AnyVar ? F[K] : never
}

type FindValue<E> =
	E extends Segments<infer Element>
		? Infer<IntervalField<Element, undefined>>
		: E extends AnyVar
			? Infer<E["field"]>
			: E extends CountAgg
				? bigint
				: E extends Agg<"sum" | "mean" | "min" | "max", infer O extends AnyVar>
					? Infer<O["field"]>
					: E extends Agg<"pack", infer V extends AnyVar>
						? Infer<V["field"]>
						: E extends ComputeExpr<infer K extends ScalarKind>
							? ComputeValue<K>
							: never

type RowOfFind<F extends FindShape> = { readonly [K in keyof F]: FindValue<F[K]> }

type ComputeFields = {
	readonly u64: U64Field
	readonly i64: I64Field
	readonly f64: F64Field
	readonly bool: BoolField
}

/**
 * The head signature as classed slots, shared by derived-stage and imported
 * query variables. Projection preserves its carrier; aggregates/computations
 * produce bare values, matching findColumnSlotOf at runtime.
 */
type HeadRecordOf<Classes extends SchemaClasses, F extends FindShape> = {
	readonly [K in keyof F]: F[K] extends Segments<infer Element>
		? { readonly field: IntervalField<Element, undefined>; readonly class: undefined }
		: F[K] extends AnyVar
			? MintSlotOf<Classes, F[K]>
			: F[K] extends ComputeExpr<infer Kind extends ScalarKind>
				? { readonly field: ComputeFields[Kind]; readonly class: undefined }
				: F[K] extends CountAgg
					? { readonly field: U64Field; readonly class: undefined }
					: F[K] extends Agg<"mean", AnyVar>
						? { readonly field: F64Field; readonly class: undefined }
						: F[K] extends Agg<"pack", infer Over extends AnyVar>
							? {
									readonly field: IntervalField<Over["field"] extends IntervalField<infer E> ? E : never, undefined>
									readonly class: undefined
								}
							: F[K] extends Agg<FoldOpName, infer Over extends AnyVar>
								? { readonly field: Over["field"]; readonly class: undefined }
								: never
}

export type { Agg, CheckFind, CheckRecFind, FindEntry, FindShape, HeadRecordOf, RowOfFind }
export { count, max, mean, min, pack, sum }
