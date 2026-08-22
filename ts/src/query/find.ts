import type { Infer } from "#fields.ts"
import type { SchemaClasses } from "#law.ts"
import type { IntervalVarOk, NumericVarOk } from "#query/atom.ts"
import type { AnyVar, Duration, MintSlotOf } from "#query/scope.ts"

type FoldOpName = "sum" | "min" | "max" | "pack"
type AggOpName = "count" | FoldOpName

interface CountAgg {
	readonly agg: "count"
}

interface Agg<Op extends FoldOpName, Over extends AnyVar | Duration> {
	readonly agg: Op
	readonly over: Over
}

type AnyAgg = CountAgg | Agg<FoldOpName, AnyVar | Duration>

type FindEntry = AnyVar | Duration | AnyAgg

type FindShape = Readonly<Record<string, FindEntry>>

function aggregate<Op extends FoldOpName, Over extends AnyVar | Duration>(op: Op, over: Over): Agg<Op, Over> {
	return Object.freeze({ agg: op, over })
}

function count(): CountAgg {
	return Object.freeze({ agg: "count" })
}

/**
 * Exact checked sum over a NUMERIC (u64/i64) variable — wide accumulator,
 * one finalize range check; overflow is the engine's typed runtime error —
 * or over the measure (`r.sum(r.duration(w))`). Bool stays refused: a
 * quantifier is not an addition (R3).
 */
function sum<const O extends AnyVar | Duration>(over: O): Agg<"sum", O> {
	return aggregate("sum", over)
}

function min<const O extends AnyVar | Duration>(over: O): Agg<"min", O> {
	return aggregate("min", over)
}

function max<const O extends AnyVar | Duration>(over: O): Agg<"max", O> {
	return aggregate("max", over)
}

function pack<const V extends AnyVar>(over: V): Agg<"pack", V> {
	return aggregate("pack", over)
}

type FoldOverOk<O> = O extends AnyVar
	? NumericVarOk<O>
	: O extends Duration<infer V extends AnyVar>
		? IntervalVarOk<V>
		: false

type FindEntryOk<E> = E extends AnyVar
	? true
	: E extends Duration<infer V extends AnyVar>
		? IntervalVarOk<V>
		: E extends CountAgg
			? true
			: E extends Agg<"sum" | "min" | "max", infer O>
				? FoldOverOk<O>
				: E extends Agg<"pack", infer V extends AnyVar>
					? IntervalVarOk<V>
					: false

type CheckFind<F extends FindShape> = {
	readonly [K in keyof F]: FindEntryOk<F[K]> extends true ? F[K] : never
}

type CheckRecFind<F extends FindShape> = {
	readonly [K in keyof F]: F[K] extends AnyVar ? F[K] : never
}

type FindValue<E> = E extends AnyVar
	? Infer<E["field"]>
	: E extends Duration<AnyVar>
		? bigint
		: E extends CountAgg
			? bigint
			: E extends Agg<"sum" | "min" | "max", infer O>
				? O extends AnyVar
					? Infer<O["field"]>
					: bigint
				: E extends Agg<"pack", infer V extends AnyVar>
					? Infer<V["field"]>
					: never

type RowOfFind<F extends FindShape> = { readonly [K in keyof F]: FindValue<F[K]> }

/**
 * The head signature of a recursive rule's find record as classed mint
 * slots (descriptor + law-computed class), keyed by column name — the
 * signature an interior join pairs against (`F` is variable-only there).
 */
type HeadRecordOf<Classes extends SchemaClasses, F extends FindShape> = {
	readonly [K in keyof F]: F[K] extends AnyVar ? MintSlotOf<Classes, F[K]> : never
}

export type {
	Agg,
	AggOpName,
	AnyAgg,
	CheckFind,
	CheckRecFind,
	CountAgg,
	FindEntry,
	FindEntryOk,
	FindShape,
	FindValue,
	FoldOpName,
	HeadRecordOf,
	RowOfFind
}
export { count, max, min, pack, sum }
