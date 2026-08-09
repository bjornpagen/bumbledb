// :answers — the untyped result lane (TODO_CPP §22–§23).
//
// This lane's carrier is RAW: bdb::AnswersRaw owns the reusable flat
// answers buffer and hands cells out as the dialect-safe bdb::Value sum.
// The typed Answers<Query> facade (synthesized row products) wraps
// exactly this object (:answers_row / :prepared). Also here: the owned
// RowSet (scan and keyed-read product).
//
// Borrow contract (§22): string/bytes alternatives of a decoded Value
// BORROW the owning carrier (AnswersRaw or RowSet) — valid only while the
// owner is alive, un-cleared, and un-re-executed. Fixed-width alternatives
// are values.
export module bumbledb:answers;

import std;
import :decode;
import bumbledb_foreign;

export namespace bdb {

/// The untyped, reusable answers carrier (TODO_CPP §22–§23): move-only
/// RAII over the bridge's flat buffer. Minted empty; execution fills it
/// (the query phase's execute_into lane); clear() retains capacity. The
/// moved-from carrier is inert (alive() == false, len/arity 0).
class [[nodiscard]] AnswersRaw {
	foreign::answers_handle handle_;

public:
	/// Mints an empty carrier (never fails).
	AnswersRaw() : handle_{foreign::answers_handle::make()} {}

	/// Whether this carrier still owns a buffer (false after move-out —
	/// the §36 inert-source witness).
	[[nodiscard]] auto alive() const -> bool {
		return handle_.alive();
	}

	/// Number of answers.
	[[nodiscard]] auto len() const -> std::size_t {
		return handle_.len();
	}

	/// Number of columns (the executed query's find terms, in order).
	[[nodiscard]] auto arity() const -> std::size_t {
		return handle_.arity();
	}

	/// Empties the carrier, retaining capacity (invalidates borrowed
	/// string/bytes cells).
	auto clear() -> void {
		handle_.clear();
	}

	/// One cell, bounds-checked (§22): nullopt out of range, never a
	/// panic. String/bytes alternatives borrow THIS carrier.
	[[nodiscard]] auto cell(Cell at) const -> std::optional<Value> {
		return handle_.cell(at.row, at.column).and_then([](foreign::bdb_value const& wire) -> std::optional<Value> {
			return decode_value(wire);
		});
	}

	/// The bridge lane (query phase): the raw handle for execute_into.
	[[nodiscard]] auto native() -> foreign::answers_handle& {
		return handle_;
	}
};

/// An owned row set (scan and keyed-read product; TODO_CPP §24/§26):
/// move-only RAII over one whole-result crossing, decoded cell by cell
/// host-side. Typed row decode arrives with the schema phase; this phase
/// reads cells as bdb::Value.
class [[nodiscard]] RowSet {
	foreign::row_set_handle handle_;

public:
	/// The bridge lane: adopts an owned row-set handle (Snapshot::scan
	/// constructs these; application code never does).
	explicit RowSet(foreign::row_set_handle handle) : handle_{std::move(handle)} {}

	/// Number of rows.
	[[nodiscard]] auto len() const -> std::size_t {
		return handle_.len();
	}

	/// The row's cell count (sealed field order); 0 out of range.
	[[nodiscard]] auto arity(std::size_t row) const -> std::size_t {
		return handle_.arity(row);
	}

	/// One cell, bounds-checked: nullopt out of range. String/bytes
	/// alternatives borrow THIS row set.
	[[nodiscard]] auto cell(Cell at) const -> std::optional<Value> {
		return handle_.cell(at.row, at.column).and_then([](foreign::bdb_value const& wire) -> std::optional<Value> {
			return decode_value(wire);
		});
	}
};

} // namespace bdb
