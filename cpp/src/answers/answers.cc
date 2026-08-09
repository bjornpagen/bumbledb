export module bumbledb:answers;

import std;
import :decode;
import bumbledb_foreign;

export namespace bdb {

/**
 * The untyped, reusable answers carrier: move-only RAII over the
 * bridge's flat buffer. Minted empty; execution fills it; clear()
 * retains capacity. The moved-from carrier is inert (alive() == false,
 * len/arity 0). String/bytes alternatives of a decoded Value BORROW this
 * carrier — valid only while it is alive, un-cleared, and un-re-executed;
 * fixed-width alternatives are values.
 */
class [[nodiscard]] AnswersRaw {
	foreign::answers_handle handle_;

public:
	AnswersRaw() : handle_{foreign::answers_handle::make()} {}

	/**
	 * Whether this carrier still owns a buffer (false after move-out).
	 */
	[[nodiscard]] auto alive() const -> bool {
		return handle_.alive();
	}

	[[nodiscard]] auto len() const -> std::size_t {
		return handle_.len();
	}

	/**
	 * Number of columns (the executed query's find terms, in order).
	 */
	[[nodiscard]] auto arity() const -> std::size_t {
		return handle_.arity();
	}

	/**
	 * Empties the carrier, retaining capacity (invalidates borrowed
	 * string/bytes cells).
	 */
	auto clear() -> void {
		handle_.clear();
	}

	/**
	 * One cell, bounds-checked: nullopt out of range, never a panic.
	 * String/bytes alternatives borrow THIS carrier.
	 */
	[[nodiscard]] auto cell(Cell at) const -> std::optional<Value> {
		return handle_.cell(at.row, at.column).and_then([](foreign::bdb_value const& wire) -> std::optional<Value> {
			return decode_value(wire);
		});
	}

	/**
	 * The bridge lane: the raw handle for execute_into (application code
	 * never needs it).
	 */
	[[nodiscard]] auto native() -> foreign::answers_handle& {
		return handle_;
	}
};

/**
 * An owned row set (the scan and keyed-read product): move-only RAII
 * over one whole-result crossing, decoded cell by cell host-side as
 * bdb::Value.
 */
class [[nodiscard]] RowSet {
	foreign::row_set_handle handle_;

public:
	/**
	 * Adopts an owned row-set handle (Snapshot::scan constructs these;
	 * application code never does).
	 */
	explicit RowSet(foreign::row_set_handle handle) : handle_{std::move(handle)} {}

	[[nodiscard]] auto len() const -> std::size_t {
		return handle_.len();
	}

	/**
	 * The row's cell count (sealed field order); 0 out of range.
	 */
	[[nodiscard]] auto arity(std::size_t row) const -> std::size_t {
		return handle_.arity(row);
	}

	/**
	 * One cell, bounds-checked: nullopt out of range. String/bytes
	 * alternatives borrow THIS row set.
	 */
	[[nodiscard]] auto cell(Cell at) const -> std::optional<Value> {
		return handle_.cell(at.row, at.column).and_then([](foreign::bdb_value const& wire) -> std::optional<Value> {
			return decode_value(wire);
		});
	}
};

}
