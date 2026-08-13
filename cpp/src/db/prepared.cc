export module bumbledb:prepared;

import std;
import :query;
import :answers_row;
import bumbledb_foreign;

export namespace bdb {

/**
 * A reusable prepared query: move-only RAII over the bridge's prepared
 * handle. The engine validated, normalized, and planned once at
 * Db::prepare<Query>(); the handle is reusable across snapshots of the
 * same database. Concurrent execution through one prepared object is
 * outside the permitted model — execution takes it non-const.
 */
template<auto Query>
class [[nodiscard]] Prepared {
	foreign::prepared_handle handle_;

	explicit Prepared(foreign::prepared_handle handle) : handle_{std::move(handle)} {}

	friend class Db;

public:
	Prepared(Prepared const&) = delete;
	auto operator=(Prepared const&) -> Prepared& = delete;
	Prepared(Prepared&&) noexcept = default;
	auto operator=(Prepared&&) noexcept -> Prepared& = default;
	~Prepared() = default;

	/**
	 * Whether this handle still owns a prepared query (false after
	 * move-out).
	 */
	[[nodiscard]] auto alive() const -> bool {
		return handle_.alive();
	}

	/**
	 * The bridge lane (Snapshot::execute drives it; application code
	 * never needs it).
	 */
	[[nodiscard]] auto native() -> foreign::prepared_handle& {
		return handle_;
	}
};

/**
 * The typed answers carrier of one query: `bdb::Answers<DownAt>` decodes
 * rows as the synthesized row product of the query's `.find` head —
 * named members, fixed-width by value, string/bytes owned copies from
 * decode.
 */
template<auto Query>
using Answers = RowAnswers<row_of<Query>>;

}
