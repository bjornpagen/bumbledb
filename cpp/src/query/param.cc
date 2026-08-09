export module bumbledb:param;

import std;
import :name;

export namespace bdb {

/**
 * A scalar query parameter. The name is the member of the typed params
 * product `execute` takes; the domain is inferred from the first anchored
 * use (a binding's field, or a comparison sibling).
 */
template<fixed_string Name>
struct param_ref {
	static constexpr name_text name = detail::to_name_text(Name.view());
};

/**
 * A set-valued query parameter (`ir::Term::ParamSet`): bound at execution
 * to a sequence of values of the anchoring field's type (the
 * params-product member is a span); a binding position matches iff the
 * field value is in the set. Legal in atom bindings (positive and
 * negated) and as one side of `eq` — nowhere else.
 */
template<fixed_string Name>
struct set_param_ref {
	static constexpr name_text name = detail::to_name_text(Name.view());
};

/**
 * Mints a scalar parameter. The free mint exists because the member
 * spelling inside the generic rule lambda demands the
 * `r.template param<"t">()` grammar.
 */
template<fixed_string Name>
[[nodiscard]] consteval auto param() -> param_ref<Name> {
	return {};
}

/** Mints a set parameter; rules as `set_param_ref`. */
template<fixed_string Name>
[[nodiscard]] consteval auto set_param() -> set_param_ref<Name> {
	return {};
}

}

namespace bdb::detail {

template<class T>
inline constexpr bool is_param_ref_v = false;

template<fixed_string Name>
inline constexpr bool is_param_ref_v<param_ref<Name>> = true;

template<class T>
inline constexpr bool is_set_param_ref_v = false;

template<fixed_string Name>
inline constexpr bool is_set_param_ref_v<set_param_ref<Name>> = true;

}
