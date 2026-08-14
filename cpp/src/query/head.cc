export module bumbledb:head;

import std;
import :name;
import :classify;
import :spec;
import :ir;
import :var;
import :pattern;

namespace bdb::detail {

/**
 * One find-pattern slot: accepts a class-equal bound variable only (the
 * boundness wall is value-tier, judged at rule assembly).
 */
template<class T, name_text Relation, name_text Field, field_class Class, bool Classed, coord_ref Law>
struct find_slot {
	static constexpr name_text field_name = Field;
	static constexpr field_class cls = Class;
	static constexpr bool classed = Classed;
	static constexpr coord_ref law = Law;

	bool mentioned = false;
	term_data term{};
	find_form form = find_form::variable;

	find_slot() = default;

	template<class VT, name_text VR, name_text VF, field_class VC, bool VCl, coord_ref VLaw>
	consteval find_slot(qvar<VT, VR, VF, VC, VCl, VLaw> variable) {
		static_assert(VC == Class, kind_mismatch_message<VR, VF, VC, Relation, Field, Class>("answer at"));
		static_assert(VCl == Classed && (!Classed || VLaw == Law),
		              cross_class_message<VR, VF, VCl, VLaw, Relation, Field, Classed, Law>("answer at"));
		term = var_term<decltype(variable)>();
		form = find_form::variable;
		mentioned = true;
	}

	template<class Var>
	consteval find_slot(measure_ref<Var>) {
		static_assert(is_qvar_v<Var>, "bumbledb find(): Duration(v) projects an interval variable's measure");
		static_assert(Var::cls == Class, kind_mismatch_message<Var::relation_name, Var::field_name, Var::cls, Relation, Field, Class>("answer at"));
		term = var_term<Var>();
		form = find_form::measure;
		mentioned = true;
	}
};

template<class S, class... Facades>
struct find_pattern_types {
	struct Pattern;
	consteval {
		auto specs = std::vector<std::meta::info>{};
		auto taken = std::vector<std::string>{};
		[[maybe_unused]] auto const add = [&]<class Facade>() {
			for (auto const member : std::meta::nonstatic_data_members_of(^^Facade, std::meta::access_context::current())) {
				auto const facts = facts_of_member(member);
				if (!facts.include) {
					continue;
				}
				auto const name = std::string{std::meta::identifier_of(member)};
				auto duplicate = false;
				for (auto const& seen : taken) {
					if (seen == name) {
						duplicate = true;
					}
				}
				if (duplicate) {
					continue;
				}
				taken.push_back(name);
				auto const law = law_of<S>(facts.relation, facts.field);
				specs.push_back(std::meta::data_member_spec(
				    std::meta::substitute(^^find_slot,
				                          {
				                              facts.value_type, std::meta::reflect_constant(facts.relation),
				                              std::meta::reflect_constant(facts.field), std::meta::reflect_constant(facts.cls),
				                              std::meta::reflect_constant(law.first), std::meta::reflect_constant(law.second)}),
				    {.name = std::meta::identifier_of(member)}));
			}
		};
		(add.template operator()<Facades>(), ...);
		std::meta::define_aggregate(^^Pattern, specs);
	}
};

}

export namespace bdb {

/**
 * The designated-init find head over the rule's matched relations: every
 * matched relation's coordinates in match order, first name wins on a
 * collision (single-relation rules never collide).
 */
template<class S, class... Facades>
using find_pattern_of = typename detail::find_pattern_types<S, Facades...>::Pattern;

}
