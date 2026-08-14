/**
 * The static query-IR view builder and the execute-time param marshal
 * (lowering.md §4/§5.1). Quarantine-zone by nature — a bdb_query is a
 * graph of borrowed (pointer, count) views — but a partition of MODULE
 * bumbledb, not bumbledb_foreign: it consumes the query IR partitions,
 * and a partition of bumbledb_foreign could not import them without a
 * module cycle.
 */
export module bumbledb:query_view;

import std;
import :interval;
import :allen;
import :classify;
import :ir;
import bumbledb_foreign;

namespace bdb::foreign {

template<auto Query, class F>
consteval auto for_each_wire_rule(F&& f) -> void {
	for (auto index = std::size_t{0}; index != Query.interiors.size(); ++index) {
		for (auto rule = std::size_t{0}; rule != Query.interiors[index].rule_count; ++rule) {
			f(Query.interiors[index].rules[rule]);
		}
	}
	if constexpr (requires { Query.rec; }) {
		for (auto rule = std::size_t{0}; rule != Query.rec.base_count; ++rule) {
			f(Query.rec.base[rule]);
		}
		for (auto rule = std::size_t{0}; rule != Query.rec.rec_count; ++rule) {
			f(Query.rec.rec[rule]);
		}
	}
	for (auto rule = std::size_t{0}; rule != Query.rules.size(); ++rule) {
		f(Query.rules[rule]);
	}
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
		out.interval_u64_start = literal.u64_interval.start;
		out.interval_u64_end = literal.u64_interval.end;
		return out;
	case value_kind::string:
	case value_kind::fixed_bytes:
	case value_kind::interval_i64:
		break;
	}
	out.kind = abi_tag(bdb_value_kind::BDB_VALUE_KIND_INTERVAL_I64);
	out.interval_i64_start = literal.i64_interval.start;
	out.interval_i64_end = literal.i64_interval.end;
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
	case find_form::measure:
		out.kind = abi_tag(bdb_find_term_kind::BDB_FIND_TERM_KIND_MEASURE);
		out.var = find.over;
		return out;
	case find_form::aggregate:
		if (find.op == fold_form::count) {
			out.kind = abi_tag(bdb_find_term_kind::BDB_FIND_TERM_KIND_COUNT);
			return out;
		}
		out.kind = abi_tag(bdb_find_term_kind::BDB_FIND_TERM_KIND_AGGREGATE);
		out.over = find.over;
		return out;
	case find_form::aggregate_measure:
		break;
	}
	out.kind = abi_tag(bdb_find_term_kind::BDB_FIND_TERM_KIND_AGGREGATE_MEASURE);
	out.over = find.over;
	return out;
}

[[nodiscard]] consteval auto head_term_of(find_data const& column) -> bdb_head_term {
	return column.form == find_form::variable || column.form == find_form::measure
	           ? bdb_head_term{
	                 .kind = abi_tag(bdb_head_term_kind::BDB_HEAD_TERM_KIND_VAR),
	                 .op = abi_tag(bdb_head_op::BDB_HEAD_OP_SUM),
	             }
	           : bdb_head_term{
	                 .kind = abi_tag(bdb_head_term_kind::BDB_HEAD_TERM_KIND_AGGREGATE),
	                 .op = head_op_of(column.op),
	             };
}

template<auto Query>
[[nodiscard]] consteval auto binding_total() -> std::size_t {
	auto total = std::size_t{0};
	for_each_wire_rule<Query>([&](wire_rule const& wire) {
		for (auto atom = std::size_t{0}; atom != wire.atom_count; ++atom) {
			total += wire.atoms[atom].binding_count;
		}
		for (auto atom = std::size_t{0}; atom != wire.negated_count; ++atom) {
			total += wire.negated[atom].binding_count;
		}
	});
	return total;
}

template<auto Query>
[[nodiscard]] consteval auto atom_total() -> std::size_t {
	auto total = std::size_t{0};
	for_each_wire_rule<Query>([&](wire_rule const& wire) { total += wire.atom_count; });
	return total;
}

template<auto Query>
[[nodiscard]] consteval auto negated_total() -> std::size_t {
	auto total = std::size_t{0};
	for_each_wire_rule<Query>([&](wire_rule const& wire) { total += wire.negated_count; });
	return total;
}

template<auto Query>
[[nodiscard]] consteval auto condition_total() -> std::size_t {
	auto total = std::size_t{0};
	for_each_wire_rule<Query>([&](wire_rule const& wire) { total += wire.condition_count; });
	return total;
}

template<auto Query>
[[nodiscard]] consteval auto find_total() -> std::size_t {
	auto total = std::size_t{0};
	for_each_wire_rule<Query>([&](wire_rule const& wire) { total += wire.find_count; });
	return total;
}

template<auto Query>
[[nodiscard]] consteval auto rule_total() -> std::size_t {
	auto total = std::size_t{0};
	for_each_wire_rule<Query>([&](wire_rule const&) { ++total; });
	return total;
}

template<auto Query>
[[nodiscard]] consteval auto head_total() -> std::size_t {
	auto total = std::size_t{0};
	for (auto index = std::size_t{0}; index != Query.interiors.size(); ++index) {
		total += Query.interiors[index].head_count;
	}
	if constexpr (requires { Query.rec; }) {
		total += Query.rec.head_count;
	}
	total += Query.head_count;
	return total;
}

/**
 * Every flattened array below is built in ONE canonical walk order —
 * interiors, then rec base, then rec arms, then main, with a rule's
 * positive-atom bindings before its negated-atom bindings — so the
 * assemblers' offset arithmetic pairs each view with its owner
 * deterministically.
 */
template<auto Query>
[[nodiscard]] consteval auto make_bindings() -> std::array<bdb_binding, binding_total<Query>()> {
	auto out = std::array<bdb_binding, binding_total<Query>()>{};
	auto at = std::size_t{0};
	for_each_wire_rule<Query>([&](wire_rule const& wire) {
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
	});
	return out;
}

template<auto Query>
inline constexpr auto query_bindings = make_bindings<Query>();

[[nodiscard]] consteval auto atom_source_of(wire_atom const& atom) -> std::uint32_t {
	return abi_tag(atom.source == atom_source::interior ? bdb_atom_source_kind::BDB_ATOM_SOURCE_KIND_INTERIOR
	                                                    : bdb_atom_source_kind::BDB_ATOM_SOURCE_KIND_EDB);
}

[[nodiscard]] consteval auto atom_of(wire_atom const& source, bdb_binding const* bindings) -> bdb_atom {
	return bdb_atom{
	    .source_kind = atom_source_of(source),
	    .relation = source.source == atom_source::edb ? source.id : 0,
	    .interior = source.source == atom_source::interior ? source.id : 0,
	    .bindings = bindings,
	    .binding_count = source.binding_count,
	};
}

template<auto Query>
[[nodiscard]] consteval auto make_atoms() -> std::array<bdb_atom, atom_total<Query>()> {
	auto out = std::array<bdb_atom, atom_total<Query>()>{};
	auto at = std::size_t{0};
	auto binding_offset = std::size_t{0};
	for_each_wire_rule<Query>([&](wire_rule const& wire) {
		for (auto atom = std::size_t{0}; atom != wire.atom_count; ++atom) {
			auto const& source = wire.atoms[atom];
			out[at] = atom_of(source, source.binding_count == 0 ? nullptr : query_bindings<Query>.data() + binding_offset);
			binding_offset += source.binding_count;
			++at;
		}
		for (auto atom = std::size_t{0}; atom != wire.negated_count; ++atom) {
			binding_offset += wire.negated[atom].binding_count;
		}
	});
	return out;
}

template<auto Query>
inline constexpr auto query_atoms = make_atoms<Query>();

template<auto Query>
[[nodiscard]] consteval auto make_negated() -> std::array<bdb_atom, negated_total<Query>()> {
	auto out = std::array<bdb_atom, negated_total<Query>()>{};
	auto at = std::size_t{0};
	auto binding_offset = std::size_t{0};
	for_each_wire_rule<Query>([&](wire_rule const& wire) {
		for (auto atom = std::size_t{0}; atom != wire.atom_count; ++atom) {
			binding_offset += wire.atoms[atom].binding_count;
		}
		for (auto atom = std::size_t{0}; atom != wire.negated_count; ++atom) {
			auto const& source = wire.negated[atom];
			out[at] = atom_of(source, source.binding_count == 0 ? nullptr : query_bindings<Query>.data() + binding_offset);
			binding_offset += source.binding_count;
			++at;
		}
	});
	return out;
}

template<auto Query>
inline constexpr auto query_negated = make_negated<Query>();

template<auto Query>
[[nodiscard]] consteval auto make_conditions() -> std::array<bdb_condition, condition_total<Query>()> {
	auto out = std::array<bdb_condition, condition_total<Query>()>{};
	auto at = std::size_t{0};
	for_each_wire_rule<Query>([&](wire_rule const& wire) {
		for (auto condition = std::size_t{0}; condition != wire.condition_count; ++condition) {
			out[at] = condition_of(wire.conditions[condition]);
			++at;
		}
	});
	return out;
}

template<auto Query>
inline constexpr auto query_conditions = make_conditions<Query>();

template<auto Query>
[[nodiscard]] consteval auto make_finds() -> std::array<bdb_find_term, find_total<Query>()> {
	auto out = std::array<bdb_find_term, find_total<Query>()>{};
	auto at = std::size_t{0};
	for_each_wire_rule<Query>([&](wire_rule const& wire) {
		for (auto find = std::size_t{0}; find != wire.find_count; ++find) {
			out[at] = find_of(wire.finds[find]);
			++at;
		}
	});
	return out;
}

template<auto Query>
inline constexpr auto query_finds = make_finds<Query>();

template<auto Query>
[[nodiscard]] consteval auto make_rules() -> std::array<bdb_rule, rule_total<Query>()> {
	auto out = std::array<bdb_rule, rule_total<Query>()>{};
	auto at = std::size_t{0};
	auto atom_offset = std::size_t{0};
	auto negated_offset = std::size_t{0};
	auto condition_offset = std::size_t{0};
	auto find_offset = std::size_t{0};
	for_each_wire_rule<Query>([&](wire_rule const& wire) {
		out[at] = bdb_rule{
		    .finds = wire.find_count == 0 ? nullptr : query_finds<Query>.data() + find_offset,
		    .find_count = wire.find_count,
		    .atoms = wire.atom_count == 0 ? nullptr : query_atoms<Query>.data() + atom_offset,
		    .atom_count = wire.atom_count,
		    .negated = wire.negated_count == 0 ? nullptr : query_negated<Query>.data() + negated_offset,
		    .negated_count = wire.negated_count,
		    .conditions = wire.condition_count == 0 ? nullptr : query_conditions<Query>.data() + condition_offset,
		    .condition_count = wire.condition_count,
		};
		find_offset += wire.find_count;
		atom_offset += wire.atom_count;
		negated_offset += wire.negated_count;
		condition_offset += wire.condition_count;
		++at;
	});
	return out;
}

template<auto Query>
inline constexpr auto query_rules = make_rules<Query>();

template<auto Query>
[[nodiscard]] consteval auto make_heads() -> std::array<bdb_head_term, head_total<Query>()> {
	auto out = std::array<bdb_head_term, head_total<Query>()>{};
	auto at = std::size_t{0};
	for (auto index = std::size_t{0}; index != Query.interiors.size(); ++index) {
		for (auto column = std::size_t{0}; column != Query.interiors[index].head_count; ++column) {
			out[at] = head_term_of(Query.interiors[index].head[column]);
			++at;
		}
	}
	if constexpr (requires { Query.rec; }) {
		for (auto column = std::size_t{0}; column != Query.rec.head_count; ++column) {
			out[at] = head_term_of(Query.rec.head[column]);
			++at;
		}
	}
	for (auto column = std::size_t{0}; column != Query.head_count; ++column) {
		out[at] = head_term_of(Query.head[column]);
		++at;
	}
	return out;
}

template<auto Query>
inline constexpr auto query_heads = make_heads<Query>();

template<auto Query>
[[nodiscard]] consteval auto make_interiors() -> std::array<bdb_interior, Query.interiors.size() == 0 ? 1 : Query.interiors.size()> {
	auto out = std::array<bdb_interior, Query.interiors.size() == 0 ? 1 : Query.interiors.size()>{};
	auto rule_offset = std::size_t{0};
	auto head_offset = std::size_t{0};
	for (auto index = std::size_t{0}; index != Query.interiors.size(); ++index) {
		auto const rules = Query.interiors[index].rule_count;
		auto const heads = Query.interiors[index].head_count;
		out[index] = bdb_interior{
		    .head = heads == 0 ? nullptr : query_heads<Query>.data() + head_offset,
		    .head_count = heads,
		    .rules = rules == 0 ? nullptr : query_rules<Query>.data() + rule_offset,
		    .rule_count = rules,
		};
		rule_offset += rules;
		head_offset += heads;
	}
	return out;
}

template<auto Query>
inline constexpr auto query_interiors = make_interiors<Query>();

template<auto Query>
[[nodiscard]] consteval auto rec_rule_offset() -> std::size_t {
	auto offset = std::size_t{0};
	for (auto index = std::size_t{0}; index != Query.interiors.size(); ++index) {
		offset += Query.interiors[index].rule_count;
	}
	return offset;
}

template<auto Query>
[[nodiscard]] consteval auto rec_head_offset() -> std::size_t {
	auto offset = std::size_t{0};
	for (auto index = std::size_t{0}; index != Query.interiors.size(); ++index) {
		offset += Query.interiors[index].head_count;
	}
	return offset;
}

template<auto Query>
[[nodiscard]] consteval auto make_rec() -> bdb_rec {
	auto const rule_offset = rec_rule_offset<Query>();
	auto const head_offset = rec_head_offset<Query>();
	return bdb_rec{
	    .head = Query.rec.head_count == 0 ? nullptr : query_heads<Query>.data() + head_offset,
	    .head_count = Query.rec.head_count,
	    .base = Query.rec.base_count == 0 ? nullptr : query_rules<Query>.data() + rule_offset,
	    .base_count = Query.rec.base_count,
	    .rec = Query.rec.rec_count == 0 ? nullptr : query_rules<Query>.data() + rule_offset + Query.rec.base_count,
	    .rec_count = Query.rec.rec_count,
	};
}

template<auto Query>
    requires requires { Query.rec; }
inline constexpr auto query_rec = make_rec<Query>();

template<auto Query>
[[nodiscard]] consteval auto main_rule_offset() -> std::size_t {
	auto offset = rec_rule_offset<Query>();
	if constexpr (requires { Query.rec; }) {
		offset += Query.rec.base_count + Query.rec.rec_count;
	}
	return offset;
}

template<auto Query>
[[nodiscard]] consteval auto main_head_offset() -> std::size_t {
	auto offset = rec_head_offset<Query>();
	if constexpr (requires { Query.rec; }) {
		offset += Query.rec.head_count;
	}
	return offset;
}

}

export namespace bdb::foreign {

template<auto Query>
[[nodiscard]] consteval auto rec_ptr() -> bdb_rec const* {
	if constexpr (requires { Query.rec; }) {
		return &query_rec<Query>;
	} else {
		return nullptr;
	}
}

/**
 * The whole lowered query as ONE static constant view graph: interiors
 * in declaration order, optional rec, then the main answer. Every
 * pointer in the graph aims at `static constexpr` storage, so the view
 * outlives any `bdb_db_prepare` call by construction.
 */
template<auto Query>
inline constexpr auto query_of = bdb_query{
    .interiors = Query.interiors.size() == 0 ? nullptr : query_interiors<Query>.data(),
    .interior_count = Query.interiors.size(),
    .rec = rec_ptr<Query>(),
    .head = Query.head_count == 0 ? nullptr : query_heads<Query>.data() + main_head_offset<Query>(),
    .head_count = Query.head_count,
    .rules = Query.rules.size() == 0 ? nullptr : query_rules<Query>.data() + main_rule_offset<Query>(),
    .rule_count = Query.rules.size(),
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

template<class Ir>
[[nodiscard]] consteval auto membership_cell_total(Ir const& ir) -> std::size_t {
	auto total = std::size_t{0};
	for (auto index = std::size_t{0}; index != ir.param_count; ++index) {
		if (ir.params[index].form == param_form::membership) {
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
[[nodiscard]] consteval auto make_membership_cells() -> std::array<bdb_value, membership_cell_total(Query)> {
	auto out = std::array<bdb_value, membership_cell_total(Query)>{};
	auto at = std::size_t{0};
	for (auto index = std::size_t{0}; index != Query.param_count; ++index) {
		auto const& parameter = Query.params[index];
		if (parameter.form != param_form::membership) {
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
[[nodiscard]] auto wire_params_for(Params const& params, param_scratch& scratch) -> std::array<bdb_param, Query.param_count> {
	auto const scalars = wire_params(params, scratch);
	auto out = std::array<bdb_param, Query.param_count>{};
	auto scalar_index = std::size_t{0};
	auto member_offset = std::size_t{0};
	for (auto index = std::size_t{0}; index != Query.param_count; ++index) {
		auto const& parameter = Query.params[index];
		if (parameter.form == param_form::membership) {
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
