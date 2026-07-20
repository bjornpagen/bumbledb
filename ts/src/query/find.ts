/**
 * Find entries and aggregates, REFERENCE-IDENTITY edition — the head
 * vocabulary, mirroring the IR's aggregate roster exactly
 * (`bumbledb/crates/bumbledb/src/ir.rs` `AggOp`/`FindTerm`;
 * `docs/architecture/20-query-ir.md` § aggregation): `count` (nullary),
 * `countDistinct`, `sum`/`min`/`max` (over an orderable variable or the
 * measure), `argMax`/`argMin` (arg-restriction: carried value + orderable
 * key), and `pack` (the coalescing fold — RELATION-SHAPED, the result
 * position interval-typed). Aggregates fold VARIABLES by reference —
 * `r.sum(w)` — and are typed by the variable's own descriptor.
 *
 * `select(strings)` is DEAD: the head is a `find` RECORD, whose KEYS name
 * the answer columns (`find({ total: r.count(), owner: h })`). The keys are
 * unique object keys, so a duplicate answer column is unrepresentable and a
 * rename is a real typed key. Grouping is implicit: the non-aggregate
 * entries are the group key; over empty input an all-aggregate find yields
 * the EMPTY SET, never a zero row. The creation quarantine is
 * representational: a head position is a variable, the measure, or an
 * aggregate — no minting or arithmetic term exists to spell (permanent law).
 */

import type { Infer } from "#fields.ts"
import type { SchemaClasses } from "#law.ts"
import type { IntervalVarOk, OrderVarOk } from "#query/atom.ts"
import type { AnyVar, Duration, MintSlotOf } from "#query/scope.ts"

/** One aggregate operator name of the find vocabulary. */
type AggOpName = "count" | "countDistinct" | "sum" | "min" | "max" | "argMax" | "argMin" | "pack"

/**
 * One aggregate find VALUE: the op, the variable (or measure) it folds BY
 * REFERENCE, and — for the Arg forms — the orderable key variable. The
 * variable's own descriptor types the result.
 */
interface Agg<
	Op extends AggOpName,
	Over extends AnyVar | Duration | undefined,
	Key extends AnyVar | undefined = undefined
> {
	readonly agg: Op
	readonly over: Over
	readonly key: Key
}

/** Any aggregate find value. */
type AnyAgg = Agg<AggOpName, AnyVar | Duration | undefined, AnyVar | undefined>

/** One find entry: a projected variable, the measure, or an aggregate. */
type FindEntry = AnyVar | Duration | AnyAgg

/** The `find` record: column name → find entry. Keys ARE the answer columns. */
type FindShape = Readonly<Record<string, FindEntry>>

/** Builds one aggregate value. */
function aggregate<Op extends AggOpName, Over extends AnyVar | Duration | undefined, Key extends AnyVar | undefined>(
	op: Op,
	over: Over,
	key: Key
): Agg<Op, Over, Key> {
	return Object.freeze({ agg: op, over, key })
}

/** Nullary count: |the group's set of distinct full bindings|, `bigint`. */
function count(): Agg<"count", undefined> {
	return aggregate("count", undefined, undefined)
}

/** |the distinct values of the variable across the group|, `bigint`; legal over every type. */
function countDistinct<const V extends AnyVar>(over: V): Agg<"countDistinct", V> {
	return aggregate("countDistinct", over, undefined)
}

/**
 * Exact checked sum over an orderable (u64/i64) variable — wide
 * accumulator, one finalize range check; overflow is the engine's typed
 * runtime error — or over the measure (`r.sum(r.duration(w))`).
 */
function sum<const O extends AnyVar | Duration>(over: O): Agg<"sum", O> {
	return aggregate("sum", over, undefined)
}

/** Minimum over an orderable variable or the measure (orderable types only). */
function min<const O extends AnyVar | Duration>(over: O): Agg<"min", O> {
	return aggregate("min", over, undefined)
}

/** Maximum over an orderable variable or the measure (orderable types only). */
function max<const O extends AnyVar | Duration>(over: O): Agg<"max", O> {
	return aggregate("max", over, undefined)
}

/**
 * Arg-restriction toward the maximum of `key` (`ir::AggOp::ArgMax`): the
 * group's binding set is restricted to the bindings attaining the extreme
 * of the orderable key, and `value` is the carried payload — a tie yields
 * every attaining row. All Arg entries of one query share one key and one
 * direction; Arg and fold aggregates never mix (both the engine's rules).
 */
function argMax<const V extends AnyVar, const K extends AnyVar>(value: V, key: K): Agg<"argMax", V, K> {
	return aggregate("argMax", value, key)
}

/** Arg-restriction toward the minimum of `key`; rules as {@link argMax}. */
function argMin<const V extends AnyVar, const K extends AnyVar>(value: V, key: K): Agg<"argMin", V, K> {
	return aggregate("argMin", value, key)
}

/**
 * The coalescing fold (Snodgrass coalesce, `ir::AggOp::Pack`): per group,
 * the maximal disjoint half-open segments of the union of the group's
 * interval point sets — RELATION-SHAPED, one answer row per (group, maximal
 * segment), the result position carrying one interval of the input's element
 * type. At most one `pack` per find, never beside a fold or an Arg entry.
 */
function pack<const V extends AnyVar>(over: V): Agg<"pack", V> {
	return aggregate("pack", over, undefined)
}

/** A fold input's judgment: an orderable variable, or the measure of an interval variable. */
type FoldOverOk<O> = O extends AnyVar
	? OrderVarOk<O>
	: O extends Duration<infer V extends AnyVar>
		? IntervalVarOk<V>
		: false

/**
 * One find entry's judgment (off the entry's own descriptor — no env, no
 * class map needed): a projected variable is ok, the measure and `pack`
 * demand interval-typed variables, folds and Arg keys demand orderable ones.
 */
type FindEntryOk<E> = E extends AnyVar
	? true
	: E extends Duration<infer V extends AnyVar>
		? IntervalVarOk<V>
		: E extends Agg<"count", undefined>
			? true
			: E extends Agg<"countDistinct", AnyVar>
				? true
				: E extends Agg<"sum" | "min" | "max", infer O>
					? FoldOverOk<O>
					: E extends Agg<"argMax" | "argMin", AnyVar, infer K extends AnyVar>
						? OrderVarOk<K> extends true
							? true
							: false
						: E extends Agg<"pack", infer V extends AnyVar>
							? IntervalVarOk<V>
							: false

/** The validated find record (intersect with the inferred entries — errors land on the offending key). */
type CheckFind<F extends FindShape> = {
	readonly [K in keyof F]: FindEntryOk<F[K]> extends true ? F[K] : never
}

/**
 * The validated find record of a RECURSIVE rule: every entry must be a plain
 * variable (aggregates and the measure are unwritable in a recursive head —
 * the strata quarantine).
 */
type CheckRecFind<F extends FindShape> = {
	readonly [K in keyof F]: F[K] extends AnyVar ? F[K] : never
}

/**
 * One find entry's answer-column value type: a variable carries its field's
 * type, the measure/count/countDistinct are `bigint`, folds carry their
 * input's type (a measure fold is `bigint`), the Arg forms carry the
 * payload's type, `pack` its interval type.
 */
type FindValue<E> = E extends AnyVar
	? Infer<E["field"]>
	: E extends Duration<AnyVar>
		? bigint
		: E extends Agg<"count", undefined>
			? bigint
			: E extends Agg<"countDistinct", AnyVar>
				? bigint
				: E extends Agg<"sum" | "min" | "max", infer O>
					? O extends AnyVar
						? Infer<O["field"]>
						: bigint
					: E extends Agg<"argMax" | "argMin", infer V extends AnyVar, AnyVar>
						? Infer<V["field"]>
						: E extends Agg<"pack", infer V extends AnyVar>
							? Infer<V["field"]>
							: never

/** The inferred answer-row object type of a find record — the keys ARE the columns. */
type RowOfFind<F extends FindShape> = { readonly [K in keyof F]: FindValue<F[K]> }

/**
 * The head signature of a recursive rule's find record as classed mint
 * slots (descriptor + law-computed class), keyed by column name — the
 * signature an `idb` join pairs against (`F` is variable-only there).
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
	FindEntry,
	FindEntryOk,
	FindShape,
	FindValue,
	HeadRecordOf,
	RowOfFind
}
export { argMax, argMin, count, countDistinct, max, min, pack, sum }
