export module bumbledb:lower;

import std;
import :name;
import :spec;
import :ir;
import :pattern;

namespace bdb::detail {

struct numberer {
	std::size_t count;
	std::array<coord_ref, max_query_vars> minted;
};

[[nodiscard]] consteval auto var_id(numberer& numbers, coord_ref variable) -> std::uint16_t {
	for (auto index = std::size_t{0}; index != numbers.count; ++index) {
		if (numbers.minted[index] == variable) {
			return static_cast<std::uint16_t>(index);
		}
	}
	if (numbers.count == max_query_vars) {
		rule_has_too_many_variables();
	}
	numbers.minted[numbers.count] = variable;
	++numbers.count;
	return static_cast<std::uint16_t>(numbers.count - 1);
}

/**
 * Requires every param use folded before numbering — a miss is then
 * unreachable by construction.
 */
[[nodiscard]] consteval auto param_id(query_ir const& ir, name_text name) -> std::uint16_t {
	for (auto index = std::size_t{0}; index != ir.param_count; ++index) {
		if (ir.params[index].name == name) {
			return static_cast<std::uint16_t>(index);
		}
	}
	query_param_is_inferred_inconsistently_across_uses();
	return 0;
}

[[nodiscard]] consteval auto wire_term_of(query_ir const& ir, numberer& numbers, term_data const& term) -> wire_term {
	auto out = wire_term{};
	out.form = term.form;
	switch (term.form) {
	case query_term_form::variable:
	case query_term_form::measure:
		out.var = var_id(numbers, term.variable);
		break;
	case query_term_form::param:
	case query_term_form::param_set:
		out.param = param_id(ir, term.param);
		break;
	case query_term_form::literal:
		out.literal = term.literal;
		break;
	case query_term_form::absent:
		break;
	}
	return out;
}

/**
 * The param registry fold: the first use mints the dense ParamId; one
 * name keeps one shape and one anchored domain.
 */
consteval auto fold_uses(query_ir& ir, rule_state const& state) -> void {
	for (auto index = std::size_t{0}; index != state.use_count; ++index) {
		auto const& use = state.uses[index];
		auto found = false;
		for (auto at = std::size_t{0}; at != ir.param_count; ++at) {
			if (!(ir.params[at].name == use.name)) {
				continue;
			}
			found = true;
			if (ir.params[at].shape != use.shape) {
				query_param_is_used_at_two_shapes();
			}
			if (!(ir.params[at].domain == use.domain)) {
				query_param_is_inferred_inconsistently_across_uses();
			}
		}
		if (found) {
			continue;
		}
		if (ir.param_count == max_query_params) {
			query_has_too_many_params();
		}
		ir.params[ir.param_count] = param_data{
		    .name = use.name,
		    .shape = use.shape,
		    .domain = use.domain,
		    .point = use.point,
		    .membership = use.membership,
		    .member_count = use.member_count,
		    .members = use.members,
		};
		++ir.param_count;
	}
}

[[nodiscard]] consteval auto term_is_bound_var(rule_state const& state, term_data const& term) -> bool {
	if (term.form != query_term_form::variable && term.form != query_term_form::measure) {
		return true;
	}
	return is_bound(state, term.variable);
}

inline constexpr std::size_t no_interior = ~std::size_t{0};

template<std::size_t NI>
struct derived_tables {
	std::array<interior_ir, NI> const& interiors;
	bool has_rec;
	rec_ir const& rec;
};

template<std::size_t NI>
[[nodiscard]] consteval auto interior_id_of(derived_tables<NI> const& tables, name_text name) -> std::size_t {
	for (auto index = std::size_t{0}; index != NI; ++index) {
		if (tables.interiors[index].name == name) {
			return index;
		}
	}
	if (tables.has_rec && tables.rec.name == name) {
		return NI;
	}
	return no_interior;
}

template<std::size_t NI>
[[nodiscard]] consteval auto sealed_head_of(derived_tables<NI> const& tables, std::size_t id)
    -> std::array<find_data, max_query_finds> const& {
	if (id < NI) {
		return tables.interiors[id].head;
	}
	return tables.rec.head;
}

template<std::size_t NI>
[[nodiscard]] consteval auto sealed_head_count(derived_tables<NI> const& tables, std::size_t id) -> std::size_t {
	if (id < NI) {
		return tables.interiors[id].head_count;
	}
	return tables.rec.head_count;
}

/**
 * Lowers one derived-table atom against its target's sealed head: binds
 * are placed and numbered in head order — `FieldId(i)` = head position i,
 * every head column bound exactly once — and each bind's variable must
 * join its head column's class.
 */
template<std::size_t NI>
[[nodiscard]] consteval auto wire_interior_of(
    derived_tables<NI> const& tables, numberer& numbers, interior_atom_data const& atom, std::size_t id) -> wire_atom {
	auto const& head = sealed_head_of(tables, id);
	auto const head_count = sealed_head_count(tables, id);
	auto out = wire_atom{};
	out.interior = true;
	out.interior_id = static_cast<std::uint32_t>(id);
	for (auto index = std::size_t{0}; index != atom.bind_count; ++index) {
		auto known = false;
		for (auto column = std::size_t{0}; column != head_count; ++column) {
			if (head[column].name == atom.binds[index].column) {
				known = true;
			}
		}
		if (!known) {
			interior_atom_binds_a_name_the_head_does_not_carry();
		}
	}
	for (auto column = std::size_t{0}; column != head_count; ++column) {
		auto const& slot = head[column];
		auto bound = no_interior;
		for (auto index = std::size_t{0}; index != atom.bind_count; ++index) {
			if (atom.binds[index].column == slot.name) {
				bound = index;
			}
		}
		if (bound == no_interior) {
			interior_atom_omits_a_head_column();
		}
		auto const& bind = atom.binds[bound];
		if (!(bind.cls == slot.answer) || bind.classed != slot.classed || (slot.classed && !(bind.law == slot.law))) {
			interior_binding_joins_only_its_head_columns_class();
		}
		auto term = wire_term{};
		term.form = query_term_form::variable;
		term.var = var_id(numbers, bind.variable);
		out.bindings[out.binding_count] = wire_binding{
		    .field = static_cast<std::uint16_t>(column),
		    .term = term,
		};
		++out.binding_count;
	}
	return out;
}

/**
 * Lowers one assembled rule to its numbered wire form. `self` is the
 * owning rec's InteriorId for recursive rules (their interior atoms may
 * target only the rec itself, positively — the self-recursion cut and the
 * monotonicity wall) and `no_interior` for interior/main rules. Requires
 * every derived head sealed and this rule's param uses folded. Variable
 * numbering walks body items in written order — bindings in written
 * order, interior binds in head order — then the finds last, where an
 * Arg key numbers before its carried value.
 */
template<std::size_t NI>
[[nodiscard]] consteval auto lower_rule(
    query_ir const& ir, derived_tables<NI> const& tables, rule_data const& rule, std::size_t self) -> wire_rule {
	if (rule.find_count == 0) {
		rule_finds_nothing();
	}

	for (auto index = std::size_t{0}; index != rule.state.item_count; ++index) {
		auto const& item = rule.state.items[index];
		if (item.form == body_form::condition) {
			if (!term_is_bound_var(rule.state, item.condition.lhs) || !term_is_bound_var(rule.state, item.condition.rhs)) {
				where_condition_variable_is_not_bound_in_this_rule();
			}
		}
		if (item.form == body_form::negated_atom) {
			for (auto binding = std::size_t{0}; binding != item.atom.binding_count; ++binding) {
				if (!term_is_bound_var(rule.state, item.atom.bindings[binding].term)) {
					negated_atom_binds_a_variable_no_positive_atom_binds();
				}
			}
		}
		if (item.form == body_form::interior_atom) {
			if (self != no_interior) {
				if (item.interior.negated) {
					a_recursive_rule_negates_no_stratum();
				}
				if (!(item.interior.name == tables.rec.name)) {
					a_recursive_rule_matches_only_its_own_rec();
				}
			}
			if (item.interior.negated) {
				for (auto bind = std::size_t{0}; bind != item.interior.bind_count; ++bind) {
					if (!is_bound(rule.state, item.interior.binds[bind].variable)) {
						negated_atom_binds_a_variable_no_positive_atom_binds();
					}
				}
			}
		}
	}
	auto pack_count = std::size_t{0};
	auto fold_count = std::size_t{0};
	for (auto index = std::size_t{0}; index != rule.find_count; ++index) {
		auto const& column = rule.finds[index];
		if (self != no_interior && column.form != find_form::variable) {
			a_recursive_rule_head_projects_bound_variables_only();
		}
		if (column.form != find_form::variable) {
			if (column.op == fold_form::pack) {
				++pack_count;
			} else {
				++fold_count;
			}
		}
		if (column.has_over && !term_is_bound_var(rule.state, column.over)) {
			find_head_variable_is_not_bound_in_this_rule();
		}
		for (auto other = std::size_t{0}; other != index; ++other) {
			if (rule.finds[other].name == rule.finds[index].name) {
				find_head_names_must_be_distinct();
			}
		}
	}
	if (pack_count > 1 || (pack_count == 1 && fold_count != 0)) {
		pack_stands_alone_never_beside_another_aggregate();
	}

	auto numbers = numberer{};
	auto out = wire_rule{};
	for (auto index = std::size_t{0}; index != rule.state.item_count; ++index) {
		auto const& item = rule.state.items[index];
		if (item.form == body_form::atom || item.form == body_form::negated_atom) {
			auto atom = wire_atom{};
			atom.relation = item.atom.relation;
			atom.binding_count = item.atom.binding_count;
			for (auto binding = std::size_t{0}; binding != item.atom.binding_count; ++binding) {
				atom.bindings[binding] = wire_binding{
				    .field = static_cast<std::uint16_t>(item.atom.bindings[binding].field),
				    .term = wire_term_of(ir, numbers, item.atom.bindings[binding].term),
				};
			}
			if (item.form == body_form::atom) {
				if (out.atom_count == max_query_atoms) {
					rule_has_too_many_atoms();
				}
				out.atoms[out.atom_count] = atom;
				++out.atom_count;
			} else {
				if (out.negated_count == max_query_atoms) {
					rule_has_too_many_atoms();
				}
				out.negated[out.negated_count] = atom;
				++out.negated_count;
			}
		} else if (item.form == body_form::interior_atom) {
			auto const id = interior_id_of(tables, item.interior.name);
			if (id == no_interior) {
				interior_atom_names_no_declared_table();
			}
			auto const atom = wire_interior_of(tables, numbers, item.interior, id);
			if (item.interior.negated) {
				if (out.negated_count == max_query_atoms) {
					rule_has_too_many_atoms();
				}
				out.negated[out.negated_count] = atom;
				++out.negated_count;
			} else {
				if (out.atom_count == max_query_atoms) {
					rule_has_too_many_atoms();
				}
				out.atoms[out.atom_count] = atom;
				++out.atom_count;
			}
		} else {
			if (out.condition_count == max_query_conditions) {
				rule_has_too_many_conditions();
			}
			out.conditions[out.condition_count] = wire_condition{
			    .op = item.condition.op,
			    .mask = item.condition.mask,
			    .lhs = wire_term_of(ir, numbers, item.condition.lhs),
			    .rhs = wire_term_of(ir, numbers, item.condition.rhs),
			};
			++out.condition_count;
		}
	}
	out.find_count = rule.find_count;
	for (auto index = std::size_t{0}; index != rule.find_count; ++index) {
		auto const& column = rule.finds[index];
		auto find = wire_find{};
		find.form = column.form;
		find.op = column.op;
		find.has_over = column.has_over;
		if (column.has_over) {
			find.over = var_id(numbers, column.over.variable);
		}
		out.finds[index] = find;
	}
	return out;
}

/**
 * The head-alignment wall shared by every predicate: rule 0 seals the
 * head; every later rule derives the same (name, shape, op, answer class,
 * law class), position for position.
 */
consteval auto align_head(std::size_t head_count, std::array<find_data, max_query_finds> const& head, rule_data const& rule) -> void {
	if (head_count != rule.find_count) {
		every_rule_of_a_query_must_derive_the_same_head();
	}
	for (auto index = std::size_t{0}; index != rule.find_count; ++index) {
		auto const& lead = head[index];
		auto const& column = rule.finds[index];
		if (!(lead.name == column.name) || lead.form != column.form || lead.op != column.op || !(lead.answer == column.answer) ||
		    lead.classed != column.classed || (lead.classed && !(lead.law == column.law))) {
			every_rule_of_a_query_must_derive_the_same_head();
		}
	}
}

/** The main-query append (one answer rule; interiors and rec already in scope). */
template<std::size_t NI>
consteval auto append_rule(
    query_ir& ir,
    std::array<interior_ir, NI> const& interiors,
    bool has_rec,
    rec_ir const& rec,
    rule_data const& rule) -> void {
	if (ir.rule_count == max_query_rules) {
		query_has_too_many_rules();
	}
	if (rule.find_count == 0) {
		rule_finds_nothing();
	}

	fold_uses(ir, rule.state);
	auto const tables = derived_tables<NI>{.interiors = interiors, .has_rec = has_rec, .rec = rec};
	auto const out = lower_rule(ir, tables, rule, no_interior);

	if (ir.rule_count == 0) {
		ir.head_count = rule.find_count;
		for (auto index = std::size_t{0}; index != rule.find_count; ++index) {
			ir.head[index] = rule.finds[index];
		}
	} else {
		align_head(ir.head_count, ir.head, rule);
	}

	ir.rules[ir.rule_count] = out;
	++ir.rule_count;
}

}
