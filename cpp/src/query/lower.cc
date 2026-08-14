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
template<class Ir>
[[nodiscard]] consteval auto param_id(Ir const& ir, name_text name) -> std::uint16_t {
	for (auto index = std::size_t{0}; index != ir.param_count; ++index) {
		if (ir.params[index].name == name) {
			return static_cast<std::uint16_t>(index);
		}
	}
	query_param_is_inferred_inconsistently_across_uses();
	return 0;
}

template<class Ir>
[[nodiscard]] consteval auto wire_term_of(Ir const& ir, numberer& numbers, term_data const& term) -> wire_term {
	auto out = wire_term{};
	out.form = term.form;
	switch (term.form) {
	case query_term_form::variable:
	case query_term_form::measure:
		out.var = var_id(numbers, term.variable);
		break;
	case query_term_form::param:
		out.param = param_id(ir, term.param);
		break;
	case query_term_form::param_set:
		out.param = param_id(ir, term.set.param);
		break;
	case query_term_form::literal:
		out.literal = term.literal;
		break;
	}
	return out;
}

/**
 * The param registry fold: the first use mints the dense ParamId; one
 * name keeps one shape and one anchored domain.
 */
template<class Ir>
consteval auto fold_uses(Ir& ir, rule_state const& state) -> void {
	for (auto index = std::size_t{0}; index != state.use_count; ++index) {
		auto const& use = state.uses[index];
		auto found = false;
		for (auto at = std::size_t{0}; at != ir.param_count; ++at) {
			if (!(ir.params[at].name == use.name)) {
				continue;
			}
			found = true;
			if (ir.params[at].form != use.form) {
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
		    .form = use.form,
		    .domain = use.domain,
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

template<class Ir>
[[nodiscard]] consteval auto interior_id_of(Ir const& ir, name_text name) -> std::size_t {
	for (auto index = std::size_t{0}; index != ir.interiors.size(); ++index) {
		if (ir.interiors[index].name == name) {
			return index;
		}
	}
	if constexpr (requires { ir.rec; }) {
		if (ir.rec.name == name) {
			return ir.interiors.size();
		}
	}
	return no_interior;
}

template<class Ir>
[[nodiscard]] consteval auto sealed_head_of(Ir const& ir, std::size_t id)
    -> std::array<find_data, max_query_finds> const& {
	if constexpr (requires { ir.rec; }) {
		if (id < ir.interiors.size()) {
			return ir.interiors[id].head;
		}
		return ir.rec.head;
	} else {
		return ir.interiors[id].head;
	}
}

template<class Ir>
[[nodiscard]] consteval auto sealed_head_count(Ir const& ir, std::size_t id) -> std::size_t {
	if constexpr (requires { ir.rec; }) {
		if (id < ir.interiors.size()) {
			return ir.interiors[id].head_count;
		}
		return ir.rec.head_count;
	} else {
		return ir.interiors[id].head_count;
	}
}

/**
 * Lowers one derived-table atom against its target's sealed head: binds
 * are placed and numbered in head order — `FieldId(i)` = head position i,
 * every head column bound exactly once — and each bind's variable must
 * join its head column's class.
 */
template<class Ir>
[[nodiscard]] consteval auto wire_interior_of(
    Ir const& ir, numberer& numbers, interior_atom_data const& atom, std::size_t id) -> wire_atom {
	auto const& head = sealed_head_of(ir, id);
	auto const head_count = sealed_head_count(ir, id);
	auto out = wire_atom{};
	out.source = atom_source::interior;
	out.id = static_cast<std::uint32_t>(id);
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
		if (!(bind.cls == slot.answer) || bind.classed() != slot.classed() || (slot.classed() && !(bind.law == slot.law))) {
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
template<class Ir>
[[nodiscard]] consteval auto lower_rule(Ir const& ir, rule_data const& rule, std::size_t self) -> wire_rule {
	if (rule.find_count == 0) {
		rule_finds_nothing();
	}

	for (auto index = std::size_t{0}; index != rule.state.item_count; ++index) {
		auto const& item = rule.state.items[index];
		if (item.form == body_form::condition) {
			for (auto node = std::size_t{0}; node != item.condition.node_count; ++node) {
				auto const& cond = item.condition.nodes[node];
				if (cond.form != condition_form::leaf) {
					continue;
				}
				if (!term_is_bound_var(rule.state, cond.lhs) || !term_is_bound_var(rule.state, cond.rhs)) {
					where_condition_variable_is_not_bound_in_this_rule();
				}
			}
		}
		if (item.form == body_form::negated_atom) {
			for (auto binding = std::size_t{0}; binding != item.atom.binding_count; ++binding) {
				if (!term_is_bound_var(rule.state, item.atom.bindings[binding].term)) {
					negated_atom_binds_a_variable_no_positive_atom_binds();
				}
			}
		}
		if (item.form == body_form::interior_atom || item.form == body_form::negated_interior) {
			if (self != no_interior) {
				if (item.form == body_form::negated_interior) {
					a_recursive_rule_negates_no_stratum();
				}
				if constexpr (requires { ir.rec; }) {
					if (!(item.interior.name == ir.rec.name)) {
						a_recursive_rule_matches_only_its_own_rec();
					}
				}
			}
			if (item.form == body_form::negated_interior) {
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
		if (column.form == find_form::aggregate || column.form == find_form::aggregate_measure) {
			if (column.op == fold_form::pack) {
				++pack_count;
			} else {
				++fold_count;
			}
		}
		auto const carries_over = column.form != find_form::aggregate || column.op != fold_form::count;
		if (carries_over && !term_is_bound_var(rule.state, column.over)) {
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
			atom.source = atom_source::edb;
			atom.id = item.atom.relation;
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
		} else if (item.form == body_form::interior_atom || item.form == body_form::negated_interior) {
			auto const id = interior_id_of(ir, item.interior.name);
			if (id == no_interior) {
				interior_atom_names_no_declared_table();
			}
			auto const atom = wire_interior_of(ir, numbers, item.interior, id);
			if (item.form == body_form::negated_interior) {
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
			++out.condition_count;
			out.condition_node_count += item.condition.node_count;
			for (auto node = std::size_t{0}; node != item.condition.node_count; ++node) {
				auto const& cond = item.condition.nodes[node];
				if (cond.form != condition_form::leaf) {
					continue;
				}
				(void)wire_term_of(ir, numbers, cond.lhs);
				(void)wire_term_of(ir, numbers, cond.rhs);
			}
		}
	}
	if (out.condition_node_count > max_query_conditions) {
		rule_has_too_many_conditions();
	}
	auto root_at = std::size_t{0};
	auto desc_at = out.condition_count;
	for (auto index = std::size_t{0}; index != rule.state.item_count; ++index) {
		auto const& item = rule.state.items[index];
		if (item.form != body_form::condition) {
			continue;
		}
		auto const& tree = item.condition;
		auto const desc_begin = desc_at;
		auto const map = [&](std::size_t old) -> std::size_t { return old == 0 ? root_at : desc_begin + old - 1; };
		auto wire_one = [&](condition_node const& node) {
			auto wired = wire_condition{
			    .form = node.form,
			    .op = node.op,
			    .mask = node.mask,
			    .child_begin = node.child_count == 0 ? 0 : map(node.child_begin),
			    .child_count = node.child_count,
			};
			if (node.form == condition_form::leaf) {
				wired.lhs = wire_term_of(ir, numbers, node.lhs);
				wired.rhs = wire_term_of(ir, numbers, node.rhs);
			}
			return wired;
		};
		out.conditions[root_at] = wire_one(tree.nodes[0]);
		for (auto k = std::size_t{1}; k != tree.node_count; ++k) {
			out.conditions[desc_at] = wire_one(tree.nodes[k]);
			++desc_at;
		}
		++root_at;
	}
	out.find_count = rule.find_count;
	for (auto index = std::size_t{0}; index != rule.find_count; ++index) {
		auto const& column = rule.finds[index];
		auto find = wire_find{};
		find.form = column.form;
		find.op = column.op;
		auto const carries_over = column.form != find_form::aggregate || column.op != fold_form::count;
		if (carries_over) {
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
		    lead.classed() != column.classed() || (lead.classed() && !(lead.law == column.law))) {
			every_rule_of_a_query_must_derive_the_same_head();
		}
	}
}

/** The main-query append (one answer rule; interiors and rec already in scope). */
template<class Ir>
consteval auto append_rule(Ir& ir, rule_data const& rule) -> void {
	if (rule.find_count == 0) {
		rule_finds_nothing();
	}

	fold_uses(ir, rule.state);
	auto const out = lower_rule(ir, rule, no_interior);

	auto const slot = ir.rules.size() - 1;
	if (slot == 0) {
		ir.head_count = rule.find_count;
		for (auto index = std::size_t{0}; index != rule.find_count; ++index) {
			ir.head[index] = rule.finds[index];
		}
	} else {
		align_head(ir.head_count, ir.head, rule);
	}

	ir.rules[slot] = out;
}

}
