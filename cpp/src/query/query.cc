/**
 * The coordinate-typed query builder: rules lower during constant
 * evaluation to the flattened `query_ir` that :foreign_program presents
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
	 * Starts the rule body with one positive recursive atom (an idb atom
	 * grounds its variables — the finished set's identity projection
	 * needs no re-grounding join).
	 */
	template<fixed_string Name, class... Binds>
	[[nodiscard]] consteval auto idb(pred_tag<Name> tag, Binds... binds) const -> rule_chain<S> {
		return rule_chain<S>{}.idb(tag, binds...);
	}
};

/**
 * A whole query as one structural literal: the lowered IR rides the value
 * (NTTP-friendly — `db.prepare<DownAt>()`), the schema ties the type.
 * `.rule` appends one rule; every rule must derive the same head.
 */
template<class S>
struct query_value {
	query_ir ir{};

	template<class Build>
	[[nodiscard]] consteval auto rule(Build build) const -> query_value {
		auto const result = build(rule_scope<S>{});
		static_assert(std::same_as<std::remove_cvref_t<decltype(result)>, rule_data>,
		              "bumbledb query.rule(): the rule body must end in .find(...)");
		auto next = *this;
		if constexpr (std::same_as<std::remove_cvref_t<decltype(result)>, rule_data>) {
			detail::append_rule(next.ir, result);
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

/** String and bytes answer members are borrowed from the answers carrier. */
[[nodiscard]] consteval auto answer_type_of(field_class cls) -> std::meta::info {
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

[[nodiscard]] consteval auto param_type_of(param_data const& parameter) -> std::meta::info {
	if (parameter.shape == param_shape::set) {
		return set_type_of(parameter.domain);
	}
	return answer_type_of(parameter.domain);
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
