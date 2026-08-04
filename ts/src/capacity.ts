/**
 * Capacity-law mints — the window, weight, and dependent-bound vocabulary
 * the one `capacity()` statement constructor consumes
 * (`docs/architecture/70-api.md` § the canonical-utterance law, restated
 * around the aggregate form). `within()` is the ONE window spelling —
 * `within(n)` exact (`within(0n)` IS the exclusion's one spelling),
 * `within(lo, hi)` range (`within(0n, hi)` the canonical ceiling),
 * `within(lo, "*")` floor — `weigh()` names the measure on the SOURCE row
 * (`weigh("watts")` a u64 field, `weigh(duration("booked"))` an interval's
 * measure), and `ref()`/`duration()` read a dependent bound from the
 * TARGET row (hi slot only — ruled 2026-07-24, C6). The ban table is
 * enforced REPRESENTATIONALLY in two tiers, split per-aggregate where
 * weight-sensitive (design § 6: a ban is canonical-utterance policing when
 * it is weight-independent, semantic deduplication when it is not):
 *
 * - **The type tier**: a banned spelling written as a LITERAL does not
 *   compile — every negative bound, `within(n, n)`, `within(0n, 0n)`, and
 *   `within(0n, "*")` are type errors naming the canonical form. The
 *   weight-SENSITIVE row rides the `capacity()` overloads themselves:
 *   `within(1n, "*")` is banned on the unit overload only (`{1..*}` on the
 *   count instance is the bare containment respelled —
 *   `window_floor_containment`), and LEGAL on the weighted one ("positive
 *   total" is not an existence claim over rows).
 * - **The construction tier**: a bound the type level cannot judge — a
 *   COMPUTED `bigint`, or an inverted `within(lo, hi)` order — is judged at
 *   construction with the same canonical-naming errors; and past both
 *   tiers the engine's own spec validation remains the law for a hostile
 *   FFI caller (the standing two-tier ban enforcement).
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

/**
 * The admission brand — a module-private symbol, deliberately unexported
 * (the standing statement-mint pattern): `CapacityWindowSpec` and
 * `WeightSpec` are public wire types, so without this brand every banned
 * spelling in the ban table would be writable as a plain object literal.
 * The symbol makes `within()` and `weigh()` the ONLY producers of the
 * values `capacity()` accepts, which is what "the ban table is unwritable"
 * means.
 */
const admitted: unique symbol = Symbol("bumbledb.capacity.admitted")

/**
 * An admitted capacity window — opaque and inert: a fact about the theory,
 * not a builder. Only `within()` produces one; the spec is carried at its
 * EXACT type so the weight-sensitive `{1..*}` ban and the dependent-bound
 * target wall are judged at the `capacity()` call.
 */
interface CapacityWindow<S extends CapacityWindowSpec = CapacityWindowSpec> {
	readonly window: S
	readonly [admitted]: true
}

/**
 * An admitted measure — `weigh()`'s product, the `[w]` bracket of the
 * operator. The spec is carried at its exact type so the u64/interval
 * source wall ({@link WeightOnSource}) reads the weighed field's name off
 * the value.
 */
interface CapacityWeight<S extends WeightSpec = WeightSpec> {
	readonly weight: S
	readonly [admitted]: true
}

/**
 * A dependent bound naming a u64 field of the TARGET row — `ref()`'s
 * product, legal in `within()`'s hi slot only (C6: a dependent floor has
 * no use case; inversion with idents is unrepresentable). Carries no
 * value: bounds resolve per target row at judge time.
 */
interface FieldRef<F extends string = string> {
	readonly kind: "field"
	readonly field: F
}

/**
 * A dependent interval-measure bound (`Duration(field)` of the TARGET
 * row), or — handed to `weigh()` — the interval-measure weight of the
 * SOURCE row. One mint, both slots: the interval enters through the
 * measure argument, never the group key.
 */
interface DurationRef<F extends string = string> {
	readonly kind: "durationField"
	readonly field: F
}

/** Narrows any value to an admitted capacity window through the module-private brand. */
function isCapacityWindow(value: unknown): value is CapacityWindow {
	return typeof value === "object" && value !== null && admitted in value && "window" in value
}

/** Narrows any value to an admitted capacity weight through the module-private brand. */
function isCapacityWeight(value: unknown): value is CapacityWeight {
	return typeof value === "object" && value !== null && admitted in value && "weight" in value
}

/**
 * The legible banned-spelling verdict: intersected into a mint's parameter
 * when the LITERAL argument spells a banned window, naming the canonical
 * form — the compile-time face of the ban table.
 */
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

/** `true` exactly when the literal bigint `N` is negative (out of the u64 bound domain). */
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

/** The ban verdict on a single bound: negatives are out of the u64 domain. */
type NegativeBan<N extends bigint> = bigint extends N
	? unknown
	: IsNegative<N> extends true
		? BannedWindow<"capacity bounds are u64 — a negative bound is out of domain">
		: unknown

/**
 * The ban verdict on `within(lo, "*")`: `{0..*}` is vacuous —
 * weight-independent (sums are ≥ 0). `{1..*}` is NOT judged here: the ban
 * is weight-sensitive (Count-instance only) and rides the `capacity()`
 * unit overload ({@link UnitWindowBan}).
 */
type FloorBan<N extends bigint> = bigint extends N
	? unknown
	: IsNegative<N> extends true
		? BannedWindow<"capacity bounds are u64 — a negative bound is out of domain">
		: N extends 0n
			? BannedWindow<"`{0..*}` is vacuous — it provably says nothing; delete the statement">
			: unknown

/**
 * The ban verdict on `within(lo, hi)` with both literals known: `{n..n}`
 * is the exact measure respelled (`within(n)`, or `within(0n)` at 0) —
 * weight-independent canonical policing. Bound ORDER (`{hi..lo}` inverted)
 * is not type-expressible — bigint literals have no type-level comparison
 * — so inversion stays a construction error.
 */
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

/**
 * The weight-SENSITIVE ban row, judged at the `capacity()` call where the
 * aggregate instance is known (design § 6): on the UNIT overload a
 * `{1..*}` floor says only what the bare containment says
 * (`window_floor_containment`) and is banned naming `contained`; on the
 * weighted overload the same window is LEGAL — "positive total" admits
 * zero-weight rows and is a different, weaker law.
 */
type UnitWindowBan<W extends CapacityWindow> = W["window"] extends {
	readonly kind: "floor"
	readonly lo: { readonly kind: "lit"; readonly value: 1n }
}
	? BannedWindow<"`{1..*}` on the unit instance says only what the bare containment says — write contained(source, target)">
	: unknown

/**
 * The C18 dimension gate's ban row, unit instance (the engine's
 * `CapacityDimensionMixing` twin, ruled 2026-07-24): a unit (count)
 * window against a `duration()` bound counts facts against a span of
 * time — a dimension error. Judged on the `capacity()` UNIT overload
 * only: Duration weights pair with Duration-capable bounds, so the
 * weighted overload takes the same window freely.
 */
type UnitDimensionBan<W extends CapacityWindow> = W["window"] extends {
	readonly hi: { readonly kind: "durationField" }
}
	? BannedWindow<"a count of facts bounded by a span of time mixes dimensions (C18) — weigh the source with weigh(duration(field)), or bound by a u64 field or literal">
	: unknown

/** The projected kind of one field of a face's source, read off the schema type. */
type KindAt<S extends FaceSource, K extends string> =
	ProjectedShape<S, K> extends readonly [infer Kind, ...unknown[]] ? Kind : undefined

/**
 * The legible off-roster verdict for a dependent bound: bound names
 * resolve against the TARGET's full field roster (C1 — the written
 * projection tuple stays the pure grouping key), and the verdict names
 * that roster.
 */
interface BoundOffTargetRoster<K, Roster> {
	readonly "dependent bound must name a field of the TARGET's own row — bound names resolve against the target's full roster": readonly [
		K,
		Roster
	]
}

/** The legible kind-mismatch verdict for a dependent bound: `ref()` needs u64, `duration()` an interval. */
interface BoundKindMismatch<K, Want> {
	readonly "dependent bound kind mismatch — ref() reads a u64 field, duration() an interval field, of the TARGET row": readonly [
		K,
		Want
	]
}

/**
 * One dependent-bound slot judged against the target face's roster and
 * kinds. A WIDE field name (the untyped-caller path — literal identity
 * already lost) passes the type tier and is judged at construction.
 */
type BoundOnTarget<K extends string, Want extends "u64" | "interval", B extends AnyFace> = string extends K
	? unknown
	: K extends FaceFields<B["source"]>
		? KindAt<B["source"], K> extends Want
			? unknown
			: BoundKindMismatch<K, Want>
		: BoundOffTargetRoster<K, FaceFields<B["source"]>>

/**
 * The dependent-bound target wall (type tier): for a `ref(K)` bound in the
 * window's hi slot, `K` must name a u64 field of the TARGET face's own
 * row; for a `duration(K)` bound, an interval field. Literal-bound windows
 * pass untouched. The runtime twin for untyped callers lives at the
 * `capacity()` construction (`statements.ts`).
 */
type BoundsOnTarget<W extends CapacityWindow, B extends AnyFace> = W["window"] extends {
	readonly hi: FieldRef<infer K>
}
	? BoundOnTarget<K, "u64", B>
	: W["window"] extends { readonly hi: DurationRef<infer K> }
		? BoundOnTarget<K, "interval", B>
		: unknown

/** The legible off-roster verdict for a weight: the vocabulary is closed at the SOURCE row. */
interface WeightOffSourceRoster<K, Roster> {
	readonly "weight must name a field of the SOURCE's own row — the weight vocabulary is closed at the row": readonly [
		K,
		Roster
	]
}

/**
 * The legible kind-mismatch verdict for a weight: a signed weight would
 * break the polarity scheduler (an insert could lower a sum), so the
 * illegal weight is unrepresentable, not checked.
 */
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
 * position. Checked at the `capacity()` call where the source face is
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

/** The unit weight — a case, not an absence (C4): the count instance's one wire spelling. */
const unitWeight: WeightSpec = Object.freeze({ kind: "unit" })

/** The construction-tier twin of {@link PathBan}: a dotted name refuses naming the idiom. */
function assertRowLocal(field: string, role: string): string {
	if (field.includes(".")) {
		throw errors.new(
			`${role} \`${field}\` walks a reference — the vocabulary is closed at the row (ruling 6): pin the column with a two-column containment (Source(ref, f) <= Catalog(id, f)) and name the local field`
		)
	}
	return field
}

/** Stamps one admitted window spec as a frozen branded value. */
function admitWindow<S extends CapacityWindowSpec>(window: S): CapacityWindow<S> {
	const value: CapacityWindow<S> = { window, [admitted]: true }
	return Object.freeze(value)
}

/** One literal bound. */
function lit(value: bigint): CapacityBoundSpec {
	return Object.freeze({ kind: "lit", value })
}

/**
 * `{n}` — THE exact-measure spelling; `within(0n)` IS the exclusion's one
 * spelling on the unit instance, and the weaker "total is zero" law on a
 * weighted one (zero-weight rows may exist — design § 6, stated loudly).
 */
function within<const N extends bigint>(
	n: N & NegativeBan<N>
): CapacityWindow<{ readonly kind: "exact"; readonly n: { readonly kind: "lit"; readonly value: N } }>
/**
 * `{lo..*}` — a floor with no ceiling. `within(0n, "*")` is vacuous
 * (unwritable as a literal); `within(1n, "*")` constructs — the `{1..*}`
 * ban is weight-sensitive and judged at the `capacity()` call (unit
 * instance only).
 */
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
/**
 * `{lo..hi}` — both bounds explicit, lo < hi; `within(0n, hi)` is the
 * canonical ceiling. `lo === hi` is the exact measure respelled
 * (unwritable as literals — {@link RangeBan} names `within(n)`), rejected
 * at construction when computed; an inverted window is unsatisfiable and
 * rejected at construction (bigint literals carry no type-level order).
 */
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

/** Stamps one admitted weight spec as a frozen branded value. */
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

/**
 * A dependent bound by name — `within(0n, ref("supply"))` reads each
 * group's ceiling from the TARGET row's u64 field (bound names resolve
 * against the target's FULL roster, C1 — the projection tuple stays the
 * pure grouping key). Carries no value.
 */
function ref<const F extends string>(field: F & PathBan<F>): FieldRef<F> {
	return Object.freeze({ kind: "field", field: assertRowLocal(field, "dependent bound") }) as FieldRef<F>
}

/**
 * `Duration(field)` — the interval-measure spelling, one mint for both
 * slots: handed to `weigh()` it is the SOURCE row's interval measure;
 * in `within()`'s hi slot it is the TARGET row's interval-measure bound
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
