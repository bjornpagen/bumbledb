/**
 * Cardinality-window counts — exactly five constructors, and nothing else
 * (`docs/architecture/70-api.md` § the canonical-utterance law). The five
 * constructors PARTITION the legal windows, and the ban table is enforced
 * REPRESENTATIONALLY, stronger than Rust's expansion errors, in two tiers:
 *
 * - **The type tier**: a banned spelling written as a LITERAL does not
 *   compile — `exactly(0n)`, `between(n, n)`, `between(0n, hi)`,
 *   `atLeast(0n)`, `atLeast(1n)`, `atMost(0n)`, and every negative bound
 *   are type errors naming the canonical form (`{n..n}`, `{0..0}`,
 *   `{0..hi}`-via-between, `{0..*}`, `{1..*}` have NO argument shape that
 *   produces them), and no sixth constructor exists at all.
 * - **The construction tier**: a bound the type level cannot judge — a
 *   COMPUTED `bigint`, whose literal identity is erased, or an inverted
 *   `between(lo, hi)` order, which type-level bigints cannot compare — is
 *   judged here at construction with the same canonical-naming errors; and
 *   past both tiers the engine's own spec validation remains the law for a
 *   hostile FFI caller (the standing two-tier ban enforcement).
 *
 * Bounds are `bigint` (u64 crosses as bigint always, PRD-04's law).
 */

import * as errors from "@superbuilders/errors"
import type { WindowSpec } from "#spec.ts"

/** The exclusion's one spelling, shared by `none`. */
const exclusion: WindowSpec = Object.freeze({ kind: "exact", n: 0n })

/**
 * The admission brand — a module-private symbol, deliberately unexported:
 * `WindowSpec` is a public wire type, so without this brand every banned
 * spelling in the ban table would be writable as a plain object literal
 * (`{ window: { kind: "floor", lo: 1n } }` typechecks structurally). The
 * symbol makes the five constructors the ONLY producers of a `Count`
 * value, which is what "the ban table is unwritable" means.
 */
const admitted: unique symbol = Symbol("bumbledb.count.admitted")

/**
 * An admitted window count — opaque and inert: a fact about the theory,
 * not a builder. Only the five constructors below produce one (the
 * module-private {@link admitted} brand forecloses structural literals).
 */
interface Count {
	readonly window: WindowSpec
	readonly [admitted]: true
}

/** Stamps one admitted window as a frozen `Count` value. */
function admit(window: WindowSpec): Count {
	const count: Count = { window, [admitted]: true }
	return Object.freeze(count)
}

/**
 * The legible banned-spelling verdict: intersected into a count
 * constructor's parameter when the LITERAL argument spells a banned window,
 * naming the canonical form — the compile-time face of the ban table.
 */
interface BannedWindow<Canonical extends string> {
	readonly "banned window spelling — the canonical-utterance law names the one legal form": Canonical
}

/** `true` exactly when the literal bigint `N` is negative (out of the u64 count domain). */
type IsNegative<N extends bigint> = `${N}` extends `-${string}` ? true : false

/** The ban verdict on `exactly(n)`: negatives are out of domain; `{0}` is the exclusion, written `none`. */
type ExactlyBan<N extends bigint> = bigint extends N
	? unknown
	: IsNegative<N> extends true
		? BannedWindow<"window counts are u64 — a negative count is out of domain">
		: N extends 0n
			? BannedWindow<"`{0}` is the exclusion — write none">
			: unknown

/** The ban verdict on `atLeast(lo)`: `{0..*}` is vacuous; `{1..*}` is the bare containment respelled. */
type AtLeastBan<N extends bigint> = bigint extends N
	? unknown
	: IsNegative<N> extends true
		? BannedWindow<"window counts are u64 — a negative count is out of domain">
		: N extends 0n
			? BannedWindow<"`{0..*}` is vacuous — it provably says nothing; delete the statement">
			: N extends 1n
				? BannedWindow<"`{1..*}` says only what the bare containment says — write contained(source, target)">
				: unknown

/** The ban verdict on `atMost(hi)`: `{0..0}` is the exclusion, written `none`. */
type AtMostBan<N extends bigint> = bigint extends N
	? unknown
	: IsNegative<N> extends true
		? BannedWindow<"window counts are u64 — a negative count is out of domain">
		: N extends 0n
			? BannedWindow<"`{0..0}` — the exclusion is written `{0}`: use none">
			: unknown

/** The ban verdict on a `between` floor of zero: `{0..hi}` is the ceiling respelled (`atMost(hi)`). */
type BetweenFloorBan<Lo extends bigint> = Lo extends 0n
	? BannedWindow<"`{0..hi}` — a ceiling is written atMost(hi)">
	: unknown

/**
 * The ban verdict on `between(lo, hi)`, judged on the second bound once
 * both literals are known: `{n..n}` is the exact count respelled
 * (`exactly(n)`, or `none` at 0), and `{0..hi}` is the ceiling respelled
 * (`atMost(hi)` — the five constructors PARTITION the legal windows, so
 * the one ceiling window keeps its one spelling). Bound ORDER (`{hi..lo}`
 * inverted) is not type-expressible — bigint literals have no type-level
 * comparison — so inversion stays a construction error below.
 */
type BetweenBan<Lo extends bigint, Hi extends bigint> = bigint extends Lo
	? unknown
	: bigint extends Hi
		? unknown
		: IsNegative<Lo> extends true
			? BannedWindow<"window counts are u64 — a negative bound is out of domain">
			: IsNegative<Hi> extends true
				? BannedWindow<"window counts are u64 — a negative bound is out of domain">
				: Lo extends Hi
					? Hi extends Lo
						? Lo extends 0n
							? BannedWindow<"`{0..0}` — the exclusion is written `{0}`: use none">
							: BannedWindow<"`{n..n}` — an exact count is written `{n}`: use exactly(n)">
						: BetweenFloorBan<Lo>
					: BetweenFloorBan<Lo>

/**
 * `{n}` — THE exact-count spelling, n ≥ 1. `exactly(0)` is the exclusion
 * respelled: unwritable as a literal ({@link ExactlyBan} names `none`),
 * rejected at construction when computed.
 */
function exactly<const N extends bigint>(n: N & ExactlyBan<N>): Count {
	if (n < 0n) {
		throw errors.new(`window counts are u64: exactly(${n}) is out of domain`)
	}
	if (n === 0n) {
		throw errors.new("`{0}` is the exclusion — write none")
	}
	return admit(Object.freeze({ kind: "exact", n }))
}

/** `{0}` — the exclusion: no source fact may pair with the target group. */
const none: Count = admit(exclusion)

/**
 * `{lo..hi}` — both bounds explicit, 1 ≤ lo < hi. `lo === hi` is the exact
 * count respelled and `lo === 0` is the ceiling respelled: unwritable as
 * literals ({@link BetweenBan} names `exactly(n)`, `none` at `{0..0}`, or
 * `atMost(hi)` at a zero floor — the five constructors PARTITION the legal
 * windows), rejected at construction when computed; an inverted window is
 * unsatisfiable and rejected at construction (bigint literals carry no
 * type-level order).
 */
function between<const Lo extends bigint, const Hi extends bigint>(lo: Lo, hi: Hi & BetweenBan<Lo, Hi>): Count {
	if (lo < 0n || hi < 0n) {
		throw errors.new(`window counts are u64: between(${lo}, ${hi}) is out of domain`)
	}
	if (hi < lo) {
		throw errors.new(
			`the window \`{${lo}..${hi}}\` is inverted — no count satisfies it; bounds are \`{lo..hi}\` with lo < hi (an exact count is \`{n}\`: exactly(n))`
		)
	}
	if (lo === hi) {
		if (lo === 0n) {
			throw errors.new("`{0..0}` — the exclusion is written `{0}`: use none")
		}
		throw errors.new(`\`{${lo}..${lo}}\` — an exact count is written \`{${lo}}\`: use exactly(${lo})`)
	}
	if (lo === 0n) {
		throw errors.new(`\`{0..${hi}}\` — a ceiling is written atMost: use atMost(${hi})`)
	}
	return admit(Object.freeze({ kind: "range", lo, hi }))
}

/**
 * `{lo..*}` — a floor with no ceiling, lo ≥ 2: `atLeast(1)` says only what
 * the bare containment says and `atLeast(0)` is vacuous — both unwritable
 * as literals ({@link AtLeastBan} names the canonical form), rejected at
 * construction when computed.
 */
function atLeast<const N extends bigint>(lo: N & AtLeastBan<N>): Count {
	if (lo < 0n) {
		throw errors.new(`window counts are u64: atLeast(${lo}) is out of domain`)
	}
	if (lo === 0n) {
		throw errors.new("the `{0..*}` window is vacuous — it provably says nothing; delete the statement")
	}
	if (lo === 1n) {
		throw errors.new(
			"`{1..*}` says only what the bare containment says — drop the annotation and write the containment: contained(source, target)"
		)
	}
	return admit(Object.freeze({ kind: "floor", lo }))
}

/**
 * `{0..hi}` — a ceiling, hi ≥ 1: `atMost(0)` is the exclusion respelled —
 * unwritable as a literal ({@link AtMostBan} names `none`), rejected at
 * construction when computed.
 */
function atMost<const N extends bigint>(hi: N & AtMostBan<N>): Count {
	if (hi < 0n) {
		throw errors.new(`window counts are u64: atMost(${hi}) is out of domain`)
	}
	if (hi === 0n) {
		throw errors.new("`{0..0}` — the exclusion is written `{0}`: use none")
	}
	return admit(Object.freeze({ kind: "range", lo: 0n, hi }))
}

export type { Count }
export { atLeast, atMost, between, exactly, none }
