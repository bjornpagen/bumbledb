// :aggregate — the named head columns (TODO_CPP §11–§12): named variable
// columns (`bdb::as<"c">(vars.id)`) and named aggregates over variables
// or the measure. The NAME is carried in the type because a C++
// designated-init head cannot mint new member names the way a TS object
// literal can; the aggregate column's name is the one datum the pattern
// product cannot express.
export module bumbledb:aggregate;

import std;
import :name;
import :classify;
import :ir;
import :var;
import :pattern;

export namespace bdb {

/// One named aggregate head column — `bdb::sum<"downtime">(r.duration(w))`,
/// `bdb::sum<"total">(vars.minor)`, `bdb::count<"n">()`,
/// `bdb::pack<"free">(vars.span)`, `bdb::arg_max<"top">(v, key)`.
/// The NAME is carried here because a C++ designated-init head cannot
/// mint new member names the way a TS object literal can; the aggregate
/// column's name is the one datum the pattern product cannot express.
/// `Over` is a qvar, a measure_ref, or void (nullary count); `Key` a qvar
/// or measure_ref on the Arg ops, void elsewhere.
template<fixed_string Name, fold_form Op, class Over, class Key>
struct agg_ref {
    using over = Over;
    using key = Key;
    static constexpr name_text column_name = detail::to_name_text(Name.view());
    static constexpr fold_form op = Op;
};

/// One NAMED variable head column — `bdb::as<"c">(vars.id)`: the head
/// column name decoupled from the field name (recursive predicates whose
/// rules match different relations need it; the TS object-literal head
/// names freely). Passed to `.find` among the trailing columns.
template<fixed_string Name, class Var>
struct named_find {
    using var = Var;
    static constexpr name_text column_name = detail::to_name_text(Name.view());
};

} // namespace bdb

namespace bdb::detail {

template<class T>
inline constexpr bool is_agg_ref_v = false;

template<fixed_string Name, fold_form Op, class Over, class Key>
inline constexpr bool is_agg_ref_v<agg_ref<Name, Op, Over, Key>> = true;

template<class T>
inline constexpr bool is_named_find_v = false;

template<fixed_string Name, class Var>
inline constexpr bool is_named_find_v<named_find<Name, Var>> = true;

/// One named aggregate's find_data (the type walls ran on the mint).
template<class Fold>
consteval auto fold_find_of() -> find_data {
    auto out = find_data{};
    out.name = Fold::column_name;
    out.op = Fold::op;
    using Over = typename Fold::over;
    using Key = typename Fold::key;
    if constexpr (std::same_as<Over, void>) {
        // Nullary count: |the group's distinct full bindings|, u64.
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
        // Folds carry their input's type; countDistinct is a cardinality
        // (u64); pack carries the interval type (ts/src/query/find.ts).
        if constexpr (Fold::op == fold_form::count_distinct) {
            out.answer = field_class{value_kind::u64, 0};
        } else {
            out.answer = Over::cls;
        }
    }
    if constexpr (!std::same_as<Key, void>) {
        // The Arg key (numbered BEFORE the carried value at lowering).
        out.key_present = true;
        if constexpr (is_measure_ref_v<Key>) {
            out.key = measure_term<typename Key::over>();
        } else {
            out.key = var_term<Key>();
        }
    }
    return out;
}

/// A numeric (sum-able) variable: u64/i64 — bool stays refused.
template<class Var>
consteval auto is_numeric_var() -> bool {
    return is_qvar_v<Var>
        && (Var::cls.kind == value_kind::u64
            || Var::cls.kind == value_kind::i64);
}

/// An orderable variable: bool folds under min/max (false < true).
template<class Var>
consteval auto is_orderable_var() -> bool {
    return is_qvar_v<Var>
        && (Var::cls.kind == value_kind::boolean
            || Var::cls.kind == value_kind::u64
            || Var::cls.kind == value_kind::i64);
}

} // namespace bdb::detail

export namespace bdb {

/// A named variable head column — `bdb::as<"c">(vars.id)` (the head name
/// decoupled from the field name; recursive heads need it).
template<fixed_string Name, class Var>
[[nodiscard]] consteval auto as(Var) -> named_find<Name, Var> {
    static_assert(detail::is_qvar_v<Var>,
        "bumbledb as(): the argument must be a query variable "
        "(vars.field)");
    return {};
}

/// A named sum-of-measure head column:
/// `bdb::sum<"downtime">(r.duration(vars.window))`.
template<fixed_string Name, class Var>
[[nodiscard]] consteval auto sum(measure_ref<Var>)
    -> agg_ref<Name, fold_form::sum, measure_ref<Var>, void> {
    return {};
}

/// A named sum over a NUMERIC variable — `bdb::sum<"total">(vars.minor)`.
/// Exact checked sum, wide accumulator; bool stays refused (a truth count
/// is spelled over an explicit 0/1 column).
template<fixed_string Name, class Var>
[[nodiscard]] consteval auto sum(Var)
    -> agg_ref<Name, fold_form::sum, Var, void> {
    static_assert(detail::is_numeric_var<Var>(),
        "bumbledb sum(): the input is a numeric (u64/i64) variable or "
        "r.duration(interval variable) — sum over bool is refused");
    return {};
}

/// A named min over an orderable variable or the measure.
template<fixed_string Name, class Var>
[[nodiscard]] consteval auto min(measure_ref<Var>)
    -> agg_ref<Name, fold_form::min, measure_ref<Var>, void> {
    return {};
}

template<fixed_string Name, class Var>
[[nodiscard]] consteval auto min(Var)
    -> agg_ref<Name, fold_form::min, Var, void> {
    static_assert(detail::is_orderable_var<Var>(),
        "bumbledb min(): the input is an orderable (bool/u64/i64) "
        "variable or r.duration(interval variable)");
    return {};
}

/// A named max over an orderable variable or the measure.
template<fixed_string Name, class Var>
[[nodiscard]] consteval auto max(measure_ref<Var>)
    -> agg_ref<Name, fold_form::max, measure_ref<Var>, void> {
    return {};
}

template<fixed_string Name, class Var>
[[nodiscard]] consteval auto max(Var)
    -> agg_ref<Name, fold_form::max, Var, void> {
    static_assert(detail::is_orderable_var<Var>(),
        "bumbledb max(): the input is an orderable (bool/u64/i64) "
        "variable or r.duration(interval variable)");
    return {};
}

/// The nullary count — `bdb::count<"n">()`: |the group's set of distinct
/// full bindings|, u64.
template<fixed_string Name>
[[nodiscard]] consteval auto count()
    -> agg_ref<Name, fold_form::count, void, void> {
    return {};
}

/// `bdb::count_distinct<"n">(vars.owner)`: |distinct values of one bound
/// variable within the group|, u64.
template<fixed_string Name, class Var>
[[nodiscard]] consteval auto count_distinct(Var)
    -> agg_ref<Name, fold_form::count_distinct, Var, void> {
    static_assert(detail::is_qvar_v<Var>,
        "bumbledb count_distinct(): the argument must be a query "
        "variable (vars.field)");
    return {};
}

/// Arg-restriction toward the maximum of `key` (`ir::AggOp::ArgMax`):
/// carries `value` from the group's key-maximal bindings.
template<fixed_string Name, class Value, class Key>
[[nodiscard]] consteval auto arg_max(Value, Key)
    -> agg_ref<Name, fold_form::arg_max, Value, Key> {
    static_assert(detail::is_qvar_v<Value>,
        "bumbledb arg_max(): the carried value must be a query variable");
    static_assert(detail::is_orderable_var<Key>()
            || detail::is_measure_ref_v<Key>,
        "bumbledb arg_max(): the key must be an orderable variable or "
        "r.duration(interval variable)");
    return {};
}

/// Arg-restriction toward the minimum of `key`; rules as arg_max.
template<fixed_string Name, class Value, class Key>
[[nodiscard]] consteval auto arg_min(Value, Key)
    -> agg_ref<Name, fold_form::arg_min, Value, Key> {
    static_assert(detail::is_qvar_v<Value>,
        "bumbledb arg_min(): the carried value must be a query variable");
    static_assert(detail::is_orderable_var<Key>()
            || detail::is_measure_ref_v<Key>,
        "bumbledb arg_min(): the key must be an orderable variable or "
        "r.duration(interval variable)");
    return {};
}

/// The coalescing fold (`ir::AggOp::Pack`) — `bdb::pack<"free">(span)`:
/// the maximal disjoint half-open segments of the union of the group's
/// interval point sets — RELATION-SHAPED, one answer row per (group,
/// maximal segment). At most one pack per find, never beside another
/// aggregate (judged at rule assembly).
template<fixed_string Name, class Var>
[[nodiscard]] consteval auto pack(Var)
    -> agg_ref<Name, fold_form::pack, Var, void> {
    static_assert(detail::is_qvar_v<Var>
            && (Var::cls.kind == value_kind::interval_u64
                || Var::cls.kind == value_kind::interval_i64),
        "bumbledb pack(): the input must be an interval-typed query "
        "variable — pack coalesces interval point sets");
    return {};
}

} // namespace bdb
