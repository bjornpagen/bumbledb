/**
 * The schema wire lane: flattened schema tables lowered to the foreign
 * owned spec builder — declared statements only, newtype slots fed from
 * the law-computed class map (lowering.md §2/§7).
 */
export module bumbledb:wire;

import std;
import :name;
import :classify;
import :spec;
import :axioms;
import :schema;
import bumbledb_foreign;

namespace bdb::detail {

[[nodiscard]] auto wire_type_of(field_data const& field) -> foreign::bdb_value_type {
	switch (field.kind) {
	case value_kind::boolean:
		return foreign::scalar_type(foreign::bdb_value_type_kind::BDB_VALUE_TYPE_KIND_BOOL);
	case value_kind::u64:
		return foreign::scalar_type(foreign::bdb_value_type_kind::BDB_VALUE_TYPE_KIND_U64);
	case value_kind::i64:
		return foreign::scalar_type(foreign::bdb_value_type_kind::BDB_VALUE_TYPE_KIND_I64);
	case value_kind::string:
		return foreign::scalar_type(foreign::bdb_value_type_kind::BDB_VALUE_TYPE_KIND_STRING);
	case value_kind::fixed_bytes:
		return foreign::fixed_bytes_type(field.fixed_len);
	case value_kind::interval_u64:
		return field.width == 0 ? foreign::interval_type(foreign::bdb_interval_element::BDB_INTERVAL_ELEMENT_U64)
		                        : foreign::fixed_interval_type(foreign::bdb_interval_element::BDB_INTERVAL_ELEMENT_U64, field.width);
	case value_kind::interval_i64:
		break;
	}
	return field.width == 0 ? foreign::interval_type(foreign::bdb_interval_element::BDB_INTERVAL_ELEMENT_I64)
	                        : foreign::fixed_interval_type(foreign::bdb_interval_element::BDB_INTERVAL_ELEMENT_I64, field.width);
}

/**
 * The coordinate's law-computed class name, rendered "Relation.field"
 * for the newtype slot (lowering.md §1.10/§7.7); nullopt on bare.
 */
template<class Classes>
[[nodiscard]] auto newtype_of(Classes const& classes, name_text relation, name_text field) -> std::optional<std::string> {
	for (auto const& entry : classes) {
		if (entry.coordinate.relation == relation && entry.coordinate.field == field) {
			if (!entry.classed) {
				return std::nullopt;
			}
			return std::string{entry.class_name.relation.view()} + "." + std::string{entry.class_name.field.view()};
		}
	}
	return std::nullopt;
}

/**
 * One schema-lane σ/axiom literal, owned. Handles cross BY NAME — the
 * engine resolves them (lowering.md §7.8); values are tagged.
 */
[[nodiscard]] auto owned_literal_of(selection_literal const& literal) -> foreign::owned_literal {
	auto out = foreign::owned_literal{};
	if (literal.is_handle) {
		out.is_handle = true;
		out.handle = std::string{literal.handle.view()};
		return out;
	}
	switch (literal.kind) {
	case value_kind::boolean:
		out.kind = foreign::bdb_value_kind::BDB_VALUE_KIND_BOOL;
		out.boolean = literal.boolean;
		return out;
	case value_kind::u64:
		out.kind = foreign::bdb_value_kind::BDB_VALUE_KIND_U64;
		out.u64 = literal.u64;
		return out;
	case value_kind::i64:
		out.kind = foreign::bdb_value_kind::BDB_VALUE_KIND_I64;
		out.i64 = literal.i64;
		return out;
	case value_kind::string:
	case value_kind::fixed_bytes:
	case value_kind::interval_u64:
	case value_kind::interval_i64:
		break;
	}
	out.kind = foreign::bdb_value_kind::BDB_VALUE_KIND_STRING;
	out.text = std::string{literal.text.view()};
	return out;
}

[[nodiscard]] auto owned_axiom_of(axiom_literal const& literal) -> foreign::owned_literal {
	auto out = foreign::owned_literal{};
	switch (literal.kind) {
	case value_kind::boolean:
		out.kind = foreign::bdb_value_kind::BDB_VALUE_KIND_BOOL;
		out.boolean = literal.boolean;
		return out;
	case value_kind::u64:
		out.kind = foreign::bdb_value_kind::BDB_VALUE_KIND_U64;
		out.u64 = literal.u64;
		return out;
	case value_kind::i64:
		out.kind = foreign::bdb_value_kind::BDB_VALUE_KIND_I64;
		out.i64 = literal.i64;
		return out;
	case value_kind::string:
	case value_kind::fixed_bytes:
	case value_kind::interval_u64:
	case value_kind::interval_i64:
		break;
	}
	out.kind = foreign::bdb_value_kind::BDB_VALUE_KIND_STRING;
	out.text = std::string{literal.text.view()};
	return out;
}

/**
 * A closed relation's declared FieldSpecs carry its intrinsic payload
 * columns only — the synthetic id is materialized by engine validation,
 * never spelled in the spec (lowering.md §7.3).
 */
template<Theory S>
[[nodiscard]] auto owned_relations_of(S const& theory) -> std::vector<foreign::owned_relation> {
	auto relations = std::vector<foreign::owned_relation>{};
	relations.reserve(theory.relation_table.size());
	for (auto index = std::size_t{0}; index != theory.relation_table.size(); ++index) {
		auto const& relation = theory.relation_table[index];
		auto const first_field = theory.relation_is_closed(index) ? std::size_t{1} : std::size_t{0};
		auto fields = std::vector<foreign::owned_field>{};
		fields.reserve(relation.field_count - first_field);
		for (auto field_index = first_field; field_index != relation.field_count; ++field_index) {
			auto const& field = relation.fields[field_index];
			fields.push_back(foreign::owned_field{
			    .name = std::string{field.name.view()},
			    .value_type = wire_type_of(field),
			    .newtype = newtype_of(theory.classes, relation.name, field.name),
			    .fresh = field.fresh,
			});
		}
		auto closed = std::optional<foreign::owned_closed>{};
		if (auto const* data = theory.closed_of(index)) {
			auto rows = std::vector<foreign::owned_closed_row>{};
			rows.reserve(data->handle_count);
			for (auto handle = std::size_t{0}; handle != data->handle_count; ++handle) {
				auto values = std::vector<foreign::owned_literal>{};
				values.reserve(data->column_count);
				for (auto column = std::size_t{0}; column != data->column_count; ++column) {
					values.push_back(owned_axiom_of(data->axioms[handle * max_closed_columns + column]));
				}
				rows.push_back(foreign::owned_closed_row{
				    .handle = std::string{data->handles[handle].view()},
				    .values = std::move(values),
				});
			}
			closed = foreign::owned_closed{
			    .newtype = std::string{relation.name.view()} + ".id",
			    .rows = std::move(rows),
			};
		}
		relations.push_back(foreign::owned_relation{
		    .name = std::string{relation.name.view()},
		    .fields = std::move(fields),
		    .closed = std::move(closed),
		});
	}
	return relations;
}

[[nodiscard]] auto owned_side_of(side_data const& side) -> foreign::owned_side {
	auto projection = std::vector<std::string>{};
	projection.reserve(side.width);
	for (auto index = std::size_t{0}; index != side.width; ++index) {
		projection.emplace_back(side.fields[index].view());
	}
	auto selection = std::vector<foreign::owned_selection>{};
	selection.reserve(side.selection_count);
	for (auto binding = std::size_t{0}; binding != side.selection_count; ++binding) {
		auto const& data = side.selections[binding];
		auto literals = std::vector<foreign::owned_literal>{};
		literals.reserve(data.literal_count);
		for (auto literal = std::size_t{0}; literal != data.literal_count; ++literal) {
			literals.push_back(owned_literal_of(data.literals[literal]));
		}
		selection.push_back(foreign::owned_selection{
		    .field = std::string{data.field.view()},
		    .literals = std::move(literals),
		});
	}
	return foreign::owned_side{
	    .relation = std::string{side.relation.view()},
	    .projection = std::move(projection),
	    .selection = std::move(selection),
	};
}

[[nodiscard]] auto owned_bound_of(bound_data const& bound) -> foreign::owned_bound {
	auto const kind = [&] {
		switch (bound.form) {
		case bound_form::lit:
			return foreign::bdb_bound_kind::BDB_BOUND_KIND_LIT;
		case bound_form::field:
			return foreign::bdb_bound_kind::BDB_BOUND_KIND_FIELD;
		case bound_form::duration_field:
			break;
		}
		return foreign::bdb_bound_kind::BDB_BOUND_KIND_DURATION_FIELD;
	}();
	return foreign::owned_bound{
	    .kind = kind,
	    .lit = bound.lit,
	    .field = std::string{bound.field.view()},
	};
}

template<Theory S>
[[nodiscard]] auto owned_statements_of(S const& theory) -> std::vector<foreign::owned_statement> {
	auto statements = std::vector<foreign::owned_statement>{};
	statements.reserve(theory.statements.size());
	for (auto const& statement : theory.statements) {
		switch (statement.form) {
		case statement_form::key: {
			auto side = owned_side_of(statement.source);
			statements.push_back(foreign::owned_fd{
			    .relation = std::move(side.relation),
			    .projection = std::move(side.projection),
			});
			break;
		}
		case statement_form::containment:
			statements.push_back(foreign::owned_containment{
			    .source = owned_side_of(statement.source),
			    .target = owned_side_of(statement.target),
			    .bidirectional = statement.bidirectional,
			});
			break;
		case statement_form::capacity: {
			auto const weight_kind = [&] {
				switch (statement.weight) {
				case weight_form::unit:
					return foreign::bdb_weight_kind::BDB_WEIGHT_KIND_UNIT;
				case weight_form::field:
					return foreign::bdb_weight_kind::BDB_WEIGHT_KIND_FIELD;
				case weight_form::duration_field:
					break;
				}
				return foreign::bdb_weight_kind::BDB_WEIGHT_KIND_DURATION_FIELD;
			}();
			auto const window_kind = [&] {
				switch (statement.window.form) {
				case window_form::exact:
					return foreign::bdb_capacity_window_kind::BDB_CAPACITY_WINDOW_KIND_EXACT;
				case window_form::range:
					return foreign::bdb_capacity_window_kind::BDB_CAPACITY_WINDOW_KIND_RANGE;
				case window_form::floor:
					break;
				}
				return foreign::bdb_capacity_window_kind::BDB_CAPACITY_WINDOW_KIND_FLOOR;
			}();
			statements.push_back(foreign::owned_capacity{
			    .target = owned_side_of(statement.target),
			    .weight =
			        foreign::owned_weight{
			            .kind = weight_kind,
			            .field = std::string{statement.weight_field.view()},
			        },
			    .window =
			        foreign::owned_capacity_window{
			            .kind = window_kind,
			            .lo = owned_bound_of(statement.window.lo),
			            .hi = owned_bound_of(statement.window.hi),
			        },
			    .source = owned_side_of(statement.source),
			});
			break;
		}
		}
	}
	return statements;
}

}
