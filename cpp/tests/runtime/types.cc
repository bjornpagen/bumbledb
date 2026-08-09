import std;
import bumbledb;

namespace {

struct CaseResult {
	std::string_view name;
	bool passed;
};

constexpr auto lo_bound = std::int64_t{-5};
constexpr auto hi_bound = std::int64_t{9};
constexpr auto byte_width = std::size_t{8};
constexpr auto equals_bit = 6U;
constexpr auto after_bit = 12U;

constexpr auto window = bdb::interval<std::int64_t>::literal(lo_bound, hi_bound);
static_assert(window.lo() == lo_bound);
static_assert(window.hi() == hi_bound);
static_assert(bdb::interval<std::uint64_t>::literal(1, 2) == bdb::interval<std::uint64_t>::literal(1, 2));

static_assert(std::same_as<bdb::bytes<byte_width>, std::array<std::byte, byte_width>>);

static_assert(std::same_as<decltype(bdb::fresh), bdb::FreshTag const>);

static_assert(bdb::allen::before.bits() == (1U << 0U));
static_assert(bdb::allen::equals.bits() == (1U << equals_bit));
static_assert(bdb::allen::after.bits() == (1U << after_bit));
static_assert(bdb::allen::full.bits() == bdb::allen_mask::all_bits);
static_assert(bdb::allen::empty.bits() == 0U);
static_assert(bdb::allen::disjoint == (bdb::allen::before | bdb::allen::meets | bdb::allen::met_by | bdb::allen::after));
static_assert((bdb::allen::intersects | bdb::allen::disjoint) == bdb::allen::full);
static_assert((bdb::allen::covers | bdb::allen::covered_by).bits() ==
              (bdb::allen::equals | bdb::allen::contains | bdb::allen::started_by | bdb::allen::finished_by | bdb::allen::during |
               bdb::allen::starts | bdb::allen::finishes)
                  .bits());

[[nodiscard]] auto check_interval_make_accepts_ordered_bounds() -> CaseResult {
	auto const made = bdb::interval<std::int64_t>::make(lo_bound, hi_bound);
	return CaseResult{
	    .name = "interval::make accepts lo < hi and exposes lo()/hi()",
	    .passed = made.has_value() && made->lo() == lo_bound && made->hi() == hi_bound,
	};
}

[[nodiscard]] auto check_interval_make_rejects_empty() -> CaseResult {
	auto const bound = std::uint64_t{9};
	auto const made = bdb::interval<std::uint64_t>::make(bound, bound);
	return CaseResult{
	    .name = "interval::make rejects lo >= hi with EmptyInterval",
	    .passed = !made.has_value() && made.error() == bdb::TypeError::EmptyInterval,
	};
}

[[nodiscard]] auto check_allen_make_accepts_low_13_bits() -> CaseResult {
	auto const made = bdb::allen_mask::make(bdb::allen_mask::all_bits);
	return CaseResult{
	    .name = "allen_mask::make accepts the full 13-bit word",
	    .passed = made.has_value() && *made == bdb::allen::full,
	};
}

[[nodiscard]] auto check_allen_make_rejects_high_bits() -> CaseResult {
	auto const first_bit_above_the_mask = std::uint16_t{bdb::allen_mask::all_bits + 1U};
	auto const made = bdb::allen_mask::make(first_bit_above_the_mask);
	return CaseResult{
	    .name = "allen_mask::make rejects bit 13 with AllenMaskOverflow",
	    .passed = !made.has_value() && made.error() == bdb::TypeError::AllenMaskOverflow,
	};
}

}

auto main() -> int {
	auto const results = std::array{
	    check_interval_make_accepts_ordered_bounds(),
	    check_interval_make_rejects_empty(),
	    check_allen_make_accepts_low_13_bits(),
	    check_allen_make_rejects_high_bits(),
	};

	auto failures = std::size_t{0};
	for (auto const& result : results) {
		if (result.passed) {
			std::println("pass: {}", result.name);
		} else {
			std::println("FAIL: {}", result.name);
			++failures;
		}
	}
	return failures == 0 ? 0 : 1;
}
