export module bumbledb:aggregate;

import std;
import :name;
import :classify;
import :ir;
import :var;
import :pattern;

export namespace bdb {

/**
 * One named aggregate head column. The name rides the type because a
 * designated-init head cannot mint new member names the way a TS object
 * literal can — the aggregate column's name is the one datum the pattern
 * product cannot express. `Over` is a qvar, a measure_ref, or void
 * (nullary count).
 */
template<fixed_string Name, fold_form Op, class Over>
struct agg_ref {
	using over = Over;
	static constexpr name_text column_name = detail::to_name_text(Name.view());
	static constexpr fold_form op = Op;
};

/**
 * One named variable head column: the head column name decoupled from the
 * field name. Recursive predicates whose rules match different relations
 * need it. Passed to `.find` among the trailing columns.
 */
template<fixed_string Name, class Var>
struct named_find {
	using var = Var;
	static constexpr name_text column_name = detail::to_name_text(Name.view());
};

}

namespace bdb::detail {

template<class T>
inline constexpr bool is_agg_ref_v = false;

template<fixed_string Name, fold_form Op, class Over>
inline constexpr bool is_agg_ref_v<agg_ref<Name, Op, Over>> = true;

template<class T>
inline constexpr bool is_named_find_v = false;

template<fixed_string Name, class Var>
inline constexpr bool is_named_find_v<named_find<Name, Var>> = true;

template<class Fold>
[[nodiscard]] consteval auto fold_find_of() -> find_data {
	auto out = find_data{};
	out.name = Fold::column_name;
	out.op = Fold::op;
	using Over = typename Fold::over;
	if constexpr (std::same_as<Over, void>) {
		out.form = find_form::aggregate;
		out.has_over = false;
		out.answer = field_class{value_kind::u64, 0};
	} else if constexpr (is_measure_ref_v<Over>) {
		out.form = find_form::aggregate_measure;
		out.has_over = true;
		out.over = var_term<typename Over::over>();
		out.answer = field_class{value_kind::u64, 0};
	} else {
		out.form = find_form::aggregate;
		out.has_over = true;
		out.over = var_term<Over>();
		out.answer = Over::cls;
	}
	return out;
}

template<class Var>
[[nodiscard]] consteval auto is_numeric_var() -> bool {
	return is_qvar_v<Var> && (Var::cls.kind == value_kind::u64 || Var::cls.kind == value_kind::i64);
}

template<class Var>
[[nodiscard]] consteval auto is_orderable_var() -> bool {
	return is_qvar_v<Var> && (Var::cls.kind == value_kind::boolean || Var::cls.kind == value_kind::u64 || Var::cls.kind == value_kind::i64);
}

}

export namespace bdb {

/**
 * A named variable head column: the head name decoupled from the field
 * name (recursive heads need it).
 */
template<fixed_string Name, class Var>
[[nodiscard]] consteval auto as(Var) -> named_find<Name, Var> {
	static_assert(detail::is_qvar_v<Var>, "bumbledb as(): the argument must be a query variable "
	                                      "(vars.field)");
	return {};
}

/** A named sum of the measure of an interval variable. */
template<fixed_string Name, class Var>
[[nodiscard]] consteval auto sum(measure_ref<Var>) -> agg_ref<Name, fold_form::sum, measure_ref<Var>> {
	return {};
}

/** Exact checked sum into a wide accumulator. */
template<fixed_string Name, class Var>
[[nodiscard]] consteval auto sum(Var) -> agg_ref<Name, fold_form::sum, Var> {
	static_assert(detail::is_numeric_var<Var>(), "bumbledb sum(): the input is a numeric (u64/i64) variable or "
	                                             "r.duration(interval variable) — sum over bool is refused");
	return {};
}

template<fixed_string Name, class Var>
[[nodiscard]] consteval auto min(measure_ref<Var>) -> agg_ref<Name, fold_form::min, measure_ref<Var>> {
	return {};
}

template<fixed_string Name, class Var>
[[nodiscard]] consteval auto min(Var) -> agg_ref<Name, fold_form::min, Var> {
	static_assert(detail::is_orderable_var<Var>(), "bumbledb min(): the input is an orderable (bool/u64/i64) "
	                                               "variable or r.duration(interval variable)");
	return {};
}

template<fixed_string Name, class Var>
[[nodiscard]] consteval auto max(measure_ref<Var>) -> agg_ref<Name, fold_form::max, measure_ref<Var>> {
	return {};
}

template<fixed_string Name, class Var>
[[nodiscard]] consteval auto max(Var) -> agg_ref<Name, fold_form::max, Var> {
	static_assert(detail::is_orderable_var<Var>(), "bumbledb max(): the input is an orderable (bool/u64/i64) "
	                                               "variable or r.duration(interval variable)");
	return {};
}

/** The nullary count: |the group's set of distinct full bindings|, u64. */
template<fixed_string Name>
[[nodiscard]] consteval auto count() -> agg_ref<Name, fold_form::count, void> {
	return {};
}

/**
 * The coalescing fold (`ir::AggOp::Pack`): the maximal disjoint half-open
 * segments of the union of the group's interval point sets —
 * relation-shaped, one answer row per (group, maximal segment). At most
 * one pack per find, never beside another aggregate (judged at rule
 * assembly).
 */
template<fixed_string Name, class Var>
[[nodiscard]] consteval auto pack(Var) -> agg_ref<Name, fold_form::pack, Var> {
	static_assert(detail::is_qvar_v<Var> && (Var::cls.kind == value_kind::interval_u64 || Var::cls.kind == value_kind::interval_i64),
	              "bumbledb pack(): the input must be an interval-typed query "
	              "variable — pack coalesces interval point sets");
	return {};
}

}
