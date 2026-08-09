// :answers_row — the typed answers carrier over one synthesized row
// product (TODO_CPP §22–§23): wraps the reusable flat buffer and decodes
// whole rows into the query's named row product (`bdb::Answers<Query>` in
// :prepared aliases this at `row_of<Query>`).
export module bumbledb:answers_row;

import std;
import :decode;
import :answers;

export namespace bdb {

/// The typed answers carrier over one synthesized row product (TODO_CPP
/// §22–§23): wraps the reusable flat buffer and decodes whole rows into
/// the query's named row product (`bdb::Answers<Query>` in :prepared
/// aliases this at `row_of<Query>`). Move-only through its carrier;
/// minted empty; execution fills it (`Snapshot::execute_into`), and
/// `clear()` retains capacity.
///
/// Borrow contract (§22): string_view / span<byte const> row members
/// BORROW this carrier — valid only while it is alive, un-cleared, and
/// un-re-executed. Fixed-width members are values. Answers are SETS — no
/// order exists; hosts sort (lowering.md §5.2).
template<class Row>
class RowAnswers {
    AnswersRaw raw_;

    /// The row product's member-type tuple, read structurally (P1061
    /// packs; no reflection syntax needed here). Never called: only its
    /// TYPE is read, so no member is ever default-constructed (interval
    /// members have no default state by design).
    static auto member_tuple(Row const& row) {
        auto const& [...members] = row;
        return std::type_identity<
            std::tuple<std::remove_cvref_t<decltype(members)>...>>{};
    }

    using RowTuple =
        decltype(member_tuple(std::declval<Row const&>()))::type;

    /// One decoded cell at its member type; nullopt on a kind mismatch.
    template<class Member>
    static auto decoded_cell(std::optional<Value> cell)
        -> std::optional<Member> {
        if (!cell.has_value() || !std::holds_alternative<Member>(*cell)) {
            return std::nullopt;
        }
        return std::get<Member>(*cell);
    }

    /// Decodes one whole row through parenthesized aggregate init — no
    /// member is ever default-constructed (interval members cannot be).
    template<std::size_t... Columns>
    auto decode_row(std::size_t index,
        [[maybe_unused]] std::index_sequence<Columns...> columns) const
        -> std::optional<Row> {
        auto cells = std::tuple{
            decoded_cell<std::tuple_element_t<Columns, RowTuple>>(
                raw_.cell({.row = index, .column = Columns}))...};
        if (!(std::get<Columns>(cells).has_value() && ...)) {
            return std::nullopt;
        }
        return Row(*std::move(std::get<Columns>(cells))...);
    }

public:
    RowAnswers() = default;

    /// Whether this carrier still owns a buffer (false after move-out).
    [[nodiscard]] auto alive() const -> bool {
        return raw_.alive();
    }

    /// Number of answers.
    [[nodiscard]] auto size() const -> std::size_t {
        return raw_.len();
    }

    /// Empties the carrier, retaining capacity (invalidates every
    /// borrowed row member).
    auto clear() -> void {
        raw_.clear();
    }

    /// One decoded row, bounds- and shape-checked (§22's recoverable
    /// accessor): nullopt out of range or on a column/row-type mismatch
    /// (possible only when this carrier was filled by a DIFFERENT query's
    /// execution — the typed execute lane fills it with its own query).
    [[nodiscard]] auto row(std::size_t index) const -> std::optional<Row> {
        constexpr auto arity = std::tuple_size_v<RowTuple>;
        if (index >= raw_.len() || arity != raw_.arity()) {
            return std::nullopt;
        }
        return decode_row(index, std::make_index_sequence<arity>{});
    }

    /// The typed row range, decoded lazily; row values borrow THIS
    /// carrier (§22) — the range is valid only while *this is alive and
    /// unchanged. A row that fails to decode is an impossible programmer
    /// state on the typed lane (the row product and the buffer come from
    /// one query); the pinned lint Clang has no C++26 contracts and this
    /// code was written for both graphs, so the one honest spelling left
    /// is termination (the raii module's rule).
    [[nodiscard]] auto rows() const {
        return std::views::iota(std::size_t{0}, raw_.len())
            | std::views::transform(
                [&self = *this](std::size_t index) -> Row {
                    auto decoded = self.row(index);
                    if (!decoded.has_value()) {
                        std::abort();
                    }
                    return *std::move(decoded);
                });
    }

    /// The bridge lane: the untyped carrier (Snapshot::execute_into
    /// drives it; application code never needs it).
    [[nodiscard]] auto native() -> AnswersRaw& {
        return raw_;
    }
};

} // namespace bdb
