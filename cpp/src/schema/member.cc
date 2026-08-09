// :schema_member — which types count as statement coordinates, relation
// facades, and schema members (the closed statement algebra's
// representation-level dispatch over the SDK's own templates), plus the
// shared coordinate-label helpers behind every §34 diagnostic.
export module bumbledb:schema_member;

import std;
import :name;
import :classify;
import :coord;
import :id;
import :closed_facade;

namespace bdb::detail {

// ————————————————————————————————————————————————————————————————————
// Type recognition (the closed statement algebra; representation-level
// dispatch over the SDK's own templates).
// ————————————————————————————————————————————————————————————————————

template<class T>
inline constexpr bool is_coordinate_v = false;

template<class T, name_text R, name_text F, std::size_t O, field_class C,
    bool Fr>
inline constexpr bool is_coordinate_v<coord<T, R, F, O, C, Fr>> = true;

// A closed relation's synthetic id IS a statement coordinate (TODO_CPP
// §8: the closed relation stays usable in schema statements).
template<name_text R, std::size_t H>
inline constexpr bool is_coordinate_v<closed_id<R, H>> = true;

consteval auto is_coord_type(std::meta::info type) -> bool {
    auto const t = std::meta::dealias(type);
    return std::meta::has_template_arguments(t)
        && std::meta::template_of(t) == ^^coord;
}

/// A coordinate-shaped facade member (a reflected field coordinate or the
/// closed synthetic id) — the filter every facade walk applies (closed
/// facades also carry handle constants, the axiom readback, and the wire
/// carrier, none of which are columns).
consteval auto is_coordinate_like_type(std::meta::info type) -> bool {
    auto const t = std::meta::dealias(type);
    if (!std::meta::has_template_arguments(t)) {
        return false;
    }
    auto const tmpl = std::meta::template_of(t);
    return tmpl == ^^coord || tmpl == ^^closed_id;
}

/// A relation facade: a class whose every member is a coordinate (the
/// injected Coords product of :facade).
consteval auto is_facade_type(std::meta::info type) -> bool {
    auto const t = std::meta::dealias(type);
    if (!std::meta::is_class_type(t)) {
        return false;
    }
    auto const members = std::meta::nonstatic_data_members_of(
        t, std::meta::access_context::current());
    if (members.empty()) {
        return false;
    }
    for (auto const member : members) {
        if (!is_coord_type(std::meta::type_of(member))) {
            return false;
        }
    }
    return true;
}

template<class T>
consteval auto is_facade() -> bool {
    return is_facade_type(^^T);
}

/// A schema MEMBER: an ordinary relation facade or a closed relation
/// facade (:closed_facade's discriminant).
consteval auto is_member_type(std::meta::info type) -> bool {
    return is_facade_type(type) || is_closed_facade_type(type);
}

template<class T>
consteval auto is_member() -> bool {
    return is_member_type(^^T);
}

/// Decimal rendering for diagnostics (std::to_string is not constexpr on
/// the pinned libstdc++).
consteval auto render_count(std::size_t value) -> std::string {
    if (value == 0) {
        return "0";
    }
    auto out = std::string{};
    while (value != 0) {
        out.insert(out.begin(),
            static_cast<char>('0' + static_cast<char>(value % 10)));
        value /= 10;
    }
    return out;
}

// The coordinate label helpers behind every §34 diagnostic.
consteval auto label(name_text relation, name_text field) -> std::string {
    return std::string{relation.view()} + "." + std::string{field.view()};
}

consteval auto quoted(name_text relation, name_text field) -> std::string {
    return "\"" + label(relation, field) + "\"";
}

template<class Coordinate>
consteval auto coordinate_label() -> std::string {
    return label(Coordinate::relation_name, Coordinate::field_name);
}

/// The first coordinate of a pack whose relation differs from First's —
/// the offender a span diagnostic names ("" when the pack is coherent).
template<class First, class... Rest>
consteval auto foreign_relation_label() -> std::string {
    auto out = std::string{};
    [[maybe_unused]] auto const check = [&]<class C>() {
        if (out.empty() && !(C::relation_name == First::relation_name)) {
            out = coordinate_label<C>();
        }
    };
    (check.template operator()<Rest>(), ...);
    return out;
}

template<class First, class... Rest>
consteval auto same_relation() -> bool {
    return ((Rest::relation_name == First::relation_name) && ...);
}

template<class First, class... Rest>
consteval auto span_message(std::string_view constructor,
    std::string_view law) -> std::string {
    return "bumbledb " + std::string{constructor}
        + "(): coordinates span two relations — \""
        + coordinate_label<First>() + "\" and \""
        + foreign_relation_label<First, Rest...>() + "\" — "
        + std::string{law};
}

} // namespace bdb::detail
