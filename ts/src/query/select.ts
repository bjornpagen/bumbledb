/**
 * Select entries and aggregates, STRUCTURAL edition — the head vocabulary,
 * mirroring the IR's aggregate roster exactly
 * (`bumbledb/crates/bumbledb/src/ir.rs` `AggOp`/`FindTerm`;
 * `docs/architecture/20-query-ir.md` § aggregation): `count` (nullary),
 * `countDistinct`, `sum`/`min`/`max` (over an orderable variable or the
 * measure), `argMax`/`argMin` (arg-restriction: carried value + orderable
 * key; a tie yields every attaining row), and `pack` (the coalescing fold —
 * RELATION-SHAPED: one answer row per (group, maximal segment), the result
 * position interval-typed). Aggregates name their variables — `r.sum("m")`
 * — and are typed by the rule environment at `.select`. Grouping is
 * implicit: the non-aggregate select entries are the group key; over empty
 * input an all-aggregate select yields the EMPTY SET, never a zero row.
 * The creation quarantine is representational: a head position is a var
 * name, the measure, or an aggregate — no minting or arithmetic term
 * exists to spell (permanent law).
 */

import type { Infer } from "#fields.ts"
import type { IntervalVarOk, OrderVarOk } from "#query/atom.ts"
import type { Duration, EnvShape, ShapeOf } from "#query/scope.ts"

/** One aggregate operator name of the select vocabulary. */
type AggOpName = "count" | "countDistinct" | "sum" | "min" | "max" | "argMax" | "argMin" | "pack"

/**
 * One aggregate select VALUE: the op, the variable name (or measure) it
 * folds, and — for the Arg forms — the orderable key's variable name. The
 * runtime representation carries the types; the rule environment types the
 * result at `.select`.
 */
interface Agg<
	Op extends AggOpName,
	Over extends string | Duration<string> | undefined,
	Key extends string | undefined = undefined
> {
	readonly agg: Op
	readonly over: Over
	readonly key: Key
}

/** Any aggregate select value. */
type AnyAgg = Agg<AggOpName, string | Duration<string> | undefined, string | undefined>

/** One select entry: a projected var name, the measure, or an aggregate. */
type SelectEntry = string | Duration<string> | AnyAgg

/** Builds one aggregate value. */
function aggregate<
	Op extends AggOpName,
	Over extends string | Duration<string> | undefined,
	Key extends string | undefined
>(op: Op, over: Over, key: Key): Agg<Op, Over, Key> {
	return Object.freeze({ agg: op, over, key })
}

/** Nullary count: |the group's set of distinct full bindings|, `bigint`; the answer column is named `count`. */
function count(): Agg<"count", undefined> {
	return aggregate("count", undefined, undefined)
}

/** |the distinct values of the named variable across the group|, `bigint`; legal over every type. */
function countDistinct<const N extends string>(over: N): Agg<"countDistinct", N> {
	return aggregate("countDistinct", over, undefined)
}

/**
 * Exact checked sum over an orderable (u64/i64) variable — wide
 * accumulator, one finalize range check; overflow is the engine's typed
 * runtime error, never a wrap — or over the measure
 * (`r.sum(r.duration("w"))`).
 */
function sum<const N extends string | Duration<string>>(over: N): Agg<"sum", N> {
	return aggregate("sum", over, undefined)
}

/** Minimum over an orderable variable or the measure (orderable types only). */
function min<const N extends string | Duration<string>>(over: N): Agg<"min", N> {
	return aggregate("min", over, undefined)
}

/** Maximum over an orderable variable or the measure (orderable types only). */
function max<const N extends string | Duration<string>>(over: N): Agg<"max", N> {
	return aggregate("max", over, undefined)
}

/**
 * Arg-restriction toward the maximum of `key` (`ir::AggOp::ArgMax`): the
 * group's binding set is restricted to the bindings attaining the extreme
 * of the orderable key, and `value` is the carried payload — a tie yields
 * every attaining row. All Arg entries of one query share one key and one
 * direction; Arg and fold aggregates never mix (both the engine's typed
 * rules).
 */
function argMax<const V extends string, const K extends string>(value: V, key: K): Agg<"argMax", V, K> {
	return aggregate("argMax", value, key)
}

/** Arg-restriction toward the minimum of `key`; rules as {@link argMax}. */
function argMin<const V extends string, const K extends string>(value: V, key: K): Agg<"argMin", V, K> {
	return aggregate("argMin", value, key)
}

/**
 * The coalescing fold (Snodgrass coalesce, `ir::AggOp::Pack`): per group,
 * the maximal disjoint half-open segments of the union of the group's
 * interval point sets — RELATION-SHAPED, one answer row per (group,
 * maximal segment), the result position carrying one interval of the
 * input's element type. At most one `pack` per select, never beside a
 * fold or an Arg entry (the engine's typed rules).
 */
function pack<const N extends string>(over: N): Agg<"pack", N> {
	return aggregate("pack", over, undefined)
}

/** A fold input's judgment: an orderable variable, or the measure of an interval variable. */
type FoldOverOk<Env extends EnvShape, O> = O extends string
	? OrderVarOk<Env, O>
	: O extends Duration<infer N extends string>
		? IntervalVarOk<Env, N>
		: false

/**
 * One select entry's judgment against the rule environment: projected
 * names must be bound, the measure and `pack` demand interval-typed
 * variables, folds and Arg keys demand orderable ones.
 */
type SelectEntryOk<Env extends EnvShape, E> = E extends string
	? E extends keyof Env
		? true
		: false
	: E extends Duration<infer N extends string>
		? IntervalVarOk<Env, N>
		: E extends Agg<"count", undefined>
			? true
			: E extends Agg<"countDistinct", infer O extends string>
				? O extends keyof Env
					? true
					: false
				: E extends Agg<"sum" | "min" | "max", infer O>
					? FoldOverOk<Env, O>
					: E extends Agg<"argMax" | "argMin", infer O extends string, infer K extends string>
						? [O extends keyof Env ? true : false, OrderVarOk<Env, K>] extends [true, true]
							? true
							: false
						: E extends Agg<"pack", infer O extends string>
							? IntervalVarOk<Env, O>
							: false

/** The validated select tuple (intersect with the inferred entries — errors land on the offending argument). */
type CheckSelect<Env extends EnvShape, S> = {
	readonly [I in keyof S]: SelectEntryOk<Env, S[I]> extends true ? S[I] : never
}

/** The validated names-only select tuple of a RECURSIVE rule (aggregates and the measure are unwritable there). */
type CheckNameSelect<Env extends EnvShape, S> = {
	readonly [I in keyof S]: S[I] extends keyof Env ? S[I] : never
}

/**
 * One select entry's answer-column fragment: the column is named by the
 * variable it projects or folds (`count` names its column `count`), and
 * its type reflects the entry — `count`/`countDistinct` are `bigint`
 * whatever they counted, folds carry their input's type, the measure is
 * `bigint`, the Arg forms carry the payload's type, `pack` its interval
 * type.
 */
type SelectEntryRow<Env extends EnvShape, E> = E extends string
	? { readonly [K in E]: Infer<Env[E & keyof Env]> }
	: E extends Duration<infer N extends string>
		? { readonly [K in N]: bigint }
		: E extends Agg<"count", undefined>
			? { readonly count: bigint }
			: E extends Agg<"countDistinct", infer O extends string>
				? { readonly [K in O]: bigint }
				: E extends Agg<"sum" | "min" | "max", infer O>
					? O extends string
						? { readonly [K in O]: Infer<Env[O & keyof Env]> }
						: O extends Duration<infer N extends string>
							? { readonly [K in N]: bigint }
							: never
					: E extends Agg<"argMax" | "argMin", infer O extends string, string>
						? { readonly [K in O]: Infer<Env[O & keyof Env]> }
						: E extends Agg<"pack", infer O extends string>
							? { readonly [K in O]: Infer<Env[O & keyof Env]> }
							: never

/** The inferred answer-row object type of a select tuple. */
type RowOfSelect<Env extends EnvShape, S extends readonly SelectEntry[]> = ShapeOf<SelectEntryRow<Env, S[number]>>

/**
 * One projected name's answer-column fragment — a NAKED parameter, so the
 * judgment distributes per name (the union of a multi-name select never
 * smears into one column's type), mirroring {@link SelectEntryRow}.
 */
type NameSelectRow<Env extends EnvShape, N> = N extends string
	? { readonly [K in N]: Infer<Env[K & keyof Env]> }
	: never

/** The inferred answer-row object type of a names-only (recursive-rule) select tuple. */
type RowOfNameSelect<Env extends EnvShape, S extends readonly string[]> = ShapeOf<NameSelectRow<Env, S[number]>>

export type {
	Agg,
	AggOpName,
	AnyAgg,
	CheckNameSelect,
	CheckSelect,
	RowOfNameSelect,
	RowOfSelect,
	SelectEntry,
	SelectEntryOk,
	SelectEntryRow
}
export { argMax, argMin, count, countDistinct, max, min, pack, sum }
