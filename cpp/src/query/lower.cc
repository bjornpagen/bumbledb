// :lower — rule assembly (ts/src/query/lower.ts:1337-1426, 1678-1994):
// param registry fold, boundness walls, dense variable numbering over the
// written walk, and the head-alignment wall.
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

[[nodiscard]] consteval auto param_id(query_ir const& ir, name_text name) -> std::uint16_t {
	for (auto index = std::size_t{0}; index != ir.param_count; ++index) {
		if (ir.params[index].name == name) {
			return static_cast<std::uint16_t>(index);
		}
	}
	// Unreachable by construction: every param term was recorded with a
	// use, and the uses were folded before numbering.
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

inline constexpr std::size_t no_rec = ~std::size_t{0};

/// The rec a recursive atom targets, by name (nowhere = the wall).
[[nodiscard]] consteval auto rec_index_of(query_ir const& ir, name_text name) -> std::size_t {
	for (auto index = std::size_t{0}; index != ir.rec_count; ++index) {
		if (ir.recs[index].head_name == name) {
			return index;
		}
	}
	return no_rec;
}

/// Lowers one recursive atom against its target's SEALED head: binds are
/// placed and numbered in HEAD order — `FieldId(i)` = head position i,
/// every head column bound exactly once (lowering.md §4.2) — and each
/// bind's variable must join its head column's class (the TS fieldJoins
/// wall on the idb pairing).
[[nodiscard]] consteval auto wire_idb_of(query_ir const& ir, numberer& numbers, idb_atom_data const& atom, std::size_t pred) -> wire_atom {
	auto const& target = ir.recs[pred];
	auto out = wire_atom{};
	out.idb = true;
	out.pred = static_cast<std::uint16_t>(pred);
	for (auto index = std::size_t{0}; index != atom.bind_count; ++index) {
		auto known = false;
		for (auto column = std::size_t{0}; column != target.head_count; ++column) {
			if (target.head[column].name == atom.binds[index].column) {
				known = true;
			}
		}
		if (!known) {
			idb_atom_binds_a_name_the_head_does_not_carry();
		}
	}
	for (auto column = std::size_t{0}; column != target.head_count; ++column) {
		auto const& head = target.head[column];
		auto bound = no_rec;
		for (auto index = std::size_t{0}; index != atom.bind_count; ++index) {
			if (atom.binds[index].column == head.name) {
				bound = index;
			}
		}
		if (bound == no_rec) {
			idb_atom_omits_a_head_column();
		}
		auto const& bind = atom.binds[bound];
		if (!(bind.cls == head.answer) || bind.classed != head.classed || (head.classed && !(bind.law == head.law))) {
			idb_binding_joins_only_its_head_columns_class();
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

/// Lowers one assembled rule to its numbered wire form. `self` is the
/// owning rec's index for RECURSIVE rules (their idb atoms may target
/// only the rec itself, positively — the self-recursion cut and the
/// monotonicity wall) and `no_rec` for output/query rules. Requires every
/// rec head in `ir.recs` to be sealed and this rule's param uses folded.
[[nodiscard]] consteval auto lower_rule(query_ir const& ir, rule_data const& rule, std::size_t self) -> wire_rule {
	if (rule.find_count == 0) {
		rule_finds_nothing();
	}

	// 1. Boundness and polarity walls (the TS construction-time walls;
	//    the engine's safety refusal stands behind them). The bound set
	//    carries POSITIVE bindings only (record_match/record_idb).
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
		if (item.form == body_form::idb_atom) {
			if (self != no_rec) {
				if (item.idb.negated) {
					a_recursive_rule_negates_no_stratum();
				}
				if (!(item.idb.pred == ir.recs[self].head_name)) {
					a_recursive_rule_matches_only_its_own_rec();
				}
			}
			if (item.idb.negated) {
				for (auto bind = std::size_t{0}; bind != item.idb.bind_count; ++bind) {
					if (!is_bound(rule.state, item.idb.binds[bind].variable)) {
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
		if (self != no_rec && column.form != find_form::variable) {
			// Aggregation/measure through a cycle is unrepresentable —
			// the strata judge's roster, made unwritable.
			a_recursive_rule_head_projects_bound_variables_only();
		}
		if (column.form != find_form::variable) {
			if (column.op == fold_form::pack) {
				++pack_count;
			} else {
				++fold_count;
			}
		}
		if ((column.has_over && !term_is_bound_var(rule.state, column.over)) ||
		    (column.key_present && !term_is_bound_var(rule.state, column.key))) {
			find_head_variable_is_not_bound_in_this_rule();
		}
		for (auto other = std::size_t{0}; other != index; ++other) {
			if (rule.finds[other].name == rule.finds[index].name) {
				find_head_names_must_be_distinct();
			}
		}
	}
	if (pack_count > 1 || (pack_count == 1 && fold_count != 0)) {
		// At most one pack per find, never beside a fold or an Arg entry
		// (ts/src/query/find.ts).
		pack_stands_alone_never_beside_another_aggregate();
	}

	// 2. Dense variable numbering over the written walk (body items in
	//    written order, bindings in written order — idb binds in HEAD
	//    order — finds LAST) and the bucketed wire rule.
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
		} else if (item.form == body_form::idb_atom) {
			auto const pred = rec_index_of(ir, item.idb.pred);
			if (pred == no_rec) {
				if (ir.rec_count == 0) {
					a_recursive_atom_requires_a_program();
				}
				idb_atom_names_no_declared_rec();
			}
			auto const atom = wire_idb_of(ir, numbers, item.idb, pred);
			if (item.idb.negated) {
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
		if (column.key_present) {
			// The Arg key numbers BEFORE the carried value (the TS object
			// literal's evaluation order — lowering.md §4.2 parity).
			find.key_present = true;
			find.key_is_measure = column.key.form == query_term_form::measure;
			find.key = var_id(numbers, column.key.variable);
		}
		if (column.has_over) {
			find.over = var_id(numbers, column.over.variable);
		}
		out.finds[index] = find;
	}
	return out;
}

/// The head-alignment wall shared by every predicate: rule 0 seals the
/// head; every later rule derives the same (name, shape, op, answer
/// class, law class), position for position.
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

/// The plain-query append (one OUTPUT rule; no recs in scope).
consteval auto append_rule(query_ir& ir, rule_data const& rule) -> void {
	if (ir.rule_count == max_query_rules) {
		query_has_too_many_rules();
	}
	if (rule.find_count == 0) {
		rule_finds_nothing();
	}

	// Param registry fold (first use mints the dense ParamId; one name
	// keeps one shape and one anchored domain), then the numbered rule.
	fold_uses(ir, rule.state);
	auto const out = lower_rule(ir, rule, no_rec);

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

} // namespace bdb::detail
