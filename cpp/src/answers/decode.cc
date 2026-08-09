export module bumbledb:decode;

import std;
import :interval;
import :allen;
import bumbledb_foreign;

export namespace bdb {

/**
 * The dialect-safe cell sum — one alternative per engine value variant
 * (lowering.md §5.2; allen_mask is bind-time vocabulary but the wire tag
 * exists, so the sum is total over bdb_value_kind). String/bytes
 * alternatives BORROW the owning carrier (AnswersRaw or RowSet) — valid
 * only while the owner is alive, un-cleared, and un-re-executed;
 * fixed-width alternatives are values.
 */
using Value = std::variant<bool, std::uint64_t, std::int64_t, std::string_view, std::span<std::byte const>, interval<std::uint64_t>,
                           interval<std::int64_t>, allen_mask>;

struct Cell {
	std::size_t row;
	std::size_t column;
};

}

namespace bdb {
namespace {

template<class Checked>
[[nodiscard]] auto lifted(std::expected<Checked, TypeError> checked) -> std::optional<Value> {
	if (!checked.has_value()) {
		return std::nullopt;
	}
	return Value{*checked};
}

}
}

export namespace bdb {

/**
 * Decodes one wire cell to the dialect sum. nullopt only on a value the
 * engine's own checks make unrepresentable (an empty interval, a mask
 * above 13 bits) — never a recoverable application state. String/bytes
 * payloads keep borrowing whatever carrier the wire cell borrowed.
 */
[[nodiscard]] auto decode_value(foreign::bdb_value const& cell) -> std::optional<Value> {
	switch (cell.kind) {
	case foreign::bdb_value_kind::BDB_VALUE_KIND_BOOL:
		return Value{cell.bool_value};
	case foreign::bdb_value_kind::BDB_VALUE_KIND_U64:
		return Value{cell.u64_value};
	case foreign::bdb_value_kind::BDB_VALUE_KIND_I64:
		return Value{cell.i64_value};
	case foreign::bdb_value_kind::BDB_VALUE_KIND_STRING:
		return Value{foreign::text_of(cell.string_value)};
	case foreign::bdb_value_kind::BDB_VALUE_KIND_FIXED_BYTES:
		return Value{foreign::bytes_span_of(cell.bytes_value)};
	case foreign::bdb_value_kind::BDB_VALUE_KIND_INTERVAL_U64:
		return lifted(interval<std::uint64_t>::make(cell.interval_u64_start, cell.interval_u64_end));
	case foreign::bdb_value_kind::BDB_VALUE_KIND_INTERVAL_I64:
		return lifted(interval<std::int64_t>::make(cell.interval_i64_start, cell.interval_i64_end));
	case foreign::bdb_value_kind::BDB_VALUE_KIND_ALLEN_MASK:
		return lifted(allen_mask::make(cell.allen_mask));
	}
	return std::nullopt;
}

}
