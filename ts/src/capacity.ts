import { AuthoringError } from "#errors.ts"
/** Structural capacity measures and windows. Constructors and generated
 * descriptions share the same checked representation. Bounds are u64;
 * dependent bounds read the target row, measures read the source row. */

import type { AnyFace, FaceFields, FaceSource, ProjectedShape } from "#face.ts"
import type { CapacityBoundSpec, CapacityWindowSpec, WeightSpec } from "#spec.ts"
import { integerValue, recordValue } from "#values.ts"

type CapacityWindow<S extends CapacityWindowSpec = CapacityWindowSpec> = S

type CapacityWeight<S extends WeightSpec = WeightSpec> = S

interface FieldRef<F extends string = string> {
	readonly kind: "field"
	readonly field: F
}

interface DurationRef<F extends string = string> {
	readonly kind: "durationField"
	readonly field: F
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

/**
 * The spelling-ban tables are DELETED: `{n..n}`, `{0..0}`,
 * unit floors `{1..*}`/`{N..*}` and the vacuous `{0..*}` are harmless
 * equivalent spellings that lower to the one canonical `(lo, hi)` law at
 * the mint. Genuinely different semantics still refuse: negative bounds
 * (out of the u64 domain) and inverted literal bounds.
 */
type FloorBan<N extends bigint> = NegativeBan<N>

type RangeBan<Lo extends bigint, Hi extends bigint> = bigint extends Lo
	? unknown
	: bigint extends Hi
		? unknown
		: IsNegative<Lo> extends true
			? BannedWindow<"capacity bounds are u64 — a negative bound is out of domain">
			: IsNegative<Hi> extends true
				? BannedWindow<"capacity bounds are u64 — a negative bound is out of domain">
				: unknown

/**
 * The C18 dimension gate's ban row, unit instance (the engine's
 * `CapacityDimensionMixing` twin, ruled 2026-07-24): a unit (count)
 * window against a `duration` bound counts facts against a span of
 * time — a dimension error. Judged on the `capacity` UNIT overload
 * only: Duration weights pair with Duration-capable bounds, so the
 * weighted overload takes the same window freely.
 */
type UnitDimensionBan<W extends CapacityWindow> = W extends {
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
	: K extends FaceFields<B["owner"]>
		? KindAt<B["owner"], K> extends Want
			? unknown
			: BoundKindMismatch<K, Want>
		: BoundOffTargetRoster<K, FaceFields<B["owner"]>>

type BoundsOnTarget<W extends CapacityWindow, B extends AnyFace> = W extends {
	readonly hi: FieldRef<infer K>
}
	? BoundOnTarget<K, "u64", B>
	: W extends { readonly hi: DurationRef<infer K> }
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
type WeightOnSource<M extends CapacityWeight, A extends AnyFace> = M extends {
	readonly kind: "field"
	readonly field: infer K extends string
}
	? string extends K
		? unknown
		: K extends FaceFields<A["owner"]>
			? KindAt<A["owner"], K> extends "u64"
				? unknown
				: WeightKindMismatch<K, "u64">
			: WeightOffSourceRoster<K, FaceFields<A["owner"]>>
	: M extends { readonly kind: "durationField"; readonly field: infer K extends string }
		? string extends K
			? unknown
			: K extends FaceFields<A["owner"]>
				? KindAt<A["owner"], K> extends "interval"
					? unknown
					: WeightKindMismatch<K, "interval">
				: WeightOffSourceRoster<K, FaceFields<A["owner"]>>
		: unknown

const unitWeight: WeightSpec = Object.freeze({ kind: "unit" })

function assertRowLocal<F extends string>(field: F, role: string): F {
	if (typeof field !== "string" || !field.isWellFormed())
		throw new AuthoringError({ message: `${role}: expected a well-formed field name` })
	if (field.includes(".")) {
		throw new AuthoringError({
			message: `${role} \`${field}\` walks a reference — the vocabulary is closed at the row (ruling 6): pin the column with a two-column containment (Source(ref, f) <= Catalog(id, f)) and name the local field`
		})
	}
	return field
}

function admitWindow<S extends CapacityWindowSpec>(window: S): CapacityWindow<S> {
	capacityWindow(window)
	return Object.freeze(window)
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
): CapacityWindow<
	| {
			readonly kind: "range"
			readonly lo: { readonly kind: "lit"; readonly value: Lo }
			readonly hi: { readonly kind: "lit"; readonly value: Hi }
	  }
	| { readonly kind: "exact"; readonly n: { readonly kind: "lit"; readonly value: Lo } }
>
function within(lo: bigint, hi?: bigint | "*" | FieldRef | DurationRef): CapacityWindow {
	integerValue("capacity lower bound", "u64", lo)
	if (typeof hi === "bigint") integerValue("capacity upper bound", "u64", hi)
	if (lo < 0n) {
		throw new AuthoringError({
			message: `capacity bounds are u64: within(${lo}${hi === undefined ? "" : ", …"}) is out of domain`
		})
	}
	if (hi === undefined) {
		return admitWindow({ kind: "exact", n: lit(lo) })
	}
	if (hi === "*") {
		// `{0..*}` (vacuous) and `{N..*}` floors are ACCEPTED canonical laws
		// (C01): normalization preserves authored statement attribution
		// instead of policing the spelling.
		return admitWindow({ kind: "floor", lo: lit(lo) })
	}
	if (typeof hi === "bigint") {
		if (hi < 0n) {
			throw new AuthoringError({ message: `capacity bounds are u64: within(${lo}, ${hi}) is out of domain` })
		}
		if (hi < lo) {
			throw new AuthoringError({
				message: `the window \`{${lo}..${hi}}\` is inverted — no measure satisfies it; bounds are \`{lo..hi}\` with lo <= hi`
			})
		}
		if (lo === hi) {
			// `{n..n}` and `{0..0}` lower to the one canonical exact law.
			return admitWindow({ kind: "exact", n: lit(lo) })
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
	capacityWeight(weight)
	return Object.freeze(weight)
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
	return Object.freeze({ kind: "field", field: assertRowLocal(field, "dependent bound") })
}

/**
 * `Duration(field)` — the interval-measure spelling, one mint for both
 * slots: handed to `weigh` it is the SOURCE row's interval measure;
 * in `within`'s hi slot it is the TARGET row's interval-measure bound
 * (Duration weights pair with Duration-capable bounds — C18).
 */
function duration<const F extends string>(field: F & PathBan<F>): DurationRef<F> {
	return Object.freeze({ kind: "durationField", field: assertRowLocal(field, "Duration measure") })
}

export type { BoundsOnTarget, CapacityWeight, CapacityWindow, DurationRef, FieldRef, UnitDimensionBan, WeightOnSource }

/** Read a checked bound without retaining a caller-owned record. */
function capacityBound(input: unknown): CapacityBoundSpec {
	const kind = typeof input === "object" && input !== null && "kind" in input ? input.kind : undefined
	if (kind === "lit") {
		const bound = recordValue("capacity bound", input, ["kind", "value"])
		return Object.freeze({ kind, value: integerValue("capacity bound", "u64", bound.value) })
	}
	if (kind === "field" || kind === "durationField") {
		const bound = recordValue("capacity bound", input, ["kind", "field"])
		if (typeof bound.field !== "string") throw new AuthoringError({ message: "capacity bound: expected a field name" })
		return Object.freeze({ kind, field: assertRowLocal(bound.field, "dependent bound") })
	}
	throw new AuthoringError({ message: "capacity bound: unknown kind" })
}

function literalBound(input: unknown): CapacityBoundSpec & { readonly kind: "lit" } {
	const bound = capacityBound(input)
	if (bound.kind !== "lit")
		throw new AuthoringError({ message: "dependent bounds are allowed only at a range's upper bound" })
	return bound
}

function capacityWindow(input: unknown): CapacityWindow {
	const kind = typeof input === "object" && input !== null && "kind" in input ? input.kind : undefined
	if (kind === "exact") {
		const window = recordValue("capacity window", input, ["kind", "n"])
		return Object.freeze({ kind, n: literalBound(window.n) })
	}
	if (kind === "floor") {
		const window = recordValue("capacity window", input, ["kind", "lo"])
		return Object.freeze({ kind, lo: literalBound(window.lo) })
	}
	if (kind === "range") {
		const window = recordValue("capacity window", input, ["kind", "lo", "hi"])
		const lo = literalBound(window.lo)
		const hi = capacityBound(window.hi)
		if (hi.kind === "lit" && hi.value < lo.value) throw new AuthoringError({ message: "capacity window is inverted" })
		return Object.freeze({ kind, lo, hi })
	}
	throw new AuthoringError({ message: "capacity window: unknown kind" })
}

function capacityWeight(input: unknown): CapacityWeight {
	const kind = typeof input === "object" && input !== null && "kind" in input ? input.kind : undefined
	if (kind === "unit") {
		recordValue("capacity weight", input, ["kind"])
		return unitWeight
	}
	const weight = capacityBound(input)
	if (weight.kind === "lit") throw new AuthoringError({ message: "capacity weight: expected unit or a source field" })
	return weight
}

export { capacityWeight, capacityWindow, duration, ref, unitWeight, weigh, within }
