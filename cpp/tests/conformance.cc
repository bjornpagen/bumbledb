import std;

[[nodiscard]] consteval auto optional_ref_writes_through() -> bool {
	int slot = 1;
	std::optional<int&> ref{slot};
	ref.value() = 5;
	return slot == 5 && &ref.value() == &slot && !std::optional<int&>{}.has_value();
}

static_assert(optional_ref_writes_through());

/**
 * try_push_back's std::optional<T&> return (not P0843R14's T*) is
 * conforming, not a GCC quirk: P3981R2 (adopted 2026-03) changed it and
 * libstdc++ tracks the working draft.
 */
[[nodiscard]] consteval auto inplace_vector_bounded_push() -> bool {
	std::inplace_vector<int, 3> v;
	v.push_back(1);
	v.push_back(2);
	v.push_back(3);
	return v.size() == 3 && v.try_push_back(4) == std::nullopt && v[0] == 1 && v[2] == 3 && decltype(v)::capacity() == 3;
}

static_assert(inplace_vector_bounded_push());

[[nodiscard]] consteval auto expected_monadic_chain() -> bool {
	using E = std::expected<int, char>;
	auto const good = E{20}
	                      .and_then([](int v) -> E {
		                      return E{v + 1};
	                      })
	                      .transform([](int v) {
		                      return v * 2;
	                      })
	                      .or_else([](char) -> E {
		                      return E{0};
	                      });
	auto const bad = E{std::unexpect, 'e'}
	                     .transform([](int v) {
		                     return v + 1;
	                     })
	                     .or_else([](char c) -> E {
		                     return E{std::unexpect, static_cast<char>(c + 1)};
	                     });
	return good == E{42} && bad == std::unexpected('f');
}

static_assert(expected_monadic_chain());

namespace {

enum class Compass { North, East, South, West };

[[nodiscard]] consteval auto named_enumerator_count() -> std::size_t {
	auto count = std::size_t{0};
	template for (constexpr auto enumerator : std::define_static_array(std::meta::enumerators_of(^^Compass))) {
		if (!std::meta::identifier_of(enumerator).empty()) {
			++count;
		}
	}
	return count;
}

[[nodiscard]] consteval auto first_enumerator_is_north() -> bool {
	template for (constexpr auto enumerator : std::define_static_array(std::meta::enumerators_of(^^Compass))) {
		return std::meta::identifier_of(enumerator) == "North";
	}
	return false;
}

}

static_assert(named_enumerator_count() == 4);
static_assert(first_enumerator_is_north());

namespace {

struct CaseResult {
	std::string_view name;
	bool passed;
};

[[nodiscard]] auto check_function_ref_binds_lambda() -> CaseResult {
	int captured = 3;
	auto const add_captured = [&captured](int x) {
		return captured + x;
	};
	std::function_ref<int(int)> const ref{add_captured};
	captured = 30;
	return CaseResult{
	    .name = "function_ref binds a capturing lambda by reference",
	    .passed = ref(4) == 34,
	};
}

[[nodiscard]] auto check_indirect_value_semantics() -> CaseResult {
	std::indirect<int> const original{11};
	auto copy = original;
	*copy += 1;
	auto const moved = std::move(copy);
	return CaseResult{
	    .name = "indirect construction, deep copy, and move are value-shaped",
	    .passed = *original == 11 && *moved == 12 && copy.valueless_after_move(),
	};
}

}

auto main() -> int {
	auto const results = std::array{
	    check_function_ref_binds_lambda(),
	    check_indirect_value_semantics(),
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
