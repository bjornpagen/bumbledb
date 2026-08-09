export module bumbledb:interval;

import std;

namespace bdb::detail {

/**
 * Never defined: reaching one of these in a consteval factory makes the
 * invalid constant a compile error whose diagnostic carries the function
 * name. Not a contract_assert because the pinned Clang 22 lint frontend
 * has no C++26 contracts and this vocabulary builds under both graphs;
 * revisit when the pinned Clang gains P2900.
 */
auto interval_literal_must_satisfy_lo_less_than_hi() -> void;
auto interval_literal_must_match_the_declared_width() -> void;

}

export namespace bdb {

/**
 * Recoverable construction failure of a checked vocabulary value — the
 * runtime `make` lanes' error type. Pre-engine: these never reach the
 * bridge.
 */
enum class TypeError : std::uint8_t {
	/**
	 * interval: half-open [lo, hi) requires lo < hi.
	 */
	EmptyInterval,
	/**
	 * fixed-width interval: hi - lo must equal the declared width.
	 */
	IntervalWidth,
	/**
	 * allen_mask: bits above the low 13 are unrepresentable.
	 */
	AllenMaskOverflow,
};

/**
 * The two interval element domains the engine's Value roster carries
 * (IntervalU64 / IntervalI64). Exact representation is the requirement.
 */
template<class T>
concept IntervalElement = std::same_as<T, std::uint64_t> || std::same_as<T, std::int64_t>;

/**
 * A checked half-open interval [lo, hi), strictly lo < hi — the C++ twin
 * of the engine's `Interval`.
 *
 * `Width` is the fixed-width family label (75-cpp-lowering.md §1.8):
 * 0 is the general interval; a nonzero width makes `hi - lo == Width`
 * part of the TYPE — a wrong-width value is unconstructible host-side
 * (the engine re-judges at commit either way) — and is a fingerprint
 * input carried by the field's ValueType.
 */
template<IntervalElement T, std::uint64_t Width = 0>
class interval {
	T lo_;
	T hi_;

	constexpr interval(T lo, T hi) : lo_{lo}, hi_{hi} {}

	[[nodiscard]] static constexpr auto width_holds(T lo, T hi) -> bool {
		if constexpr (Width == 0) {
			return true;
		} else {
			return static_cast<std::uint64_t>(hi) - static_cast<std::uint64_t>(lo) == Width;
		}
	}

public:
	static constexpr std::uint64_t width = Width;

	/**
	 * The constant lane: an invalid literal is a compile error.
	 */
	[[nodiscard]] static consteval auto literal(T lo, T hi) -> interval {
		if (!(lo < hi)) {
			detail::interval_literal_must_satisfy_lo_less_than_hi();
		}
		if (!width_holds(lo, hi)) {
			detail::interval_literal_must_match_the_declared_width();
		}
		return interval{lo, hi};
	}

	/**
	 * The runtime lane: an invalid pair is a typed recoverable error.
	 */
	[[nodiscard]] static constexpr auto make(T lo, T hi) -> std::expected<interval, TypeError> {
		if (!(lo < hi)) {
			return std::unexpected{TypeError::EmptyInterval};
		}
		if (!width_holds(lo, hi)) {
			return std::unexpected{TypeError::IntervalWidth};
		}
		return interval{lo, hi};
	}

	[[nodiscard]] constexpr auto lo() const -> T {
		return lo_;
	}

	[[nodiscard]] constexpr auto hi() const -> T {
		return hi_;
	}

	/**
	 * Member, not hidden-friend: the pinned GCC 16.1 ICEs streaming a
	 * defaulted friend operator== across a module import.
	 */
	[[nodiscard]] constexpr auto operator==(interval const&) const -> bool = default;
};

}
