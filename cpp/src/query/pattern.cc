export module bumbledb:pattern;

import std;
import :name;
import :classify;
import :interval;
import :handle;
import :coord;
import :spec;
import :ir;
import :var;
import :param;

namespace bdb::detail {

[[nodiscard]] consteval auto render_size(std::size_t value) -> std::string {
	if (value == 0) {
		return "0";
	}
	auto out = std::string{};
	while (value != 0) {
		out.insert(out.begin(), static_cast<char>('0' + static_cast<char>(value % 10)));
		value /= 10;
	}
	return out;
}

[[nodiscard]] consteval auto coordinate_label(name_text relation, name_text field) -> std::string {
	return std::string{relation.view()} + "." + std::string{field.view()};
}

[[nodiscard]] consteval auto kind_label(field_class cls) -> std::string {
	switch (cls.kind) {
	case value_kind::boolean:
		return "bool";
	case value_kind::u64:
		return "u64";
	case value_kind::i64:
		return "i64";
	case value_kind::string:
		return "str";
	case value_kind::fixed_bytes:
		return "bytes<" + render_size(cls.fixed_len) + ">";
	case value_kind::interval_u64:
		return "interval<u64>";
	case value_kind::interval_i64:
		break;
	}
	return "interval<i64>";
}

[[nodiscard]] consteval auto class_label(bool classed, coord_ref law) -> std::string {
	if (!classed) {
		return "bare";
	}
	return "class \"" + coordinate_label(law.relation, law.field) + "\"";
}

template<name_text VarRel, name_text VarField, bool VarClassed, coord_ref VarLaw, name_text AtRel, name_text AtField, bool AtClassed,
         coord_ref AtLaw>
[[nodiscard]] consteval auto cross_class_message(std::string_view verb) -> std::string {
	return "bumbledb query: variable \"" + coordinate_label(VarRel, VarField) + "\" (" + class_label(VarClassed, VarLaw) + ") cannot " +
	       std::string{verb} + " coordinate \"" + coordinate_label(AtRel, AtField) + "\" (" + class_label(AtClassed, AtLaw) +
	       ") — a variable joins only class-equal columns (one variable, "
	       "one law class)";
}

template<name_text HandleRoster, name_text Handle, name_text AtRel, name_text AtField, name_text FieldRoster>
[[nodiscard]] consteval auto handle_binding_message() -> std::string {
	return "bumbledb query: handle \"" + std::string{Handle.view()} + "\" of closed relation \"" + std::string{HandleRoster.view()} +
	       "\" cannot bind coordinate \"" + coordinate_label(AtRel, AtField) + "\" — the field references closed relation \"" +
	       std::string{FieldRoster.view()} + "\"";
}

template<name_text VarRel, name_text VarField, field_class VarClass, name_text AtRel, name_text AtField, field_class AtClass>
[[nodiscard]] consteval auto kind_mismatch_message(std::string_view verb) -> std::string {
	return "bumbledb query: variable \"" + coordinate_label(VarRel, VarField) + "\" (" + kind_label(VarClass) + ") cannot " +
	       std::string{verb} + " coordinate \"" + coordinate_label(AtRel, AtField) + "\" (" + kind_label(AtClass) +
	       ") — the structural kinds differ";
}

template<class Var>
[[nodiscard]] consteval auto var_term() -> term_data {
	auto out = term_data{};
	out.form = query_term_form::variable;
	out.variable = coord_ref{.relation = Var::relation_name, .field = Var::field_name};
	return out;
}

template<class Var>
[[nodiscard]] consteval auto measure_term() -> term_data {
	auto out = var_term<Var>();
	out.form = query_term_form::measure;
	return out;
}

template<class Param>
[[nodiscard]] consteval auto param_term() -> term_data {
	auto out = term_data{};
	out.form = query_term_form::param;
	out.param = Param::name;
	return out;
}

template<class Param>
[[nodiscard]] consteval auto set_param_term() -> term_data {
	auto out = term_data{};
	out.form = query_term_form::param_set;
	out.param = Param::name;
	return out;
}

[[nodiscard]] consteval auto literal_term(query_literal literal) -> term_data {
	auto out = term_data{};
	out.form = query_term_form::literal;
	out.literal = literal;
	return out;
}

/**
 * The membership registry entry's content-addressed synthetic name:
 * "in <Roster> <sorted row ids>" — one identical array in two positions
 * folds to one set param. Never a params-product member; the embedded
 * space keeps it disjoint from every user-spellable param name.
 */
[[nodiscard]] consteval auto membership_param_name(name_text roster, std::array<std::uint64_t, max_membership_handles> const& members, std::size_t count)
    -> name_text {
	auto text = std::string{"in "} + std::string{roster.view()};
	for (auto index = std::size_t{0}; index != count; ++index) {
		text += index == 0 ? " " : ",";
		text += render_size(members[index]);
	}
	return to_name_text(text);
}

/**
 * Tags one integral host literal at a scalar domain — the sibling/field
 * directs the tag (lowering.md §4.2).
 */
template<class T>
[[nodiscard]] consteval auto scalar_literal(value_kind kind, T value) -> query_literal {
	auto out = query_literal{};
	out.kind = kind;
	if (kind == value_kind::boolean) {
		out.boolean = static_cast<bool>(value);
	} else if (kind == value_kind::u64) {
		out.u64 = static_cast<std::uint64_t>(value);
	} else {
		out.i64 = static_cast<std::int64_t>(value);
	}
	return out;
}

[[nodiscard]] consteval auto interval_literal(interval<std::uint64_t> value) -> query_literal {
	auto out = query_literal{};
	out.kind = value_kind::interval_u64;
	out.u64_start = value.lo();
	out.u64_end = value.hi();
	return out;
}

[[nodiscard]] consteval auto interval_literal(interval<std::int64_t> value) -> query_literal {
	auto out = query_literal{};
	out.kind = value_kind::interval_i64;
	out.i64_start = value.lo();
	out.i64_end = value.hi();
	return out;
}

consteval auto add_use(rule_state& state, param_use use) -> void {
	if (state.use_count == state.uses.size()) {
		query_has_too_many_params();
	}
	state.uses[state.use_count] = use;
	++state.use_count;
}

consteval auto add_bound(rule_state& state, coord_ref variable) -> void {
	for (auto index = std::size_t{0}; index != state.bound_count; ++index) {
		if (state.bound[index] == variable) {
			return;
		}
	}
	if (state.bound_count == max_query_vars) {
		rule_has_too_many_variables();
	}
	state.bound[state.bound_count] = variable;
	++state.bound_count;
}

[[nodiscard]] consteval auto is_bound(rule_state const& state, coord_ref variable) -> bool {
	for (auto index = std::size_t{0}; index != state.bound_count; ++index) {
		if (state.bound[index] == variable) {
			return true;
		}
	}
	return false;
}

/**
 * One match-pattern slot. Identity rides the slot type (coordinate +
 * classes as NTTPs) so the converting constructors can run the class
 * walls as static_asserts naming semantic coordinates. Default state is
 * the wildcard — an unmentioned designated-init member binds nothing
 * (`ir::Atom`'s absence rule).
 */
template<class T, name_text Relation, name_text Field, std::size_t Ordinal, field_class Class, bool Classed, coord_ref Law>
struct binding_slot {
	static constexpr name_text relation_name = Relation;
	static constexpr name_text field_name = Field;
	static constexpr std::size_t ordinal = Ordinal;
	static constexpr field_class cls = Class;

	term_data term{};

	binding_slot() = default;

	/**
	 * A variable binding: the class walls run here, where both the
	 * variable's mint slot and the field's slot are template-visible.
	 */
	template<class VT, name_text VR, name_text VF, field_class VC, bool VCl, coord_ref VLaw>
	consteval binding_slot(qvar<VT, VR, VF, VC, VCl, VLaw> variable) {
		static_assert(VC == Class, kind_mismatch_message<VR, VF, VC, Relation, Field, Class>("bind"));
		static_assert(VCl == Classed && (!Classed || VLaw == Law),
		              cross_class_message<VR, VF, VCl, VLaw, Relation, Field, Classed, Law>("bind"));
		term = var_term<decltype(variable)>();
	}

	/** A scalar param binding, anchored at this field's domain. */
	template<fixed_string Name>
	consteval binding_slot(param_ref<Name> parameter) {
		term = param_term<decltype(parameter)>();
	}

	/**
	 * A set-param binding: the position matches iff the field value is in
	 * the bound set (`ir::Term::ParamSet`, anchored at this field's
	 * domain). Unlike a membership array, the set arrives at execution.
	 */
	template<fixed_string Name>
	consteval binding_slot(set_param_ref<Name> parameter) {
		term = set_param_term<decltype(parameter)>();
	}

	/** A bare literal at a fixed-width field (field-directed tagging). */
	consteval binding_slot(T value)
	    requires(!is_closed_ref_v<T> &&
	             (Class.kind == value_kind::boolean || Class.kind == value_kind::u64 || Class.kind == value_kind::i64))
	{
		term = literal_term(scalar_literal(Class.kind, value));
	}

	consteval binding_slot(T value)
	    requires(Class.kind == value_kind::interval_u64 || Class.kind == value_kind::interval_i64)
	{
		term = literal_term(interval_literal(value));
	}

	/**
	 * A handle literal at a closed-reference field: the host resolves it —
	 * roster-verified here, lowered as the declaration-order row id,
	 * u64-tagged (lowering.md §4.2, §7.8).
	 */
	template<name_text HandleRoster, name_text Handle, std::uint64_t Index>
	consteval binding_slot(handle_value<HandleRoster, Handle, Index>)
	    requires is_closed_ref_v<T>
	{
		static_assert(HandleRoster == T::roster_name, handle_binding_message<HandleRoster, Handle, Relation, Field, T::roster_name>());
		term = literal_term(scalar_literal(value_kind::u64, std::uint64_t{Index}));
	}

	/**
	 * A membership array at a closed-reference field — closed-only in
	 * match records, folded to a pre-resolved ∈-set over a synthetic
	 * content-addressed registry entry whose address is the sorted row
	 * ids (lowering.md §4.2). Each element's roster wall runs on the
	 * closed_ref conversion.
	 */
	consteval binding_slot(std::initializer_list<T> handles)
	    requires is_closed_ref_v<T>
	{
		auto members = std::array<std::uint64_t, max_membership_handles>{};
		auto count = std::size_t{0};
		for (auto const& entry : handles) {
			if (count == max_membership_handles) {
				membership_array_has_too_many_handles();
			}
			members[count] = entry.row;
			++count;
		}
		if (count < 2) {
			membership_array_needs_at_least_two_handles();
		}
		for (auto first = std::size_t{1}; first != count; ++first) {
			auto const value = members[first];
			auto at = first;
			while (at != 0 && members[at - 1] > value) {
				members[at] = members[at - 1];
				--at;
			}
			members[at] = value;
		}
		for (auto index = std::size_t{1}; index != count; ++index) {
			if (members[index - 1] == members[index]) {
				membership_array_has_duplicate_handles();
			}
		}
		term.form = query_term_form::param_set;
		term.param = membership_param_name(T::roster_name, members, count);
		term.member_count = count;
		term.members = members;
	}
};

template<class S, class Facade>
struct match_pattern_types {
	struct Pattern;
	consteval {
		auto specs = std::vector<std::meta::info>{};
		for (auto const member : std::meta::nonstatic_data_members_of(^^Facade, std::meta::access_context::current())) {
			auto const facts = facts_of_member(member);
			if (!facts.include) {
				continue;
			}
			auto const law = law_of<S>(facts.relation, facts.field);
			specs.push_back(std::meta::data_member_spec(
			    std::meta::substitute(^^binding_slot,
			                          {
			                              facts.value_type, std::meta::reflect_constant(facts.relation),
			                              std::meta::reflect_constant(facts.field), std::meta::reflect_constant(facts.ordinal),
			                              std::meta::reflect_constant(facts.cls), std::meta::reflect_constant(law.first),
			                              std::meta::reflect_constant(law.second)}),
			    {.name = std::meta::identifier_of(member)}));
		}
		std::meta::define_aggregate(^^Pattern, specs);
	}
};

}

export namespace bdb {

/** The designated-init match pattern of one relation under one schema. */
template<class S, class Facade>
using match_pattern_of = typename detail::match_pattern_types<S, Facade>::Pattern;

}
