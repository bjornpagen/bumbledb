// :name — compile-time name storage for the reflective surfaces
// (TODO_CPP §6–§7): the NTTP string mint, the structural inline name
// carrier, and the wire-name-override annotation.
export module bumbledb:name;

import std;

export namespace bdb {

/// The NTTP string mint for reflective surfaces
/// (`bdb::relation<"Service", ...>`, later `bdb::schema<"Uptime">`).
///
/// Placement note: TODO_CPP lists fixed_string with the value vocabulary,
/// but it lives HERE because its literal constructor must take
/// `char const (&)[M]` — the one spelling a string-literal template
/// argument can deduce through — and that spelling is unrepresentable
/// under the lint graph's C-array ban. Every consumer of a fixed_string
/// NTTP is reflective (GCC-only) anyway.
template<std::size_t N>
struct fixed_string {
    std::array<char, N> chars{};

    template<std::size_t M>
        requires (M == N + 1)
    consteval fixed_string(char const (&text)[M]) {
        std::ranges::copy_n(std::ranges::begin(text), N, chars.begin());
    }

    [[nodiscard]] constexpr auto view() const -> std::string_view {
        return std::string_view{chars.data(), N};
    }

    // Member (not hidden-friend) comparison: the pinned GCC 16.1 ICEs
    // streaming a defaulted friend operator== across a module import.
    constexpr auto operator==(fixed_string const&) const -> bool = default;
};

template<std::size_t M>
fixed_string(char const (&)[M]) -> fixed_string<M - 1>;

/// Capacity of one reflected name inside a coordinate. Names are stored
/// inline in a fixed buffer — not as a view — so coordinates stay
/// structural (NTTP-usable) literal types; string_view is not structural
/// on the pinned toolchain.
inline constexpr std::size_t max_name_length = 64;

/// Inline compile-time name storage: the structural carrier behind the
/// coordinate name hooks (and a coordinate's NTTP identity, so the buffer
/// is always zero-padded past `length` — equal names are equal values).
struct name_text {
    std::array<char, max_name_length> chars{};
    std::size_t length{};

    [[nodiscard]] constexpr auto view() const -> std::string_view {
        return std::string_view{chars.data(), length};
    }

    // Member (not hidden-friend) comparison: the pinned GCC 16.1 ICEs
    // streaming a defaulted friend operator== across a module import.
    constexpr auto operator==(name_text const&) const -> bool = default;
};

/// The field-name-override annotation's tag (`[[=bdb::named<"operator">]]`):
/// some cookbook wire names are C++ keywords, so the reflected identifier
/// cannot always BE the wire name. The override names the WIRE field; the
/// facade member keeps the C++ identifier.
struct NameTag {
    name_text name;
};

} // namespace bdb

export namespace bdb::detail {

/// The consteval-failure hook for an over-long reflected name (the
/// :interval diagnostic convention: reaching a call to this
/// never-defined, non-constexpr function makes the evaluation non-constant
/// and the function's name IS the diagnostic). A contract_assert cannot
/// serve here: the pinned GCC 16.1 rejects contract conditions inside the
/// class-scope consteval injection context as non-constant.
auto reflected_name_must_fit_max_name_length() -> void;

consteval auto to_name_text(std::string_view text) -> name_text {
    if (text.size() > max_name_length) {
        reflected_name_must_fit_max_name_length();
    }
    auto result = name_text{};
    std::ranges::copy(text, result.chars.begin());
    result.length = text.size();
    return result;
}

/// The data_member_spec name payload that folds under the SANITIZER
/// graphs (pinned GCC 16.1 quirk): `std::string`'s (pointer, size)
/// constructor carries a null check that does not constant-fold against
/// ASan-instrumented storage (template parameter objects, string
/// literals, define_static_string globals) — but the iterator-pair
/// constructor folds. Every injected member name computed from a
/// name_text/derived view routes through THIS; names straight from
/// `identifier_of` (reflection-internal storage) need no detour.
consteval auto spec_name(std::string_view text) -> std::string {
    return std::string(text.begin(), text.end());
}

} // namespace bdb::detail

export namespace bdb {

/// The wire-name override annotation: `[[=bdb::named<"operator">]]` on a
/// row field spells the cross-host wire name when it is not a legal C++
/// identifier. The facade member keeps the C++ identifier; every wire
/// surface (field spec, statements, class map) reads the override.
template<fixed_string Name>
inline constexpr auto named = NameTag{detail::to_name_text(Name.view())};

} // namespace bdb
