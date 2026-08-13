export module bumbledb:rule;

import std;
import :name;
import :classify;
import :spec;
import :ir;
import :var;
import :pattern;
import :head;
import :aggregate;

export namespace bdb {

/**
 * A recursive predicate's reference tag: names the rec an `.idb`/
 * `.not_idb` atom targets (resolution to the dense PredId happens at
 * program assembly).
 */
template<fixed_string Name>
struct pred_tag {
	static constexpr name_text name = detail::to_name_text(Name.view());
};

template<fixed_string Name>
inline constexpr auto pred = pred_tag<Name>{};

/**
 * One named binding of a recursive atom: the target head column BY NAME,
 * the bound variable as the value.
 */
template<fixed_string Column, class Var>
struct idb_bind {
	using var = Var;
	static constexpr name_text column = detail::to_name_text(Column.view());
};

template<fixed_string Column, class Var>
[[nodiscard]] consteval auto bind(Var) -> idb_bind<Column, Var> {
	static_assert(detail::is_qvar_v<Var>, "bumbledb bind(): the argument must be a query variable "
	                                      "(vars.field) — idb bindings are variable terms only");
	return {};
}

}

namespace bdb::detail {

template<class T>
inline constexpr bool is_idb_bind_v = false;

template<fixed_string Column, class Var>
inline constexpr bool is_idb_bind_v<idb_bind<Column, Var>> = true;

template<class S, class Facade>
consteval auto record_match(rule_state& state, match_pattern_of<S, Facade> const& pattern, bool negated) -> void {
	if (state.item_count == state.items.size()) {
		rule_has_too_many_atoms();
	}
	auto atom = atom_data{};
	atom.relation = static_cast<std::uint32_t>(relation_ordinal<S, Facade>());

	using Pattern = match_pattern_of<S, Facade>;
	constexpr auto members =
	    std::define_static_array(std::meta::nonstatic_data_members_of(^^Pattern, std::meta::access_context::current()));
	template for (constexpr auto index : index_array<members.size()>()) {
		using Slot = [:std::meta::type_of(members[index]):];
		auto const& slot = pattern.[:members[index]:];
		if (slot.term.form == query_term_form::absent) {
			continue;
		}
		atom.bindings[atom.binding_count] = binding_data{
		    .field = Slot::ordinal,
		    .term = slot.term,
		};
		++atom.binding_count;
		if (slot.term.form == query_term_form::variable && !negated) {
			add_bound(state, slot.term.variable);
		}
		if (slot.term.form == query_term_form::param) {
			auto use = param_use{};
			use.name = slot.term.param;
			use.shape = param_shape::value;
			use.domain = Slot::cls;
			add_use(state, use);
		}
		if (slot.term.form == query_term_form::param_set) {
			auto use = param_use{};
			use.name = slot.term.param;
			use.shape = param_shape::set;
			use.domain = Slot::cls;
			use.membership = slot.term.member_count != 0;
			use.member_count = slot.term.member_count;
			use.members = slot.term.members;
			add_use(state, use);
		}
	}
	state.items[state.item_count] = body_item{
	    .form = negated ? body_form::negated_atom : body_form::atom,
	    .atom = atom,
	    .idb = idb_atom_data{},
	    .condition = condition_data{},
	};
	++state.item_count;
}

/**
 * Records one recursive atom (either polarity). A positive idb atom binds
 * its variables (grounding); a negated one binds nothing.
 */
consteval auto record_idb(rule_state& state, idb_atom_data const& atom) -> void {
	if (state.item_count == state.items.size()) {
		rule_has_too_many_atoms();
	}
	if (!atom.negated) {
		for (auto index = std::size_t{0}; index != atom.bind_count; ++index) {
			add_bound(state, atom.binds[index].variable);
		}
	}
	state.items[state.item_count] = body_item{
	    .form = body_form::idb_atom,
	    .atom = atom_data{},
	    .idb = atom,
	    .condition = condition_data{},
	};
	++state.item_count;
}

consteval auto record_condition(rule_state& state, cond_value const& cond) -> void {
	if (state.item_count == state.items.size()) {
		rule_has_too_many_conditions();
	}
	state.items[state.item_count] = body_item{
	    .form = body_form::condition,
	    .atom = atom_data{},
	    .idb = idb_atom_data{},
	    .condition = cond.data,
	};
	++state.item_count;
	for (auto index = std::size_t{0}; index != cond.use_count; ++index) {
		add_use(state, cond.uses[index]);
	}
}

}

export namespace bdb {

/**
 * The chain after at least one `.match`: value state accumulates; the
 * matched facades ride the TYPE (they shape the find pattern).
 */
template<class S, class... Facades>
struct rule_chain {
	rule_state state{};

	/**
	 * Joins another relation into the rule (shared variables ARE the join
	 * — reuse a `vars` member across patterns).
	 */
	template<class Facade>
	[[nodiscard]] consteval auto match(Facade, match_pattern_of<S, Facade> const& pattern) const -> rule_chain<S, Facades..., Facade> {
		static_assert(detail::is_query_member<Facade>(), "bumbledb match(): the first argument must be a relation "
		                                                 "facade (bdb::relation<...> or bdb::closed<...>)");
		static_assert(detail::facade_in_schema<S, Facade>(), detail::foreign_relation_message<S, Facade>());
		auto next = rule_chain<S, Facades..., Facade>{.state = state};
		detail::record_match<S, Facade>(next.state, pattern, false);
		return next;
	}

	/**
	 * One negated EDB atom (the anti-join — `ir::Rule::negated`): the rule
	 * keeps every binding NO matching fact extends. A negated atom binds
	 * nothing — its variables must be bound by positive atoms (the safety
	 * rule; judged at rule assembly). The matched facade does NOT join the
	 * find pattern.
	 */
	template<class Facade>
	[[nodiscard]] consteval auto not_match(Facade, match_pattern_of<S, Facade> const& pattern) const -> rule_chain {
		static_assert(detail::is_query_member<Facade>(), "bumbledb not_match(): the first argument must be a relation "
		                                                 "facade (bdb::relation<...> or bdb::closed<...>)");
		static_assert(detail::facade_in_schema<S, Facade>(), detail::foreign_relation_message<S, Facade>());
		auto next = *this;
		detail::record_match<S, Facade>(next.state, pattern, true);
		return next;
	}

	/**
	 * One positive recursive atom: grounds this rule against the named
	 * predicate's set and binds its variables. Inside a rec's own rules
	 * only the rec itself may be named; output rules join any finished
	 * stratum. Every head column of the target must be bound exactly once
	 * (judged at program assembly).
	 */
	template<fixed_string Name, class... Binds>
	[[nodiscard]] consteval auto idb(pred_tag<Name>, Binds... binds) const -> rule_chain {
		return with_idb<Name, false>(binds...);
	}

	/**
	 * The negated finished-stratum atom: rejects every binding the
	 * finished stratum extends (output rules only — a recursive rule
	 * negates no stratum). Binds nothing.
	 */
	template<fixed_string Name, class... Binds>
	[[nodiscard]] consteval auto not_idb(pred_tag<Name>, Binds... binds) const -> rule_chain {
		return with_idb<Name, true>(binds...);
	}

	/** Conjoins conditions (each a predicate value). */
	template<class... Conds>
	[[nodiscard]] consteval auto where(Conds const&... conds) const -> rule_chain {
		static_assert((std::same_as<std::remove_cvref_t<Conds>, cond_value> && ...),
		              "bumbledb where(): every argument must be a predicate value "
		              "(bdb::point_in / bdb::allen / bdb::eq / ...)");
		auto next = *this;
		(detail::record_condition(next.state, conds), ...);
		return next;
	}

	/**
	 * Ends the rule with its answer head: a designated-init pattern over
	 * the matched relations' coordinates (bound variables only), plus
	 * optional trailing columns — named variable columns and named
	 * aggregates. Head order = pattern coordinate order, then the trailing
	 * columns in written order.
	 */
	template<class... Extras>
	[[nodiscard]] consteval auto find(find_pattern_of<S, Facades...> const& head, Extras const&... extras) const -> rule_data {
		auto out = rule_data{.state = state, .find_count = 0, .finds = {}};

		using Pattern = find_pattern_of<S, Facades...>;
		constexpr auto members =
		    std::define_static_array(std::meta::nonstatic_data_members_of(^^Pattern, std::meta::access_context::current()));
		template for (constexpr auto index : detail::index_array<members.size()>()) {
			using Slot = [:std::meta::type_of(members[index]):];
			auto const& slot = head.[:members[index]:];
			if (slot.term.form != query_term_form::absent) {
				if (out.find_count == max_query_finds) {
					detail::rule_has_too_many_finds();
				}
				out.finds[out.find_count] = find_data{
				    .name = Slot::field_name,
				    .form = find_form::variable,
				    .op = fold_form::sum,
				    .over = slot.term,
				    .answer = Slot::cls,
				    .has_over = true,
				    .classed = Slot::classed,
				    .law = Slot::law,
				};
				++out.find_count;
			}
		}

		[[maybe_unused]] auto const add_extra = [&]<class Extra>(Extra const&) {
			static_assert(detail::is_agg_ref_v<Extra> || detail::is_named_find_v<Extra>,
			              "bumbledb find(): every trailing argument must be a named "
			              "head column — bdb::as<\"c\">(var) or a named aggregate "
			              "(bdb::sum / min / max / count / pack)");
			if (out.find_count == max_query_finds) {
				detail::rule_has_too_many_finds();
			}
			if constexpr (detail::is_named_find_v<Extra>) {
				using Var = typename Extra::var;
				out.finds[out.find_count] = find_data{
				    .name = Extra::column_name,
				    .form = find_form::variable,
				    .op = fold_form::sum,
				    .over = detail::var_term<Var>(),
				    .answer = Var::cls,
				    .has_over = true,
				    .classed = Var::classed,
				    .law = Var::law,
				};
			} else {
				out.finds[out.find_count] = detail::fold_find_of<Extra>();
			}
			++out.find_count;
		};
		(add_extra(extras), ...);
		return out;
	}

private:
	template<fixed_string Name, bool Negated, class... Binds>
	[[nodiscard]] consteval auto with_idb(Binds...) const -> rule_chain {
		static_assert((detail::is_idb_bind_v<Binds> && ...), "bumbledb idb(): every binding must be spelled "
		                                                     "bdb::bind<\"column\">(variable)");
		static_assert(sizeof...(Binds) <= max_query_finds, "bumbledb idb(): the bindings exceed the head width");
		auto atom = idb_atom_data{};
		atom.pred = detail::to_name_text(Name.view());
		atom.negated = Negated;
		[[maybe_unused]] auto const add = [&]<class Bind>() {
			using Var = typename Bind::var;
			static_assert(detail::is_qvar_v<Var>, "bumbledb bind(): the bound value must be a query "
			                                      "variable (vars.field) — idb bindings are variable terms "
			                                      "only");
			atom.binds[atom.bind_count] = idb_bind_data{
			    .column = Bind::column,
			    .variable = coord_ref{.relation = Var::relation_name, .field = Var::field_name},
			    .cls = Var::cls,
			    .classed = Var::classed,
			    .law = Var::law,
			};
			++atom.bind_count;
		};
		(add.template operator()<Binds>(), ...);
		auto next = *this;
		detail::record_idb(next.state, atom);
		return next;
	}
};

}
