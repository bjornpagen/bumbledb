// The untyped answers carrier (TODO_CPP §22–§23, §36): mint, emptiness,
// bounds-checked cell access, clear, and move semantics leaving the source
// inert. Nothing executes into the carrier until the query phase, so this
// test proves exactly the resource/lifetime half. Reflection-free: part of
// BOTH graphs.
import std;
import bumbledb;

namespace {

struct CaseResult {
	std::string_view name;
	bool passed;
};

auto check_fresh_carrier_is_empty() -> CaseResult {
	auto const answers = bdb::AnswersRaw{};
	return CaseResult{
	    .name = "a fresh carrier is alive with len 0 and arity 0",
	    .passed = answers.alive() && answers.len() == 0 && answers.arity() == 0,
	};
}

auto check_cell_is_bounds_checked() -> CaseResult {
	auto const answers = bdb::AnswersRaw{};
	auto const out_of_range = answers.cell(bdb::Cell{.row = 0, .column = 0});
	return CaseResult{
	    .name = "cell() past len/arity is nullopt, never a panic (§22)",
	    .passed = !out_of_range.has_value(),
	};
}

auto check_clear_keeps_the_carrier_usable() -> CaseResult {
	auto answers = bdb::AnswersRaw{};
	answers.clear();
	return CaseResult{
	    .name = "clear() on an empty carrier is a no-op that keeps it alive",
	    .passed = answers.alive() && answers.len() == 0,
	};
}

auto check_move_leaves_the_source_inert() -> CaseResult {
	auto source = bdb::AnswersRaw{};
	auto target = std::move(source);
	auto const source_inert =
	    !source.alive() && source.len() == 0 && source.arity() == 0 && !source.cell(bdb::Cell{.row = 0, .column = 0}).has_value();
	return CaseResult{
	    .name = "moving AnswersRaw leaves the source inert and valid (§36)",
	    .passed = source_inert && target.alive(),
	};
}

auto check_move_assign_releases_and_adopts() -> CaseResult {
	auto first = bdb::AnswersRaw{};
	auto second = bdb::AnswersRaw{};
	first = std::move(second);
	return CaseResult{
	    .name = "move-assign releases the old carrier and adopts the new",
	    .passed = first.alive() && !second.alive(),
	};
}

} // namespace

auto main() -> int {
	auto const results = std::array{
	    check_fresh_carrier_is_empty(),       check_cell_is_bounds_checked(),          check_clear_keeps_the_carrier_usable(),
	    check_move_leaves_the_source_inert(), check_move_assign_releases_and_adopts(),
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
