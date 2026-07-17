/**
 * Nominal branding — the TS analog of the Rust macro's host newtypes
 * (`docs/architecture/10-data-model.md`, the nominal-safety layer). A brand
 * is a phantom: it exists only in the type, tsc polices the wall exactly as
 * rustc polices newtype domains, and nothing is allocated or wrapped at
 * runtime.
 */

import * as errors from "@superbuilders/errors"

/**
 * The brand-minting guard behind {@link span} — the one nominal step, as a
 * type guard carrying the interval's REAL invariant (`start < end`, the
 * same check Rust's `Interval::new` runs): a value that passes IS a legal
 * interval of any brand, exactly as a Rust newtype wraps a checked
 * `Interval<T>` at construction.
 */
function isNonemptyInterval<Name extends string>(value: IntervalValue): value is Interval<Name> {
	return value.start < value.end
}

/**
 * The brand key. A real runtime symbol (so modules can import it without a
 * `declare`-only lie), but no branded value ever carries the property — the
 * brand is purely a typing device.
 */
const brand: unique symbol = Symbol("bumbledb.brand")

/**
 * The phantom-value key used by field values, field references, and faces
 * to carry their value type without any runtime representation.
 */
const phantom: unique symbol = Symbol("bumbledb.phantom")

/**
 * A branded scalar: `T` walled off under the literal name `Name`. A
 * `Brand<bigint, "HolderId">` is not assignable where a
 * `Brand<bigint, "AccountId">` is expected — the Rust newtype wall,
 * verbatim. Scalars brand as `bigint` (u64/i64), `Uint8Array` (bytes), and
 * whole interval objects (`Interval<Name>`); `bool` and `str` take no
 * newtype, exactly as the macro's `as` grammar refuses them.
 */
type Brand<T, Name extends string> = T & { readonly [brand]: Name }

/**
 * A half-open interval `[start, end)` as a plain value object. The ray is
 * representable (`end` = the element domain's MAX_END); widths and
 * signedness are NOT modeled here — the engine judges widths at the typed
 * write boundary, the brand blocks cross-field assignment, and nothing
 * else is TS's business. Interval newtypes derive no order (the Rust
 * refusal, `docs/architecture/10-data-model.md`), so no comparators exist.
 */
interface IntervalValue {
	readonly start: bigint
	readonly end: bigint
}

/**
 * A branded interval: the whole `{ start, end }` object walled under
 * `Name` — the `interval<i64> as ActiveDuring` analog.
 */
type Interval<Name extends string> = Brand<IntervalValue, Name>

/**
 * Constructs an interval literal — the `start..end` spelling. Half-open
 * and nonempty by construction: `start >= end` is a typed construction
 * error (parse, don't validate — the same invariant Rust's
 * `Interval::new` enforces at the host boundary). The default `never`
 * brand makes a fresh literal assignable to any interval field or brand,
 * the wrap-at-construction idiom; pass the brand explicitly
 * (`span<"ActiveDuring">(0n, 10n)`) to pin it.
 */
function span<Name extends string = never>(start: bigint, end: bigint): Interval<Name> {
	const value: IntervalValue = Object.freeze({ start, end })
	if (!isNonemptyInterval<Name>(value)) {
		throw errors.new(`interval is half-open and nonempty: start must be < end (got ${start}..${end})`)
	}
	return value
}

export type { Brand, Interval, IntervalValue }
export { brand, phantom, span }
