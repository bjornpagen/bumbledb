export module bumbledb:allen;

import std;
import :interval;

namespace bdb::detail {

/**
 * Never defined: reaching this in the consteval factory makes the
 * invalid constant a compile error whose diagnostic carries the function
 * name (the :interval static-failure convention).
 */
auto allen_mask_literal_must_fit_the_low_13_bits() -> void;

}

export namespace bdb {

/**
 * A set of Allen basic relations: a checked 13-bit mask, bit i = basic i
 * in the engine's palindromic bit order — before, meets, overlaps,
 * starts, during, finishes, equals, finished-by, contains, started-by,
 * overlapped-by, met-by, after (crates/bumbledb-theory/src/allen.rs; the
 * bit order is a specified representation). Query-literal operator
 * vocabulary only — never a row field type, never a bind value
 * (75-cpp-lowering.md §1.6).
 */
class allen_mask {
	std::uint16_t bits_;

	constexpr explicit allen_mask(std::uint16_t word) : bits_{word} {}

public:
	static constexpr auto all_bits = std::uint16_t{(1U << 13U) - 1U};

	/**
	 * The constant lane: a bit above the low 13 is a compile error.
	 */
	[[nodiscard]] static consteval auto literal(std::uint16_t word) -> allen_mask {
		if ((word & ~all_bits) != 0) {
			detail::allen_mask_literal_must_fit_the_low_13_bits();
		}
		return allen_mask{word};
	}

	/**
	 * The runtime lane: a bit above the low 13 is a typed error.
	 */
	[[nodiscard]] static constexpr auto make(std::uint16_t word) -> std::expected<allen_mask, TypeError> {
		if ((word & ~all_bits) != 0) {
			return std::unexpected{TypeError::AllenMaskOverflow};
		}
		return allen_mask{word};
	}

	/**
	 * The raw 13-bit word — what crosses the ABI as
	 * the comparison operator's literal mask word.
	 */
	[[nodiscard]] constexpr auto bits() const -> std::uint16_t {
		return bits_;
	}

	/**
	 * Mask union; preserves the low-13 invariant by construction.
	 */
	[[nodiscard]] friend constexpr auto operator|(allen_mask left, allen_mask right) -> allen_mask {
		return allen_mask{static_cast<std::uint16_t>(left.bits_ | right.bits_)};
	}

	/**
	 * Member, not hidden-friend: the production GCC ICEs streaming a
	 * defaulted friend operator== across a module import.
	 */
	[[nodiscard]] constexpr auto operator==(allen_mask const&) const -> bool = default;
};

/**
 * The engine's Allen vocabulary, under Allen's own names — the 13
 * singletons in bit order plus the engine's named compositions
 * (crates/bumbledb-theory/src/allen.rs).
 */
namespace allen {

inline constexpr auto before = allen_mask::literal(1U << 0U);
inline constexpr auto meets = allen_mask::literal(1U << 1U);
inline constexpr auto overlaps = allen_mask::literal(1U << 2U);
inline constexpr auto starts = allen_mask::literal(1U << 3U);
inline constexpr auto during = allen_mask::literal(1U << 4U);
inline constexpr auto finishes = allen_mask::literal(1U << 5U);
inline constexpr auto equals = allen_mask::literal(1U << 6U);
inline constexpr auto finished_by = allen_mask::literal(1U << 7U);
inline constexpr auto contains = allen_mask::literal(1U << 8U);
inline constexpr auto started_by = allen_mask::literal(1U << 9U);
inline constexpr auto overlapped_by = allen_mask::literal(1U << 10U);
inline constexpr auto met_by = allen_mask::literal(1U << 11U);
inline constexpr auto after = allen_mask::literal(1U << 12U);

/**
 * The point-sets share a point — under half-open intervals, meets shares
 * no point.
 */
inline constexpr auto intersects = overlaps | starts | during | finishes | equals | finished_by | contains | started_by | overlapped_by;

/**
 * Point-set superset.
 */
inline constexpr auto covers = equals | contains | started_by | finished_by;

/**
 * Point-set subset — covers' converse.
 */
inline constexpr auto covered_by = equals | during | starts | finishes;

/**
 * The point-sets share no point — the pointwise key judgment's per-pair
 * statement.
 */
inline constexpr auto disjoint = before | meets | met_by | after;

/**
 * All 13 basics — a value of the algebra; vacuous as a condition (the
 * query boundary rejects it, engine-side).
 */
inline constexpr auto full = allen_mask::literal(allen_mask::all_bits);

/**
 * No basic — likewise a value, vacuous as a condition.
 */
inline constexpr auto empty = allen_mask::literal(0U);

}

}
