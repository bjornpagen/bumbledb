/**
 * Select entries and aggregates (PRD-08) — the head vocabulary, mirroring
 * the IR's aggregate roster exactly (`bumbledb/crates/bumbledb/src/ir.rs`
 * `AggOp`/`FindTerm`; `docs/architecture/20-query-ir.md` § aggregation):
 * `count` (nullary), `countDistinct`, `sum`/`min`/`max` (over a u64/i64
 * variable or the measure), `argmax`/`argmin` (arg-restriction: carried
 * value + orderable key; a tie yields every attaining row), and `pack`
 * (the coalescing fold — RELATION-SHAPED: one answer row per (group,
 * maximal segment), the result position interval-typed). Grouping is
 * implicit: the non-aggregate select entries are the group key; over empty
 * input an all-aggregate select yields the EMPTY SET, never a zero row.
 */

import type { IntervalValue } from "#brand.ts"
import { phantom } from "#brand.ts"
import type { Duration } from "#query/atom.ts"
import type { AnyVar, Flatten, Var } from "#query/scope.ts"
import { term } from "#query/scope.ts"

/** The three folds the measure admits (and the plain-variable folds). */
type FoldOp = "sum" | "min" | "max"

/** One aggregate's runtime description. */
type AggregateData =
	| { readonly op: "count" }
	| { readonly op: "countDistinct"; readonly over: AnyVar }
	| { readonly op: "fold"; readonly fold: FoldOp; readonly over: AnyVar | Duration }
	| {
			readonly op: "arg"
			readonly direction: "argMax" | "argMin"
			readonly key: AnyVar
			readonly over: AnyVar
	  }
	| { readonly op: "pack"; readonly over: AnyVar }

/**
 * One aggregate select value; the phantom carries the answer column's
 * host type (`count`/`countDistinct` → `bigint` whatever they counted;
 * folds carry their input's type; the Arg forms the carried payload's
 * type; `pack` its interval type).
 */
interface Aggregate<R> {
	readonly aggregate: AggregateData
	readonly [phantom]?: R
}

/** What a select record's values may be: a projection, a measure, or an aggregate. */
type SelectEntryInput = AnyVar | Duration | Aggregate<unknown>

/** The select record: answer column name to entry, written order = column order. */
type SelectShape = Readonly<Record<string, SelectEntryInput>>

/** One select entry's answer type. */
type SelectValue<T> = T extends { readonly aggregate: AggregateData; readonly [phantom]?: infer R }
	? Exclude<R, undefined>
	: T extends { readonly measure: AnyVar }
		? bigint
		: T extends { readonly [term]: "var"; readonly [phantom]?: infer V }
			? Exclude<V, undefined>
			: never

/** The inferred answer-row object type of a select record. */
type RowOf<Sel extends SelectShape> = Flatten<{ [K in keyof Sel]: SelectValue<Sel[K]> }>

/** Nullary count: |the group's set of distinct full bindings|, `bigint`. */
function count(): Aggregate<bigint> {
	return Object.freeze({ aggregate: Object.freeze({ op: "count" as const }) })
}

/** |the distinct values of `over` across the group|, `bigint`; legal over every type. */
function countDistinct<V>(over: Var<V>): Aggregate<bigint> {
	return Object.freeze({ aggregate: Object.freeze({ op: "countDistinct" as const, over }) })
}

/**
 * Exact checked sum over a u64/i64 variable (wide accumulator, one
 * finalize range check — overflow is the engine's typed runtime error,
 * never a wrap), or over the measure (`sum(duration(v))`).
 */
function sum<V extends bigint>(over: Var<V>): Aggregate<V>
function sum(over: Duration): Aggregate<bigint>
function sum(over: Var<bigint> | Duration): Aggregate<bigint> {
	return Object.freeze({
		aggregate: Object.freeze({ op: "fold" as const, fold: "sum" as const, over })
	})
}

/** Minimum over a u64/i64 variable or the measure (orderable types only). */
function min<V extends bigint>(over: Var<V>): Aggregate<V>
function min(over: Duration): Aggregate<bigint>
function min(over: Var<bigint> | Duration): Aggregate<bigint> {
	return Object.freeze({
		aggregate: Object.freeze({ op: "fold" as const, fold: "min" as const, over })
	})
}

/** Maximum over a u64/i64 variable or the measure (orderable types only). */
function max<V extends bigint>(over: Var<V>): Aggregate<V>
function max(over: Duration): Aggregate<bigint>
function max(over: Var<bigint> | Duration): Aggregate<bigint> {
	return Object.freeze({
		aggregate: Object.freeze({ op: "fold" as const, fold: "max" as const, over })
	})
}

/**
 * Arg-restriction toward the maximum of `key` (`ir::AggOp::ArgMax`): the
 * group's binding set is restricted to the bindings attaining the extreme
 * of the orderable key, and `value` is the carried payload — a tie yields
 * every attaining row. All Arg entries of one query share one key and one
 * direction; Arg and fold aggregates never mix (both the engine's typed
 * rules).
 */
function argmax<K extends bigint, V>(key: Var<K>, value: Var<V>): Aggregate<V> {
	return Object.freeze({
		aggregate: Object.freeze({ op: "arg" as const, direction: "argMax" as const, key, over: value })
	})
}

/** Arg-restriction toward the minimum of `key`; rules as {@link argmax}. */
function argmin<K extends bigint, V>(key: Var<K>, value: Var<V>): Aggregate<V> {
	return Object.freeze({
		aggregate: Object.freeze({ op: "arg" as const, direction: "argMin" as const, key, over: value })
	})
}

/**
 * The coalescing fold (Snodgrass coalesce, `ir::AggOp::Pack`): per group,
 * the maximal disjoint half-open segments of the union of the group's
 * interval point sets — RELATION-SHAPED, one answer row per (group,
 * maximal segment), the result position carrying one interval of the
 * input's element type. At most one `pack` per select, never beside a
 * fold or an Arg entry (the engine's typed rules).
 */
function pack<IV extends IntervalValue>(over: Var<IV>): Aggregate<IV> {
	return Object.freeze({ aggregate: Object.freeze({ op: "pack" as const, over }) })
}

export type { Aggregate, AggregateData, FoldOp, RowOf, SelectEntryInput, SelectShape, SelectValue }
export { argmax, argmin, count, countDistinct, max, min, pack, sum }
