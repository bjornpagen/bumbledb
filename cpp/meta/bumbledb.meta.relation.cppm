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
/// coordinate name hooks.
struct name_text {
    std::array<char, max_name_length> chars{};
    std::size_t length{};

    [[nodiscard]] constexpr auto view() const -> std::string_view {
        return std::string_view{chars.data(), length};
    }
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
};

/// One compile-time semantic coordinate (`Service.id`): relation name,
/// field name, ordinal, structural kind, and fresh mark, synthesized from
/// the reflected row declaration. Not a runtime field value. A structural
/// literal type — coordinates are NTTP-friendly by design.
template<class T>
struct coord {
    using value_type = T;

    name_text relation_name;
    name_text field_name;
    std::size_t ordinal;
    value_kind kind;
    std::uint16_t fixed_len;
    bool fresh;

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

consteval auto to_name_text(std::string_view text) -> name_text {
    contract_assert(text.size() <= max_name_length);
    auto result = name_text{};
    std::ranges::copy(text, result.chars.begin());
    result.length = text.size();
    return result;
}

} // namespace bdb::detail

namespace bdb::detail {

consteval auto coord_specs(std::meta::info row)
    -> std::vector<std::meta::info> {
    auto specs = std::vector<std::meta::info>{};
    for (auto const member : row_members(row)) {
        specs.push_back(std::meta::data_member_spec(
            std::meta::substitute(^^coord, {std::meta::type_of(member)}),
            {.name = std::meta::identifier_of(member)}));
    }
    return specs;
}

// The proven injection pattern (TODO_CPP §38): define_aggregate may only
// be evaluated from a consteval block, so the facade type is synthesized
// at class-template scope. Coords gets one member per row field, named
// identically, of type coord<FieldType>.
template<class Row>
struct RelationTypes {
    struct Coords;
    consteval {
        std::meta::define_aggregate(^^Coords, coord_specs(^^Row));
    }
};

} // namespace bdb::detail

export namespace bdb {

/// Builds the coordinate facade value for one relation. The static_asserts
/// are the §34 diagnostics; the fill below them stays total (value_or) so
/// a rejected row produces exactly one error.
template<fixed_string Name, class Row>
consteval auto make_relation() ->
    typename detail::RelationTypes<Row>::Coords {
    static_assert(
        detail::row_is_supported(^^Row),
        detail::unsupported_field_message(
            detail::relation_subject(Name.view()), ^^Row));
    static_assert(
        detail::fresh_marks_are_u64(^^Row),
        detail::misplaced_fresh_message(
            detail::relation_subject(Name.view()), ^^Row));

    using Facade = typename detail::RelationTypes<Row>::Coords;
    constexpr auto ctx = std::meta::access_context::current();
    constexpr auto members = std::define_static_array(
        std::meta::nonstatic_data_members_of(^^Row, ctx));
    constexpr auto facade_members = std::define_static_array(
        std::meta::nonstatic_data_members_of(^^Facade, ctx));

    auto facade = Facade{};
    constexpr auto relation_text = detail::to_name_text(Name.view());
    template for (
        constexpr auto index :
        detail::index_array<members.size()>()) {
        constexpr auto member = members[index];
        constexpr auto cls = classify(std::meta::type_of(member))
            .value_or(field_class{value_kind::u64, 0});
        facade.[:facade_members[index]:] =
            coord<typename [:std::meta::type_of(member):]>{
                .relation_name = relation_text,
                .field_name = detail::to_name_text(
                    std::meta::identifier_of(member)),
                .ordinal = index,
                .kind = cls.kind,
                .fixed_len = cls.fixed_len,
                .fresh = detail::is_fresh_marked(member),
            };
    }
    return facade;
}

/// The relation reflector (TODO_CPP §6): `bdb::relation<"Service",
/// ServiceRow>` is a coordinate facade with one member per row field,
/// named identically — `Service.id`, `Service.name` — each a
/// `bdb::coord<FieldType>` carrying compile-time semantic data. Member
/// access is deliberately the only binding style.
template<fixed_string Name, class Row>
inline constexpr auto relation = make_relation<Name, Row>();

} // namespace bdb
