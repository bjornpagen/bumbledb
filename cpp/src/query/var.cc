export module bumbledb:var;

import std;
import :name;
import :classify;
import :coord;
import :handle;
import :id;
import :closed_facade;
import :spec;

export namespace bdb {

/**
 * One query variable, minted by `r.vars(Relation)`: identity and typing
 * live in the TYPE — the mint coordinate, the column's structural class,
 * and the column's law-computed class (the C++ image of the TS
 * object-reference identity: one schema mints one variable per
 * coordinate). Values are empty structural literals.
 */
template<class T, name_text Relation, name_text Field, field_class Class, bool Classed, coord_ref Law>
struct qvar {
	using value_type = T;

	static constexpr name_text relation_name = Relation;
	static constexpr name_text field_name = Field;
	static constexpr field_class cls = Class;
	static constexpr bool classed = Classed;
	static constexpr coord_ref law = Law;
};

/**
 * The measure of an interval variable: `|[s, e)| = e − s`, u64
 * (`ir::Term::Measure`).
 */
template<class Var>
struct measure_ref {
	using over = Var;
};

}

namespace bdb::detail {

template<class T>
inline constexpr bool is_qvar_v = false;

template<class T, name_text R, name_text F, field_class C, bool Cl, coord_ref L>
inline constexpr bool is_qvar_v<qvar<T, R, F, C, Cl, L>> = true;

template<class T>
inline constexpr bool is_measure_ref_v = false;

template<class Var>
inline constexpr bool is_measure_ref_v<measure_ref<Var>> = true;

[[nodiscard]] consteval auto is_query_coord_type(std::meta::info type) -> bool {
	auto const t = std::meta::dealias(type);
	return std::meta::has_template_arguments(t) && std::meta::template_of(t) == ^^coord;
}

/**
 * A relation facade: a class whose every member is a coordinate (the
 * injected Coords product of :facade).
 */
template<class Facade>
[[nodiscard]] consteval auto is_query_facade() -> bool {
	auto const t = std::meta::dealias(^^Facade);
	if (!std::meta::is_class_type(t)) {
		return false;
	}
	auto const members = std::meta::nonstatic_data_members_of(t, std::meta::access_context::current());
	if (members.empty()) {
		return false;
	}
	for (auto const member : members) {
		if (!is_query_coord_type(std::meta::type_of(member))) {
			return false;
		}
	}
	return true;
}

/**
 * A queryable schema member: an ordinary all-coordinate facade or a
 * closed relation facade (closed relations stay query atoms).
 */
template<class Facade>
[[nodiscard]] consteval auto is_query_member() -> bool {
	return is_query_facade<Facade>() || is_closed_facade<Facade>();
}

/**
 * One facade member's column facts, uniform over both member kinds:
 * `include == false` on a closed facade's non-column members (handle
 * constants, the axiom readback, the wire carrier).
 */
struct member_facts {
	bool include;
	std::meta::info value_type;
	name_text relation;
	name_text field;
	std::size_t ordinal;
	field_class cls;
};

[[nodiscard]] consteval auto facts_of_member(std::meta::info member) -> member_facts {
	auto const t = std::meta::dealias(std::meta::type_of(member));
	if (!std::meta::is_class_type(t) || !std::meta::has_template_arguments(t)) {
		return member_facts{.include = false, .value_type = std::meta::info{}, .relation = {}, .field = {}, .ordinal = 0, .cls = {}};
	}
	auto const tmpl = std::meta::template_of(t);
	if (tmpl == ^^coord) {
		auto const args = std::meta::template_arguments_of(t);
		return member_facts{.include = true,
		                    .value_type = args[0],
		                    .relation = std::meta::extract<name_text>(args[1]),
		                    .field = std::meta::extract<name_text>(args[2]),
		                    .ordinal = std::meta::extract<std::size_t>(args[3]),
		                    .cls = std::meta::extract<field_class>(args[4])};
	}
	if (tmpl == ^^closed_id) {
		auto const args = std::meta::template_arguments_of(t);
		return member_facts{.include = true,
		                    .value_type = std::meta::substitute(^^closed_ref,
		                                                        {
		                                                            args[0]}),
		                    .relation = std::meta::extract<name_text>(args[0]),
		                    .field = to_name_text("id"),
		                    .ordinal = 0,
		                    .cls = field_class{value_kind::u64, 0}};
	}
	return member_facts{.include = false, .value_type = std::meta::info{}, .relation = {}, .field = {}, .ordinal = 0, .cls = {}};
}

template<class Facade>
[[nodiscard]] consteval auto facade_relation_name() -> name_text {
	auto const members = std::meta::nonstatic_data_members_of(^^Facade, std::meta::access_context::current());
	return facts_of_member(members[0]).relation;
}

template<class S, class Facade>
[[nodiscard]] consteval auto foreign_relation_message() -> std::string {
	return std::string{"bumbledb query over schema \""} + std::string{S::declared_name.view()} + "\": relation \"" +
	       std::string{facade_relation_name<Facade>().view()} + "\" is not a member of the schema";
}

template<class S>
[[nodiscard]] consteval auto law_of(name_text relation, name_text field) -> std::pair<bool, coord_ref> {
	for (auto const& entry : S::member_class_map()) {
		if (entry.coordinate.relation == relation && entry.coordinate.field == field) {
			return {entry.classed, entry.class_name};
		}
	}
	return {false, coord_ref{}};
}

inline constexpr std::size_t no_relation = ~std::size_t{0};

template<class S, class Facade>
[[nodiscard]] consteval auto relation_ordinal() -> std::size_t {
	auto const table = S::member_relation_table();
	auto const relation = facade_relation_name<Facade>();
	for (auto index = std::size_t{0}; index != table.size(); ++index) {
		if (table[index].name == relation) {
			return index;
		}
	}
	return no_relation;
}

template<class S, class Facade>
[[nodiscard]] consteval auto facade_in_schema() -> bool {
	return relation_ordinal<S, Facade>() != no_relation;
}

template<class S, class Facade>
struct rule_vars_types {
	struct Vars;
	consteval {
		auto specs = std::vector<std::meta::info>{};
		for (auto const member : std::meta::nonstatic_data_members_of(^^Facade, std::meta::access_context::current())) {
			auto const facts = facts_of_member(member);
			if (!facts.include) {
				continue;
			}
			auto const law = law_of<S>(facts.relation, facts.field);
			specs.push_back(std::meta::data_member_spec(
			    std::meta::substitute(^^qvar,
			                          {
			                              facts.value_type, std::meta::reflect_constant(facts.relation),
			                              std::meta::reflect_constant(facts.field), std::meta::reflect_constant(facts.cls),
			                              std::meta::reflect_constant(law.first), std::meta::reflect_constant(law.second)}),
			    {.name = std::meta::identifier_of(member)}));
		}
		std::meta::define_aggregate(^^Vars, specs);
	}
};

}

export namespace bdb {

/**
 * The synthesized variable product of one relation under one schema: one
 * member per field, named identically, each a `bdb::qvar` carrying the
 * coordinate and its law class. Member access is the only supported
 * binding, deliberately: structured bindings are positional and would
 * hide the field name at the binding site, so nothing here enables the
 * tuple protocol.
 */
template<class S, class Facade>
using vars_of = typename detail::rule_vars_types<S, Facade>::Vars;

}
