/**
 * The coordinate-typed query builder: rules lower during constant
 * evaluation to the flattened `query_ir` that :query_view presents
 * to the bridge, mirroring the TS builder's lowering exactly (lowering.md
 * §4.2). Also the product synthesis off the query value — the answer-row
 * product (`row_of`) and the params product (`params_of`).
 */
export module bumbledb:query;

import std;
import :name;
import :classify;
import :interval;
import :allen;
import :spec;
import :ir;
import :var;
import :param;
import :aggregate;
import :pattern;
import :predicate;
import :rule;
import :lower;
import :schema;

export namespace bdb {

/**
 * What the rule lambda receives: variable mints, params, measures,
 * aggregates, and the chain starters.
 */
template<class S>
struct rule_scope {
	/** Mints the relation's variable product (member access only — :var). */
	template<class Facade>
	[[nodiscard]] consteval auto vars(Facade) const -> vars_of<S, Facade> {
		static_assert(detail::is_query_member<Facade>(), "bumbledb r.vars(): the argument must be a relation facade "
		                                                 "(bdb::relation<...> or bdb::closed<...>)");
		static_assert(detail::facade_in_schema<S, Facade>(), detail::foreign_relation_message<S, Facade>());
		return {};
	}

	/**
	 * The member twin of `bdb::param` (spell it `r.template param<"t">()`
	 * — the grammar's price for a dependent template-name).
	 */
	template<fixed_string Name>
	[[nodiscard]] consteval auto param() const -> param_ref<Name> {
		return {};
	}

	/** The measure of an interval variable (u64 point count). */
	template<class Var>
	[[nodiscard]] consteval auto duration(Var) const -> measure_ref<Var> {
		static_assert(detail::is_qvar_v<Var> && (Var::cls.kind == value_kind::interval_u64 || Var::cls.kind == value_kind::interval_i64),
		              "bumbledb r.duration(): the argument must be an interval-typed "
		              "query variable — a duration is an interval's measure");
		return {};
	}

	/** The member twin of `bdb::sum` (spell it `r.template sum<...>`). */
	template<fixed_string Name, class Var>
	[[nodiscard]] consteval auto sum(measure_ref<Var>) const -> agg_ref<Name, fold_form::sum, measure_ref<Var>> {
		return {};
	}

	/**
	 * Member twins of `bdb::And` / `bdb::Or` (C++ cannot name them `and` /
	 * `or` — those are alternative tokens).
	 */
	template<class... Conds>
	[[nodiscard]] consteval auto And(Conds const&... conds) const -> cond_value {
		return bdb::And(conds...);
	}

	template<class... Conds>
	[[nodiscard]] consteval auto Or(Conds const&... conds) const -> cond_value {
		return bdb::Or(conds...);
	}

	/** Starts the rule body with one positive EDB atom. */
	template<class Facade>
	[[nodiscard]] consteval auto match(Facade facade, match_pattern_of<S, Facade> const& pattern) const -> rule_chain<S, Facade> {
		return rule_chain<S>{}.match(facade, pattern);
	}

	/**
	 * Starts the rule body with one positive derived-table atom (an
	 * interior atom grounds its variables — the finished set's identity
	 * projection needs no re-grounding join).
	 */
	template<fixed_string Name, class... Binds>
	[[nodiscard]] consteval auto interior(Binds... binds) const -> rule_chain<S> {
		return rule_chain<S>{}.template interior<Name>(binds...);
	}
};

/**
 * Tagged pack of base arms for `query_value::reach`.
 */
template<class... Builds>
struct base {
	std::tuple<Builds...> builds;
	consteval base(Builds... builds_) : builds{std::tuple{builds_...}} {}
};

/**
 * Tagged pack of rec arms for `query_value::reach`.
 */
template<class... Builds>
struct rec {
	std::tuple<Builds...> builds;
	consteval rec(Builds... builds_) : builds{std::tuple{builds_...}} {}
};

/**
 * A whole query as one structural literal: the lowered IR rides the value
 * (NTTP-friendly — `db.prepare<DownAt>()`), the schema ties the type.
 * Phase lives in the template: `NI` interiors, `HasRec` the rec arm,
 * `NR` main rules. `.interior` / `.reach` exist only before rec and
 * main; `prepare` requires `NR >= 1`.
 */
template<class S, std::size_t NI = 0, bool HasRec = false, std::size_t NR = 0>
struct query_value : query_ir<NI, HasRec, NR> {
	template<class Build>
	[[nodiscard]] consteval auto rule(Build build) const -> query_value<S, NI, HasRec, NR + 1> {
		if (NR == max_query_rules) {
			detail::query_has_too_many_rules();
		}
		auto const result = build(rule_scope<S>{});
		static_assert(std::same_as<std::remove_cvref_t<decltype(result)>, rule_data>,
		              "bumbledb query.rule(): the rule body must end in .find(...)");
		auto next = query_value<S, NI, HasRec, NR + 1>{};
		next.interiors = this->interiors;
		if constexpr (HasRec) {
			next.rec = this->rec;
		}
		next.head_count = this->head_count;
		next.head = this->head;
		next.param_count = this->param_count;
		next.params = this->params;
		for (auto index = std::size_t{0}; index != NR; ++index) {
			next.rules[index] = this->rules[index];
		}
		if constexpr (std::same_as<std::remove_cvref_t<decltype(result)>, rule_data>) {
			detail::append_rule(next, result);
		}
		return next;
	}

	/**
	 * Declares one named interior (finite CQ, eval once). Multiple
	 * builders = set union. Names unique. Ill-formed after recursive
	 * or after a main rule.
	 */
	template<fixed_string Name, class... Builds>
	    requires (!HasRec && NR == 0)
	[[nodiscard]] consteval auto interior(Builds... builds) const -> query_value<S, NI + 1, false, 0> {
		static_assert(sizeof...(Builds) >= 1, "bumbledb query.interior(): an interior needs at least one rule");
		static_assert(sizeof...(Builds) <= max_query_rules, "bumbledb query.interior(): too many rules for one interior");
		auto const name = detail::to_name_text(Name.view());
		for (auto index = std::size_t{0}; index != NI; ++index) {
			if (this->interiors[index].name == name) {
				detail::interior_names_must_be_distinct();
			}
		}

		auto rules = std::array<rule_data, max_query_rules>{};
		auto rule_count = std::size_t{0};
		auto const add = [&](auto const& build) {
			auto const result = build(rule_scope<S>{});
			static_assert(std::same_as<std::remove_cvref_t<decltype(result)>, rule_data>,
			              "bumbledb query.interior(): the rule body must end in .find(...)");
			if constexpr (std::same_as<std::remove_cvref_t<decltype(result)>, rule_data>) {
				rules[rule_count] = result;
				++rule_count;
			}
		};
		(add(builds), ...);

		auto next = query_value<S, NI + 1, false, 0>{};
		for (auto index = std::size_t{0}; index != NI; ++index) {
			next.interiors[index] = this->interiors[index];
		}
		next.param_count = this->param_count;
		next.params = this->params;

		for (auto index = std::size_t{0}; index != rule_count; ++index) {
			detail::fold_uses(next, rules[index].state);
		}

		auto const& seal = rules[0];
		next.interiors[NI].name = name;
		next.interiors[NI].head_count = seal.find_count;
		for (auto column = std::size_t{0}; column != seal.find_count; ++column) {
			next.interiors[NI].head[column] = seal.finds[column];
		}
		for (auto index = std::size_t{1}; index != rule_count; ++index) {
			detail::align_head(next.interiors[NI].head_count, next.interiors[NI].head, rules[index]);
		}

		next.interiors[NI].rule_count = rule_count;
		auto lookup = *this;
		lookup.param_count = next.param_count;
		lookup.params = next.params;
		for (auto index = std::size_t{0}; index != rule_count; ++index) {
			next.interiors[NI].rules[index] = detail::lower_rule(lookup, rules[index], detail::no_interior);
		}
		return next;
	}

	/**
	 * Declares the (at most one) linear rec: two tagged packs, base then
	 * rec. Main rules follow via `.rule`. Ill-formed if a rec is already
	 * present or a main rule already exists.
	 */
	template<fixed_string Name, class... BaseBuilds, class... RecBuilds>
	    requires (!HasRec && NR == 0)
	[[nodiscard]] consteval auto reach(bdb::base<BaseBuilds...> const& bases, bdb::rec<RecBuilds...> const& recs) const
	    -> query_value<S, NI, true, 0> {
		static_assert(sizeof...(BaseBuilds) >= 1, "bumbledb query.reach(): needs at least one base rule");
		static_assert(sizeof...(RecBuilds) >= 1, "bumbledb query.reach(): needs at least one rec rule");
		static_assert(sizeof...(BaseBuilds) + sizeof...(RecBuilds) <= max_query_rules,
		              "bumbledb query.reach(): base and rec arms together exceed max_query_rules");
		auto const name = detail::to_name_text(Name.view());
		for (auto index = std::size_t{0}; index != NI; ++index) {
			if (this->interiors[index].name == name) {
				detail::interior_names_must_be_distinct();
			}
		}

		auto base_rules = std::array<rule_data, max_query_rules>{};
		auto base_count = std::size_t{0};
		auto rec_rules = std::array<rule_data, max_query_rules>{};
		auto rec_count = std::size_t{0};
		auto const add_base = [&](auto const& build) {
			auto const result = build(rule_scope<S>{});
			static_assert(std::same_as<std::remove_cvref_t<decltype(result)>, rule_data>,
			              "bumbledb query.reach(): a base rule body must end in .find(...)");
			if constexpr (std::same_as<std::remove_cvref_t<decltype(result)>, rule_data>) {
				base_rules[base_count] = result;
				++base_count;
			}
		};
		auto const add_rec = [&](auto const& build) {
			auto const result = build(rule_scope<S>{});
			static_assert(std::same_as<std::remove_cvref_t<decltype(result)>, rule_data>,
			              "bumbledb query.reach(): a rec rule body must end in .find(...)");
			if constexpr (std::same_as<std::remove_cvref_t<decltype(result)>, rule_data>) {
				rec_rules[rec_count] = result;
				++rec_count;
			}
		};
		std::apply(
		    [&](auto const&... builds) {
			    (add_base(builds), ...);
		    },
		    bases.builds);
		std::apply(
		    [&](auto const&... builds) {
			    (add_rec(builds), ...);
		    },
		    recs.builds);
		if (base_count == 0) {
			detail::reach_needs_at_least_one_base_rule();
		}
		if (rec_count == 0) {
			detail::reach_needs_at_least_one_rec_rule();
		}

		auto next = query_value<S, NI, true, 0>{};
		next.interiors = this->interiors;
		next.param_count = this->param_count;
		next.params = this->params;
		next.rec.name = name;

		for (auto index = std::size_t{0}; index != base_count; ++index) {
			detail::fold_uses(next, base_rules[index].state);
		}
		for (auto index = std::size_t{0}; index != rec_count; ++index) {
			detail::fold_uses(next, rec_rules[index].state);
		}

		auto const& seal = base_rules[0];
		next.rec.head_count = seal.find_count;
		for (auto column = std::size_t{0}; column != seal.find_count; ++column) {
			next.rec.head[column] = seal.finds[column];
		}
		for (auto index = std::size_t{1}; index != base_count; ++index) {
			detail::align_head(next.rec.head_count, next.rec.head, base_rules[index]);
		}
		for (auto index = std::size_t{0}; index != rec_count; ++index) {
			detail::align_head(next.rec.head_count, next.rec.head, rec_rules[index]);
		}

		auto const self = NI;
		next.rec.base_count = base_count;
		next.rec.rec_count = rec_count;
		for (auto index = std::size_t{0}; index != base_count; ++index) {
			next.rec.rules[index] = detail::lower_rule(next, base_rules[index], self);
		}
		for (auto index = std::size_t{0}; index != rec_count; ++index) {
			next.rec.rules[base_count + index] = detail::lower_rule(next, rec_rules[index], self);
		}
		return next;
	}
};

/**
 * The query entry point: the schema VALUE selects the TYPE, and the type
 * carries everything the elaboration needs.
 */
template<Theory S>
[[nodiscard]] consteval auto query(S const&) -> query_value<S> {
	return {};
}

}

namespace bdb::detail {

/* PIN(reflect-using-decl): route ^^ through a template parameter so the alias resolves before ^^ applies */
template<class T>
inline constexpr auto query_type_reflection = ^^T;

/** String and bytes answer members are owned copies taken at decode. */
[[nodiscard]] consteval auto answer_type_of(field_class cls) -> std::meta::info {
	switch (cls.kind) {
	case value_kind::boolean:
		return query_type_reflection<bool>;
	case value_kind::u64:
		return query_type_reflection<std::uint64_t>;
	case value_kind::i64:
		return query_type_reflection<std::int64_t>;
	case value_kind::string:
		return query_type_reflection<std::string>;
	case value_kind::fixed_bytes:
		return query_type_reflection<std::vector<std::byte>>;
	case value_kind::interval_u64:
		return query_type_reflection<interval<std::uint64_t>>;
	case value_kind::interval_i64:
		break;
	}
	return query_type_reflection<interval<std::int64_t>>;
}

/**
 * A runtime set param's member type: a borrowed sequence of the anchored
 * element type (the empty span is legal and matches nothing). Borrow
 * contract: the span (and any string/bytes elements) must stay alive for
 * the execute call only — the bridge copies before returning.
 */
[[nodiscard]] consteval auto set_type_of(field_class cls) -> std::meta::info {
	switch (cls.kind) {
	case value_kind::boolean:
		return query_type_reflection<std::span<bool const>>;
	case value_kind::u64:
		return query_type_reflection<std::span<std::uint64_t const>>;
	case value_kind::i64:
		return query_type_reflection<std::span<std::int64_t const>>;
	case value_kind::string:
		return query_type_reflection<std::span<std::string_view const>>;
	case value_kind::fixed_bytes:
		return query_type_reflection<std::span<std::span<std::byte const> const>>;
	case value_kind::interval_u64:
		return query_type_reflection<std::span<interval<std::uint64_t> const>>;
	case value_kind::interval_i64:
		break;
	}
	return query_type_reflection<std::span<interval<std::int64_t> const>>;
}

/**
 * A runtime scalar param's member type: string/bytes borrow the caller's
 * storage for the execute call only — the bridge copies before returning.
 */
[[nodiscard]] consteval auto param_scalar_type_of(field_class cls) -> std::meta::info {
	switch (cls.kind) {
	case value_kind::boolean:
		return query_type_reflection<bool>;
	case value_kind::u64:
		return query_type_reflection<std::uint64_t>;
	case value_kind::i64:
		return query_type_reflection<std::int64_t>;
	case value_kind::string:
		return query_type_reflection<std::string_view>;
	case value_kind::fixed_bytes:
		return query_type_reflection<std::span<std::byte const>>;
	case value_kind::interval_u64:
		return query_type_reflection<interval<std::uint64_t>>;
	case value_kind::interval_i64:
		break;
	}
	return query_type_reflection<interval<std::int64_t>>;
}

[[nodiscard]] consteval auto param_type_of(param_data const& parameter) -> std::meta::info {
	if (parameter.form == param_form::set) {
		return set_type_of(parameter.domain);
	}
	return param_scalar_type_of(parameter.domain);
}

/**
 * The synthesized answer-row product: one member per head column, named
 * per column, typed by the column's answer class.
 */
template<auto Query>
struct query_row_types {
	struct Row;
	consteval {
		auto specs = std::vector<std::meta::info>{};
		for (auto index = std::size_t{0}; index != Query.head_count; ++index) {
			specs.push_back(std::meta::data_member_spec(answer_type_of(Query.head[index].answer),
			                                            {.name = spec_name(Query.head[index].name.view())}));
		}
		std::meta::define_aggregate(^^Row, specs);
	}
};

/**
 * The synthesized params product: one member per registered param in
 * registry order (= positional bind order), named per param, typed by the
 * anchored domain — a wrong name or type at `execute` is a compile error.
 * Membership entries are skipped: their sets were pre-resolved at build,
 * and execution injects the frozen set positionally.
 */
template<auto Query>
struct query_params_types {
	struct Params;
	consteval {
		auto specs = std::vector<std::meta::info>{};
		for (auto index = std::size_t{0}; index != Query.param_count; ++index) {
			if (Query.params[index].form == param_form::membership) {
				continue;
			}
			specs.push_back(std::meta::data_member_spec(param_type_of(Query.params[index]),
			                                            {.name = spec_name(Query.params[index].name.view())}));
		}
		std::meta::define_aggregate(^^Params, specs);
	}
};

}

export namespace bdb {

/** The query's synthesized answer-row product. */
template<auto Query>
using row_of = typename detail::query_row_types<Query>::Row;

/** The query's synthesized params product. */
template<auto Query>
using params_of = typename detail::query_params_types<Query>::Params;

}
