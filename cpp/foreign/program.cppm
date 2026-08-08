// bumbledb.foreign.program — the static program-IR view builder and the
// execute-time param marshal (TODO_CPP §13, §20–§21; lowering.md §4).
//
// Quarantine-zone by nature (AGENTS.md §5.3): a `bdb_program` is a graph
// of borrowed (pointer, count) views, so building one REQUIRES interior
// raw pointers — exactly like the owned_schema_spec lane in raii.cppm.
// Everything here presents a compile-time `query_ir` (the lowered query
// VALUE from bumbledb.meta.query) as `static constexpr` C view arrays:
// the bindings/atoms/conditions/finds/rules/head/predicate objects live
// in static storage for the program's whole lifetime, and the bridge
// copies the graph into an owned Rust `Program` inside `bdb_db_prepare`
// before returning (bumbledb_c.h view-lifetime contract).
//
// GCC-only (imports the reflective meta layer's IR value types).
export module bumbledb.foreign.program;

import std;
import bumbledb.types;
import bumbledb.foreign;
import bumbledb.meta.relation;
import bumbledb.meta.schema;
import bumbledb.meta.query;

namespace bdb::foreign {

// --- IR value -> C view folds (consteval; every output is data) -------------

consteval auto value_of(query_literal const& literal) -> bdb_value {
    auto out = bdb_value{};
    switch (literal.kind) {
    case value_kind::boolean:
        out.kind = bdb_value_kind::BDB_VALUE_KIND_BOOL;
        out.bool_value = literal.boolean;
        return out;
    case value_kind::u64:
        out.kind = bdb_value_kind::BDB_VALUE_KIND_U64;
        out.u64_value = literal.u64;
        return out;
    case value_kind::i64:
        out.kind = bdb_value_kind::BDB_VALUE_KIND_I64;
        out.i64_value = literal.i64;
        return out;
    case value_kind::interval_u64:
        out.kind = bdb_value_kind::BDB_VALUE_KIND_INTERVAL_U64;
        out.interval_u64_start = literal.u64_start;
        out.interval_u64_end = literal.u64_end;
        return out;
    case value_kind::string:
    case value_kind::fixed_bytes:
    case value_kind::interval_i64:
        break;
    }
    // string/bytes are unrepresentable as query literals by construction
    // (bumbledb.meta.query keeps the query value structural), so the
    // remaining case is the i64 interval.
    out.kind = bdb_value_kind::BDB_VALUE_KIND_INTERVAL_I64;
    out.interval_i64_start = literal.i64_start;
    out.interval_i64_end = literal.i64_end;
    return out;
}

consteval auto term_of(wire_term const& term) -> bdb_term {
    auto out = bdb_term{};
    switch (term.form) {
    case query_term_form::variable:
        out.kind = bdb_term_kind::BDB_TERM_KIND_VAR;
        out.var = term.var;
        return out;
    case query_term_form::param:
        out.kind = bdb_term_kind::BDB_TERM_KIND_PARAM;
        out.param = term.param;
        return out;
    case query_term_form::measure:
        out.kind = bdb_term_kind::BDB_TERM_KIND_MEASURE;
        out.var = term.var;
        return out;
    case query_term_form::literal:
    case query_term_form::absent:
        break;
    }
    out.kind = bdb_term_kind::BDB_TERM_KIND_LITERAL;
    out.literal = value_of(term.literal);
    return out;
}

consteval auto cmp_kind_of(query_cmp op) -> bdb_cmp_op_kind {
    switch (op) {
    case query_cmp::eq:
        return bdb_cmp_op_kind::BDB_CMP_OP_KIND_EQ;
    case query_cmp::ne:
        return bdb_cmp_op_kind::BDB_CMP_OP_KIND_NE;
    case query_cmp::lt:
        return bdb_cmp_op_kind::BDB_CMP_OP_KIND_LT;
    case query_cmp::le:
        return bdb_cmp_op_kind::BDB_CMP_OP_KIND_LE;
    case query_cmp::gt:
        return bdb_cmp_op_kind::BDB_CMP_OP_KIND_GT;
    case query_cmp::ge:
        return bdb_cmp_op_kind::BDB_CMP_OP_KIND_GE;
    case query_cmp::allen:
        return bdb_cmp_op_kind::BDB_CMP_OP_KIND_ALLEN;
    case query_cmp::point_in:
        break;
    }
    return bdb_cmp_op_kind::BDB_CMP_OP_KIND_POINT_IN;
}

consteval auto head_op_of(fold_form op) -> bdb_head_op {
    switch (op) {
    case fold_form::sum:
        return bdb_head_op::BDB_HEAD_OP_SUM;
    case fold_form::min:
        return bdb_head_op::BDB_HEAD_OP_MIN;
    case fold_form::max:
        break;
    }
    return bdb_head_op::BDB_HEAD_OP_MAX;
}

consteval auto condition_of(wire_condition const& condition)
    -> bdb_condition {
    return bdb_condition{
        .kind = bdb_condition_kind::BDB_CONDITION_KIND_LEAF,
        .cmp = bdb_comparison{
            .op = bdb_cmp_op{
                .kind = cmp_kind_of(condition.op),
                .mask_kind =
                    bdb_mask_term_kind::BDB_MASK_TERM_KIND_LITERAL,
                .mask = condition.mask,
                .mask_param = 0,
            },
            .lhs = term_of(condition.lhs),
            .rhs = term_of(condition.rhs),
        },
        .children = nullptr,
        .child_count = 0,
    };
}

consteval auto find_of(wire_find const& find) -> bdb_find_term {
    auto out = bdb_find_term{};
    out.op = bdb_agg_op{
        .kind = head_op_of(find.op),
        .arg_key_kind = bdb_arg_key_kind::BDB_ARG_KEY_KIND_VAR,
        .arg_key_var = 0,
    };
    if (find.form == find_form::variable) {
        out.kind = bdb_find_term_kind::BDB_FIND_TERM_KIND_VAR;
        out.var = find.over;
        return out;
    }
    out.kind = bdb_find_term_kind::BDB_FIND_TERM_KIND_AGGREGATE_MEASURE;
    out.has_over = true;
    out.over = find.over;
    return out;
}

// --- flattened totals ---------------------------------------------------------

consteval auto binding_total(query_ir const& ir) -> std::size_t {
    auto total = std::size_t{0};
    for (auto rule = std::size_t{0}; rule != ir.rule_count; ++rule) {
        for (auto atom = std::size_t{0};
            atom != ir.rules[rule].atom_count; ++atom) {
            total += ir.rules[rule].atoms[atom].binding_count;
        }
    }
    return total;
}

consteval auto atom_total(query_ir const& ir) -> std::size_t {
    auto total = std::size_t{0};
    for (auto rule = std::size_t{0}; rule != ir.rule_count; ++rule) {
        total += ir.rules[rule].atom_count;
    }
    return total;
}

consteval auto condition_total(query_ir const& ir) -> std::size_t {
    auto total = std::size_t{0};
    for (auto rule = std::size_t{0}; rule != ir.rule_count; ++rule) {
        total += ir.rules[rule].condition_count;
    }
    return total;
}

consteval auto find_total(query_ir const& ir) -> std::size_t {
    auto total = std::size_t{0};
    for (auto rule = std::size_t{0}; rule != ir.rule_count; ++rule) {
        total += ir.rules[rule].find_count;
    }
    return total;
}

// --- the flattened static arrays (one definition per query value) -----------

template<auto Query>
consteval auto make_bindings()
    -> std::array<bdb_binding, binding_total(Query.ir)> {
    auto out = std::array<bdb_binding, binding_total(Query.ir)>{};
    auto at = std::size_t{0};
    for (auto rule = std::size_t{0}; rule != Query.ir.rule_count; ++rule) {
        for (auto atom = std::size_t{0};
            atom != Query.ir.rules[rule].atom_count; ++atom) {
            auto const& wire = Query.ir.rules[rule].atoms[atom];
            for (auto binding = std::size_t{0};
                binding != wire.binding_count; ++binding) {
                out[at] = bdb_binding{
                    .field = wire.bindings[binding].field,
                    .term = term_of(wire.bindings[binding].term),
                };
                ++at;
            }
        }
    }
    return out;
}

template<auto Query>
inline constexpr auto program_bindings = make_bindings<Query>();

template<auto Query>
consteval auto make_atoms() -> std::array<bdb_atom, atom_total(Query.ir)> {
    auto out = std::array<bdb_atom, atom_total(Query.ir)>{};
    auto at = std::size_t{0};
    auto binding_offset = std::size_t{0};
    for (auto rule = std::size_t{0}; rule != Query.ir.rule_count; ++rule) {
        for (auto atom = std::size_t{0};
            atom != Query.ir.rules[rule].atom_count; ++atom) {
            auto const& wire = Query.ir.rules[rule].atoms[atom];
            out[at] = bdb_atom{
                .source_kind =
                    bdb_atom_source_kind::BDB_ATOM_SOURCE_KIND_EDB,
                .relation = wire.relation,
                .pred = 0,
                .bindings = wire.binding_count == 0
                    ? nullptr
                    : program_bindings<Query>.data() + binding_offset,
                .binding_count = wire.binding_count,
            };
            binding_offset += wire.binding_count;
            ++at;
        }
    }
    return out;
}

template<auto Query>
inline constexpr auto program_atoms = make_atoms<Query>();

template<auto Query>
consteval auto make_conditions()
    -> std::array<bdb_condition, condition_total(Query.ir)> {
    auto out = std::array<bdb_condition, condition_total(Query.ir)>{};
    auto at = std::size_t{0};
    for (auto rule = std::size_t{0}; rule != Query.ir.rule_count; ++rule) {
        for (auto condition = std::size_t{0};
            condition != Query.ir.rules[rule].condition_count;
            ++condition) {
            out[at] =
                condition_of(Query.ir.rules[rule].conditions[condition]);
            ++at;
        }
    }
    return out;
}

template<auto Query>
inline constexpr auto program_conditions = make_conditions<Query>();

template<auto Query>
consteval auto make_finds()
    -> std::array<bdb_find_term, find_total(Query.ir)> {
    auto out = std::array<bdb_find_term, find_total(Query.ir)>{};
    auto at = std::size_t{0};
    for (auto rule = std::size_t{0}; rule != Query.ir.rule_count; ++rule) {
        for (auto find = std::size_t{0};
            find != Query.ir.rules[rule].find_count; ++find) {
            out[at] = find_of(Query.ir.rules[rule].finds[find]);
            ++at;
        }
    }
    return out;
}

template<auto Query>
inline constexpr auto program_finds = make_finds<Query>();

template<auto Query>
consteval auto make_rules()
    -> std::array<bdb_rule, Query.ir.rule_count> {
    auto out = std::array<bdb_rule, Query.ir.rule_count>{};
    auto atom_offset = std::size_t{0};
    auto condition_offset = std::size_t{0};
    auto find_offset = std::size_t{0};
    for (auto rule = std::size_t{0}; rule != Query.ir.rule_count; ++rule) {
        auto const& wire = Query.ir.rules[rule];
        out[rule] = bdb_rule{
            .finds = wire.find_count == 0
                ? nullptr
                : program_finds<Query>.data() + find_offset,
            .find_count = wire.find_count,
            .atoms = wire.atom_count == 0
                ? nullptr
                : program_atoms<Query>.data() + atom_offset,
            .atom_count = wire.atom_count,
            .negated = nullptr,
            .negated_count = 0,
            .conditions = wire.condition_count == 0
                ? nullptr
                : program_conditions<Query>.data() + condition_offset,
            .condition_count = wire.condition_count,
        };
        find_offset += wire.find_count;
        atom_offset += wire.atom_count;
        condition_offset += wire.condition_count;
    }
    return out;
}

template<auto Query>
inline constexpr auto program_rules = make_rules<Query>();

template<auto Query>
consteval auto make_head()
    -> std::array<bdb_head_term, Query.ir.head_count> {
    auto out = std::array<bdb_head_term, Query.ir.head_count>{};
    for (auto index = std::size_t{0}; index != Query.ir.head_count;
        ++index) {
        auto const& column = Query.ir.head[index];
        out[index] = column.form == find_form::variable
            ? bdb_head_term{
                  .kind = bdb_head_term_kind::BDB_HEAD_TERM_KIND_VAR,
                  .op = bdb_head_op::BDB_HEAD_OP_SUM,
              }
            : bdb_head_term{
                  .kind =
                      bdb_head_term_kind::BDB_HEAD_TERM_KIND_AGGREGATE,
                  .op = head_op_of(column.op),
              };
    }
    return out;
}

template<auto Query>
inline constexpr auto program_head = make_head<Query>();

template<auto Query>
inline constexpr auto program_predicates = std::array<bdb_predicate, 1>{
    bdb_predicate{
        .head = Query.ir.head_count == 0
            ? nullptr
            : program_head<Query>.data(),
        .head_count = Query.ir.head_count,
        .rules = Query.ir.rule_count == 0
            ? nullptr
            : program_rules<Query>.data(),
        .rule_count = Query.ir.rule_count,
    },
};

} // namespace bdb::foreign

export namespace bdb::foreign {

/// The whole lowered program as ONE static constant view graph: a plain
/// query is the degenerate one-predicate program, output 0 (lowering.md
/// §4.1). Every pointer in the graph aims at `static constexpr` storage,
/// so the view outlives any `bdb_db_prepare` call by construction.
template<auto Query>
inline constexpr auto program_of = bdb_program{
    .predicates = program_predicates<Query>.data(),
    .predicate_count = 1,
    .output = 0,
};

// --- execute-time param marshalling (TODO_CPP §21; lowering.md §5.1) --------
// One overload per params-product member type; the member types were
// synthesized from the query's anchored domains, so the fold is total.

[[nodiscard]] inline auto wire_param(bool value) -> bdb_param {
    auto out = bdb_param{};
    out.kind = bdb_param_kind::BDB_PARAM_KIND_SCALAR;
    out.scalar.kind = bdb_value_kind::BDB_VALUE_KIND_BOOL;
    out.scalar.bool_value = value;
    return out;
}

[[nodiscard]] inline auto wire_param(std::uint64_t value) -> bdb_param {
    auto out = bdb_param{};
    out.kind = bdb_param_kind::BDB_PARAM_KIND_SCALAR;
    out.scalar.kind = bdb_value_kind::BDB_VALUE_KIND_U64;
    out.scalar.u64_value = value;
    return out;
}

[[nodiscard]] inline auto wire_param(std::int64_t value) -> bdb_param {
    auto out = bdb_param{};
    out.kind = bdb_param_kind::BDB_PARAM_KIND_SCALAR;
    out.scalar.kind = bdb_value_kind::BDB_VALUE_KIND_I64;
    out.scalar.i64_value = value;
    return out;
}

/// Borrowed for the call; the bridge copies before returning.
[[nodiscard]] inline auto wire_param(std::string_view value) -> bdb_param {
    auto out = bdb_param{};
    out.kind = bdb_param_kind::BDB_PARAM_KIND_SCALAR;
    out.scalar.kind = bdb_value_kind::BDB_VALUE_KIND_STRING;
    out.scalar.string_value = bdb_string_view{
        .data = value.empty()
            ? nullptr
            : std::bit_cast<std::uint8_t const*>(value.data()),
        .len = value.size(),
    };
    return out;
}

/// Borrowed for the call; the bridge copies before returning.
[[nodiscard]] inline auto wire_param(std::span<std::byte const> value)
    -> bdb_param {
    auto out = bdb_param{};
    out.kind = bdb_param_kind::BDB_PARAM_KIND_SCALAR;
    out.scalar.kind = bdb_value_kind::BDB_VALUE_KIND_FIXED_BYTES;
    out.scalar.bytes_value = bdb_bytes_view{
        .data = value.empty()
            ? nullptr
            : std::bit_cast<std::uint8_t const*>(value.data()),
        .len = value.size(),
    };
    return out;
}

[[nodiscard]] inline auto wire_param(interval<std::uint64_t> value)
    -> bdb_param {
    auto out = bdb_param{};
    out.kind = bdb_param_kind::BDB_PARAM_KIND_SCALAR;
    out.scalar.kind = bdb_value_kind::BDB_VALUE_KIND_INTERVAL_U64;
    out.scalar.interval_u64_start = value.lo();
    out.scalar.interval_u64_end = value.hi();
    return out;
}

[[nodiscard]] inline auto wire_param(interval<std::int64_t> value)
    -> bdb_param {
    auto out = bdb_param{};
    out.kind = bdb_param_kind::BDB_PARAM_KIND_SCALAR;
    out.scalar.kind = bdb_value_kind::BDB_VALUE_KIND_INTERVAL_I64;
    out.scalar.interval_i64_start = value.lo();
    out.scalar.interval_i64_end = value.hi();
    return out;
}

/// An Allen mask travels as a scalar AllenMask value (TODO_CPP §21).
[[nodiscard]] inline auto wire_param(allen_mask value) -> bdb_param {
    auto out = bdb_param{};
    out.kind = bdb_param_kind::BDB_PARAM_KIND_SCALAR;
    out.scalar.kind = bdb_value_kind::BDB_VALUE_KIND_ALLEN_MASK;
    out.scalar.allen_mask = value.bits();
    return out;
}

/// The whole params product marshalled POSITIONALLY: member declaration
/// order IS the registry order (the product was synthesized from the
/// registry), which IS the engine's positional ParamId order.
template<class Params>
[[nodiscard]] auto wire_params(Params const& params) {
    auto const& [...values] = params;
    return std::array<bdb_param, sizeof...(values)>{wire_param(values)...};
}

} // namespace bdb::foreign
