// :param — named query parameters (TODO_CPP §21): the scalar param and
// the runtime ∈-set param, with type/point-domain inferred from the first
// anchored use. The free mints live here because a member spelled with
// explicit template arguments inside the generic rule lambda would demand
// the `r.template param<"t">()` grammar.
export module bumbledb:param;

import std;
import :name;

export namespace bdb {

/// A scalar query parameter — `r.param<"t">()`. The name is the member of
/// the typed params product `execute` takes; the domain is inferred from
/// the first anchored use (a binding's field, or a comparison sibling).
template<fixed_string Name>
struct param_ref {
    static constexpr name_text name = detail::to_name_text(Name.view());
};

/// A set-valued query parameter (`ir::Term::ParamSet`, TODO_CPP §21) —
/// `bdb::set_param<"frontier">()`: bound at execution to a SEQUENCE of
/// values of the anchoring field's type (the params-product member is a
/// span); a binding position matches iff the field value is in the set.
/// Legal in atom bindings (positive and negated) and as one side of `eq`
/// — nowhere else, exactly as the IR rules it.
template<fixed_string Name>
struct set_param_ref {
    static constexpr name_text name = detail::to_name_text(Name.view());
};

/// A named scalar parameter; type/point-domain inferred from use
/// (TODO_CPP §21).
template<fixed_string Name>
[[nodiscard]] consteval auto param() -> param_ref<Name> {
    return {};
}

/// A named SET parameter (`ir::Term::ParamSet`, TODO_CPP §21): bound at
/// execution to a sequence of the anchoring field's element type.
template<fixed_string Name>
[[nodiscard]] consteval auto set_param() -> set_param_ref<Name> {
    return {};
}

} // namespace bdb

namespace bdb::detail {

template<class T>
inline constexpr bool is_param_ref_v = false;

template<fixed_string Name>
inline constexpr bool is_param_ref_v<param_ref<Name>> = true;

template<class T>
inline constexpr bool is_set_param_ref_v = false;

template<fixed_string Name>
inline constexpr bool is_set_param_ref_v<set_param_ref<Name>> = true;

} // namespace bdb::detail
