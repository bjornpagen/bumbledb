export module bumbledb:predicate;

import std;
import :classify;
import :interval;
import :allen;
import :spec;
import :ir;
import :var;
import :param;
import :pattern;

export namespace bdb {

/**
 * Point membership: `point_in(t, w)` holds iff `w.start ≤ t < w.end`. The
 * point side is a param, a bound element-typed variable, or an integer
 * literal; the interval side is a bound interval variable. The stored
 * condition is interval-LEFT whatever the surface order (lowering.md
 * §4.2).
 */
template<class Point, class IntervalVar>
[[nodiscard]] consteval auto point_in(Point point, IntervalVar) -> cond_value {
	static_assert(detail::is_qvar_v<IntervalVar> &&
	                  (IntervalVar::cls.kind == value_kind::interval_u64 || IntervalVar::cls.kind == value_kind::interval_i64),
	              "bumbledb point_in(): the interval side must be an interval-typed "
	              "query variable (vars.window)");
	constexpr auto element = IntervalVar::cls.kind == value_kind::interval_u64 ? value_kind::u64 : value_kind::i64;

	auto out = cond_value{};
	out.data = leaf_condition(query_cmp::point_in, 0, detail::var_term<IntervalVar>(), {});
	if constexpr (detail::is_param_ref_v<Point>) {
		out.data.nodes[0].rhs = detail::param_term<Point>();
		auto use = param_use{};
		use.name = Point::name;
		use.form = param_form::point;
		use.domain = field_class{element, 0};
		out.uses[0] = use;
		out.use_count = 1;
	} else if constexpr (detail::is_qvar_v<Point>) {
		static_assert(Point::cls.kind == element,
		              detail::kind_mismatch_message<Point::relation_name, Point::field_name, Point::cls, IntervalVar::relation_name,
		                                            IntervalVar::field_name, field_class{element, 0}>("be the point of"));
		out.data.nodes[0].rhs = detail::var_term<Point>();
	} else {
		static_assert(std::integral<Point>, "bumbledb point_in(): the point side is a param, a bound "
		                                    "variable, or an integer literal");
		out.data.nodes[0].rhs = detail::literal_term(detail::scalar_literal(element, point));
	}
	return out;
}

/**
 * The interval-pair comparison (`ir::CmpOp::Allen`), satisfied iff the
 * pair's Allen classification is in the 13-bit mask. Sides are bound
 * interval variables, params (anchored by the variable sibling), or
 * interval literals. Only the element domain must match — mixed widths
 * compare freely (widths type storage, not comparison).
 */
template<class Left, class Right>
[[nodiscard]] consteval auto allen_in(Left left, allen_mask mask, Right right) -> cond_value {
	constexpr auto left_is_var = detail::is_qvar_v<Left>;
	constexpr auto right_is_var = detail::is_qvar_v<Right>;
	static_assert(left_is_var || right_is_var, "bumbledb allen_in(): at least one side must be a bound interval "
	                                           "variable (the anchor that types the comparison)");

	constexpr auto domain = [] {
		if constexpr (left_is_var) {
			return Left::cls;
		} else {
			return Right::cls;
		}
	}();
	static_assert(domain.kind == value_kind::interval_u64 || domain.kind == value_kind::interval_i64,
	              "bumbledb allen_in(): the variable side must be interval-typed");

	auto const side = [&]<class Side>(Side value) -> term_data {
		if constexpr (detail::is_qvar_v<Side>) {
			static_assert(Side::cls.kind == domain.kind, "bumbledb allen_in(): both interval sides must share one "
			                                             "element domain");
			return detail::var_term<Side>();
		} else if constexpr (detail::is_param_ref_v<Side>) {
			return detail::param_term<Side>();
		} else {
			static_assert(std::same_as<Side, interval<std::uint64_t>> || std::same_as<Side, interval<std::int64_t>>,
			              "bumbledb allen_in(): a literal side must be a "
			              "bdb::interval of the variable side's element domain");
			return detail::literal_term(detail::interval_literal(value));
		}
	};

	auto out = cond_value{};
	out.data = leaf_condition(query_cmp::allen, mask.bits(), side(left), side(right));
	if constexpr (detail::is_param_ref_v<Left>) {
		auto use = param_use{};
		use.name = Left::name;
		use.form = param_form::value;
		use.domain = domain;
		out.uses[out.use_count] = use;
		++out.use_count;
	}
	if constexpr (detail::is_param_ref_v<Right>) {
		auto use = param_use{};
		use.name = Right::name;
		use.form = param_form::value;
		use.domain = domain;
		out.uses[out.use_count] = use;
		++out.use_count;
	}
	return out;
}

}

namespace bdb::detail {

template<class Side>
[[nodiscard]] consteval auto side_is_term() -> bool {
	return is_qvar_v<Side> || is_param_ref_v<Side> || is_set_param_ref_v<Side> || is_measure_ref_v<Side>;
}

/**
 * The shared scalar-comparison constructor: sides are bound variables,
 * params, measures (order ops), or integral/bool literals. The anchoring
 * domain is a variable side's class, else the measure's u64 — it types
 * the param uses and tags the literals.
 */
template<query_cmp Op, class Left, class Right>
[[nodiscard]] consteval auto comparison_of(Left left, Right right) -> cond_value {
	constexpr auto ordered = Op == query_cmp::lt || Op == query_cmp::le || Op == query_cmp::gt || Op == query_cmp::ge;
	static_assert(side_is_term<Left>() || side_is_term<Right>(), "bumbledb comparison: at least one side must be a bound variable, "
	                                                             "a measure, or a param (two literals compare nothing)");
	static_assert((!is_measure_ref_v<Left> && !is_measure_ref_v<Right>) || ordered,
	              "bumbledb comparison: a duration/measure side is legal in order "
	              "comparisons only (lt/le/gt/ge)");
	static_assert((!is_set_param_ref_v<Left> && !is_set_param_ref_v<Right>) || Op == query_cmp::eq,
	              "bumbledb comparison: a set param is legal in atom bindings and "
	              "one side of eq only (ir::Term::ParamSet)");

	constexpr auto domain = [] {
		if constexpr (is_qvar_v<Left>) {
			return Left::cls;
		} else if constexpr (is_qvar_v<Right>) {
			return Right::cls;
		} else {
			return field_class{value_kind::u64, 0};
		}
	}();

	if constexpr (is_qvar_v<Left> && is_qvar_v<Right>) {
		static_assert(
		    Left::cls == Right::cls,
		    kind_mismatch_message<Left::relation_name, Left::field_name, Left::cls, Right::relation_name, Right::field_name, Right::cls>(
		        "join"));
		static_assert(Left::classed == Right::classed && (!Left::classed || Left::law == Right::law),
		              cross_class_message<Left::relation_name, Left::field_name, Left::classed, Left::law, Right::relation_name,
		                                  Right::field_name, Right::classed, Right::law>("join"));
	}
	if constexpr (ordered) {
		static_assert(domain.kind == value_kind::boolean || domain.kind == value_kind::u64 || domain.kind == value_kind::i64,
		              "bumbledb comparison: order comparisons take orderable scalar "
		              "sides only (bool/u64/i64/measure) — intervals compare through "
		              "bdb::allen and bdb::point_in");
	}

	auto out = cond_value{};
	auto const side = [&]<class Side>(Side value) -> term_data {
		if constexpr (is_qvar_v<Side>) {
			return var_term<Side>();
		} else if constexpr (is_measure_ref_v<Side>) {
			return measure_term<typename Side::over>();
		} else if constexpr (is_param_ref_v<Side>) {
			auto use = param_use{};
			use.name = Side::name;
			use.form = param_form::value;
			use.domain = domain;
			out.uses[out.use_count] = use;
			++out.use_count;
			return param_term<Side>();
		} else if constexpr (is_set_param_ref_v<Side>) {
			auto use = param_use{};
			use.name = Side::name;
			use.form = param_form::set;
			use.domain = domain;
			out.uses[out.use_count] = use;
			++out.use_count;
			return set_param_term<Side>();
		} else if constexpr (std::same_as<Side, bool>) {
			static_assert(domain.kind == value_kind::boolean, "bumbledb comparison: a bool literal needs a bool-typed "
			                                                  "sibling");
			return literal_term(scalar_literal(domain.kind, value));
		} else {
			static_assert(std::integral<Side>, "bumbledb comparison: a literal side must be integral "
			                                   "(strings/bytes/intervals bind through params)");
			static_assert(domain.kind == value_kind::u64 || domain.kind == value_kind::i64,
			              "bumbledb comparison: an integer literal needs a "
			              "u64/i64/measure sibling to type it");
			return literal_term(scalar_literal(domain.kind, value));
		}
	};
	out.data = leaf_condition(Op, 0, side(left), side(right));
	return out;
}

}

export namespace bdb {

template<class Left, class Right>
[[nodiscard]] consteval auto eq(Left left, Right right) -> cond_value {
	return detail::comparison_of<query_cmp::eq>(left, right);
}

template<class Left, class Right>
[[nodiscard]] consteval auto ne(Left left, Right right) -> cond_value {
	return detail::comparison_of<query_cmp::ne>(left, right);
}

template<class Left, class Right>
[[nodiscard]] consteval auto lt(Left left, Right right) -> cond_value {
	return detail::comparison_of<query_cmp::lt>(left, right);
}

template<class Left, class Right>
[[nodiscard]] consteval auto le(Left left, Right right) -> cond_value {
	return detail::comparison_of<query_cmp::le>(left, right);
}

template<class Left, class Right>
[[nodiscard]] consteval auto gt(Left left, Right right) -> cond_value {
	return detail::comparison_of<query_cmp::gt>(left, right);
}

template<class Left, class Right>
[[nodiscard]] consteval auto ge(Left left, Right right) -> cond_value {
	return detail::comparison_of<query_cmp::ge>(left, right);
}

namespace detail {

template<condition_form Form, class... Conds>
[[nodiscard]] consteval auto combine_conditions(Conds const&... conds) -> cond_value {
	static_assert((std::same_as<std::remove_cvref_t<Conds>, cond_value> && ...),
	              "bumbledb And/Or: every argument must be a condition value "
	              "(bdb::eq / bdb::And / bdb::Or / ...)");
	constexpr auto n = sizeof...(Conds);
	if (1 + n > max_query_conditions) {
		rule_has_too_many_conditions();
	}
	auto out = cond_value{};
	out.data.nodes[0].form = Form;
	out.data.nodes[0].child_begin = 1;
	out.data.nodes[0].child_count = n;
	auto desc_at = std::size_t{1 + n};
	auto child_index = std::size_t{0};
	auto absorb = [&](cond_value const& child) {
		if (desc_at + (child.data.node_count > 0 ? child.data.node_count - 1 : 0) > max_query_conditions) {
			rule_has_too_many_conditions();
		}
		auto const root_at = std::size_t{1 + child_index};
		auto const desc_begin = desc_at;
		auto const map = [&](std::size_t old) -> std::size_t { return old == 0 ? root_at : desc_begin + old - 1; };
		auto copy_node = [&](condition_node node) {
			if (node.child_count != 0) {
				node.child_begin = map(node.child_begin);
			}
			return node;
		};
		out.data.nodes[root_at] = copy_node(child.data.nodes[0]);
		for (auto k = std::size_t{1}; k != child.data.node_count; ++k) {
			out.data.nodes[desc_at] = copy_node(child.data.nodes[k]);
			++desc_at;
		}
		for (auto u = std::size_t{0}; u != child.use_count; ++u) {
			if (out.use_count == out.uses.size()) {
				query_has_too_many_params();
			}
			out.uses[out.use_count] = child.uses[u];
			++out.use_count;
		}
		++child_index;
	};
	(absorb(conds), ...);
	out.data.node_count = desc_at;
	if (out.data.node_count > max_query_conditions) {
		rule_has_too_many_conditions();
	}
	return out;
}

}

/**
 * Conjunction node of the input condition grammar (`ConditionTree::And`).
 * The rule's condition list is already a conjunction — `And` exists for
 * nesting under `Or`, and the empty combination keeps the IR's algebraic
 * reading (`And([])` is true). Named `And` because `and` is a C++
 * alternative token.
 */
template<class... Conds>
[[nodiscard]] consteval auto And(Conds const&... conds) -> cond_value {
	return detail::combine_conditions<condition_form::and_node>(conds...);
}

/**
 * Disjunction node of the input condition grammar (`ConditionTree::Or`) —
 * the one place the surface admits a nested OR; validation distributes it
 * to DNF rules engine-side. `Or([])` keeps its algebraic reading (false).
 * Named `Or` because `or` is a C++ alternative token.
 */
template<class... Conds>
[[nodiscard]] consteval auto Or(Conds const&... conds) -> cond_value {
	return detail::combine_conditions<condition_form::or_node>(conds...);
}

}
