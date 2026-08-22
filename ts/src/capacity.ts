/**
 * Capacity-law mints — the window, weight, and dependent-bound vocabulary
 * the one `capacity` statement constructor consumes
 * § the canonical-utterance law, restated
 * around the aggregate form). `within` is the ONE window spelling —
 * `within(n)` exact (`within(0n)` IS the exclusion's one spelling),
 * `within(lo, hi)` range (`within(0n, hi)` the canonical ceiling),
 * `within(lo, "*")` floor — `weigh` names the measure on the SOURCE row
 * (`weigh("watts")` a u64 field, `weigh(duration("booked"))` an interval's
 * measure), and `ref`/`duration` read a dependent bound from the
 * TARGET row (hi slot only — ruled 2026-07-24, C6). The ban table is
 * enforced REPRESENTATIONALLY in two tiers, split per-aggregate where
 * weight-sensitive (design § 6: a ban is canonical-utterance policing when
 * it is weight-independent, semantic deduplication when it is not):
 *
 * - **The type tier**: a banned spelling written as a LITERAL does not
 * compile — every negative bound, `within(n, n)`, `within(0n, 0n)`, and
 * `within(0n, "*")` are type errors naming the canonical form. The
 * weight-SENSITIVE row rides the `capacity` overloads themselves:
 * `within(1n, "*")` is banned on the unit overload only (`{1..*}` on the
 * count instance is the bare containment respelled —
 * `window_floor_containment`), and LEGAL on the weighted one ("positive
 * total" is not an existence claim over rows).
 * - **The construction tier**: a bound the type level cannot judge — a
 * COMPUTED `bigint`, or an inverted `within(lo, hi)` order — is judged at
 * construction with the same canonical-naming errors; and past both
 * tiers the engine's own spec validation remains the law for a hostile
 * FFI caller (the standing two-tier ban enforcement).
 *
 * The weight vocabulary is closed at the row (ruled 2026-07-24, ruling 6):
 * a path weight (`weigh("model.watts")`) is a typed refusal at BOTH tiers
 * whose diagnostic names the pinned-column idiom — the two-column
 * containment IS the join, stated as a law. Bounds are `bigint` (u64
 * crosses as bigint always, PRD-04's law); the witnessed measure comes
 * back as `bigint` too (u128-wide engine accumulator, C3).
 */

import * as errors from "@superbuilders/errors"
import type { AnyFace, FaceFields, FaceSource, ProjectedShape } from "#face.ts"
import type { CapacityBoundSpec, CapacityWindowSpec, WeightSpec } from "#spec.ts"

const admitted: unique symbol = Symbol("bumbledb.capacity.admitted")

interface CapacityWindow<S extends CapacityWindowSpec = CapacityWindowSpec> {
	readonly window: S
	readonly [admitted]: true
}

interface CapacityWeight<S extends WeightSpec = WeightSpec> {
	readonly weight: S
	readonly [admitted]: true
}

interface FieldRef<F extends string = string> {
	readonly kind: "field"
	readonly field: F
}

interface DurationRef<F extends string = string> {
	readonly kind: "durationField"
	readonly field: F
}

function isCapacityWindow(value: unknown): value is CapacityWindow {
	return typeof value === "object" && value !== null && admitted in value && "window" in value
}

function isCapacityWeight(value: unknown): value is CapacityWeight {
	return typeof value === "object" && value !== null && admitted in value && "weight" in value
}

interface BannedWindow<Canonical extends string> {
	readonly "banned window spelling — the canonical-utterance law names the one legal form": Canonical
}

/**
 * The path-weight refusal (ruling 6 — a boundary, not a deferral): the
 * verdict names the pinned-column idiom, the same diagnostic the macro
 * expansion and the spec resolver carry.
 */
interface RefusedPath<Idiom extends string> {
	readonly "path spelling refused — the vocabulary is closed at the row": Idiom
}

type IsNegative<N extends bigint> = `${N}` extends `-${string}` ? true : false

/**
 * The dotted-name ban, both mints: a `a.b` spelling is a typed refusal
 * whose verdict names the composition idiom — pin the column with a
 * two-column containment (`Device(model, watts) <= Model(id, watts)`) and
 * name the local field.
 */
type PathBan<F extends string> = string extends F
	? unknown
	: F extends `${string}.${string}`
		? RefusedPath<"pin the column — a two-column containment (Source(ref, f) <= Catalog(id, f)) proves the local copy, then name the local field">
		: unknown

type NegativeBan<N extends bigint> = bigint extends N
	? unknown
	: IsNegative<N> extends true
		? BannedWindow<"capacity bounds are u64 — a negative bound is out of domain">
		: unknown

type FloorBan<N extends bigint> = bigint extends N
	? unknown
	: IsNegative<N> extends true
		? BannedWindow<"capacity bounds are u64 — a negative bound is out of domain">
		: N extends 0n
			? BannedWindow<"`{0..*}` is vacuous — it provably says nothing; delete the statement">
			: unknown

type RangeBan<Lo extends bigint, Hi extends bigint> = bigint extends Lo
	? unknown
	: bigint extends Hi
		? unknown
		: IsNegative<Lo> extends true
			? BannedWindow<"capacity bounds are u64 — a negative bound is out of domain">
			: IsNegative<Hi> extends true
				? BannedWindow<"capacity bounds are u64 — a negative bound is out of domain">
				: Lo extends Hi
					? Hi extends Lo
						? Lo extends 0n
							? BannedWindow<"`{0..0}` — the point window is written `{0}`: use within(0n)">
							: BannedWindow<"`{n..n}` — an exact measure is written `{n}`: use within(n)">
						: unknown
					: unknown

type UnitWindowBan<W extends CapacityWindow> = W["window"] extends {
	readonly kind: "floor"
	readonly lo: { readonly kind: "lit"; readonly value: 1n }
}
	? BannedWindow<"`{1..*}` on the unit instance says only what the bare containment says — write contained(source, target)">
	: unknown

/**
 * The C18 dimension gate's ban row, unit instance (the engine's
 * `CapacityDimensionMixing` twin, ruled 2026-07-24): a unit (count)
 * window against a `duration` bound counts facts against a span of
 * time — a dimension error. Judged on the `capacity` UNIT overload
 * only: Duration weights pair with Duration-capable bounds, so the
 * weighted overload takes the same window freely.
 */
type UnitDimensionBan<W extends CapacityWindow> = W["window"] extends {
	readonly hi: { readonly kind: "durationField" }
}
	? BannedWindow<"a count of facts bounded by a span of time mixes dimensions (C18) — weigh the source with weigh(duration(field)), or bound by a u64 field or literal">
	: unknown

type KindAt<S extends FaceSource, K extends string> =
	ProjectedShape<S, K> extends readonly [infer Kind, ...unknown[]] ? Kind : undefined

interface BoundOffTargetRoster<K, Roster> {
	readonly "dependent bound must name a field of the TARGET's own row — bound names resolve against the target's full roster": readonly [
		K,
		Roster
	]
}

interface BoundKindMismatch<K, Want> {
	readonly "dependent bound kind mismatch — ref() reads a u64 field, duration() an interval field, of the TARGET row": readonly [
		K,
		Want
	]
}

type BoundOnTarget<K extends string, Want extends "u64" | "interval", B extends AnyFace> = string extends K
	? unknown
	: K extends FaceFields<B["source"]>
		? KindAt<B["source"], K> extends Want
			? unknown
			: BoundKindMismatch<K, Want>
		: BoundOffTargetRoster<K, FaceFields<B["source"]>>

type BoundsOnTarget<W extends CapacityWindow, B extends AnyFace> = W["window"] extends {
	readonly hi: FieldRef<infer K>
}
	? BoundOnTarget<K, "u64", B>
	: W["window"] extends { readonly hi: DurationRef<infer K> }
		? BoundOnTarget<K, "interval", B>
		: unknown

interface WeightOffSourceRoster<K, Roster> {
	readonly "weight must name a field of the SOURCE's own row — the weight vocabulary is closed at the row": readonly [
		K,
		Roster
	]
}

interface WeightKindMismatch<K, Want> {
	readonly "weight kind mismatch — weigh(field) reads a u64 field, weigh(duration(field)) an interval field, of the SOURCE row": readonly [
		K,
		Want
	]
}

/**
 * The weight source wall (type tier): `weigh(K)`'s field must be a
 * u64-encoded position of the SOURCE face's own row (signed encodings are
 * the typed polarity refusal), `weigh(duration(K))`'s an interval
 * position. Checked at the `capacity` call where the source face is
 * inferred — the proven constrain-after-inference pattern.
 */
type WeightOnSource<M extends CapacityWeight, A extends AnyFace> = M["weight"] extends {
	readonly kind: "field"
	readonly field: infer K extends string
}
	? string extends K
		? unknown
		: K extends FaceFields<A["source"]>
			? KindAt<A["source"], K> extends "u64"
				? unknown
				: WeightKindMismatch<K, "u64">
			: WeightOffSourceRoster<K, FaceFields<A["source"]>>
	: M["weight"] extends { readonly kind: "durationField"; readonly field: infer K extends string }
		? string extends K
			? unknown
			: K extends FaceFields<A["source"]>
				? KindAt<A["source"], K> extends "interval"
					? unknown
					: WeightKindMismatch<K, "interval">
				: WeightOffSourceRoster<K, FaceFields<A["source"]>>
		: unknown

const unitWeight: WeightSpec = Object.freeze({ kind: "unit" })

function assertRowLocal(field: string, role: string): string {
	if (field.includes(".")) {
		throw errors.new(
			`${role} \`${field}\` walks a reference — the vocabulary is closed at the row (ruling 6): pin the column with a two-column containment (Source(ref, f) <= Catalog(id, f)) and name the local field`
		)
	}
	return field
}

function admitWindow<S extends CapacityWindowSpec>(window: S): CapacityWindow<S> {
	const value: CapacityWindow<S> = { window, [admitted]: true }
	return Object.freeze(value)
}

function lit(value: bigint): CapacityBoundSpec {
	return Object.freeze({ kind: "lit", value })
}

function within<const N extends bigint>(
	n: N & NegativeBan<N>
): CapacityWindow<{ readonly kind: "exact"; readonly n: { readonly kind: "lit"; readonly value: N } }>

function within<const Lo extends bigint>(
	lo: Lo & FloorBan<Lo>,
	hi: "*"
): CapacityWindow<{ readonly kind: "floor"; readonly lo: { readonly kind: "lit"; readonly value: Lo } }>
/**
 * `{lo..field}` / `{lo..Duration(field)}` — the dependent ceiling, read
 * from the TARGET row per group (C6: hi slot only). The ref carries no
 * value, so no inversion judgment exists at construction — a per-row
 * inverted window is the judge's typed refusal.
 */
function within<const Lo extends bigint, const R extends FieldRef | DurationRef>(
	lo: Lo & NegativeBan<Lo>,
	hi: R
): CapacityWindow<{
	readonly kind: "range"
	readonly lo: { readonly kind: "lit"; readonly value: Lo }
	readonly hi: R
}>

function within<const Lo extends bigint, const Hi extends bigint>(
	lo: Lo,
	hi: Hi & RangeBan<Lo, Hi>
): CapacityWindow<{
	readonly kind: "range"
	readonly lo: { readonly kind: "lit"; readonly value: Lo }
	readonly hi: { readonly kind: "lit"; readonly value: Hi }
}>
function within(lo: bigint, hi?: bigint | "*" | FieldRef | DurationRef): CapacityWindow {
	if (lo < 0n) {
		throw errors.new(`capacity bounds are u64: within(${lo}${hi === undefined ? "" : ", …"}) is out of domain`)
	}
	if (hi === undefined) {
		return admitWindow({ kind: "exact", n: lit(lo) })
	}
	if (hi === "*") {
		if (lo === 0n) {
			throw errors.new("the `{0..*}` window is vacuous — it provably says nothing; delete the statement")
		}
		return admitWindow({ kind: "floor", lo: lit(lo) })
	}
	if (typeof hi === "bigint") {
		if (hi < 0n) {
			throw errors.new(`capacity bounds are u64: within(${lo}, ${hi}) is out of domain`)
		}
		if (hi < lo) {
			throw errors.new(
				`the window \`{${lo}..${hi}}\` is inverted — no measure satisfies it; bounds are \`{lo..hi}\` with lo < hi (an exact measure is \`{n}\`: within(n))`
			)
		}
		if (lo === hi) {
			if (lo === 0n) {
				throw errors.new("`{0..0}` — the point window is written `{0}`: use within(0n)")
			}
			throw errors.new(`\`{${lo}..${lo}}\` — an exact measure is written \`{${lo}}\`: use within(${lo}n)`)
		}
		return admitWindow({ kind: "range", lo: lit(lo), hi: lit(hi) })
	}
	const field = assertRowLocal(hi.field, "dependent bound")
	if (hi.kind === "durationField") {
		return admitWindow({ kind: "range", lo: lit(lo), hi: Object.freeze({ kind: "durationField", field }) })
	}
	return admitWindow({ kind: "range", lo: lit(lo), hi: Object.freeze({ kind: "field", field }) })
}

function admitWeight<S extends WeightSpec>(weight: S): CapacityWeight<S> {
	const value: CapacityWeight<S> = { weight, [admitted]: true }
	return Object.freeze(value)
}

/**
 * `[field]` — the measure: a u64-encoded field of the SOURCE row summed
 * per target group. A dotted path is the typed refusal naming the
 * pinned-column idiom (ruling 6); `weigh(duration(field))` is the
 * interval-measure weight — calendar capacity as one statement.
 */
function weigh<const F extends string>(
	field: F & PathBan<F>
): CapacityWeight<{ readonly kind: "field"; readonly field: F }>
function weigh<const F extends string>(
	measure: DurationRef<F>
): CapacityWeight<{ readonly kind: "durationField"; readonly field: F }>
function weigh(measure: string | DurationRef): CapacityWeight {
	if (typeof measure === "string") {
		return admitWeight({ kind: "field", field: assertRowLocal(measure, "weight") })
	}
	return admitWeight({ kind: "durationField", field: assertRowLocal(measure.field, "weight") })
}

function ref<const F extends string>(field: F & PathBan<F>): FieldRef<F> {
	return Object.freeze({ kind: "field", field: assertRowLocal(field, "dependent bound") }) as FieldRef<F>
}

/**
 * `Duration(field)` — the interval-measure spelling, one mint for both
 * slots: handed to `weigh` it is the SOURCE row's interval measure;
 * in `within`'s hi slot it is the TARGET row's interval-measure bound
 * (Duration weights pair with Duration-capable bounds — C18).
 */
function duration<const F extends string>(field: F & PathBan<F>): DurationRef<F> {
	return Object.freeze({ kind: "durationField", field: assertRowLocal(field, "Duration measure") }) as DurationRef<F>
}

export type {
	BoundsOnTarget,
	CapacityWeight,
	CapacityWindow,
	DurationRef,
	FieldRef,
	UnitDimensionBan,
	UnitWindowBan,
	WeightOnSource
}
export { duration, isCapacityWeight, isCapacityWindow, ref, unitWeight, weigh, within }
