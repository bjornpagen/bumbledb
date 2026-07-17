/**
 * Cardinality-window counts — exactly five constructors, and nothing else
 * (`docs/architecture/70-api.md` § the canonical-utterance law). The ban
 * table is enforced REPRESENTATIONALLY, stronger than Rust's expansion
 * errors: `{1..*}`, `{n..n}`, `{0..0}`, `{0..*}`, and inverted windows have
 * NO constructor — the spellings that could produce them are construction
 * errors naming the canonical form, and no other spelling exists at all.
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
	return Object.freeze({ window, [admitted]: true as const })
}

/**
 * `{n}` — THE exact-count spelling, n ≥ 1. `exactly(0)` is the exclusion
 * respelled and rejected naming `none`.
 */
function exactly(n: bigint): Count {
	if (n < 0n) {
		throw errors.new(`window counts are u64: exactly(${n}) is out of domain`)
	}
	if (n === 0n) {
		throw errors.new("`{0..0}`-shaped spelling: the exclusion is written `{0}` — use none")
	}
	return admit(Object.freeze({ kind: "exact", n }))
}

/** `{0}` — the exclusion: no source fact may pair with the target group. */
const none: Count = admit(exclusion)

/**
 * `{lo..hi}` — both bounds explicit, 0 ≤ lo < hi. `lo === hi` is the exact
 * count respelled (rejected naming `exactly(n)`, or `none` at 0); an
 * inverted window is unsatisfiable and rejected.
 */
function between(lo: bigint, hi: bigint): Count {
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
	return admit(Object.freeze({ kind: "range", lo, hi }))
}

/**
 * `{lo..*}` — a floor with no ceiling, lo ≥ 2: `atLeast(1)` says only what
 * the bare containment says (rejected naming `contained`), and
 * `atLeast(0)` is vacuous (rejected naming deletion).
 */
function atLeast(lo: bigint): Count {
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
 * `{0..hi}` — a ceiling, hi ≥ 1: `atMost(0)` is the exclusion respelled
 * and rejected naming `none`.
 */
function atMost(hi: bigint): Count {
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
