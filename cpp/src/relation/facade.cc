export module bumbledb:facade;

import std;
import :name;
import :classify;
import :coord;

namespace bdb::detail {

/**
 * Unsupported field types classify under a total fallback so the
 * injection always succeeds and make_relation's static_asserts stay the
 * one diagnostic a rejected row produces.
 */
[[nodiscard]] consteval auto coord_specs(std::string_view relation_name, std::meta::info row) -> std::vector<std::meta::info> {
	auto specs = std::vector<std::meta::info>{};
	auto ordinal = std::size_t{0};
	for (auto const member : row_members(row)) {
		auto const cls = classify(std::meta::type_of(member)).value_or(field_class{value_kind::u64, 0});
		specs.push_back(std::meta::data_member_spec(
		    std::meta::substitute(^^coord,
		                          {
		                              std::meta::type_of(member), std::meta::reflect_constant(to_name_text(relation_name)),
		                              std::meta::reflect_constant(to_name_text(wire_field_name(member))),
		                              std::meta::reflect_constant(ordinal), std::meta::reflect_constant(cls),
		                              std::meta::reflect_constant(is_fresh_marked(member))}),
		    {.name = std::meta::identifier_of(member)}));
		++ordinal;
	}
	return specs;
}

/**
 * define_aggregate may only be evaluated from a consteval block, so the
 * facade type is synthesized at class-template scope. Coords gets one
 * member per row field, named identically, of coordinate type; the Name
 * NTTP makes the facade TYPE carry the relation identity too (two
 * same-row relations are two types).
 */
template<fixed_string Name, class Row>
struct RelationTypes {
	struct Coords;
	consteval {
		std::meta::define_aggregate(^^Coords, coord_specs(Name.view(), ^^Row));
	}
};

}

export namespace bdb {

/**
 * Builds the coordinate facade value for one relation. The
 * static_asserts render the product diagnostics; a rejected row produces
 * exactly one error (coord_specs classifies totally, so the injection
 * itself never fires).
 */
template<fixed_string Name, class Row>
[[nodiscard]] consteval auto make_relation() -> typename detail::RelationTypes<Name, Row>::Coords {
	static_assert(detail::row_is_supported(^^Row), detail::unsupported_field_message(detail::relation_subject(Name.view()), ^^Row));
	static_assert(detail::fresh_marks_are_u64(^^Row), detail::misplaced_fresh_message(detail::relation_subject(Name.view()), ^^Row));

	return typename detail::RelationTypes<Name, Row>::Coords{};
}

/**
 * The relation reflector: `bdb::relation<"Service", ServiceRow>` is a
 * coordinate facade with one member per row field, named identically —
 * `Service.id`, `Service.name` — each a `bdb::coord` specialization
 * carrying compile-time semantic data in its type. Member access is
 * deliberately the only binding style.
 */
template<fixed_string Name, class Row>
inline constexpr auto relation = make_relation<Name, Row>();

}
