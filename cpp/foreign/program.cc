/**
 * The static program-IR view builder and the execute-time param marshal
 * (lowering.md §4/§5.1). Quarantine-zone by nature — a bdb_program is a
 * graph of borrowed (pointer, count) views — but a partition of MODULE
 * bumbledb, not bumbledb_foreign: it consumes the query IR partitions,
 * and a partition of bumbledb_foreign could not import them without a
 * module cycle.
 */
export module bumbledb:foreign_program;

import std;
import :interval;
import :allen;
import :classify;
import :ir;
import bumbledb_foreign;

namespace bdb::foreign {

[[nodiscard]] consteval auto predicate_total(query_ir const& ir) -> std::size_t {
	return ir.rec_count + 1;
}

[[nodiscard]] consteval auto pred_rule_count(query_ir const& ir, std::size_t pred) -> std::size_t {
	return pred < ir.rec_count ? ir.recs[pred].rule_count : ir.rule_count;
}

[[nodiscard]] consteval auto pred_rule(query_ir const& ir, std::size_t pred, std::size_t rule) -> wire_rule const& {
	return pred < ir.rec_count ? ir.recs[pred].rules[rule] : ir.rules[rule];
}

[[nodiscard]] consteval auto pred_head_count(query_ir const& ir, std::size_t pred) -> std::size_t {
	return pred < ir.rec_count ? ir.recs[pred].head_count : ir.head_count;
}

[[nodiscard]] consteval auto pred_head(query_ir const& ir, std::size_t pred, std::size_t column) -> find_data const& {
	return pred < ir.rec_count ? ir.recs[pred].head[column] : ir.head[column];
}

/**
 * string/bytes are unrepresentable as query literals by construction
 * (:ir keeps the query value structural), so the fall-through case is
 * the i64 interval.
 */
[[nodiscard]] consteval auto value_of(query_literal const& literal) -> bdb_value {
	auto out = bdb_value{};
	switch (literal.kind) {
	case value_kind::boolean:
		out.kind = abi_tag(bdb_value_kind::BDB_VALUE_KIND_BOOL);
		out.bool_value = abi_flag(literal.boolean);
		return out;
	case value_kind::u64:
		out.kind = abi_tag(bdb_value_kind::BDB_VALUE_KIND_U64);
		out.u64_value = literal.u64;
		return out;
	case value_kind::i64:
		out.kind = abi_tag(bdb_value_kind::BDB_VALUE_KIND_I64);
		out.i64_value = literal.i64;
		return out;
	case value_kind::interval_u64:
		out.kind = abi_tag(bdb_value_kind::BDB_VALUE_KIND_INTERVAL_U64);
		out.interval_u64_start = literal.u64_start;
		out.interval_u64_end = literal.u64_end;
		return out;
	case value_kind::string:
	case value_kind::fixed_bytes:
	case value_kind::interval_i64:
		break;
	}
	out.kind = abi_tag(bdb_value_kind::BDB_VALUE_KIND_INTERVAL_I64);
	out.interval_i64_start = literal.i64_start;
	out.interval_i64_end = literal.i64_end;
	return out;
}

[[nodiscard]] consteval auto term_of(wire_term const& term) -> bdb_term {
	auto out = bdb_term{};
	switch (term.form) {
	case query_term_form::variable:
		out.kind = abi_tag(bdb_term_kind::BDB_TERM_KIND_VAR);
		out.var = term.var;
		return out;
	case query_term_form::param:
		out.kind = abi_tag(bdb_term_kind::BDB_TERM_KIND_PARAM);
		out.param = term.param;
		return out;
	case query_term_form::param_set:
		out.kind = abi_tag(bdb_term_kind::BDB_TERM_KIND_PARAM_SET);
		out.param = term.param;
		return out;
	case query_term_form::measure:
		out.kind = abi_tag(bdb_term_kind::BDB_TERM_KIND_MEASURE);
		out.var = term.var;
		return out;
	case query_term_form::literal:
	case query_term_form::absent:
		break;
	}
	out.kind = abi_tag(bdb_term_kind::BDB_TERM_KIND_LITERAL);
	out.literal = value_of(term.literal);
	return out;
}

[[nodiscard]] consteval auto cmp_kind_of(query_cmp op) -> std::uint32_t {
	switch (op) {
	case query_cmp::eq:
		return abi_tag(bdb_cmp_op_kind::BDB_CMP_OP_KIND_EQ);
	case query_cmp::ne:
		return abi_tag(bdb_cmp_op_kind::BDB_CMP_OP_KIND_NE);
	case query_cmp::lt:
		return abi_tag(bdb_cmp_op_kind::BDB_CMP_OP_KIND_LT);
	case query_cmp::le:
		return abi_tag(bdb_cmp_op_kind::BDB_CMP_OP_KIND_LE);
	case query_cmp::gt:
		return abi_tag(bdb_cmp_op_kind::BDB_CMP_OP_KIND_GT);
	case query_cmp::ge:
		return abi_tag(bdb_cmp_op_kind::BDB_CMP_OP_KIND_GE);
	case query_cmp::allen:
		return abi_tag(bdb_cmp_op_kind::BDB_CMP_OP_KIND_ALLEN);
	case query_cmp::point_in:
		break;
	}
	return abi_tag(bdb_cmp_op_kind::BDB_CMP_OP_KIND_POINT_IN);
}

[[nodiscard]] consteval auto head_op_of(fold_form op) -> std::uint32_t {
	switch (op) {
	case fold_form::sum:
		return abi_tag(bdb_head_op::BDB_HEAD_OP_SUM);
	case fold_form::min:
		return abi_tag(bdb_head_op::BDB_HEAD_OP_MIN);
	case fold_form::max:
		return abi_tag(bdb_head_op::BDB_HEAD_OP_MAX);
	case fold_form::count:
		return abi_tag(bdb_head_op::BDB_HEAD_OP_COUNT);
	case fold_form::pack:
		break;
	}
	return abi_tag(bdb_head_op::BDB_HEAD_OP_PACK);
}

[[nodiscard]] consteval auto condition_of(wire_condition const& condition) -> bdb_condition {
	return bdb_condition{
	    .kind = abi_tag(bdb_condition_kind::BDB_CONDITION_KIND_LEAF),
	    .cmp =
	        bdb_comparison{
	            .op =
	                bdb_cmp_op{
	                    .kind = cmp_kind_of(condition.op),
	                    .mask = condition.mask,
	                },
	            .lhs = term_of(condition.lhs),
	            .rhs = term_of(condition.rhs),
	        },
	    .children = nullptr,
	    .child_count = 0,
	};
}

[[nodiscard]] consteval auto find_of(wire_find const& find) -> bdb_find_term {
	auto out = bdb_find_term{};
	out.op = bdb_agg_op{
	    .kind = head_op_of(find.op),
	};
	switch (find.form) {
	case find_form::variable:
		out.kind = abi_tag(bdb_find_term_kind::BDB_FIND_TERM_KIND_VAR);
		out.var = find.over;
		return out;
	case find_form::aggregate:
		out.kind = abi_tag(bdb_find_term_kind::BDB_FIND_TERM_KIND_AGGREGATE);
		out.has_over = abi_flag(find.has_over);
		out.over = find.over;
		return out;
	case find_form::aggregate_measure:
		break;
	}
	out.kind = abi_tag(bdb_find_term_kind::BDB_FIND_TERM_KIND_AGGREGATE_MEASURE);
	out.has_over = abi_flag(true);
	out.over = find.over;
	return out;
}

[[nodiscard]] consteval auto binding_total(query_ir const& ir) -> std::size_t {
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

[[nodiscard]] consteval auto atom_total(query_ir const& ir) -> std::size_t {
	auto total = std::size_t{0};
	for (auto pred = std::size_t{0}; pred != predicate_total(ir); ++pred) {
		for (auto rule = std::size_t{0}; rule != pred_rule_count(ir, pred); ++rule) {
			total += pred_rule(ir, pred, rule).atom_count;
		}
	}
	return total;
}

[[nodiscard]] consteval auto negated_total(query_ir const& ir) -> std::size_t {
	auto total = std::size_t{0};
	for (auto pred = std::size_t{0}; pred != predicate_total(ir); ++pred) {
		for (auto rule = std::size_t{0}; rule != pred_rule_count(ir, pred); ++rule) {
			total += pred_rule(ir, pred, rule).negated_count;
		}
	}
	return total;
}

[[nodiscard]] consteval auto condition_total(query_ir const& ir) -> std::size_t {
	auto total = std::size_t{0};
	for (auto pred = std::size_t{0}; pred != predicate_total(ir); ++pred) {
		for (auto rule = std::size_t{0}; rule != pred_rule_count(ir, pred); ++rule) {
			total += pred_rule(ir, pred, rule).condition_count;
		}
	}
	return total;
}

[[nodiscard]] consteval auto find_total(query_ir const& ir) -> std::size_t {
	auto total = std::size_t{0};
	for (auto pred = std::size_t{0}; pred != predicate_total(ir); ++pred) {
		for (auto rule = std::size_t{0}; rule != pred_rule_count(ir, pred); ++rule) {
			total += pred_rule(ir, pred, rule).find_count;
		}
	}
	return total;
}

[[nodiscard]] consteval auto rule_total(query_ir const& ir) -> std::size_t {
	auto total = std::size_t{0};
	for (auto pred = std::size_t{0}; pred != predicate_total(ir); ++pred) {
		total += pred_rule_count(ir, pred);
	}
	return total;
}

[[nodiscard]] consteval auto head_total(query_ir const& ir) -> std::size_t {
	auto total = std::size_t{0};
	for (auto pred = std::size_t{0}; pred != predicate_total(ir); ++pred) {
		total += pred_head_count(ir, pred);
	}
	return total;
}

/**
 * Every flattened array below is built in ONE canonical walk order —
 * predicate, then rule, then item, with a rule's positive-atom bindings
 * before its negated-atom bindings — so the assemblers' offset
 * arithmetic pairs each view with its owner deterministically.
 */
template<auto Query>
[[nodiscard]] consteval auto make_bindings() -> std::array<bdb_binding, binding_total(Query.ir)> {
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

[[nodiscard]] consteval auto atom_source_of(wire_atom const& atom) -> std::uint32_t {
	return abi_tag(atom.idb ? abi_tag(bdb_atom_source_kind::BDB_ATOM_SOURCE_KIND_IDB) : abi_tag(bdb_atom_source_kind::BDB_ATOM_SOURCE_KIND_EDB));
}

template<auto Query>
[[nodiscard]] consteval auto make_atoms() -> std::array<bdb_atom, atom_total(Query.ir)> {
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
[[nodiscard]] consteval auto make_negated() -> std::array<bdb_atom, negated_total(Query.ir)> {
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
[[nodiscard]] consteval auto make_conditions() -> std::array<bdb_condition, condition_total(Query.ir)> {
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
[[nodiscard]] consteval auto make_finds() -> std::array<bdb_find_term, find_total(Query.ir)> {
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
[[nodiscard]] consteval auto make_rules() -> std::array<bdb_rule, rule_total(Query.ir)> {
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
[[nodiscard]] consteval auto make_heads() -> std::array<bdb_head_term, head_total(Query.ir)> {
	auto out = std::array<bdb_head_term, head_total(Query.ir)>{};
	auto at = std::size_t{0};
	for (auto pred = std::size_t{0}; pred != predicate_total(Query.ir); ++pred) {
		for (auto index = std::size_t{0}; index != pred_head_count(Query.ir, pred); ++index) {
			auto const& column = pred_head(Query.ir, pred, index);
			out[at] = column.form == find_form::variable
                ? bdb_head_term{
                      .kind = abi_tag(bdb_head_term_kind::BDB_HEAD_TERM_KIND_VAR),
                      .op = abi_tag(bdb_head_op::BDB_HEAD_OP_SUM),
                  }
                : bdb_head_term{
                      .kind =
                          abi_tag(bdb_head_term_kind::BDB_HEAD_TERM_KIND_AGGREGATE),
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
[[nodiscard]] consteval auto make_predicates() -> std::array<bdb_predicate, predicate_total(Query.ir)> {
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

}

export namespace bdb::foreign {

/**
 * The whole lowered program as ONE static constant view graph: the recs
 * in declaration order, the output predicate last (`output = rec count`
 * — lowering.md §4.2; a plain query is the degenerate no-rec program,
 * output 0). Every pointer in the graph aims at `static constexpr`
 * storage, so the view outlives any `bdb_db_prepare` call by
 * construction.
 */
template<auto Query>
inline constexpr auto program_of = bdb_program{
    .predicates = program_predicates<Query>.data(),
    .predicate_count = predicate_total(Query.ir),
    .output = static_cast<std::uint16_t>(Query.ir.rec_count),
};

[[nodiscard]] inline auto wire_param(bool value) -> bdb_param {
	auto out = bdb_param{};
	out.kind = abi_tag(bdb_param_kind::BDB_PARAM_KIND_SCALAR);
	out.scalar.kind = abi_tag(bdb_value_kind::BDB_VALUE_KIND_BOOL);
	out.scalar.bool_value = abi_flag(value);
	return out;
}

[[nodiscard]] inline auto wire_param(std::uint64_t value) -> bdb_param {
	auto out = bdb_param{};
	out.kind = abi_tag(bdb_param_kind::BDB_PARAM_KIND_SCALAR);
	out.scalar.kind = abi_tag(bdb_value_kind::BDB_VALUE_KIND_U64);
	out.scalar.u64_value = value;
	return out;
}

[[nodiscard]] inline auto wire_param(std::int64_t value) -> bdb_param {
	auto out = bdb_param{};
	out.kind = abi_tag(bdb_param_kind::BDB_PARAM_KIND_SCALAR);
	out.scalar.kind = abi_tag(bdb_value_kind::BDB_VALUE_KIND_I64);
	out.scalar.i64_value = value;
	return out;
}

/**
 * Borrowed for the call; the bridge copies before returning.
 */
[[nodiscard]] inline auto wire_param(std::string_view value) -> bdb_param {
	auto out = bdb_param{};
	out.kind = abi_tag(bdb_param_kind::BDB_PARAM_KIND_SCALAR);
	out.scalar.kind = abi_tag(bdb_value_kind::BDB_VALUE_KIND_STRING);
	out.scalar.string_value = bdb_string_view{
	    .data = value.empty() ? nullptr : std::bit_cast<std::uint8_t const*>(value.data()),
	    .len = value.size(),
	};
	return out;
}

/**
 * Borrowed for the call; the bridge copies before returning.
 */
[[nodiscard]] inline auto wire_param(std::span<std::byte const> value) -> bdb_param {
	auto out = bdb_param{};
	out.kind = abi_tag(bdb_param_kind::BDB_PARAM_KIND_SCALAR);
	out.scalar.kind = abi_tag(bdb_value_kind::BDB_VALUE_KIND_FIXED_BYTES);
	out.scalar.bytes_value = bdb_bytes_view{
	    .data = value.empty() ? nullptr : std::bit_cast<std::uint8_t const*>(value.data()),
	    .len = value.size(),
	};
	return out;
}

[[nodiscard]] inline auto wire_param(interval<std::uint64_t> value) -> bdb_param {
	auto out = bdb_param{};
	out.kind = abi_tag(bdb_param_kind::BDB_PARAM_KIND_SCALAR);
	out.scalar.kind = abi_tag(bdb_value_kind::BDB_VALUE_KIND_INTERVAL_U64);
	out.scalar.interval_u64_start = value.lo();
	out.scalar.interval_u64_end = value.hi();
	return out;
}

[[nodiscard]] inline auto wire_param(interval<std::int64_t> value) -> bdb_param {
	auto out = bdb_param{};
	out.kind = abi_tag(bdb_param_kind::BDB_PARAM_KIND_SCALAR);
	out.scalar.kind = abi_tag(bdb_value_kind::BDB_VALUE_KIND_INTERVAL_I64);
	out.scalar.interval_i64_start = value.lo();
	out.scalar.interval_i64_end = value.hi();
	return out;
}

/**
 * The set-cell scratch of one execute call: every runtime ∈-set param's
 * tagged cells live here for exactly the call's extent (the bridge
 * copies before returning). The OUTER vector may grow (its inner buffers
 * never move), so earlier set views stay valid.
 */
using param_scratch = std::vector<std::vector<bdb_value>>;

}

namespace bdb::foreign {

template<class Member>
[[nodiscard]] auto wire_one(Member const& value, param_scratch&) -> bdb_param {
	return wire_param(value);
}

template<class Element>
[[nodiscard]] auto wire_one(std::span<Element const> values, param_scratch& scratch) -> bdb_param {
	auto cells = std::vector<bdb_value>{};
	cells.reserve(values.size());
	for (auto const& element : values) {
		cells.push_back(wire_param(element).scalar);
	}
	auto const& stored = scratch.emplace_back(std::move(cells));
	auto out = bdb_param{};
	out.kind = abi_tag(bdb_param_kind::BDB_PARAM_KIND_SET);
	out.set = stored.empty() ? nullptr : stored.data();
	out.set_len = stored.size();
	return out;
}

}

export namespace bdb::foreign {

/**
 * The whole params product marshalled POSITIONALLY: member declaration
 * order IS the registry order, which IS the engine's positional ParamId
 * order. Runtime ∈-set members marshal through `scratch`, which must
 * outlive the execute call.
 */
template<class Params>
[[nodiscard]] auto wire_params(Params const& params, param_scratch& scratch) {
	auto const& [... values] = params;
	return std::array<bdb_param, sizeof...(values)>{wire_one(values, scratch)...};
}

}

namespace bdb::foreign {

[[nodiscard]] consteval auto membership_cell_total(query_ir const& ir) -> std::size_t {
	auto total = std::size_t{0};
	for (auto index = std::size_t{0}; index != ir.param_count; ++index) {
		if (ir.params[index].membership) {
			total += ir.params[index].member_count;
		}
	}
	return total;
}

/**
 * A closed-membership array's set is a PROGRAM CONSTANT pre-resolved at
 * build; execution injects it positionally — the params product never
 * carries it (lowering.md §4.2). The cells live in static constexpr
 * storage, like the rest of the program view graph.
 */
template<auto Query>
[[nodiscard]] consteval auto make_membership_cells() -> std::array<bdb_value, membership_cell_total(Query.ir)> {
	auto out = std::array<bdb_value, membership_cell_total(Query.ir)>{};
	auto at = std::size_t{0};
	for (auto index = std::size_t{0}; index != Query.ir.param_count; ++index) {
		auto const& parameter = Query.ir.params[index];
		if (!parameter.membership) {
			continue;
		}
		for (auto member = std::size_t{0}; member != parameter.member_count; ++member) {
			out[at].kind = abi_tag(bdb_value_kind::BDB_VALUE_KIND_U64);
			out[at].u64_value = parameter.members[member];
			++at;
		}
	}
	return out;
}

template<auto Query>
inline constexpr auto membership_cells = make_membership_cells<Query>();

}

export namespace bdb::foreign {

/**
 * The query-directed execute marshal: the caller's params product fills
 * the scalar/runtime-set entries in registry order, and every MEMBERSHIP
 * entry is injected from the query's frozen set constant (positional
 * ParamId order — lowering.md §5.1). `scratch` owns the runtime set
 * cells and must outlive the execute call.
 */
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
			set.kind = abi_tag(bdb_param_kind::BDB_PARAM_KIND_SET);
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

}
