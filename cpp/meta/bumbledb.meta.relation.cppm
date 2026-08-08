// bumbledb.meta.relation — the C++26 relation reflector (TODO_CPP §6–§7).
//
// GCC-only reflection zone: this module derives everything from the row
// declaration itself — field names, declaration order, physical types,
// fresh annotations — and synthesizes the coordinate facade
// (`bdb::relation<"Service", ServiceRow>` yielding `Service.id`,
// `Service.name`) via the proven class-template-scope `define_aggregate`
// injection pattern (TODO_CPP §38). Reflection metadata is the sole field
// source of truth; there is no parallel field list anywhere.
//
// Diagnostics are product (TODO_CPP §34): an unsupported field type or a
// misplaced fresh mark fails compilation with a static_assert message that
// names the relation, the field, and the offending type. The compile-fail
// suite pins those messages.
export module bumbledb.meta.relation;

import std;
import bumbledb.types;

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

/// The structural ValueType classification of one row field — the C++
/// image of the engine's closed value roster (TODO_CPP §7).
enum class value_kind : std::uint8_t {
    boolean,
    u64,
    i64,
    string,
    fixed_bytes,
    interval_u64,
    interval_i64,
};

/// A field's classification: the kind plus the FixedBytes length (the
/// length IS part of the type and a fingerprint input; 0 elsewhere).
struct field_class {
    value_kind kind;
    std::uint16_t fixed_len;

    constexpr auto operator==(field_class const&) const -> bool = default;
};

/// One compile-time semantic coordinate (`Service.id`): relation name,
/// field name, ordinal, structural kind, and fresh mark, synthesized from
/// the reflected row declaration. Not a runtime field value.
///
/// The identity lives in the TYPE (every datum is an NTTP): two facade
/// members are two distinct coordinate types, which is what lets the
/// statement algebra (bumbledb.meta.schema) carry projections, run the
/// class laws, and render §34 diagnostics naming semantic coordinates
/// entirely at compile time. Values of this type are empty structural
/// literals — coordinates stay NTTP-friendly by design.
template<class T, name_text RelationName, name_text FieldName,
    std::size_t Ordinal, field_class Class, bool Fresh>
struct coord {
    using value_type = T;

    static constexpr name_text relation_name = RelationName;
    static constexpr name_text field_name = FieldName;
    static constexpr std::size_t ordinal = Ordinal;
    static constexpr value_kind kind = Class.kind;
    static constexpr std::uint16_t fixed_len = Class.fixed_len;
    static constexpr bool fresh = Fresh;

    /// Name hooks (the to-string surface over the inline storage).
    [[nodiscard]] constexpr auto relation() const -> std::string_view {
        return relation_name.view();
    }
    [[nodiscard]] constexpr auto field() const -> std::string_view {
        return field_name.view();
    }
};

} // namespace bdb

namespace bdb::detail {

// `^^std::uint64_t` is ill-formed on the pinned GCC ("'^^' cannot be
// applied to a using-declaration"); routing through a template parameter
// resolves the alias during substitution first.
template<class T>
inline constexpr auto type_reflection = ^^T;

} // namespace bdb::detail

export namespace bdb {

/// Classifies one reflected field type against the closed vocabulary;
/// nullopt = unsupported (the caller renders the product diagnostic).
consteval auto classify(std::meta::info type)
    -> std::optional<field_class> {
    auto const t = std::meta::dealias(type);
    if (t == ^^bool) {
        return field_class{value_kind::boolean, 0};
    }
    if (t == detail::type_reflection<std::uint64_t>) {
        return field_class{value_kind::u64, 0};
    }
    if (t == detail::type_reflection<std::int64_t>) {
        return field_class{value_kind::i64, 0};
    }
    if (t == std::meta::dealias(detail::type_reflection<std::string>)) {
        return field_class{value_kind::string, 0};
    }
    if (!std::meta::has_template_arguments(t)) {
        return std::nullopt;
    }
    auto const tmpl = std::meta::template_of(t);
    auto const args = std::meta::template_arguments_of(t);
    if (tmpl == ^^std::array
        && std::meta::dealias(args[0]) == ^^std::byte) {
        // bdb::bytes<N> dealiases to std::array<std::byte, N>; the engine
        // admits 1 ≤ N ≤ 64 (ValueType::FixedBytes len — lowering.md §1.8).
        auto const len = std::meta::extract<std::size_t>(args[1]);
        if (len >= 1 && len <= 64) {
            return field_class{
                value_kind::fixed_bytes, static_cast<std::uint16_t>(len)};
        }
        return std::nullopt;
    }
    if (tmpl == ^^interval) {
        if (std::meta::dealias(args[0])
            == detail::type_reflection<std::uint64_t>) {
            return field_class{value_kind::interval_u64, 0};
        }
        return field_class{value_kind::interval_i64, 0};
    }
    return std::nullopt;
}

} // namespace bdb

export namespace bdb::detail {

/// Whether the member carries the `[[=bdb::fresh]]` annotation (matched by
/// the annotation's type — FreshTag; annotation objects reflect const).
consteval auto is_fresh_marked(std::meta::info member) -> bool {
    for (auto const annotation : std::meta::annotations_of(member)) {
        auto const type =
            std::meta::remove_const(std::meta::type_of(annotation));
        if (type == ^^FreshTag) {
            return true;
        }
    }
    return false;
}

/// The row's fields, in declaration order — the one enumeration everything
/// else derives from.
consteval auto row_members(std::meta::info row)
    -> std::vector<std::meta::info> {
    return std::meta::nonstatic_data_members_of(
        row, std::meta::access_context::current());
}

consteval auto field_count(std::meta::info row) -> std::size_t {
    return row_members(row).size();
}

consteval auto row_is_supported(std::meta::info row) -> bool {
    for (auto const member : row_members(row)) {
        if (!classify(std::meta::type_of(member)).has_value()) {
            return false;
        }
    }
    return true;
}

consteval auto fresh_marks_are_u64(std::meta::info row) -> bool {
    for (auto const member : row_members(row)) {
        if (!is_fresh_marked(member)) {
            continue;
        }
        auto const cls = classify(std::meta::type_of(member));
        if (!cls.has_value() || cls->kind != value_kind::u64) {
            return false;
        }
    }
    return true;
}

/// Diagnostic subjects: `bumbledb relation "Service"` for the relation
/// lane, `bumbledb row type 'ServiceRow'` for the marshalling lane.
consteval auto relation_subject(std::string_view name) -> std::string {
    return std::string{"bumbledb relation \""} + std::string{name} + "\"";
}

consteval auto row_subject(std::meta::info row) -> std::string {
    return std::string{"bumbledb row type '"}
        + std::string{std::meta::display_string_of(row)} + "'";
}

/// The pinned unsupported-field diagnostic (compile-fail suite pins its
/// shape): names the subject, the first offending field, and its type.
consteval auto unsupported_field_message(
    std::string subject, std::meta::info row) -> std::string {
    for (auto const member : row_members(row)) {
        if (classify(std::meta::type_of(member)).has_value()) {
            continue;
        }
        return subject + ": field \""
            + std::string{std::meta::identifier_of(member)}
            + "\" has unsupported row type '"
            + std::string{std::meta::display_string_of(
                  std::meta::type_of(member))}
            + "' — the value vocabulary is closed: bool, std::uint64_t, "
              "std::int64_t, std::string, bdb::bytes<1..=64>, "
              "bdb::interval<std::uint64_t>, bdb::interval<std::int64_t>";
    }
    return {};
}

/// The pinned misplaced-fresh diagnostic: fresh is u64-only (the TS SDK
/// twin rule; engine validation re-judges).
consteval auto misplaced_fresh_message(
    std::string subject, std::meta::info row) -> std::string {
    for (auto const member : row_members(row)) {
        if (!is_fresh_marked(member)) {
            continue;
        }
        auto const cls = classify(std::meta::type_of(member));
        if (cls.has_value() && cls->kind == value_kind::u64) {
            continue;
        }
        return subject + ": field \""
            + std::string{std::meta::identifier_of(member)}
            + "\" is marked [[=bdb::fresh]] but has type '"
            + std::string{std::meta::display_string_of(
                  std::meta::type_of(member))}
            + "' — fresh is legal on std::uint64_t fields only";
    }
    return {};
}

/// Compile-time index range for pairing two parallel reflected member
/// walks under `template for`.
template<std::size_t Count>
consteval auto index_array() -> std::array<std::size_t, Count> {
    auto indices = std::array<std::size_t, Count>{};
    for (auto index = std::size_t{0}; index != Count; ++index) {
        indices[index] = index;
    }
    return indices;
}

/// The consteval-failure hook for an over-long reflected name (the
/// bumbledb.types diagnostic convention: reaching a call to this
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

namespace bdb::detail {

consteval auto coord_specs(std::string_view relation_name,
    std::meta::info row) -> std::vector<std::meta::info> {
    auto specs = std::vector<std::meta::info>{};
    auto ordinal = std::size_t{0};
    for (auto const member : row_members(row)) {
        // Unsupported field types classify under a total fallback so the
        // injection succeeds and make_relation's static_asserts stay the
        // ONE diagnostic a rejected row produces.
        auto const cls = classify(std::meta::type_of(member))
            .value_or(field_class{value_kind::u64, 0});
        specs.push_back(std::meta::data_member_spec(
            std::meta::substitute(^^coord,
                {std::meta::type_of(member),
                    std::meta::reflect_constant(
                        to_name_text(relation_name)),
                    std::meta::reflect_constant(to_name_text(
                        std::meta::identifier_of(member))),
                    std::meta::reflect_constant(ordinal),
                    std::meta::reflect_constant(cls),
                    std::meta::reflect_constant(
                        is_fresh_marked(member))}),
            {.name = std::meta::identifier_of(member)}));
        ++ordinal;
    }
    return specs;
}

// The proven injection pattern (TODO_CPP §38): define_aggregate may only
// be evaluated from a consteval block, so the facade type is synthesized
// at class-template scope. Coords gets one member per row field, named
// identically, of coordinate type — the Name NTTP makes the facade TYPE
// carry the relation identity too (two same-row relations are two types).
template<fixed_string Name, class Row>
struct RelationTypes {
    struct Coords;
    consteval {
        std::meta::define_aggregate(
            ^^Coords, coord_specs(Name.view(), ^^Row));
    }
};

} // namespace bdb::detail

export namespace bdb {

/// Builds the coordinate facade value for one relation. The static_asserts
/// are the §34 diagnostics; a rejected row produces exactly one error
/// (coord_specs classifies totally, so the injection itself never fires).
template<fixed_string Name, class Row>
consteval auto make_relation() ->
    typename detail::RelationTypes<Name, Row>::Coords {
    static_assert(
        detail::row_is_supported(^^Row),
        detail::unsupported_field_message(
            detail::relation_subject(Name.view()), ^^Row));
    static_assert(
        detail::fresh_marks_are_u64(^^Row),
        detail::misplaced_fresh_message(
            detail::relation_subject(Name.view()), ^^Row));

    // Coordinates carry their whole payload in the type; the facade value
    // is the empty product of them.
    return typename detail::RelationTypes<Name, Row>::Coords{};
}

/// The relation reflector (TODO_CPP §6): `bdb::relation<"Service",
/// ServiceRow>` is a coordinate facade with one member per row field,
/// named identically — `Service.id`, `Service.name` — each a
/// `bdb::coord` specialization carrying compile-time semantic data in its
/// type. Member access is deliberately the only binding style.
template<fixed_string Name, class Row>
inline constexpr auto relation = make_relation<Name, Row>();

} // namespace bdb
