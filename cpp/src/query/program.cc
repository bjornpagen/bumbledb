// :program — stratified recursion (ts/src/query/predicate.ts;
// lowering.md §4): `bdb::rec<"name">(rules...)`, `bdb::output(rules...)`,
// and the program entry point that seals the whole lowered program as an
// ordinary query value.
export module bumbledb:program;

import std;
import :name;
import :ir;
import :lower;
import :schema;
import :query;

export namespace bdb {

// ————————————————————————————————————————————————————————————————————
// Stratified recursion (ts/src/query/predicate.ts; lowering.md §4).
// ————————————————————————————————————————————————————————————————————

/// One recursive predicate's DEFINITION: the name plus its rule builders
/// (evaluated inside bdb::program, where the schema is known). Rule 0
/// seals the head signature; every later rule derives the same head.
template<fixed_string Name, class... Builds>
struct rec_def {
	std::tuple<Builds...> builds;
};

/// Declares one recursive predicate — `bdb::rec<"reach">(rule0, rule1)`.
/// Declaration order inside bdb::program = the dense PredId.
template<fixed_string Name, class... Builds>
[[nodiscard]] consteval auto rec(Builds... builds) -> rec_def<Name, Builds...> {
	static_assert(sizeof...(Builds) >= 1, "bumbledb rec(): a predicate with no defining clause seals no "
	                                      "signature — give the rec at least one rule");
	static_assert(sizeof...(Builds) <= max_query_rules, "bumbledb rec(): too many rules for one predicate");
	return {std::tuple{builds...}};
}

/// The OUTPUT predicate's definition (one rule per build; multiple rules
/// = set union). Must be bdb::program's LAST argument.
template<class... Builds>
struct output_def {
	std::tuple<Builds...> builds;
};

template<class... Builds>
[[nodiscard]] consteval auto output(Builds... builds) -> output_def<Builds...> {
	static_assert(sizeof...(Builds) >= 1, "bumbledb output(): the output needs at least one rule");
	static_assert(sizeof...(Builds) <= max_query_rules, "bumbledb output(): too many output rules");
	return {std::tuple{builds...}};
}

} // namespace bdb

namespace bdb::detail {

template<class T>
inline constexpr bool is_rec_def_v = false;

template<fixed_string Name, class... Builds>
inline constexpr bool is_rec_def_v<rec_def<Name, Builds...>> = true;

template<class T>
inline constexpr bool is_output_def_v = false;

template<class... Builds>
inline constexpr bool is_output_def_v<output_def<Builds...>> = true;

template<class T>
struct rec_name_of_t;

template<fixed_string Name, class... Builds>
struct rec_name_of_t<rec_def<Name, Builds...>> {
	static constexpr name_text value = to_name_text(Name.view());
};

/// Evaluates one rule builder under the schema's rule scope.
template<class S, class Build>
[[nodiscard]] consteval auto built_rule(Build const& build) -> rule_data {
	auto const result = build(rule_scope<S>{});
	static_assert(std::same_as<std::remove_cvref_t<decltype(result)>, rule_data>,
	              "bumbledb program rule: the rule body must end in .find(...)");
	return result;
}

/// One predicate's evaluated rules.
struct built_pred {
	name_text name;
	std::size_t rule_count;
	std::array<rule_data, max_query_rules> rules;
};

template<class S, class Part>
[[nodiscard]] consteval auto built_pred_of(Part const& part) -> built_pred {
	auto out = built_pred{};
	out.name = rec_name_of_t<Part>::value;
	auto const add = [&](rule_data const& rule) {
		out.rules[out.rule_count] = rule;
		++out.rule_count;
	};
	std::apply(
	    [&](auto const&... builds) {
		    (add(built_rule<S>(builds)), ...);
	    },
	    part.builds);
	return out;
}

} // namespace bdb::detail

export namespace bdb {

/// The stratified-program entry point (the TS `program(S, p => ...)`):
/// `bdb::program(S, bdb::rec<"reach">(rule...), ..., bdb::output(rule...))`.
/// Recs in declaration order (PredId = index), the output predicate last
/// (`output = rec count` — lowering.md §4.2). Each rec's rule 0 seals its
/// head; a rec's rules may `.idb` ONLY the rec itself (the self-recursion
/// cut) and project bound variables only; output rules join/negate any
/// FINISHED stratum and aggregate freely. The param registry folds the
/// recs' uses first (declaration order, rules in order), then the
/// output's — registry order = positional bind order. The sealed program
/// is an ordinary query value: `db.prepare<Program>()`.
template<Theory S, class... Parts>
[[nodiscard]] consteval auto program(S const&, Parts const&... parts) -> query_value<S> {
	constexpr auto part_count = sizeof...(Parts);
	static_assert(part_count >= 1 && (detail::is_output_def_v<Parts...[part_count - 1]>),
	              "bumbledb program(): the LAST argument must be bdb::output(...) — "
	              "the sealed program IS the query value");
	static_assert(((detail::is_rec_def_v<Parts> || detail::is_output_def_v<Parts>) && ...),
	              "bumbledb program(): every argument after the schema is a "
	              "bdb::rec<\"name\">(rules...) or the final bdb::output(rules...)");
	constexpr auto rec_total = (std::size_t{0} + ... + (detail::is_rec_def_v<Parts> ? 1U : 0U));
	static_assert(rec_total + 1 == part_count, "bumbledb program(): bdb::output(...) is declared once, last");
	if constexpr (rec_total > max_program_recs) {
		detail::program_has_too_many_recs();
	}

	// 1. Evaluate every rule builder (recs in declaration order, output
	//    last) under the schema's rule scope.
	auto recs = std::array<detail::built_pred, rec_total == 0 ? 1 : rec_total>{};
	auto output_rules = detail::built_pred{};
	{
		auto rec_index = std::size_t{0};
		auto const eat = [&]<class Part>(Part const& part) {
			if constexpr (detail::is_rec_def_v<Part>) {
				recs[rec_index] = detail::built_pred_of<S>(part);
				++rec_index;
			} else {
				auto out = detail::built_pred{};
				auto const add = [&](rule_data const& rule) {
					out.rules[out.rule_count] = rule;
					++out.rule_count;
				};
				std::apply(
				    [&](auto const&... builds) {
					    (add(detail::built_rule<S>(builds)), ...);
				    },
				    part.builds);
				output_rules = out;
			}
		};
		(eat(parts), ...);
	}

	// 2. Distinct rec names; sealed heads (rule 0 of each rec) with the
	//    recursive-head roster wall (bound variables only — judged in
	//    lower_rule) and the alignment wall across each rec's rules.
	auto out = query_value<S>{};
	auto& ir = out.ir;
	ir.rec_count = rec_total;
	for (auto index = std::size_t{0}; index != rec_total; ++index) {
		for (auto other = std::size_t{0}; other != index; ++other) {
			if (recs[other].name == recs[index].name) {
				detail::program_rec_names_must_be_distinct();
			}
		}
		if (recs[index].rule_count == 0) {
			detail::program_rec_defines_at_least_one_rule();
		}
		auto const& seal = recs[index].rules[0];
		ir.recs[index].head_name = recs[index].name;
		ir.recs[index].head_count = seal.find_count;
		for (auto column = std::size_t{0}; column != seal.find_count; ++column) {
			ir.recs[index].head[column] = seal.finds[column];
		}
		for (auto rule = std::size_t{1}; rule != recs[index].rule_count; ++rule) {
			detail::align_head(ir.recs[index].head_count, ir.recs[index].head, recs[index].rules[rule]);
		}
	}

	// 3. The param registry fold: recs first (declaration order, rules in
	//    order), output rules last (lowering.md §4.2).
	for (auto index = std::size_t{0}; index != rec_total; ++index) {
		for (auto rule = std::size_t{0}; rule != recs[index].rule_count; ++rule) {
			detail::fold_uses(ir, recs[index].rules[rule].state);
		}
	}
	for (auto rule = std::size_t{0}; rule != output_rules.rule_count; ++rule) {
		detail::fold_uses(ir, output_rules.rules[rule].state);
	}

	// 4. Lower every rule (rec rules under the self-recursion cut; the
	//    output rules under the finished-strata rules), then seal the
	//    output head.
	for (auto index = std::size_t{0}; index != rec_total; ++index) {
		ir.recs[index].rule_count = recs[index].rule_count;
		for (auto rule = std::size_t{0}; rule != recs[index].rule_count; ++rule) {
			ir.recs[index].rules[rule] = detail::lower_rule(ir, recs[index].rules[rule], index);
		}
	}
	for (auto rule = std::size_t{0}; rule != output_rules.rule_count; ++rule) {
		auto const& data = output_rules.rules[rule];
		auto const lowered = detail::lower_rule(ir, data, detail::no_rec);
		if (rule == 0) {
			ir.head_count = data.find_count;
			for (auto column = std::size_t{0}; column != data.find_count; ++column) {
				ir.head[column] = data.finds[column];
			}
		} else {
			detail::align_head(ir.head_count, ir.head, data);
		}
		ir.rules[ir.rule_count] = lowered;
		++ir.rule_count;
	}
	return out;
}

} // namespace bdb
