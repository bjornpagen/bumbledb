export module bumbledb:name;

import std;

export namespace bdb {

/**
 * The NTTP string mint for reflective surfaces
 * (`bdb::relation<"Service", ...>`). Lives in a GCC-only partition: the
 * literal constructor must take `char const (&)[M]` — the one spelling a
 * string-literal template argument can deduce through — and that
 * spelling is unrepresentable under the lint graph's C-array ban.
 */
template<std::size_t N>
struct fixed_string {
	std::array<char, N> chars{};

	template<std::size_t M>
	    requires(M == N + 1)
	consteval fixed_string(char const (&text)[M]) {
		std::ranges::copy_n(std::ranges::begin(text), N, chars.begin());
	}

	[[nodiscard]] constexpr auto view() const -> std::string_view {
		return std::string_view{chars.data(), N};
	}

	/**
	 * Member, not hidden-friend: the pinned GCC 16.1 ICEs streaming a
	 * defaulted friend operator== across a module import.
	 */
	[[nodiscard]] constexpr auto operator==(fixed_string const&) const -> bool = default;
};

template<std::size_t M>
fixed_string(char const (&)[M]) -> fixed_string<M - 1>;

/**
 * Capacity of one reflected name inside a coordinate. Names are stored
 * inline in a fixed buffer — not as a view — so coordinates stay
 * structural (NTTP-usable) literal types; string_view is not structural
 * on the pinned toolchain.
 */
inline constexpr std::size_t max_name_length = 64;

/**
 * Inline compile-time name storage: the structural carrier behind the
 * coordinate name hooks. It is a coordinate's NTTP identity, so the
 * buffer is always zero-padded past `length` — equal names are equal
 * values.
 */
struct name_text {
	std::array<char, max_name_length> chars{};
	std::size_t length{};

	[[nodiscard]] constexpr auto view() const -> std::string_view {
		return std::string_view{chars.data(), length};
	}

	/**
	 * Member, not hidden-friend: the pinned GCC 16.1 ICEs streaming a
	 * defaulted friend operator== across a module import.
	 */
	[[nodiscard]] constexpr auto operator==(name_text const&) const -> bool = default;
};

/**
 * The wire-name-override annotation's tag; `bdb::named` mints the
 * annotation objects.
 */
struct NameTag {
	name_text name;
};

}

export namespace bdb::detail {

/**
 * Never defined: reaching this makes an over-long reflected name a
 * compile error whose diagnostic carries the function name (the
 * :interval static-failure convention). A contract_assert cannot serve
 * here: the pinned GCC 16.1 rejects contract conditions inside the
 * class-scope consteval injection context as non-constant.
 */
auto reflected_name_must_fit_max_name_length() -> void;

[[nodiscard]] consteval auto to_name_text(std::string_view text) -> name_text {
	if (text.size() > max_name_length) {
		reflected_name_must_fit_max_name_length();
	}
	auto result = name_text{};
	std::ranges::copy(text, result.chars.begin());
	result.length = text.size();
	return result;
}

/* PIN(ubsan-constexpr-string): the iterator-pair constructor folds under the sanitizer graphs; every synthesized spec name routes through this funnel */
[[nodiscard]] consteval auto spec_name(std::string_view text) -> std::string {
	return std::string(text.begin(), text.end());
}

}

export namespace bdb {

/**
 * The wire-name override: `[[=bdb::named<"operator">]]` on a row field
 * spells the cross-host wire name when it is not a legal C++ identifier
 * (some cookbook wire names are C++ keywords). The facade member keeps
 * the C++ identifier; every wire surface reads the override.
 */
template<fixed_string Name>
inline constexpr auto named = NameTag{detail::to_name_text(Name.view())};

}
