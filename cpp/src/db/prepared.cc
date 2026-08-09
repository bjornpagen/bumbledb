// :prepared — the reusable prepared query (TODO_CPP §20) and the typed
// answers alias of one query.
export module bumbledb:prepared;

import std;
import :query;
import :answers_row;
import bumbledb_foreign;

export namespace bdb {

/// A reusable prepared query (TODO_CPP §20): move-only RAII over the
/// bridge's prepared handle. The engine validated, normalized, and
/// planned ONCE at `Db::prepare<Query>()`; the handle is reusable across
/// snapshots of the same database. Concurrent execution through one
/// prepared object is outside the dialect's permitted model — execution
/// takes it non-const (§22).
template<auto Query>
class Prepared {
    foreign::prepared_handle handle_;

    explicit Prepared(foreign::prepared_handle handle)
        : handle_{std::move(handle)} {}

    friend class Db;

public:
    Prepared(Prepared const&) = delete;
    auto operator=(Prepared const&) -> Prepared& = delete;
    Prepared(Prepared&&) noexcept = default;
    auto operator=(Prepared&&) noexcept -> Prepared& = default;
    ~Prepared() = default;

    /// Whether this handle still owns a prepared query (false after
    /// move-out — the §36 inert-source witness).
    [[nodiscard]] auto alive() const -> bool {
        return handle_.alive();
    }

    /// The bridge lane (Snapshot::execute drives it).
    [[nodiscard]] auto native() -> foreign::prepared_handle& {
        return handle_;
    }
};

/// The typed answers carrier of one query (TODO_CPP §12, §22–§23):
/// `bdb::Answers<DownAt>` decodes rows as the synthesized row product of
/// DownAt's `.find` head — named members, fixed-width by value,
/// string_view/span borrowed from the carrier.
template<auto Query>
using Answers = RowAnswers<row_of<Query>>;

} // namespace bdb
