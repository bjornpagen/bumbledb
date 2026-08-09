// :row — reflected row-to-Value marshalling (TODO_CPP §24).
//
// This is the dynamic-insert lane used by `tx.insert(Relation, Row{...})`:
// a relation row product lowers to a contiguous array of tagged
// `bdb_value` cells, one per field, in declaration order, ready for
// `bdb_tx_insert`-style calls. The row declaration and reflection are the
// sole source of truth — no serializer registration, no manual
// to_bumbledb(), no per-relation specialization.
export module bumbledb:row;

import std;
import :classify;
import :handle;
import bumbledb_foreign;

export namespace bdb {

/// The row's field count (declaration order is the marshalling order).
template<class Row>
inline constexpr std::size_t row_field_count = detail::field_count(^^Row);

/// Lowers one row value to the engine's dynamic value representation:
/// `[U64(id), String("search")]`-shaped cells in field declaration order.
///
/// BORROW CONTRACT (TODO_CPP §24): string and bytes cells are borrowed
/// views into `row` — valid exactly while `row` is alive and unchanged,
/// i.e. for the duration of the bridge call the cells are built for (the
/// C ABI copies inbound views before returning; bumbledb_c.h pins that).
/// The returned array must not outlive `row`.
template<class Row>
[[nodiscard]] auto marshal_row(Row const& row) -> std::array<foreign::bdb_value, row_field_count<Row>> {
	static_assert(detail::row_is_supported(^^Row), detail::unsupported_field_message(detail::row_subject(^^Row), ^^Row));

	constexpr auto ctx = std::meta::access_context::current();
	constexpr auto members = std::define_static_array(std::meta::nonstatic_data_members_of(^^Row, ctx));

	auto cells = std::array<foreign::bdb_value, row_field_count<Row>>{};
	template for (constexpr auto index : detail::index_array<members.size()>()) {
		constexpr auto member = members[index];
		constexpr auto cls = classify(std::meta::type_of(member)).value_or(field_class{value_kind::u64, 0});
		auto& cell = cells[index];
		auto const& value = row.[:member:];
		if constexpr (cls.kind == value_kind::boolean) {
			cell.kind = foreign::bdb_value_kind::BDB_VALUE_KIND_BOOL;
			cell.bool_value = value;
		} else if constexpr (cls.kind == value_kind::u64) {
			cell.kind = foreign::bdb_value_kind::BDB_VALUE_KIND_U64;
			if constexpr (is_closed_ref_v<std::remove_cvref_t<decltype(value)>>) {
				// A closed reference crosses as the engine's u64 handle
				// row id (lowering.md §5.3); the vocabulary rides the
				// TYPE only.
				cell.u64_value = value.row;
			} else {
				cell.u64_value = value;
			}
		} else if constexpr (cls.kind == value_kind::i64) {
			cell.kind = foreign::bdb_value_kind::BDB_VALUE_KIND_I64;
			cell.i64_value = value;
		} else if constexpr (cls.kind == value_kind::string) {
			// Borrowed: the ABI speaks uint8_t, the row speaks char; the
			// pointer-type bit_cast is the checked conversion the dialect
			// blesses for exactly this boundary (AGENTS.md §22).
			cell.kind = foreign::bdb_value_kind::BDB_VALUE_KIND_STRING;
			cell.string_value = foreign::bdb_string_view{
			    .data = std::bit_cast<std::uint8_t const*>(value.data()),
			    .len = value.size(),
			};
		} else if constexpr (cls.kind == value_kind::fixed_bytes) {
			// Borrowed, as above (std::byte -> uint8_t).
			cell.kind = foreign::bdb_value_kind::BDB_VALUE_KIND_FIXED_BYTES;
			cell.bytes_value = foreign::bdb_bytes_view{
			    .data = std::bit_cast<std::uint8_t const*>(value.data()),
			    .len = value.size(),
			};
		} else if constexpr (cls.kind == value_kind::interval_u64) {
			cell.kind = foreign::bdb_value_kind::BDB_VALUE_KIND_INTERVAL_U64;
			cell.interval_u64_start = value.lo();
			cell.interval_u64_end = value.hi();
		} else {
			cell.kind = foreign::bdb_value_kind::BDB_VALUE_KIND_INTERVAL_I64;
			cell.interval_i64_start = value.lo();
			cell.interval_i64_end = value.hi();
		}
	}
	return cells;
}

} // namespace bdb
