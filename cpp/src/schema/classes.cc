// :classes — reading facades and statements into the flattened tables,
// the class-law analysis driver over them, and the §34 diagnostics that
// name semantic coordinates (TODO_CPP §9–§10, §34; lowering.md §2–§3).
export module bumbledb:classes;

import std;
import :name;
import :classify;
import :spec;
import :schema_member;
import :closed_facade;
import :key;
import :contained;
import :capacity;
import :unionfind;

namespace bdb::detail {

template<class T>
inline constexpr bool is_statement_v = is_key_v<T> || is_containment_v<T> || is_capacity_v<T>;

// The :interval diagnostic convention (see :capacity's hook block).
auto relation_exceeds_max_relation_fields() -> void;

// ————————————————————————————————————————————————————————————————————
// Reading facades and statements into the flattened tables.
// ————————————————————————————————————————————————————————————————————

/// One facade's flattened relation entry, read off its coordinate-shaped
/// members. Ordinary facades contribute every member; a CLOSED facade's
/// columns are its sealed roster — the synthetic `id` (a `closed_id`,
/// index 0) plus the payload coordinates — while its handle constants,
/// axiom readback, and wire carrier are filtered out. The closed axioms
/// themselves are VALUE data; schema() copies them off the facade value.
template<class Facade>
[[nodiscard]] consteval auto relation_entry() -> relation_data {
	constexpr auto members = std::define_static_array(std::meta::nonstatic_data_members_of(^^Facade, std::meta::access_context::current()));

	auto out = relation_data{};
	out.closed = is_closed_facade_type(^^Facade);
	template for (constexpr auto index : index_array<members.size()>()) {
		if constexpr (is_coordinate_like_type(std::meta::type_of(members[index]))) {
			using Coord = [:std::meta::type_of(members[index]):];
			if (out.field_count == 0) {
				out.name = Coord::relation_name;
			}
			if (out.field_count == max_relation_fields) {
				relation_exceeds_max_relation_fields();
			}
			out.fields[out.field_count] = field_data{
			    .name = Coord::field_name,
			    .kind = Coord::kind,
			    .fixed_len = Coord::fixed_len,
			    .width = Coord::cls.width,
			    .fresh = Coord::fresh,
			};
			++out.field_count;
		}
	}
	return out;
}

template<class Facade>
[[nodiscard]] consteval auto facade_relation_name_of() -> name_text {
	constexpr auto members = std::define_static_array(std::meta::nonstatic_data_members_of(^^Facade, std::meta::access_context::current()));
	using FirstCoord = [:std::meta::type_of(members[0]):];
	return FirstCoord::relation_name;
}

template<class... Args>
[[nodiscard]] consteval auto relation_count() -> std::size_t {
	return (std::size_t{0} + ... + (is_member<Args>() ? 1U : 0U));
}

template<class... Args>
[[nodiscard]] consteval auto statement_count() -> std::size_t {
	return (std::size_t{0} + ... + (is_statement_v<Args> ? 1U : 0U));
}

template<class... Args>
[[nodiscard]] consteval auto coord_count() -> std::size_t {
	auto count = std::size_t{0};
	auto const add = [&]<class A>() {
		if constexpr (is_member_type(^^A)) {
			count += relation_entry<A>().field_count;
		}
	};
	(add.template operator()<Args>(), ...);
	return count;
}

template<class... Args>
[[nodiscard]] consteval auto relation_table() -> std::array<relation_data, relation_count<Args...>()> {
	auto out = std::array<relation_data, relation_count<Args...>()>{};
	auto index = std::size_t{0};
	auto const add = [&]<class A>() {
		if constexpr (is_member_type(^^A)) {
			out[index] = relation_entry<A>();
			++index;
		}
	};
	(add.template operator()<Args>(), ...);
	return out;
}

/// One face type flattened to side data.
template<class Face>
[[nodiscard]] consteval auto side_of() -> side_data {
	auto out = side_data{};
	out.relation = Face::relation_name;
	out.width = Face::width;
	for (auto position = std::size_t{0}; position != Face::width; ++position) {
		out.fields[position] = Face::projection[position];
	}
	return out;
}

/// One statement type flattened (the capacity window's numeric payload is
/// value-borne and filled by schema() from the argument value).
template<class Statement>
[[nodiscard]] consteval auto statement_shape() -> statement_data {
	auto out = statement_data{};
	if constexpr (is_key_v<Statement>) {
		out.form = statement_form::key;
		out.source.relation = Statement::relation_name;
		out.source.width = Statement::width;
		for (auto position = std::size_t{0}; position != Statement::width; ++position) {
			out.source.fields[position] = Statement::projection[position];
		}
	} else if constexpr (is_containment_v<Statement>) {
		out.form = statement_form::containment;
		out.source = side_of<typename Statement::source_face>();
		out.target = side_of<typename Statement::target_face>();
		out.bidirectional = Statement::bidirectional;
	} else {
		out.form = statement_form::capacity;
		out.target = side_of<typename Statement::target_face>();
		out.source = side_of<typename Statement::source_face>();
		auto const weight = shape_of_weight(typename Statement::weight_type{});
		out.weight = weight.form;
		out.weight_field = weight.field;
	}
	return out;
}

template<class... Args>
[[nodiscard]] consteval auto statement_shapes() -> std::array<statement_data, statement_count<Args...>()> {
	auto out = std::array<statement_data, statement_count<Args...>()>{};
	auto index = std::size_t{0};
	auto const add = [&]<class A>() {
		if constexpr (is_statement_v<A>) {
			out[index] = statement_shape<A>();
			++index;
		}
	};
	(add.template operator()<Args>(), ...);
	return out;
}

template<class... Args>
[[nodiscard]] consteval auto analyze_schema() -> law_verdict<coord_count<Args...>()> {
	return analyze<coord_count<Args...>()>(relation_table<Args...>(), statement_shapes<Args...>());
}

// ————————————————————————————————————————————————————————————————————
// Diagnostics (§34: semantic coordinates, never template internals).
// ————————————————————————————————————————————————————————————————————

[[nodiscard]] consteval auto schema_subject(std::string_view name) -> std::string {
	return std::string{"bumbledb schema \""} + std::string{name} + "\"";
}

/// Renders one flattened statement for the wall diagnostic.
[[nodiscard]] consteval auto render_statement(statement_data const& data) -> std::string {
	auto const render_side = [](side_data const& side) -> std::string {
		auto out = std::string{"on("};
		for (auto position = std::size_t{0}; position != side.width; ++position) {
			if (position != 0) {
				out += ", ";
			}
			out += label(side.relation, side.fields[position]);
		}
		return out + ")";
	};
	if (data.form == statement_form::key) {
		auto out = std::string{"key("};
		for (auto position = std::size_t{0}; position != data.source.width; ++position) {
			if (position != 0) {
				out += ", ";
			}
			out += label(data.source.relation, data.source.fields[position]);
		}
		return out + ")";
	}
	if (data.form == statement_form::containment) {
		auto const constructor = data.bidirectional ? "mirrors(" : "contained(";
		return constructor + render_side(data.source) + ", " + render_side(data.target) + ")";
	}
	return "capacity(" + render_side(data.target) + ", ..., " + render_side(data.source) + ")";
}

template<class... Args>
[[nodiscard]] consteval auto membership_message(std::string_view name) -> std::string {
	auto const verdict = analyze_schema<Args...>();
	auto const coordinate = quoted(verdict.unknown_coordinate.relation, verdict.unknown_coordinate.field);
	auto out = schema_subject(name) + ": statement " + render_count(verdict.unknown_statement) + " references coordinate " + coordinate;
	if (verdict.relation_missing) {
		out += " but relation \"" + std::string{verdict.unknown_coordinate.relation.view()} + "\" is not a member of the schema";
	} else {
		out += " but relation \"" + std::string{verdict.unknown_coordinate.relation.view()} + "\" declares no such field";
	}
	return out;
}

template<class... Args>
[[nodiscard]] consteval auto wall_message(std::string_view name) -> std::string {
	auto const verdict = analyze_schema<Args...>();
	auto const statements = statement_shapes<Args...>();
	return schema_subject(name) + ": the statements unify two generators into one class — " +
	       quoted(verdict.generator_a.relation, verdict.generator_a.field) + " and " +
	       quoted(verdict.generator_b.relation, verdict.generator_b.field) + " (two mints cannot share a carrier) — " +
	       render_statement(statements[verdict.wall_statement]);
}

template<class... Args>
[[nodiscard]] consteval auto restated_key_message(std::string_view name) -> std::string {
	auto const verdict = analyze_schema<Args...>();
	return schema_subject(name) + ": " + render_statement(statement_shapes<Args...>()[verdict.restated_statement]) +
	       " restates the fresh-implied key of " + quoted(verdict.restated_fresh.relation, verdict.restated_fresh.field) +
	       " — the engine materializes implied keys; restating one doubles "
	       "it and moves the fingerprint";
}

template<class... Args>
[[nodiscard]] consteval auto duplicate_key_message(std::string_view name) -> std::string {
	auto const verdict = analyze_schema<Args...>();
	return schema_subject(name) + ": " + render_statement(statement_shapes<Args...>()[verdict.duplicate_statement]) +
	       " duplicates an earlier declared key";
}

/// Whether relations precede statements (the pinned argument shape).
template<class... Args>
[[nodiscard]] consteval auto relations_lead() -> bool {
	auto seen_statement = false;
	auto ordered = true;
	auto const step = [&]<class A>() {
		if constexpr (is_statement_v<A>) {
			seen_statement = true;
		} else {
			if (seen_statement) {
				ordered = false;
			}
		}
	};
	(step.template operator()<Args>(), ...);
	return ordered;
}

template<class... Args>
[[nodiscard]] consteval auto args_recognized() -> bool {
	return ((is_member<Args>() || is_statement_v<Args>) && ...);
}

template<class... Args>
[[nodiscard]] consteval auto relation_names_distinct() -> bool {
	auto const relations = relation_table<Args...>();
	for (auto first = std::size_t{0}; first != relations.size(); ++first) {
		for (auto second = first + 1; second != relations.size(); ++second) {
			if (relations[first].name == relations[second].name) {
				return false;
			}
		}
	}
	return true;
}

} // namespace bdb::detail
