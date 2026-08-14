export module bumbledb:axioms;

import std;
import :name;
import :classify;

export namespace bdb {

/**
 * Engine `MAX_EXTENSION_ROWS` (`crates/bumbledb-theory/src/schema.rs`) —
 * the one closed-vocabulary size. Flattened NTTP arrays use this number
 * as storage; live counts are pack lengths.
 */
inline constexpr std::size_t max_extension_rows = 256;

/**
 * One ground-axiom literal, flattened structural. The schema-lane wire
 * carries it as a VALUE literal — never pre-resolved (lowering.md §7.8).
 */
struct axiom_literal {
	value_kind kind;
	bool boolean;
	std::uint64_t u64;
	std::int64_t i64;
	name_text text;
};

/**
 * A closed relation's sealed extension, flattened for the wire lane:
 * handles in declaration order (row id = index) and one axiom literal
 * per (handle, declared payload column), row-major at a fixed stride
 * (lowering.md §7.4).
 */
struct closed_info {
	std::size_t handle_count{};
	std::array<name_text, max_extension_rows> handles{};
	std::size_t column_count{};
	std::array<axiom_literal, max_extension_rows * 16> axioms{};
};

}

namespace bdb::detail {

auto closed_extension_exceeds_flattened_storage() -> void;

/**
 * The flattened wire carrier: handles + axiom literals off the payload
 * values, declaration order everywhere (lowering.md §7.4).
 */
template<fixed_string Name, class Payload, std::size_t Count>
[[nodiscard]] consteval auto closed_info_of(std::array<name_text, Count> const& handles, std::array<Payload, Count> const& payloads) -> closed_info {
	auto out = closed_info{};
	out.handle_count = Count;
	for (auto index = std::size_t{0}; index != Count; ++index) {
		out.handles[index] = handles[index];
	}
	constexpr auto columns =
	    std::define_static_array(std::meta::nonstatic_data_members_of(^^Payload, std::meta::access_context::current()));
	out.column_count = columns.size();
	if (Count * out.column_count > out.axioms.size()) {
		closed_extension_exceeds_flattened_storage();
	}
	template for (constexpr auto column : index_array<columns.size()>()) {
		constexpr auto cls = classify(std::meta::type_of(columns[column])).value_or(field_class{value_kind::u64, 0});
		for (auto handle = std::size_t{0}; handle != Count; ++handle) {
			auto& literal = out.axioms[handle * out.column_count + column];
			auto const& value = payloads[handle].[:columns[column]:];
			literal.kind = cls.kind;
			if constexpr (cls.kind == value_kind::boolean) {
				literal.boolean = value;
			} else if constexpr (cls.kind == value_kind::u64) {
				literal.u64 = value;
			} else if constexpr (cls.kind == value_kind::i64) {
				literal.i64 = value;
			} else if constexpr (cls.kind == value_kind::string) {
				literal.text = to_name_text(std::string_view{value});
			}
		}
	}
	return out;
}

}
