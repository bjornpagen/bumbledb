// :foreign_program — the static program-IR view builder and the
// execute-time param marshal (TODO_CPP §13, §20–§21; lowering.md §4).
//
// Quarantine-zone by nature (AGENTS.md §5.3): a `bdb_program` is a graph
// of borrowed (pointer, count) views, so building one REQUIRES interior
// raw pointers — exactly like the owned_schema_spec lane in raii.cc.
// Everything here presents a compile-time `query_ir` (the lowered query
// VALUE from the :query partitions) as `static constexpr` C view arrays:
// the bindings/atoms/conditions/finds/rules/head/predicate objects live
// in static storage for the program's whole lifetime, and the bridge
// copies the graph into an owned Rust `Program` inside `bdb_db_prepare`
// before returning (bumbledb_c.h view-lifetime contract).
//
// Predicate layout (lowering.md §4.2): the recs in declaration order
// (`PredId` = index), the OUTPUT predicate appended last, `output = rec
// count`. A plain query is the degenerate no-rec program, output 0.
//
// A partition of MODULE bumbledb (not bumbledb_foreign), though the file
// lives in the foreign/ quarantine: it consumes the query IR partitions,
// and a partition of bumbledb_foreign could not import them without a
// module cycle (bumbledb's partitions import bumbledb_foreign).
export module bumbledb:foreign_program;

import std;
import :interval;
import :allen;
import :classify;
import :ir;
import bumbledb_foreign;

namespace bdb::foreign {

// --- the predicate walk (recs first, output last) ----------------------------

consteval auto predicate_total(query_ir const& ir) -> std::size_t {
	return ir.rec_count + 1;
}

consteval auto pred_rule_count(query_ir const& ir, std::size_t pred) -> std::size_t {
	return pred < ir.rec_count ? ir.recs[pred].rule_count : ir.rule_count;
}

consteval auto pred_rule(query_ir const& ir, std::size_t pred, std::size_t rule) -> wire_rule const& {
	return pred < ir.rec_count ? ir.recs[pred].rules[rule] : ir.rules[rule];
}

consteval auto pred_head_count(query_ir const& ir, std::size_t pred) -> std::size_t {
	return pred < ir.rec_count ? ir.recs[pred].head_count : ir.head_count;
}

consteval auto pred_head(query_ir const& ir, std::size_t pred, std::size_t column) -> find_data const& {
	return pred < ir.rec_count ? ir.recs[pred].head[column] : ir.head[column];
}

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
	// (:ir keeps the query value structural), so the
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
	case query_term_form::param_set:
		out.kind = bdb_term_kind::BDB_TERM_KIND_PARAM_SET;
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
		return bdb_head_op::BDB_HEAD_OP_MAX;
	case fold_form::count:
		return bdb_head_op::BDB_HEAD_OP_COUNT;
	case fold_form::count_distinct:
		return bdb_head_op::BDB_HEAD_OP_COUNT_DISTINCT;
	case fold_form::arg_max:
		return bdb_head_op::BDB_HEAD_OP_ARG_MAX;
	case fold_form::arg_min:
		return bdb_head_op::BDB_HEAD_OP_ARG_MIN;
	case fold_form::pack:
		break;
	}
	return bdb_head_op::BDB_HEAD_OP_PACK;
}

consteval auto condition_of(wire_condition const& condition) -> bdb_condition {
	return bdb_condition{
	    .kind = bdb_condition_kind::BDB_CONDITION_KIND_LEAF,
	    .cmp =
	        bdb_comparison{
	            .op =
	                bdb_cmp_op{
	                    .kind = cmp_kind_of(condition.op),
	                    .mask_kind = bdb_mask_term_kind::BDB_MASK_TERM_KIND_LITERAL,
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
	    .arg_key_kind = find.key_is_measure ? bdb_arg_key_kind::BDB_ARG_KEY_KIND_MEASURE : bdb_arg_key_kind::BDB_ARG_KEY_KIND_VAR,
	    .arg_key_var = find.key,
	};
	switch (find.form) {
	case find_form::variable:
		out.kind = bdb_find_term_kind::BDB_FIND_TERM_KIND_VAR;
		out.var = find.over;
		return out;
	case find_form::aggregate:
		out.kind = bdb_find_term_kind::BDB_FIND_TERM_KIND_AGGREGATE;
		out.has_over = find.has_over;
		out.over = find.over;
		return out;
	case find_form::aggregate_measure:
		break;
	}
	out.kind = bdb_find_term_kind::BDB_FIND_TERM_KIND_AGGREGATE_MEASURE;
	out.has_over = true;
	out.over = find.over;
	return out;
}

// --- flattened totals (across every predicate) -------------------------------

consteval auto binding_total(query_ir const& ir) -> std::size_t {
	auto total = std::size_t{0};
	for (auto pred = std::size_t{0}; pred != predicate_total(ir); ++pred) {
		for (auto rule = std::size_t{0}; rule != pred_rule_count(ir, pred); ++rule) {
			auto const& wire = pred_rule(ir, pred, rule);
			for (auto atom = std::size_t{0}; atom != wire.atom_count; ++atom) {
				total += wire.atoms[atom].binding_count;
			}
			for (auto atom = std::size_t{0}; atom != wire.negated_count; ++atom) {
				total += wire.negated[atom].binding_count;
			}
		}
	}
	return total;
}

consteval auto atom_total(query_ir const& ir) -> std::size_t {
	auto total = std::size_t{0};
	for (auto pred = std::size_t{0}; pred != predicate_total(ir); ++pred) {
		for (auto rule = std::size_t{0}; rule != pred_rule_count(ir, pred); ++rule) {
			total += pred_rule(ir, pred, rule).atom_count;
		}
	}
	return total;
}

consteval auto negated_total(query_ir const& ir) -> std::size_t {
	auto total = std::size_t{0};
	for (auto pred = std::size_t{0}; pred != predicate_total(ir); ++pred) {
		for (auto rule = std::size_t{0}; rule != pred_rule_count(ir, pred); ++rule) {
			total += pred_rule(ir, pred, rule).negated_count;
		}
	}
	return total;
}

consteval auto condition_total(query_ir const& ir) -> std::size_t {
	auto total = std::size_t{0};
	for (auto pred = std::size_t{0}; pred != predicate_total(ir); ++pred) {
		for (auto rule = std::size_t{0}; rule != pred_rule_count(ir, pred); ++rule) {
			total += pred_rule(ir, pred, rule).condition_count;
		}
	}
	return total;
}

consteval auto find_total(query_ir const& ir) -> std::size_t {
	auto total = std::size_t{0};
	for (auto pred = std::size_t{0}; pred != predicate_total(ir); ++pred) {
		for (auto rule = std::size_t{0}; rule != pred_rule_count(ir, pred); ++rule) {
			total += pred_rule(ir, pred, rule).find_count;
		}
	}
	return total;
}

consteval auto rule_total(query_ir const& ir) -> std::size_t {
	auto total = std::size_t{0};
	for (auto pred = std::size_t{0}; pred != predicate_total(ir); ++pred) {
		total += pred_rule_count(ir, pred);
	}
	return total;
}

consteval auto head_total(query_ir const& ir) -> std::size_t {
	auto total = std::size_t{0};
	for (auto pred = std::size_t{0}; pred != predicate_total(ir); ++pred) {
		total += pred_head_count(ir, pred);
	}
	return total;
}

// --- the flattened static arrays (one definition per query value) -----------
// Every array flattens in ONE canonical walk order — predicate, then rule,
// then item — so the offset arithmetic in the assemblers below pairs each
// view with its owner deterministically. Positive-atom bindings flatten
// BEFORE the same rule's negated-atom bindings.

template<auto Query>
consteval auto make_bindings() -> std::array<bdb_binding, binding_total(Query.ir)> {
	auto out = std::array<bdb_binding, binding_total(Query.ir)>{};
	auto at = std::size_t{0};
	for (auto pred = std::size_t{0}; pred != predicate_total(Query.ir); ++pred) {
		for (auto rule = std::size_t{0}; rule != pred_rule_count(Query.ir, pred); ++rule) {
			auto const& wire = pred_rule(Query.ir, pred, rule);
			auto const copy = [&](wire_atom const& atom) {
				for (auto binding = std::size_t{0}; binding != atom.binding_count; ++binding) {
					out[at] = bdb_binding{
					    .field = atom.bindings[binding].field,
					    .term = term_of(atom.bindings[binding].term),
					};
					++at;
				}
			};
			for (auto atom = std::size_t{0}; atom != wire.atom_count; ++atom) {
				copy(wire.atoms[atom]);
			}
			for (auto atom = std::size_t{0}; atom != wire.negated_count; ++atom) {
				copy(wire.negated[atom]);
			}
		}
	}
	return out;
}

template<auto Query>
inline constexpr auto program_bindings = make_bindings<Query>();

consteval auto atom_source_of(wire_atom const& atom) -> bdb_atom_source_kind {
	return atom.idb ? bdb_atom_source_kind::BDB_ATOM_SOURCE_KIND_IDB : bdb_atom_source_kind::BDB_ATOM_SOURCE_KIND_EDB;
}

template<auto Query>
consteval auto make_atoms() -> std::array<bdb_atom, atom_total(Query.ir)> {
	auto out = std::array<bdb_atom, atom_total(Query.ir)>{};
	auto at = std::size_t{0};
	auto binding_offset = std::size_t{0};
	for (auto pred = std::size_t{0}; pred != predicate_total(Query.ir); ++pred) {
		for (auto rule = std::size_t{0}; rule != pred_rule_count(Query.ir, pred); ++rule) {
			auto const& wire = pred_rule(Query.ir, pred, rule);
			for (auto atom = std::size_t{0}; atom != wire.atom_count; ++atom) {
				auto const& source = wire.atoms[atom];
				out[at] = bdb_atom{
				    .source_kind = atom_source_of(source),
				    .relation = source.relation,
				    .pred = source.pred,
				    .bindings = source.binding_count == 0 ? nullptr : program_bindings<Query>.data() + binding_offset,
				    .binding_count = source.binding_count,
				};
				binding_offset += source.binding_count;
				++at;
			}
			// The rule's negated bindings follow its positive ones in
			// the flattened walk.
			for (auto atom = std::size_t{0}; atom != wire.negated_count; ++atom) {
				binding_offset += wire.negated[atom].binding_count;
			}
		}
	}
	return out;
}

template<auto Query>
inline constexpr auto program_atoms = make_atoms<Query>();

template<auto Query>
consteval auto make_negated() -> std::array<bdb_atom, negated_total(Query.ir)> {
	auto out = std::array<bdb_atom, negated_total(Query.ir)>{};
	auto at = std::size_t{0};
	auto binding_offset = std::size_t{0};
	for (auto pred = std::size_t{0}; pred != predicate_total(Query.ir); ++pred) {
		for (auto rule = std::size_t{0}; rule != pred_rule_count(Query.ir, pred); ++rule) {
			auto const& wire = pred_rule(Query.ir, pred, rule);
			for (auto atom = std::size_t{0}; atom != wire.atom_count; ++atom) {
				binding_offset += wire.atoms[atom].binding_count;
			}
			for (auto atom = std::size_t{0}; atom != wire.negated_count; ++atom) {
				auto const& source = wire.negated[atom];
				out[at] = bdb_atom{
				    .source_kind = atom_source_of(source),
				    .relation = source.relation,
				    .pred = source.pred,
				    .bindings = source.binding_count == 0 ? nullptr : program_bindings<Query>.data() + binding_offset,
				    .binding_count = source.binding_count,
				};
				binding_offset += source.binding_count;
				++at;
			}
		}
	}
	return out;
}

template<auto Query>
inline constexpr auto program_negated = make_negated<Query>();

template<auto Query>
consteval auto make_conditions() -> std::array<bdb_condition, condition_total(Query.ir)> {
	auto out = std::array<bdb_condition, condition_total(Query.ir)>{};
	auto at = std::size_t{0};
	for (auto pred = std::size_t{0}; pred != predicate_total(Query.ir); ++pred) {
		for (auto rule = std::size_t{0}; rule != pred_rule_count(Query.ir, pred); ++rule) {
			auto const& wire = pred_rule(Query.ir, pred, rule);
			for (auto condition = std::size_t{0}; condition != wire.condition_count; ++condition) {
				out[at] = condition_of(wire.conditions[condition]);
				++at;
			}
		}
	}
	return out;
}

template<auto Query>
inline constexpr auto program_conditions = make_conditions<Query>();

template<auto Query>
consteval auto make_finds() -> std::array<bdb_find_term, find_total(Query.ir)> {
	auto out = std::array<bdb_find_term, find_total(Query.ir)>{};
	auto at = std::size_t{0};
	for (auto pred = std::size_t{0}; pred != predicate_total(Query.ir); ++pred) {
		for (auto rule = std::size_t{0}; rule != pred_rule_count(Query.ir, pred); ++rule) {
			auto const& wire = pred_rule(Query.ir, pred, rule);
			for (auto find = std::size_t{0}; find != wire.find_count; ++find) {
				out[at] = find_of(wire.finds[find]);
				++at;
			}
		}
	}
	return out;
}

template<auto Query>
inline constexpr auto program_finds = make_finds<Query>();

template<auto Query>
consteval auto make_rules() -> std::array<bdb_rule, rule_total(Query.ir)> {
	auto out = std::array<bdb_rule, rule_total(Query.ir)>{};
	auto at = std::size_t{0};
	auto atom_offset = std::size_t{0};
	auto negated_offset = std::size_t{0};
	auto condition_offset = std::size_t{0};
	auto find_offset = std::size_t{0};
	for (auto pred = std::size_t{0}; pred != predicate_total(Query.ir); ++pred) {
		for (auto rule = std::size_t{0}; rule != pred_rule_count(Query.ir, pred); ++rule) {
			auto const& wire = pred_rule(Query.ir, pred, rule);
			out[at] = bdb_rule{
			    .finds = wire.find_count == 0 ? nullptr : program_finds<Query>.data() + find_offset,
			    .find_count = wire.find_count,
			    .atoms = wire.atom_count == 0 ? nullptr : program_atoms<Query>.data() + atom_offset,
			    .atom_count = wire.atom_count,
			    .negated = wire.negated_count == 0 ? nullptr : program_negated<Query>.data() + negated_offset,
			    .negated_count = wire.negated_count,
			    .conditions = wire.condition_count == 0 ? nullptr : program_conditions<Query>.data() + condition_offset,
			    .condition_count = wire.condition_count,
			};
			find_offset += wire.find_count;
			atom_offset += wire.atom_count;
			negated_offset += wire.negated_count;
			condition_offset += wire.condition_count;
			++at;
		}
	}
	return out;
}

template<auto Query>
inline constexpr auto program_rules = make_rules<Query>();

template<auto Query>
consteval auto make_heads() -> std::array<bdb_head_term, head_total(Query.ir)> {
	auto out = std::array<bdb_head_term, head_total(Query.ir)>{};
	auto at = std::size_t{0};
	for (auto pred = std::size_t{0}; pred != predicate_total(Query.ir); ++pred) {
		for (auto index = std::size_t{0}; index != pred_head_count(Query.ir, pred); ++index) {
			auto const& column = pred_head(Query.ir, pred, index);
			out[at] = column.form == find_form::variable
                ? bdb_head_term{
                      .kind = bdb_head_term_kind::BDB_HEAD_TERM_KIND_VAR,
                      .op = bdb_head_op::BDB_HEAD_OP_SUM,
                  }
                : bdb_head_term{
                      .kind =
                          bdb_head_term_kind::BDB_HEAD_TERM_KIND_AGGREGATE,
                      .op = head_op_of(column.op),
                  };
			++at;
		}
	}
	return out;
}

template<auto Query>
inline constexpr auto program_heads = make_heads<Query>();

template<auto Query>
consteval auto make_predicates() -> std::array<bdb_predicate, predicate_total(Query.ir)> {
	auto out = std::array<bdb_predicate, predicate_total(Query.ir)>{};
	auto rule_offset = std::size_t{0};
	auto head_offset = std::size_t{0};
	for (auto pred = std::size_t{0}; pred != predicate_total(Query.ir); ++pred) {
		auto const rules = pred_rule_count(Query.ir, pred);
		auto const heads = pred_head_count(Query.ir, pred);
		out[pred] = bdb_predicate{
		    .head = heads == 0 ? nullptr : program_heads<Query>.data() + head_offset,
		    .head_count = heads,
		    .rules = rules == 0 ? nullptr : program_rules<Query>.data() + rule_offset,
		    .rule_count = rules,
		};
		rule_offset += rules;
		head_offset += heads;
	}
	return out;
}

template<auto Query>
inline constexpr auto program_predicates = make_predicates<Query>();

} // namespace bdb::foreign

export namespace bdb::foreign {

/// The whole lowered program as ONE static constant view graph: the recs
/// in declaration order, the output predicate last (`output = rec count`
/// — lowering.md §4.2; a plain query is the degenerate no-rec program,
/// output 0). Every pointer in the graph aims at `static constexpr`
/// storage, so the view outlives any `bdb_db_prepare` call by
/// construction.
template<auto Query>
inline constexpr auto program_of = bdb_program{
    .predicates = program_predicates<Query>.data(),
    .predicate_count = predicate_total(Query.ir),
    .output = static_cast<std::uint16_t>(Query.ir.rec_count),
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
	    .data = value.empty() ? nullptr : std::bit_cast<std::uint8_t const*>(value.data()),
	    .len = value.size(),
	};
	return out;
}

/// Borrowed for the call; the bridge copies before returning.
[[nodiscard]] inline auto wire_param(std::span<std::byte const> value) -> bdb_param {
	auto out = bdb_param{};
	out.kind = bdb_param_kind::BDB_PARAM_KIND_SCALAR;
	out.scalar.kind = bdb_value_kind::BDB_VALUE_KIND_FIXED_BYTES;
	out.scalar.bytes_value = bdb_bytes_view{
	    .data = value.empty() ? nullptr : std::bit_cast<std::uint8_t const*>(value.data()),
	    .len = value.size(),
	};
	return out;
}

[[nodiscard]] inline auto wire_param(interval<std::uint64_t> value) -> bdb_param {
	auto out = bdb_param{};
	out.kind = bdb_param_kind::BDB_PARAM_KIND_SCALAR;
	out.scalar.kind = bdb_value_kind::BDB_VALUE_KIND_INTERVAL_U64;
	out.scalar.interval_u64_start = value.lo();
	out.scalar.interval_u64_end = value.hi();
	return out;
}

[[nodiscard]] inline auto wire_param(interval<std::int64_t> value) -> bdb_param {
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

/// The set-cell scratch of one execute call: every runtime ∈-set param's
/// tagged cells live here for exactly the call's extent (the bridge
/// copies before returning). The OUTER vector may grow (its inner buffers
/// never move), so earlier set views stay valid.
using param_scratch = std::vector<std::vector<bdb_value>>;

} // namespace bdb::foreign

namespace bdb::foreign {

/// The scalar lane of one member (no scratch consulted).
template<class Member>
[[nodiscard]] auto wire_one(Member const& value, param_scratch&) -> bdb_param {
	return wire_param(value);
}

/// The set lane of one member: the span's elements tag one by one into
/// the scratch, and the param views the scratch cells.
template<class Element>
[[nodiscard]] auto wire_one(std::span<Element const> values, param_scratch& scratch) -> bdb_param {
	auto cells = std::vector<bdb_value>{};
	cells.reserve(values.size());
	for (auto const& element : values) {
		cells.push_back(wire_param(element).scalar);
	}
	auto const& stored = scratch.emplace_back(std::move(cells));
	auto out = bdb_param{};
	out.kind = bdb_param_kind::BDB_PARAM_KIND_SET;
	out.set = stored.empty() ? nullptr : stored.data();
	out.set_len = stored.size();
	return out;
}

} // namespace bdb::foreign

export namespace bdb::foreign {

/// The whole params product marshalled POSITIONALLY: member declaration
/// order IS the registry order (the product was synthesized from the
/// registry), which IS the engine's positional ParamId order. Runtime
/// ∈-set members marshal through `scratch`, which must outlive the
/// execute call.
template<class Params>
[[nodiscard]] auto wire_params(Params const& params, param_scratch& scratch) {
	auto const& [... values] = params;
	return std::array<bdb_param, sizeof...(values)>{wire_one(values, scratch)...};
}

} // namespace bdb::foreign

namespace bdb::foreign {

// --- membership set constants (lowering.md §4.2's membership arrays) --------
// A closed-membership array's set is a PROGRAM CONSTANT pre-resolved at
// build; execution injects it positionally — the params product never
// carries it (ts/src/query/run.ts:57-63). The cells live in static
// constexpr storage, like the rest of the program view graph.

consteval auto membership_cell_total(query_ir const& ir) -> std::size_t {
	auto total = std::size_t{0};
	for (auto index = std::size_t{0}; index != ir.param_count; ++index) {
		if (ir.params[index].membership) {
			total += ir.params[index].member_count;
		}
	}
	return total;
}

template<auto Query>
consteval auto make_membership_cells() -> std::array<bdb_value, membership_cell_total(Query.ir)> {
	auto out = std::array<bdb_value, membership_cell_total(Query.ir)>{};
	auto at = std::size_t{0};
	for (auto index = std::size_t{0}; index != Query.ir.param_count; ++index) {
		auto const& parameter = Query.ir.params[index];
		if (!parameter.membership) {
			continue;
		}
		for (auto member = std::size_t{0}; member != parameter.member_count; ++member) {
			out[at].kind = bdb_value_kind::BDB_VALUE_KIND_U64;
			out[at].u64_value = parameter.members[member];
			++at;
		}
	}
	return out;
}

template<auto Query>
inline constexpr auto membership_cells = make_membership_cells<Query>();

} // namespace bdb::foreign

export namespace bdb::foreign {

/// The query-directed execute marshal: the caller's params product fills
/// the value/mask/runtime-set entries in registry order, and every
/// MEMBERSHIP entry is injected from the query's frozen set constant
/// (positional ParamId order — lowering.md §5.1). `scratch` owns the
/// runtime set cells and must outlive the execute call.
template<auto Query, class Params>
[[nodiscard]] auto wire_params_for(Params const& params, param_scratch& scratch) -> std::array<bdb_param, Query.ir.param_count> {
	auto const scalars = wire_params(params, scratch);
	auto out = std::array<bdb_param, Query.ir.param_count>{};
	auto scalar_index = std::size_t{0};
	auto member_offset = std::size_t{0};
	for (auto index = std::size_t{0}; index != Query.ir.param_count; ++index) {
		auto const& parameter = Query.ir.params[index];
		if (parameter.membership) {
			auto set = bdb_param{};
			set.kind = bdb_param_kind::BDB_PARAM_KIND_SET;
			set.set = membership_cells<Query>.data() + member_offset;
			set.set_len = parameter.member_count;
			member_offset += parameter.member_count;
			out[index] = set;
		} else {
			out[index] = scalars[scalar_index];
			++scalar_index;
		}
	}
	return out;
}

} // namespace bdb::foreign
