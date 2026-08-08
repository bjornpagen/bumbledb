// bumbledb.answers — the untyped result lane (TODO_CPP §22–§23).
//
// This phase's carrier is RAW: bdb::AnswersRaw owns the reusable flat
// answers buffer and hands cells out as the dialect-safe bdb::Value sum.
// The typed Answers<Query> facade (synthesized row products) arrives with
// the query phase and will wrap exactly this object.
//
// Borrow contract (§22): string/bytes alternatives of a decoded Value
// BORROW the owning carrier (AnswersRaw or RowSet) — valid only while the
// owner is alive, un-cleared, and un-re-executed. Fixed-width alternatives
// are values.
//
// Reflection-free dialect code, part of BOTH graphs.
export module bumbledb.answers;

import std;
import bumbledb.types;
import bumbledb.foreign;
import bumbledb.foreign.raii;

export namespace bdb {

/// The dialect-safe cell sum — one alternative per engine value variant
/// (lowering.md §5.2; allen_mask is bind-time vocabulary but the wire tag
/// exists, so the sum is total over bdb_value_kind).
using Value = std::variant<
    bool,
    std::uint64_t,
    std::int64_t,
    std::string_view,
    std::span<std::byte const>,
    interval<std::uint64_t>,
    interval<std::int64_t>,
    allen_mask>;

/// One cell address (explicit aggregate — AGENTS.md §26).
struct Cell {
    std::size_t row;
    std::size_t column;
};

} // namespace bdb

namespace bdb {
namespace {

// Lifts a checked-construction result into the optional cell sum (the
// engine guarantees validity, so an error here is a boundary anomaly, not
// an application state). TU-local: referenced only from the non-inline
// decode_value below, so no exported inline function exposes it.
template<class Checked>
auto lifted(std::expected<Checked, TypeError> checked)
    -> std::optional<Value> {
    if (!checked.has_value()) {
        return std::nullopt;
    }
    return Value{*checked};
}

} // namespace
} // namespace bdb

export namespace bdb {

/// Decodes one wire cell to the dialect sum. nullopt only on a value the
/// engine's own checks make unrepresentable (an empty interval, a mask
/// above 13 bits) — never a recoverable application state. String/bytes
/// payloads keep borrowing whatever carrier the wire cell borrowed.
[[nodiscard]] auto decode_value(foreign::bdb_value const& cell)
    -> std::optional<Value> {
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
        return lifted(interval<std::uint64_t>::make(
            cell.interval_u64_start, cell.interval_u64_end));
    case foreign::bdb_value_kind::BDB_VALUE_KIND_INTERVAL_I64:
        return lifted(interval<std::int64_t>::make(
            cell.interval_i64_start, cell.interval_i64_end));
    case foreign::bdb_value_kind::BDB_VALUE_KIND_ALLEN_MASK:
        return lifted(allen_mask::make(cell.allen_mask));
    }
    return std::nullopt;
}

/// The untyped, reusable answers carrier (TODO_CPP §22–§23): move-only
/// RAII over the bridge's flat buffer. Minted empty; execution fills it
/// (the query phase's execute_into lane); clear() retains capacity. The
/// moved-from carrier is inert (alive() == false, len/arity 0).
class AnswersRaw {
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
        return handle_.cell(at.row, at.column)
            .and_then([](foreign::bdb_value const& wire)
                    -> std::optional<Value> {
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
class RowSet {
    foreign::row_set_handle handle_;

public:
    /// The bridge lane: adopts an owned row-set handle (Snapshot::scan
    /// constructs these; application code never does).
    explicit RowSet(foreign::row_set_handle handle)
        : handle_{std::move(handle)} {}

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
        return handle_.cell(at.row, at.column)
            .and_then([](foreign::bdb_value const& wire)
                    -> std::optional<Value> {
                return decode_value(wire);
            });
    }
};

} // namespace bdb
