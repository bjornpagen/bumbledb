/**
 * The safe ownership surface over the raw C ABI: RAII handle owners,
 * std::expected outcomes, span/view parameters, lexical callback
 * trampolines. Nothing it exports carries a raw pointer (the opaque view
 * structs' interior pointers are the one sanctioned exception — the
 * pre-schema spec lane). Status protocol (bumbledb_c.h): OK / ABORTED
 * are success shapes, ERROR hands over an owned bdb_error, and MISUSE is
 * a contract violation these wrappers make unreachable by construction;
 * a MISUSE that fires anyway is an impossible programmer state, and —
 * this module is in both graphs and the pinned lint Clang 22 has no
 * C++26 contracts — the one honest spelling left is termination. The
 * owned_* spec builders are the pre-schema lane: runtime-test
 * scaffolding the schema elaborator replaces wholesale.
 */
export module bumbledb_foreign:raii;

import std;
import :abi;

namespace bdb::foreign {

[[noreturn]] auto unreachable_boundary_state() -> void {
	std::abort();
}

[[nodiscard]] auto view_of(std::string_view text) -> bdb_string_view {
	if (text.empty()) {
		return bdb_string_view{.data = nullptr, .len = 0};
	}
	return bdb_string_view{
	    .data = std::bit_cast<std::uint8_t const*>(text.data()),
	    .len = text.size(),
	};
}

[[nodiscard]] auto absent_view() -> bdb_string_view {
	return bdb_string_view{.data = nullptr, .len = 0};
}

}

export namespace bdb::foreign {

/**
 * Borrow-decode of an ABI text view (the ABI speaks uint8_t, the host
 * speaks char). The result borrows whatever carrier the view borrows.
 */
[[nodiscard]] auto text_of(bdb_string_view view) -> std::string_view {
	if (view.data == nullptr) {
		return std::string_view{};
	}
	return std::string_view{std::bit_cast<char const*>(view.data), view.len};
}

/**
 * Borrow-decode of an ABI byte view; same borrow contract as text_of.
 */
[[nodiscard]] auto bytes_span_of(bdb_bytes_view view) -> std::span<std::byte const> {
	if (view.data == nullptr) {
		return std::span<std::byte const>{};
	}
	return std::span{std::bit_cast<std::byte const*>(view.data), view.len};
}

struct generation_moved_payload {
	std::uint64_t witnessed;
	std::uint64_t current;
};

/**
 * One rendered violation, copied OUT of the owning error so the value is
 * ownership-closed (no borrow survives the error's death).
 */
struct violation_copy {
	std::uint16_t statement;
	bdb_statement_kind kind;
	std::string spelling;
	bdb_violation_direction direction;
	bool has_measure;
	std::uint64_t measure_lo;
	std::uint64_t measure_hi;
};

/**
 * Owning RAII over bdb_error*. Move-only; the moved-from handle is inert
 * — every accessor on it is the unreachable boundary state, so never
 * hand a moved-from error onward.
 */
class error_handle {
	bdb_error* raw_{nullptr};

public:
	/**
	 * Adopts an owned error written by a BDB_STATUS_ERROR return.
	 */
	explicit error_handle(bdb_error* owned) : raw_{owned} {
		if (owned == nullptr) {
			unreachable_boundary_state();
		}
	}

	error_handle(error_handle const&) = delete;
	auto operator=(error_handle const&) -> error_handle& = delete;

	error_handle(error_handle&& other) noexcept : raw_{std::exchange(other.raw_, nullptr)} {}

	auto operator=(error_handle&& other) noexcept -> error_handle& {
		if (this != &other) {
			destroy();
			raw_ = std::exchange(other.raw_, nullptr);
		}
		return *this;
	}

	~error_handle() {
		destroy();
	}

	[[nodiscard]] auto kind() const -> bdb_error_kind {
		return bdb_error_get_kind(raw_);
	}

	/**
	 * The rendered message, copied out (the borrowed view dies with the
	 * error; the copy does not).
	 */
	[[nodiscard]] auto message() const -> std::string {
		auto view = absent_view();
		if (bdb_error_get_message(raw_, &view) != bdb_status::BDB_STATUS_OK) {
			unreachable_boundary_state();
		}
		return std::string{text_of(view)};
	}

	[[nodiscard]] auto generation_moved() const -> std::optional<generation_moved_payload> {
		auto witnessed = std::uint64_t{0};
		auto current = std::uint64_t{0};
		if (bdb_error_get_generation_moved(raw_, &witnessed, &current) != bdb_status::BDB_STATUS_OK) {
			return std::nullopt;
		}
		return generation_moved_payload{.witnessed = witnessed, .current = current};
	}

	[[nodiscard]] auto bulk_committed() const -> std::optional<std::uint64_t> {
		auto committed = std::uint64_t{0};
		if (bdb_error_get_bulk_committed(raw_, &committed) != bdb_status::BDB_STATUS_OK) {
			return std::nullopt;
		}
		return committed;
	}

	[[nodiscard]] auto violation_count() const -> std::size_t {
		return bdb_error_violation_count(raw_);
	}

	/**
	 * One violation, spelling copied out; nullopt past violation_count().
	 */
	[[nodiscard]] auto violation(std::size_t index) const -> std::optional<violation_copy> {
		if (index >= violation_count()) {
			return std::nullopt;
		}
		auto raw_violation = bdb_violation{};
		if (bdb_error_get_violation(raw_, index, &raw_violation) != bdb_status::BDB_STATUS_OK) {
			unreachable_boundary_state();
		}
		return violation_copy{
		    .statement = raw_violation.statement,
		    .kind = raw_violation.kind,
		    .spelling = std::string{text_of(raw_violation.spelling)},
		    .direction = raw_violation.direction,
		    .has_measure = raw_violation.has_measure,
		    .measure_lo = raw_violation.measure_lo,
		    .measure_hi = raw_violation.measure_hi,
		};
	}

private:
	auto destroy() -> void {
		if (raw_ != nullptr) {
			std::ignore = bdb_error_destroy(raw_);
			raw_ = nullptr;
		}
	}
};

/**
 * How a lexical callback run ended when it did not fail: the callback
 * returned OK (completed — a write committed) or ABORT (the write delta
 * dropped; a read simply ended early).
 */
enum class callback_done : std::uint8_t {
	completed,
	aborted,
};

}

namespace bdb::foreign {

[[nodiscard]] auto callback_outcome(bdb_status status, bdb_error* error) -> std::expected<callback_done, error_handle> {
	switch (status) {
	case bdb_status::BDB_STATUS_OK:
		return callback_done::completed;
	case bdb_status::BDB_STATUS_ABORTED:
		return callback_done::aborted;
	case bdb_status::BDB_STATUS_ERROR:
		return std::unexpected{error_handle{error}};
	case bdb_status::BDB_STATUS_MISUSE:
		break;
	}
	unreachable_boundary_state();
}

template<class T>
[[nodiscard]] auto value_outcome(bdb_status status, bdb_error* error, T value) -> std::expected<T, error_handle> {
	switch (status) {
	case bdb_status::BDB_STATUS_OK:
		return value;
	case bdb_status::BDB_STATUS_ERROR:
		return std::unexpected{error_handle{error}};
	case bdb_status::BDB_STATUS_ABORTED:
	case bdb_status::BDB_STATUS_MISUSE:
		break;
	}
	unreachable_boundary_state();
}

[[nodiscard]] auto status_outcome(bdb_status status, bdb_error* error) -> std::expected<void, error_handle> {
	switch (status) {
	case bdb_status::BDB_STATUS_OK:
		return {};
	case bdb_status::BDB_STATUS_ERROR:
		return std::unexpected{error_handle{error}};
	case bdb_status::BDB_STATUS_ABORTED:
	case bdb_status::BDB_STATUS_MISUSE:
		break;
	}
	unreachable_boundary_state();
}

}

export namespace bdb::foreign {

/**
 * Owning RAII over an answers carrier. Move-only; minted empty; clear()
 * retains capacity. cell() is bounds-checked HERE so the bridge's MISUSE
 * lane is unreachable.
 */
class answers_handle {
	bdb_answers* raw_{nullptr};

	explicit answers_handle(bdb_answers* owned) : raw_{owned} {}

public:
	/**
	 * Mints an empty carrier (the ABI's never-fails constructor; an
	 * allocation failure is process death, not a recoverable state).
	 */
	[[nodiscard]] static auto make() -> answers_handle {
		auto* raw = bdb_answers_new();
		if (raw == nullptr) {
			unreachable_boundary_state();
		}
		return answers_handle{raw};
	}

	answers_handle(answers_handle const&) = delete;
	auto operator=(answers_handle const&) -> answers_handle& = delete;

	answers_handle(answers_handle&& other) noexcept : raw_{std::exchange(other.raw_, nullptr)} {}

	auto operator=(answers_handle&& other) noexcept -> answers_handle& {
		if (this != &other) {
			destroy();
			raw_ = std::exchange(other.raw_, nullptr);
		}
		return *this;
	}

	~answers_handle() {
		destroy();
	}

	/**
	 * Whether this handle still owns a carrier (false after a move-out).
	 */
	[[nodiscard]] auto alive() const -> bool {
		return raw_ != nullptr;
	}

	/**
	 * Row count (0 for a moved-from handle — the ABI's own null answer).
	 */
	[[nodiscard]] auto len() const -> std::size_t {
		return bdb_answers_len(raw_);
	}

	/**
	 * Column count (0 for a moved-from handle).
	 */
	[[nodiscard]] auto arity() const -> std::size_t {
		return bdb_answers_arity(raw_);
	}

	auto clear() -> void {
		if (raw_ != nullptr) {
			std::ignore = bdb_answers_clear(raw_);
		}
	}

	/**
	 * One cell, bounds-checked host-side: nullopt out of range. The
	 * returned value's string/bytes payloads BORROW this carrier.
	 */
	[[nodiscard]] auto cell(std::size_t row, std::size_t column) const -> std::optional<bdb_value> {
		if (row >= len() || column >= arity()) {
			return std::nullopt;
		}
		auto value = bdb_value{};
		if (bdb_answers_get(raw_, row, column, &value) != bdb_status::BDB_STATUS_OK) {
			unreachable_boundary_state();
		}
		return value;
	}

private:
	friend class prepared_handle;

	auto destroy() -> void {
		if (raw_ != nullptr) {
			std::ignore = bdb_answers_destroy(raw_);
			raw_ = nullptr;
		}
	}
};

/**
 * Owning RAII over a prepared query. Move-only — the moved-from handle
 * is inert (alive() == false). The prepared value co-owns its engine
 * below the boundary, so the environment outlives every prepared query
 * by construction.
 */
class prepared_handle {
	bdb_prepared* raw_{nullptr};

	explicit prepared_handle(bdb_prepared* owned) : raw_{owned} {
		if (owned == nullptr) {
			unreachable_boundary_state();
		}
	}

	friend class db_handle;

public:
	prepared_handle(prepared_handle const&) = delete;
	auto operator=(prepared_handle const&) -> prepared_handle& = delete;

	prepared_handle(prepared_handle&& other) noexcept : raw_{std::exchange(other.raw_, nullptr)} {}

	auto operator=(prepared_handle&& other) noexcept -> prepared_handle& {
		if (this != &other) {
			destroy();
			raw_ = std::exchange(other.raw_, nullptr);
		}
		return *this;
	}

	~prepared_handle() {
		destroy();
	}

	/**
	 * Whether this handle still owns a prepared query (false after a
	 * move-out).
	 */
	[[nodiscard]] auto alive() const -> bool {
		return raw_ != nullptr;
	}

	/**
	 * Executes against a snapshot with positional params, filling the
	 * caller's reusable carrier (cleared first, capacity retained).
	 * Non-const: execution takes the prepared query exclusively.
	 */
	[[nodiscard]] auto execute(bdb_snapshot_ref const& snapshot, std::span<bdb_param const> params, answers_handle& answers)
	    -> std::expected<void, error_handle> {
		bdb_error* error = nullptr;
		auto const status = bdb_snapshot_execute(&snapshot, raw_, params.data(), params.size(), answers.raw_, &error);
		return status_outcome(status, error);
	}

private:
	auto destroy() -> void {
		if (raw_ != nullptr) {
			std::ignore = bdb_prepared_destroy(raw_);
			raw_ = nullptr;
		}
	}
};

/**
 * Owning RAII over a scan/point-read row set. Move-only; cell() is
 * bounds-checked host-side. A default-constructed/moved-from handle is
 * the empty row set (len 0) — the ABI's own null answer.
 */
class row_set_handle {
	bdb_row_set* raw_{nullptr};

public:
	/**
	 * The empty row set (also what bdb_tx_get/bdb_snapshot_get write on
	 * a key miss).
	 */
	row_set_handle() = default;

	/**
	 * Adopts an owned row set written by a successful get/scan.
	 */
	explicit row_set_handle(bdb_row_set* owned) : raw_{owned} {}

	row_set_handle(row_set_handle const&) = delete;
	auto operator=(row_set_handle const&) -> row_set_handle& = delete;

	row_set_handle(row_set_handle&& other) noexcept : raw_{std::exchange(other.raw_, nullptr)} {}

	auto operator=(row_set_handle&& other) noexcept -> row_set_handle& {
		if (this != &other) {
			destroy();
			raw_ = std::exchange(other.raw_, nullptr);
		}
		return *this;
	}

	~row_set_handle() {
		destroy();
	}

	[[nodiscard]] auto len() const -> std::size_t {
		return bdb_row_set_len(raw_);
	}

	[[nodiscard]] auto arity(std::size_t row) const -> std::size_t {
		if (row >= len()) {
			return 0;
		}
		return bdb_row_set_arity(raw_, row);
	}

	/**
	 * One cell, bounds-checked host-side: nullopt out of range. The
	 * returned value's string/bytes payloads BORROW this row set.
	 */
	[[nodiscard]] auto cell(std::size_t row, std::size_t column) const -> std::optional<bdb_value> {
		if (row >= len() || column >= arity(row)) {
			return std::nullopt;
		}
		auto value = bdb_value{};
		if (bdb_row_set_get(raw_, row, column, &value) != bdb_status::BDB_STATUS_OK) {
			unreachable_boundary_state();
		}
		return value;
	}

private:
	auto destroy() -> void {
		if (raw_ != nullptr) {
			std::ignore = bdb_row_set_destroy(raw_);
			raw_ = nullptr;
		}
	}
};

template<class Body>
concept SnapshotBody = requires(Body& body, bdb_snapshot_ref const& snapshot) {
	{ body(snapshot) } -> std::same_as<bdb_callback_control>;
};

template<class Body>
concept TxBody = requires(Body& body, bdb_tx_ref& transaction) {
	{ body(transaction) } -> std::same_as<bdb_callback_control>;
};

/**
 * Owning RAII over bdb_db*. Move-only; the moved-from handle is inert
 * (alive() == false; using it further is the unreachable boundary
 * state). Destruction destroys the handle; the environment lock releases
 * when the engine's last co-owner (prepared queries, below the boundary)
 * is gone.
 */
class db_handle {
	bdb_db* raw_{nullptr};

	explicit db_handle(bdb_db* owned) : raw_{owned} {}

	[[nodiscard]] static auto from_status(bdb_status status, bdb_db* database, bdb_error* error) -> std::expected<db_handle, error_handle> {
		return value_outcome(status, error, database).transform([](bdb_db* owned) {
			return db_handle{owned};
		});
	}

public:
	/**
	 * Creates a fresh DURABLE store at path from a schema spec view.
	 */
	[[nodiscard]] static auto create(std::string_view path, bdb_schema_spec const& spec) -> std::expected<db_handle, error_handle> {
		bdb_db* database = nullptr;
		bdb_error* error = nullptr;
		auto const status = bdb_db_create(view_of(path), &spec, &database, &error);
		return from_status(status, database, error);
	}

	/**
	 * Opens an existing durable store, verifying the fingerprint.
	 */
	[[nodiscard]] static auto open(std::string_view path, bdb_schema_spec const& spec) -> std::expected<db_handle, error_handle> {
		bdb_db* database = nullptr;
		bdb_error* error = nullptr;
		auto const status = bdb_db_open(view_of(path), &spec, &database, &error);
		return from_status(status, database, error);
	}

	/**
	 * Opens or initializes an EPHEMERAL store at path.
	 */
	[[nodiscard]] static auto ephemeral(std::string_view path, bdb_schema_spec const& spec) -> std::expected<db_handle, error_handle> {
		bdb_db* database = nullptr;
		bdb_error* error = nullptr;
		auto const status = bdb_db_ephemeral(view_of(path), &spec, &database, &error);
		return from_status(status, database, error);
	}

	db_handle(db_handle const&) = delete;
	auto operator=(db_handle const&) -> db_handle& = delete;

	db_handle(db_handle&& other) noexcept : raw_{std::exchange(other.raw_, nullptr)} {}

	auto operator=(db_handle&& other) noexcept -> db_handle& {
		if (this != &other) {
			destroy();
			raw_ = std::exchange(other.raw_, nullptr);
		}
		return *this;
	}

	~db_handle() {
		destroy();
	}

	/**
	 * Whether this handle still owns a store (false after a move-out).
	 */
	[[nodiscard]] auto alive() const -> bool {
		return raw_ != nullptr;
	}

	/**
	 * The admitted store's fingerprint, copied out as 64 lowercase hex
	 * chars.
	 */
	[[nodiscard]] auto fingerprint() const -> std::expected<std::string, error_handle> {
		auto raw_fingerprint = bdb_fingerprint{};
		bdb_error* error = nullptr;
		auto const status = bdb_db_fingerprint(raw_, &raw_fingerprint, &error);
		return value_outcome(status, error, raw_fingerprint).transform([](bdb_fingerprint value) {
			auto text = std::string{};
			text.reserve(std::size(value.hex));
			for (auto const character : value.hex) {
				text.push_back(static_cast<char>(character));
			}
			return text;
		});
	}

	/**
	 * Runs body over one consistent read snapshot, synchronously on this
	 * thread.
	 */
	template<SnapshotBody Body>
	[[nodiscard]] auto read(Body&& body) const -> std::expected<callback_done, error_handle> {
		/* SAFETY: the void* context smuggles &body through the C trampoline; the call is synchronous, so the borrow spans exactly the callback */
		auto trampoline = [](void* context, bdb_snapshot_ref const* snapshot) -> bdb_callback_control {
			auto& live_body = *static_cast<std::remove_reference_t<Body>*>(context);
			return live_body(*snapshot);
		};
		bdb_error* error = nullptr;
		auto const status = bdb_db_read(raw_, trampoline, std::addressof(body), &error);
		return callback_outcome(status, error);
	}

	/**
	 * Runs body as the single writer; OK from the body commits, ABORT
	 * drops the delta. A re-entrant write is refused by the bridge with
	 * a typed EnvironmentLocked error.
	 */
	template<TxBody Body>
	[[nodiscard]] auto write(Body&& body) const -> std::expected<callback_done, error_handle> {
		/* SAFETY: the void* context smuggles &body through the C trampoline; the call is synchronous, so the borrow spans exactly the callback */
		auto trampoline = [](void* context, bdb_tx_ref* transaction) -> bdb_callback_control {
			auto& live_body = *static_cast<std::remove_reference_t<Body>*>(context);
			return live_body(*transaction);
		};
		bdb_error* error = nullptr;
		auto const status = bdb_db_write(raw_, trampoline, std::addressof(body), &error);
		return callback_outcome(status, error);
	}

	/**
	 * Prepares a program IR view against the store: the engine
	 * validates, normalizes, reads statistics, and plans ONCE; the
	 * returned handle is reusable across snapshots of this database. The
	 * view graph is copied by the bridge before this returns.
	 */
	[[nodiscard]] auto prepare(bdb_program const& program) const -> std::expected<prepared_handle, error_handle> {
		bdb_prepared* prepared = nullptr;
		bdb_error* error = nullptr;
		auto const status = bdb_db_prepare(raw_, &program, &prepared, &error);
		return value_outcome(status, error, prepared).transform([](bdb_prepared* owned) {
			return prepared_handle{owned};
		});
	}

	/**
	 * write conditional on a still-live snapshot — callable only from
	 * inside the read callback that owns the snapshot ref.
	 */
	template<TxBody Body>
	[[nodiscard]] auto write_from(bdb_snapshot_ref const& snapshot, Body&& body) const -> std::expected<callback_done, error_handle> {
		/* SAFETY: the snapshot ref comes from the owning read callback and is never stored — alive for the whole nested call */
		auto trampoline = [](void* context, bdb_tx_ref* transaction) -> bdb_callback_control {
			auto& live_body = *static_cast<std::remove_reference_t<Body>*>(context);
			return live_body(*transaction);
		};
		bdb_error* error = nullptr;
		auto const status = bdb_db_write_from(raw_, &snapshot, trampoline, std::addressof(body), &error);
		return callback_outcome(status, error);
	}

private:
	auto destroy() -> void {
		if (raw_ != nullptr) {
			std::ignore = bdb_db_destroy(raw_);
			raw_ = nullptr;
		}
	}
};

/**
 * Records an insert into the delta; true = the final state changed.
 */
[[nodiscard]] auto tx_insert(bdb_tx_ref const& transaction, std::uint32_t relation, std::span<bdb_value const> values)
    -> std::expected<bool, error_handle> {
	auto changed = false;
	bdb_error* error = nullptr;
	auto const status = bdb_tx_insert(&transaction, relation, values.data(), values.size(), &changed, &error);
	return value_outcome(status, error, changed);
}

/**
 * Records a delete into the delta; true = the final state changed.
 */
[[nodiscard]] auto tx_remove(bdb_tx_ref const& transaction, std::uint32_t relation, std::span<bdb_value const> values)
    -> std::expected<bool, error_handle> {
	auto changed = false;
	bdb_error* error = nullptr;
	auto const status = bdb_tx_delete(&transaction, relation, values.data(), values.size(), &changed, &error);
	return value_outcome(status, error, changed);
}

/**
 * Final-state membership (base + pending delta).
 */
[[nodiscard]] auto tx_contains(bdb_tx_ref const& transaction, std::uint32_t relation, std::span<bdb_value const> values)
    -> std::expected<bool, error_handle> {
	auto contains = false;
	bdb_error* error = nullptr;
	auto const status = bdb_tx_contains(&transaction, relation, values.data(), values.size(), &contains, &error);
	return value_outcome(status, error, contains);
}

/**
 * Mints the next fresh value for (relation, field).
 */
[[nodiscard]] auto tx_alloc(bdb_tx_ref const& transaction, std::uint32_t relation, std::uint16_t field) -> std::expected<std::uint64_t, error_handle> {
	auto id = std::uint64_t{0};
	bdb_error* error = nullptr;
	auto const status = bdb_tx_alloc(&transaction, relation, field, &id, &error);
	return value_outcome(status, error, id);
}

/**
 * Committed-state membership of one dynamic fact.
 */
[[nodiscard]] auto snapshot_contains(bdb_snapshot_ref const& snapshot, std::uint32_t relation, std::span<bdb_value const> values)
    -> std::expected<bool, error_handle> {
	auto contains = false;
	bdb_error* error = nullptr;
	auto const status = bdb_snapshot_contains(&snapshot, relation, values.data(), values.size(), &contains, &error);
	return value_outcome(status, error, contains);
}

/**
 * Full-relation export in row_id order: one owned row-set crossing.
 */
[[nodiscard]] auto snapshot_scan(bdb_snapshot_ref const& snapshot, std::uint32_t relation) -> std::expected<row_set_handle, error_handle> {
	bdb_row_set* rows = nullptr;
	bdb_error* error = nullptr;
	auto const status = bdb_snapshot_scan(&snapshot, relation, &rows, &error);
	return value_outcome(status, error, rows).transform([](bdb_row_set* owned) {
		return row_set_handle{owned};
	});
}

/**
 * Committed-state point lookup through a key statement: key values in
 * the statement's projection order. A miss is the empty row set (the ABI
 * writes null; row_set_handle owns either way).
 */
[[nodiscard]] auto snapshot_get(bdb_snapshot_ref const& snapshot, std::uint32_t relation, std::uint16_t key_statement,
                  std::span<bdb_value const> key_values) -> std::expected<row_set_handle, error_handle> {
	bdb_row_set* row = nullptr;
	bdb_error* error = nullptr;
	auto const status = bdb_snapshot_get(&snapshot, relation, key_statement, key_values.data(), key_values.size(), &row, &error);
	return value_outcome(status, error, row).transform([](bdb_row_set* owned) {
		return row_set_handle{owned};
	});
}

/**
 * Final-state point lookup through a key statement (base + pending
 * delta); miss = the empty row set.
 */
[[nodiscard]] auto tx_get(bdb_tx_ref const& transaction, std::uint32_t relation, std::uint16_t key_statement, std::span<bdb_value const> key_values)
    -> std::expected<row_set_handle, error_handle> {
	bdb_row_set* row = nullptr;
	bdb_error* error = nullptr;
	auto const status = bdb_tx_get(&transaction, relation, key_statement, key_values.data(), key_values.size(), &row, &error);
	return value_outcome(status, error, row).transform([](bdb_row_set* owned) {
		return row_set_handle{owned};
	});
}

/**
 * The relation names of a spec view, copied out in declaration order —
 * declaration index IS the minted RelationId (lowering.md §1.1), which
 * is how the pre-schema lane resolves coordinates to wire ids.
 */
[[nodiscard]] auto relation_names_of(bdb_schema_spec const& spec) -> std::vector<std::string> {
	auto names = std::vector<std::string>{};
	names.reserve(spec.relation_count);
	for (auto const& relation : std::span{spec.relations, spec.relation_count}) {
		names.emplace_back(text_of(relation.name));
	}
	return names;
}

/**
 * A scalar structural value type (no payload beyond the tag).
 */
[[nodiscard]] auto scalar_type(bdb_value_type_kind kind) -> bdb_value_type {
	return bdb_value_type{
	    .kind = kind,
	    .fixed_len = 0,
	    .element = bdb_interval_element::BDB_INTERVAL_ELEMENT_U64,
	    .has_width = false,
	    .width = 0,
	};
}

/**
 * The FixedBytes structural type (the length IS the type).
 */
[[nodiscard]] auto fixed_bytes_type(std::uint16_t len) -> bdb_value_type {
	return bdb_value_type{
	    .kind = bdb_value_type_kind::BDB_VALUE_TYPE_KIND_FIXED_BYTES,
	    .fixed_len = len,
	    .element = bdb_interval_element::BDB_INTERVAL_ELEMENT_U64,
	    .has_width = false,
	    .width = 0,
	};
}

/**
 * The general (widthless) interval structural type.
 */
[[nodiscard]] auto interval_type(bdb_interval_element element) -> bdb_value_type {
	return bdb_value_type{
	    .kind = bdb_value_type_kind::BDB_VALUE_TYPE_KIND_INTERVAL,
	    .fixed_len = 0,
	    .element = element,
	    .has_width = false,
	    .width = 0,
	};
}

/**
 * The fixed-width interval structural type (the width is a fingerprint
 * input; lowering.md §1.8).
 */
[[nodiscard]] auto fixed_interval_type(bdb_interval_element element, std::uint64_t width) -> bdb_value_type {
	return bdb_value_type{
	    .kind = bdb_value_type_kind::BDB_VALUE_TYPE_KIND_INTERVAL,
	    .fixed_len = 0,
	    .element = element,
	    .has_width = true,
	    .width = width,
	};
}

/**
 * One field description (name, structural type, law-class newtype label,
 * fresh mark) — the owned twin of bdb_field_spec.
 */
struct owned_field {
	std::string name;
	bdb_value_type value_type;
	std::optional<std::string> newtype;
	bool fresh;
};

/**
 * One literal as spelled: a closed handle BY NAME (the engine resolves
 * schema-lane handles — lowering.md §7.8), or a tagged value.
 */
struct owned_literal {
	bool is_handle;
	std::string handle;
	bdb_value_kind kind;
	bool boolean;
	std::uint64_t u64;
	std::int64_t i64;
	std::string text;
};

/**
 * One σ binding: `field == literal-or-set` (1 literal = One; ≥2 = Many).
 */
struct owned_selection {
	std::string field;
	std::vector<owned_literal> literals;
};

/**
 * One ground axiom of a closed relation: the handle plus one literal per
 * declared intrinsic column, in field-declaration order.
 */
struct owned_closed_row {
	std::string handle;
	std::vector<owned_literal> values;
};

/**
 * A closed relation's closed half (lowering.md §1.3): the handle newtype
 * (`"<Name>.id"`, always present) and the ground axioms.
 */
struct owned_closed {
	std::string newtype;
	std::vector<owned_closed_row> rows;
};

/**
 * One relation description; `closed` engaged = closed relation (fields
 * then carry the DECLARED intrinsic columns only — lowering.md §7.3).
 * Defaulted so ordinary-relation spellings stay valid designated inits.
 */
struct owned_relation {
	std::string name;
	std::vector<owned_field> fields;
	std::optional<owned_closed> closed{};
};

/**
 * key(R, [f...]) — the fd statement form.
 */
struct owned_fd {
	std::string relation;
	std::vector<std::string> projection;
};

/**
 * One statement side: projection + σ selection (lowered as-is; defaulted
 * so bare-face spellings stay valid designated inits).
 */
struct owned_side {
	std::string relation;
	std::vector<std::string> projection;
	std::vector<owned_selection> selection{};
};

/**
 * contained(source, target) / mirrors via the bidirectional flag.
 */
struct owned_containment {
	owned_side source;
	owned_side target;
	bool bidirectional;
};

/**
 * A capacity weight; `field` is read for Field/DurationField.
 */
struct owned_weight {
	bdb_weight_kind kind;
	std::string field;
};

/**
 * One capacity bound; `lit` for Lit, `field` for Field/DurationField.
 */
struct owned_bound {
	bdb_bound_kind kind;
	std::uint64_t lit;
	std::string field;
};

/**
 * One capacity window (Exact/Floor read `lo`; Range reads both).
 */
struct owned_capacity_window {
	bdb_capacity_window_kind kind;
	owned_bound lo;
	owned_bound hi;
};

/**
 * capacity(target, weight, window, source) — the operator read order.
 */
struct owned_capacity {
	owned_side target;
	owned_weight weight;
	owned_capacity_window window;
	owned_side source;
};

using owned_statement = std::variant<owned_fd, owned_containment, owned_capacity>;

/**
 * Owns every byte of a schema spec and materializes the borrowed ABI
 * view once at construction. Non-copyable and non-movable so the
 * interior view pointers stay valid for exactly this object's lifetime;
 * view() is valid while *this is alive.
 */
class owned_schema_spec {
	std::vector<owned_relation> relations_;
	std::vector<owned_statement> statements_;
	std::vector<std::vector<bdb_field_spec>> field_views_;
	std::vector<std::vector<bdb_literal>> literal_views_;
	std::vector<std::vector<bdb_selection_binding>> selection_views_;
	std::vector<std::vector<bdb_closed_row>> closed_row_views_;
	std::vector<bdb_closed_spec> closed_views_;
	std::vector<bdb_relation_spec> relation_views_;
	std::vector<std::vector<bdb_string_view>> projection_views_;
	std::vector<bdb_statement_spec> statement_views_;
	bdb_schema_spec view_{};

	[[nodiscard]] static auto view_of_owned(std::string const& text) -> bdb_string_view {
		return view_of(std::string_view{text});
	}

	[[nodiscard]] static auto projection_view(std::vector<std::string> const& names) -> std::vector<bdb_string_view> {
		auto views = std::vector<bdb_string_view>{};
		views.reserve(names.size());
		for (auto const& name : names) {
			views.push_back(view_of_owned(name));
		}
		return views;
	}

	/**
	 * One literal, viewed. String payloads borrow the owned literal —
	 * stable because the owning vectors never move after construction.
	 * The frontends spell schema-lane literals as bool/u64/i64/str/handle
	 * only (the closed-payload roster); anything else is the unreachable
	 * boundary state.
	 */
	[[nodiscard]] static auto literal_view(owned_literal const& literal) -> bdb_literal {
		auto out = bdb_literal{};
		if (literal.is_handle) {
			out.kind = bdb_literal_kind::BDB_LITERAL_KIND_HANDLE;
			out.handle = view_of_owned(literal.handle);
			return out;
		}
		out.kind = bdb_literal_kind::BDB_LITERAL_KIND_VALUE;
		out.value.kind = literal.kind;
		switch (literal.kind) {
		case bdb_value_kind::BDB_VALUE_KIND_BOOL:
			out.value.bool_value = literal.boolean;
			break;
		case bdb_value_kind::BDB_VALUE_KIND_U64:
			out.value.u64_value = literal.u64;
			break;
		case bdb_value_kind::BDB_VALUE_KIND_I64:
			out.value.i64_value = literal.i64;
			break;
		case bdb_value_kind::BDB_VALUE_KIND_STRING:
			out.value.string_value = view_of_owned(literal.text);
			break;
		default:
			unreachable_boundary_state();
		}
		return out;
	}

	[[nodiscard]] auto literals_view(std::vector<owned_literal> const& literals) -> std::vector<bdb_literal> const& {
		auto views = std::vector<bdb_literal>{};
		views.reserve(literals.size());
		for (auto const& literal : literals) {
			views.push_back(literal_view(literal));
		}
		return literal_views_.emplace_back(std::move(views));
	}

	[[nodiscard]] auto side_view(owned_side const& side, std::vector<bdb_string_view> const& projection) -> bdb_side {
		auto out = bdb_side{
		    .relation = view_of_owned(side.relation),
		    .projection = projection.data(),
		    .projection_count = projection.size(),
		    .selection = nullptr,
		    .selection_count = 0,
		};
		if (side.selection.empty()) {
			return out;
		}
		auto bindings = std::vector<bdb_selection_binding>{};
		bindings.reserve(side.selection.size());
		for (auto const& binding : side.selection) {
			auto const& literals = literals_view(binding.literals);
			bindings.push_back(bdb_selection_binding{
			    .field = view_of_owned(binding.field),
			    .set =
			        bdb_literal_set{
			            .kind = literals.size() == 1 ? bdb_literal_set_kind::BDB_LITERAL_SET_KIND_ONE
			                                         : bdb_literal_set_kind::BDB_LITERAL_SET_KIND_MANY,
			            .literals = literals.data(),
			            .literal_count = literals.size(),
			        },
			});
		}
		auto const& stored = selection_views_.emplace_back(std::move(bindings));
		out.selection = stored.data();
		out.selection_count = stored.size();
		return out;
	}

	[[nodiscard]] static auto bound_view(owned_bound const& bound) -> bdb_bound {
		return bdb_bound{
		    .kind = bound.kind,
		    .lit = bound.lit,
		    .field = view_of_owned(bound.field),
		};
	}

public:
	owned_schema_spec(std::vector<owned_relation> relations, std::vector<owned_statement> statements)
	    : relations_{std::move(relations)}, statements_{std::move(statements)} {
		/* SAFETY: the view graph references emplaced inner vectors; outer reallocation would invalidate them — reserve exact totals first */
		auto literal_lists = std::size_t{0};
		auto closed_relations = std::size_t{0};
		auto selection_lists = std::size_t{0};
		for (auto const& relation : relations_) {
			if (relation.closed.has_value()) {
				++closed_relations;
				literal_lists += relation.closed->rows.size();
			}
		}
		auto const count_side = [&](owned_side const& side) {
			if (!side.selection.empty()) {
				++selection_lists;
				literal_lists += side.selection.size();
			}
		};
		for (auto const& statement : statements_) {
			if (auto const* containment = std::get_if<owned_containment>(&statement)) {
				count_side(containment->source);
				count_side(containment->target);
			} else if (auto const* capacity = std::get_if<owned_capacity>(&statement)) {
				count_side(capacity->target);
				count_side(capacity->source);
			}
		}
		literal_views_.reserve(literal_lists);
		selection_views_.reserve(selection_lists);
		closed_row_views_.reserve(closed_relations);
		closed_views_.reserve(closed_relations);

		field_views_.reserve(relations_.size());
		relation_views_.reserve(relations_.size());
		for (auto const& relation : relations_) {
			auto& fields = field_views_.emplace_back();
			fields.reserve(relation.fields.size());
			for (auto const& field : relation.fields) {
				fields.push_back(bdb_field_spec{
				    .name = view_of_owned(field.name),
				    .value_type = field.value_type,
				    .newtype = field.newtype.has_value() ? view_of_owned(*field.newtype) : absent_view(),
				    .fresh = field.fresh,
				});
			}
			auto const* closed_spec = [&]() -> bdb_closed_spec const* {
				if (!relation.closed.has_value()) {
					return nullptr;
				}
				auto rows = std::vector<bdb_closed_row>{};
				rows.reserve(relation.closed->rows.size());
				for (auto const& row : relation.closed->rows) {
					auto const& values = literals_view(row.values);
					rows.push_back(bdb_closed_row{
					    .handle = view_of_owned(row.handle),
					    .values = values.empty() ? nullptr : values.data(),
					    .value_count = values.size(),
					});
				}
				auto const& stored = closed_row_views_.emplace_back(std::move(rows));
				closed_views_.push_back(bdb_closed_spec{
				    .newtype = view_of_owned(relation.closed->newtype),
				    .rows = stored.empty() ? nullptr : stored.data(),
				    .row_count = stored.size(),
				});
				return &closed_views_.back();
			}();
			relation_views_.push_back(bdb_relation_spec{
			    .name = view_of_owned(relation.name),
			    .fields = fields.data(),
			    .field_count = fields.size(),
			    .closed = closed_spec,
			});
		}

		statement_views_.reserve(statements_.size());
		auto projection_lists = std::size_t{0};
		for (auto const& statement : statements_) {
			projection_lists += std::holds_alternative<owned_fd>(statement) ? 1U : 2U;
		}
		projection_views_.reserve(projection_lists);
		for (auto const& statement : statements_) {
			auto view = bdb_statement_spec{};
			if (auto const* fd = std::get_if<owned_fd>(&statement)) {
				auto const& projection = projection_views_.emplace_back(projection_view(fd->projection));
				view.kind = bdb_statement_spec_kind::BDB_STATEMENT_SPEC_KIND_FD;
				view.fd_relation = view_of_owned(fd->relation);
				view.fd_projection = projection.data();
				view.fd_projection_count = projection.size();
			} else if (auto const* containment = std::get_if<owned_containment>(&statement)) {
				auto const& source_projection = projection_views_.emplace_back(projection_view(containment->source.projection));
				auto const& target_projection = projection_views_.emplace_back(projection_view(containment->target.projection));
				view.kind = bdb_statement_spec_kind::BDB_STATEMENT_SPEC_KIND_CONTAINMENT;
				view.source = side_view(containment->source, source_projection);
				view.target = side_view(containment->target, target_projection);
				view.bidirectional = containment->bidirectional;
			} else {
				auto const& capacity = std::get<owned_capacity>(statement);
				auto const& target_projection = projection_views_.emplace_back(projection_view(capacity.target.projection));
				auto const& source_projection = projection_views_.emplace_back(projection_view(capacity.source.projection));
				view.kind = bdb_statement_spec_kind::BDB_STATEMENT_SPEC_KIND_CAPACITY;
				view.target = side_view(capacity.target, target_projection);
				view.source = side_view(capacity.source, source_projection);
				view.weight = bdb_weight{
				    .kind = capacity.weight.kind,
				    .field = view_of_owned(capacity.weight.field),
				};
				view.window = bdb_capacity_window{
				    .kind = capacity.window.kind,
				    .lo = bound_view(capacity.window.lo),
				    .hi = bound_view(capacity.window.hi),
				};
			}
			statement_views_.push_back(view);
		}

		view_ = bdb_schema_spec{
		    .relations = relation_views_.data(),
		    .relation_count = relation_views_.size(),
		    .statements = statement_views_.empty() ? nullptr : statement_views_.data(),
		    .statement_count = statement_views_.size(),
		};
	}

	owned_schema_spec(owned_schema_spec const&) = delete;
	auto operator=(owned_schema_spec const&) -> owned_schema_spec& = delete;
	owned_schema_spec(owned_schema_spec&&) = delete;
	auto operator=(owned_schema_spec&&) -> owned_schema_spec& = delete;
	~owned_schema_spec() = default;

	/**
	 * The borrowed ABI view; valid while *this is alive.
	 */
	[[nodiscard]] auto view() const -> bdb_schema_spec const& {
		return view_;
	}
};

}
