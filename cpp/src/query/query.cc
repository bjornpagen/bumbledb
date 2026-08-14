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
 * Tagged pack of base arms for `query_value::recursive`.
 */
template<class... Builds>
struct base {
	std::tuple<Builds...> builds;
	consteval base(Builds... builds_) : builds{std::tuple{builds_...}} {}
};

/**
 * Tagged pack of rec arms for `query_value::recursive`.
 */
template<class... Builds>
struct rec {
	std::tuple<Builds...> builds;
	consteval rec(Builds... builds_) : builds{std::tuple{builds_...}} {}
};

/**
 * A whole query as one structural literal: the lowered IR rides the value
 * (NTTP-friendly — `db.prepare<DownAt>()`), the schema ties the type.
 * Named interiors are a variadic pack (`NI`); `.rule` appends one main
 * rule; every main rule must derive the same head.
 */
template<class S, std::size_t NI = 0>
struct query_value {
	std::array<interior_ir, NI> interiors{};
	bool has_rec = false;
	rec_ir rec{};
	query_ir ir{};

	template<class Build>
	[[nodiscard]] consteval auto rule(Build build) const -> query_value {
		auto const result = build(rule_scope<S>{});
		static_assert(std::same_as<std::remove_cvref_t<decltype(result)>, rule_data>,
		              "bumbledb query.rule(): the rule body must end in .find(...)");
		auto next = *this;
		if constexpr (std::same_as<std::remove_cvref_t<decltype(result)>, rule_data>) {
			detail::append_rule(next.ir, next.interiors, next.has_rec, next.rec, result);
		}
		return next;
	}

	/**
	 * Declares one named interior (finite CQ, eval once). Multiple
	 * builders = set union. Names unique; interior after recursive or
	 * after a main rule is a consteval error.
	 */
	template<fixed_string Name, class... Builds>
	[[nodiscard]] consteval auto interior(Builds... builds) const -> query_value<S, NI + 1> {
		static_assert(sizeof...(Builds) >= 1, "bumbledb query.interior(): an interior needs at least one rule");
		static_assert(sizeof...(Builds) <= max_query_rules, "bumbledb query.interior(): too many rules for one interior");
		if (has_rec) {
			detail::interior_after_recursive();
		}
		if (ir.rule_count != 0) {
			detail::interior_or_recursive_after_a_main_rule();
		}
		auto const name = detail::to_name_text(Name.view());
		for (auto index = std::size_t{0}; index != NI; ++index) {
			if (interiors[index].name == name) {
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

		auto next = query_value<S, NI + 1>{};
		for (auto index = std::size_t{0}; index != NI; ++index) {
			next.interiors[index] = interiors[index];
		}
		next.ir = ir;

		for (auto index = std::size_t{0}; index != rule_count; ++index) {
			detail::fold_uses(next.ir, rules[index].state);
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

		auto const tables = detail::derived_tables<NI>{
		    .interiors = interiors,
		    .has_rec = false,
		    .rec = rec,
		};
		next.interiors[NI].rule_count = rule_count;
		for (auto index = std::size_t{0}; index != rule_count; ++index) {
			next.interiors[NI].rules[index] = detail::lower_rule(next.ir, tables, rules[index], detail::no_interior);
		}
		return next;
	}

	/**
	 * Declares the (at most one) linear rec: two tagged packs, base then
	 * rec. Main rules follow via `.rule`. A second recursive, or
	 * recursive after a main rule, is a consteval error.
	 */
	template<fixed_string Name, class... BaseBuilds, class... RecBuilds>
	[[nodiscard]] consteval auto recursive(bdb::base<BaseBuilds...> const& bases, bdb::rec<RecBuilds...> const& recs) const
	    -> query_value {
		static_assert(sizeof...(BaseBuilds) >= 1, "bumbledb query.recursive(): needs at least one base rule");
		static_assert(sizeof...(RecBuilds) >= 1, "bumbledb query.recursive(): needs at least one rec rule");
		static_assert(sizeof...(BaseBuilds) + sizeof...(RecBuilds) <= max_query_rules,
		              "bumbledb query.recursive(): base and rec arms together exceed max_query_rules");
		if (has_rec) {
			detail::a_second_recursive_is_refused();
		}
		if (ir.rule_count != 0) {
			detail::interior_or_recursive_after_a_main_rule();
		}
		auto const name = detail::to_name_text(Name.view());
		for (auto index = std::size_t{0}; index != NI; ++index) {
			if (interiors[index].name == name) {
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
			              "bumbledb query.recursive(): a base rule body must end in .find(...)");
			if constexpr (std::same_as<std::remove_cvref_t<decltype(result)>, rule_data>) {
				base_rules[base_count] = result;
				++base_count;
			}
		};
		auto const add_rec = [&](auto const& build) {
			auto const result = build(rule_scope<S>{});
			static_assert(std::same_as<std::remove_cvref_t<decltype(result)>, rule_data>,
			              "bumbledb query.recursive(): a rec rule body must end in .find(...)");
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
			detail::recursive_needs_at_least_one_base_rule();
		}
		if (rec_count == 0) {
			detail::recursive_needs_at_least_one_rec_rule();
		}

		auto next = *this;
		next.has_rec = true;
		next.rec.name = name;

		for (auto index = std::size_t{0}; index != base_count; ++index) {
			detail::fold_uses(next.ir, base_rules[index].state);
		}
		for (auto index = std::size_t{0}; index != rec_count; ++index) {
			detail::fold_uses(next.ir, rec_rules[index].state);
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

		auto sealed = next.rec;
		auto const tables = detail::derived_tables<NI>{
		    .interiors = next.interiors,
		    .has_rec = true,
		    .rec = sealed,
		};
		auto const self = NI;
		next.rec.base_count = base_count;
		for (auto index = std::size_t{0}; index != base_count; ++index) {
			next.rec.base[index] = detail::lower_rule(next.ir, tables, base_rules[index], self);
		}
		next.rec.rec_count = rec_count;
		for (auto index = std::size_t{0}; index != rec_count; ++index) {
			next.rec.rec[index] = detail::lower_rule(next.ir, tables, rec_rules[index], self);
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
	if (parameter.shape == param_shape::set) {
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
		for (auto index = std::size_t{0}; index != Query.ir.head_count; ++index) {
			specs.push_back(std::meta::data_member_spec(answer_type_of(Query.ir.head[index].answer),
			                                            {.name = spec_name(Query.ir.head[index].name.view())}));
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
		for (auto index = std::size_t{0}; index != Query.ir.param_count; ++index) {
			if (Query.ir.params[index].membership) {
				continue;
			}
			specs.push_back(std::meta::data_member_spec(param_type_of(Query.ir.params[index]),
			                                            {.name = spec_name(Query.ir.params[index].name.view())}));
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
