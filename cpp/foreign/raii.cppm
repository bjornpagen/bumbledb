// The safe ownership surface over the raw C ABI (TODO_CPP §15–§19, §27,
// §31): RAII handle owners, std::expected outcomes, span/view parameters,
// and the lexical callback trampolines. This module lives in foreign/
// because adapting the ABI REQUIRES raw pointers and void* context
// smuggling (AGENTS.md §5.3); everything it exports upward is
// dialect-safe — no raw pointer crosses this boundary as a parameter or
// return type of the exported surface (the opaque view structs' interior
// pointers are the one sanctioned exception, TODO_CPP §31's pre-schema
// lane).
//
// Status protocol (bumbledb_c.h module doc): OK / ABORTED are success
// shapes, ERROR hands over an owned bdb_error, and MISUSE is a contract
// violation that these wrappers make unreachable by construction —
// non-null owned handles, live lexical refs, host-side bounds checks. A
// MISUSE that fires anyway is an impossible programmer state; this module
// is part of BOTH graphs and the pinned Clang 22 lint frontend has no
// C++26 contracts, so the one honest spelling left is termination
// (AGENTS.md §11's failure ladder).
export module bumbledb.foreign.raii;

import std;
import bumbledb.foreign;

namespace bdb::foreign {

// Termination for boundary states the wrappers prove unreachable (see the
// module comment).
[[noreturn]] auto unreachable_boundary_state() -> void {
    std::abort();
}

auto view_of(std::string_view text) -> bdb_string_view {
    if (text.empty()) {
        return bdb_string_view{.data = nullptr, .len = 0};
    }
    return bdb_string_view{
        .data = std::bit_cast<std::uint8_t const*>(text.data()),
        .len = text.size(),
    };
}

auto absent_view() -> bdb_string_view {
    return bdb_string_view{.data = nullptr, .len = 0};
}

} // namespace bdb::foreign

export namespace bdb::foreign {

/// Borrow-decode of an ABI text view (the ABI speaks uint8_t, the host
/// speaks char). The result borrows whatever carrier the view borrows.
auto text_of(bdb_string_view view) -> std::string_view {
    if (view.data == nullptr) {
        return std::string_view{};
    }
    return std::string_view{std::bit_cast<char const*>(view.data), view.len};
}

/// Borrow-decode of an ABI byte view; same borrow contract as text_of.
auto bytes_span_of(bdb_bytes_view view) -> std::span<std::byte const> {
    if (view.data == nullptr) {
        return std::span<std::byte const>{};
    }
    return std::span{std::bit_cast<std::byte const*>(view.data), view.len};
}

/// The GenerationMoved payload (witnessed/current generations).
struct generation_moved_payload {
    std::uint64_t witnessed;
    std::uint64_t current;
};

/// One rendered violation, copied OUT of the owning error so the value is
/// ownership-closed (no borrow survives the error's death).
struct violation_copy {
    std::uint16_t statement;
    bdb_statement_kind kind;
    std::string spelling;
    bdb_violation_direction direction;
    bool has_measure;
    std::uint64_t measure_lo;
    std::uint64_t measure_hi;
};

/// Owning RAII over bdb_error* (TODO_CPP §27). Move-only; the moved-from
/// handle is inert (every accessor on it is the unreachable boundary
/// state — never hand a moved-from error onward).
class error_handle {
    bdb_error* raw_{nullptr};

public:
    /// Adopts an owned error written by a BDB_STATUS_ERROR return.
    explicit error_handle(bdb_error* owned) : raw_{owned} {
        if (owned == nullptr) {
            unreachable_boundary_state();
        }
    }

    error_handle(error_handle const&) = delete;
    auto operator=(error_handle const&) -> error_handle& = delete;

    error_handle(error_handle&& other) noexcept
        : raw_{std::exchange(other.raw_, nullptr)} {}

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

    /// The rendered message, copied out (the borrowed view dies with the
    /// error; the copy does not).
    [[nodiscard]] auto message() const -> std::string {
        auto view = absent_view();
        if (bdb_error_get_message(raw_, &view)
            != bdb_status::BDB_STATUS_OK) {
            unreachable_boundary_state();
        }
        return std::string{text_of(view)};
    }

    [[nodiscard]] auto generation_moved() const
        -> std::optional<generation_moved_payload> {
        auto witnessed = std::uint64_t{0};
        auto current = std::uint64_t{0};
        if (bdb_error_get_generation_moved(raw_, &witnessed, &current)
            != bdb_status::BDB_STATUS_OK) {
            return std::nullopt;
        }
        return generation_moved_payload{
            .witnessed = witnessed, .current = current};
    }

    [[nodiscard]] auto violation_count() const -> std::size_t {
        return bdb_error_violation_count(raw_);
    }

    /// One violation, spelling copied out; nullopt past violation_count().
    [[nodiscard]] auto violation(std::size_t index) const
        -> std::optional<violation_copy> {
        if (index >= violation_count()) {
            return std::nullopt;
        }
        auto raw_violation = bdb_violation{};
        if (bdb_error_get_violation(raw_, index, &raw_violation)
            != bdb_status::BDB_STATUS_OK) {
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
            static_cast<void>(bdb_error_destroy(raw_));
            raw_ = nullptr;
        }
    }
};

/// How a lexical callback run ended when it did not fail: the callback
/// returned OK (completed — a write committed) or ABORT (the write delta
/// dropped; a read simply ended early).
enum class callback_done : std::uint8_t {
    completed,
    aborted,
};

} // namespace bdb::foreign

namespace bdb::foreign {

// The status → outcome fold shared by every callback-shaped entry point.
auto callback_outcome(bdb_status status, bdb_error* error)
    -> std::expected<callback_done, error_handle> {
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

// The status → value fold for plain fallible calls (no ABORTED shape).
template<class T>
auto value_outcome(bdb_status status, bdb_error* error, T value)
    -> std::expected<T, error_handle> {
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

} // namespace bdb::foreign

export namespace bdb::foreign {

/// Owning RAII over an answers carrier (TODO_CPP §22–§23). Move-only;
/// minted empty; clear() retains capacity. cell() is bounds-checked HERE
/// so the bridge's MISUSE lane is unreachable (§22).
class answers_handle {
    bdb_answers* raw_{nullptr};

    explicit answers_handle(bdb_answers* owned) : raw_{owned} {}

public:
    /// Mints an empty carrier (the ABI's never-fails constructor; an
    /// allocation failure is process death, not a recoverable state).
    static auto make() -> answers_handle {
        auto* raw = bdb_answers_new();
        if (raw == nullptr) {
            unreachable_boundary_state();
        }
        return answers_handle{raw};
    }

    answers_handle(answers_handle const&) = delete;
    auto operator=(answers_handle const&) -> answers_handle& = delete;

    answers_handle(answers_handle&& other) noexcept
        : raw_{std::exchange(other.raw_, nullptr)} {}

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

    /// Whether this handle still owns a carrier (false after a move-out).
    [[nodiscard]] auto alive() const -> bool {
        return raw_ != nullptr;
    }

    /// Row count (0 for a moved-from handle — the ABI's own null answer).
    [[nodiscard]] auto len() const -> std::size_t {
        return bdb_answers_len(raw_);
    }

    /// Column count (0 for a moved-from handle).
    [[nodiscard]] auto arity() const -> std::size_t {
        return bdb_answers_arity(raw_);
    }

    /// Empties the carrier, retaining capacity.
    auto clear() -> void {
        if (raw_ != nullptr) {
            static_cast<void>(bdb_answers_clear(raw_));
        }
    }

    /// One cell, bounds-checked host-side: nullopt out of range. The
    /// returned value's string/bytes payloads BORROW this carrier.
    [[nodiscard]] auto cell(std::size_t row, std::size_t column) const
        -> std::optional<bdb_value> {
        if (row >= len() || column >= arity()) {
            return std::nullopt;
        }
        auto value = bdb_value{};
        if (bdb_answers_get(raw_, row, column, &value)
            != bdb_status::BDB_STATUS_OK) {
            unreachable_boundary_state();
        }
        return value;
    }

private:
    auto destroy() -> void {
        if (raw_ != nullptr) {
            static_cast<void>(bdb_answers_destroy(raw_));
            raw_ = nullptr;
        }
    }
};

/// Owning RAII over a scan/point-read row set. Move-only; cell() is
/// bounds-checked host-side. A default-constructed/moved-from handle is
/// the empty row set (len 0) — the ABI's own null answer.
class row_set_handle {
    bdb_row_set* raw_{nullptr};

public:
    /// The empty row set (also what bdb_tx_get/bdb_snapshot_get write on
    /// a key miss).
    row_set_handle() = default;

    /// Adopts an owned row set written by a successful get/scan.
    explicit row_set_handle(bdb_row_set* owned) : raw_{owned} {}

    row_set_handle(row_set_handle const&) = delete;
    auto operator=(row_set_handle const&) -> row_set_handle& = delete;

    row_set_handle(row_set_handle&& other) noexcept
        : raw_{std::exchange(other.raw_, nullptr)} {}

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

    /// One cell, bounds-checked host-side: nullopt out of range. The
    /// returned value's string/bytes payloads BORROW this row set.
    [[nodiscard]] auto cell(std::size_t row, std::size_t column) const
        -> std::optional<bdb_value> {
        if (row >= len() || column >= arity(row)) {
            return std::nullopt;
        }
        auto value = bdb_value{};
        if (bdb_row_set_get(raw_, row, column, &value)
            != bdb_status::BDB_STATUS_OK) {
            unreachable_boundary_state();
        }
        return value;
    }

private:
    auto destroy() -> void {
        if (raw_ != nullptr) {
            static_cast<void>(bdb_row_set_destroy(raw_));
            raw_ = nullptr;
        }
    }
};

/// A callable usable as the body of a lexical read (TODO_CPP §16).
template<class Body>
concept SnapshotBody = requires(Body& body, bdb_snapshot_ref const& snapshot) {
    { body(snapshot) } -> std::same_as<bdb_callback_control>;
};

/// A callable usable as the body of a lexical write (TODO_CPP §17).
template<class Body>
concept TxBody = requires(Body& body, bdb_tx_ref& transaction) {
    { body(transaction) } -> std::same_as<bdb_callback_control>;
};

/// Owning RAII over bdb_db* (TODO_CPP §15). Move-only; the moved-from
/// handle is inert (alive() == false; using it further is the unreachable
/// boundary state). Destruction destroys the handle; the environment lock
/// releases when the engine's last co-owner (prepared queries, below the
/// boundary) is gone.
class db_handle {
    bdb_db* raw_{nullptr};

    explicit db_handle(bdb_db* owned) : raw_{owned} {}

    static auto from_status(bdb_status status, bdb_db* database,
        bdb_error* error) -> std::expected<db_handle, error_handle> {
        return value_outcome(status, error, database)
            .transform([](bdb_db* owned) { return db_handle{owned}; });
    }

public:
    /// Creates a fresh DURABLE store at path from a schema spec view.
    static auto create(std::string_view path, bdb_schema_spec const& spec)
        -> std::expected<db_handle, error_handle> {
        bdb_db* database = nullptr;
        bdb_error* error = nullptr;
        auto const status =
            bdb_db_create(view_of(path), &spec, &database, &error);
        return from_status(status, database, error);
    }

    /// Opens an existing durable store, verifying the fingerprint.
    static auto open(std::string_view path, bdb_schema_spec const& spec)
        -> std::expected<db_handle, error_handle> {
        bdb_db* database = nullptr;
        bdb_error* error = nullptr;
        auto const status =
            bdb_db_open(view_of(path), &spec, &database, &error);
        return from_status(status, database, error);
    }

    /// Opens or initializes an EPHEMERAL store at path.
    static auto ephemeral(std::string_view path, bdb_schema_spec const& spec)
        -> std::expected<db_handle, error_handle> {
        bdb_db* database = nullptr;
        bdb_error* error = nullptr;
        auto const status =
            bdb_db_ephemeral(view_of(path), &spec, &database, &error);
        return from_status(status, database, error);
    }

    db_handle(db_handle const&) = delete;
    auto operator=(db_handle const&) -> db_handle& = delete;

    db_handle(db_handle&& other) noexcept
        : raw_{std::exchange(other.raw_, nullptr)} {}

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

    /// Whether this handle still owns a store (false after a move-out).
    [[nodiscard]] auto alive() const -> bool {
        return raw_ != nullptr;
    }

    /// The admitted store's fingerprint, copied out as 64 lowercase hex
    /// chars.
    [[nodiscard]] auto fingerprint() const
        -> std::expected<std::string, error_handle> {
        auto raw_fingerprint = bdb_fingerprint{};
        bdb_error* error = nullptr;
        auto const status =
            bdb_db_fingerprint(raw_, &raw_fingerprint, &error);
        return value_outcome(status, error, raw_fingerprint)
            .transform([](bdb_fingerprint value) {
                auto text = std::string{};
                text.reserve(std::size(value.hex));
                for (auto const character : value.hex) {
                    text.push_back(static_cast<char>(character));
                }
                return text;
            });
    }

    /// Runs body over one consistent read snapshot (TODO_CPP §16): the
    /// void* context smuggles &body through the C trampoline; the call is
    /// synchronous on this thread, so the borrow is sound for exactly the
    /// callback's extent.
    template<SnapshotBody Body>
    [[nodiscard]] auto read(Body&& body) const
        -> std::expected<callback_done, error_handle> {
        auto trampoline = [](void* context,
                              bdb_snapshot_ref const* snapshot)
            -> bdb_callback_control {
            auto& live_body =
                *static_cast<std::remove_reference_t<Body>*>(context);
            return live_body(*snapshot);
        };
        bdb_error* error = nullptr;
        auto const status =
            bdb_db_read(raw_, trampoline, std::addressof(body), &error);
        return callback_outcome(status, error);
    }

    /// Runs body as the single writer (TODO_CPP §17); OK from the body
    /// commits, ABORT drops the delta. Re-entrant writes are refused by
    /// the bridge with a typed EnvironmentLocked error.
    template<TxBody Body>
    [[nodiscard]] auto write(Body&& body) const
        -> std::expected<callback_done, error_handle> {
        auto trampoline = [](void* context, bdb_tx_ref* transaction)
            -> bdb_callback_control {
            auto& live_body =
                *static_cast<std::remove_reference_t<Body>*>(context);
            return live_body(*transaction);
        };
        bdb_error* error = nullptr;
        auto const status =
            bdb_db_write(raw_, trampoline, std::addressof(body), &error);
        return callback_outcome(status, error);
    }

    /// write conditional on a still-live snapshot (TODO_CPP §18). SAFETY:
    /// the snapshot ref is forwarded only from inside the read callback
    /// that owns it — this wrapper never stores it — so the underlying
    /// &Snapshot is alive for the whole nested synchronous call.
    template<TxBody Body>
    [[nodiscard]] auto write_from(bdb_snapshot_ref const& snapshot,
        Body&& body) const -> std::expected<callback_done, error_handle> {
        auto trampoline = [](void* context, bdb_tx_ref* transaction)
            -> bdb_callback_control {
            auto& live_body =
                *static_cast<std::remove_reference_t<Body>*>(context);
            return live_body(*transaction);
        };
        bdb_error* error = nullptr;
        auto const status = bdb_db_write_from(
            raw_, &snapshot, trampoline, std::addressof(body), &error);
        return callback_outcome(status, error);
    }

private:
    auto destroy() -> void {
        if (raw_ != nullptr) {
            static_cast<void>(bdb_db_destroy(raw_));
            raw_ = nullptr;
        }
    }
};

/// Records an insert into the delta; true = the final state changed.
auto tx_insert(bdb_tx_ref const& transaction, std::uint32_t relation,
    std::span<bdb_value const> values) -> std::expected<bool, error_handle> {
    auto changed = false;
    bdb_error* error = nullptr;
    auto const status = bdb_tx_insert(
        &transaction, relation, values.data(), values.size(), &changed,
        &error);
    return value_outcome(status, error, changed);
}

/// Records a delete into the delta; true = the final state changed.
auto tx_remove(bdb_tx_ref const& transaction, std::uint32_t relation,
    std::span<bdb_value const> values) -> std::expected<bool, error_handle> {
    auto changed = false;
    bdb_error* error = nullptr;
    auto const status = bdb_tx_delete(
        &transaction, relation, values.data(), values.size(), &changed,
        &error);
    return value_outcome(status, error, changed);
}

/// Final-state membership (base + pending delta).
auto tx_contains(bdb_tx_ref const& transaction, std::uint32_t relation,
    std::span<bdb_value const> values) -> std::expected<bool, error_handle> {
    auto contains = false;
    bdb_error* error = nullptr;
    auto const status = bdb_tx_contains(
        &transaction, relation, values.data(), values.size(), &contains,
        &error);
    return value_outcome(status, error, contains);
}

/// Mints the next fresh value for (relation, field).
auto tx_alloc(bdb_tx_ref const& transaction, std::uint32_t relation,
    std::uint16_t field) -> std::expected<std::uint64_t, error_handle> {
    auto id = std::uint64_t{0};
    bdb_error* error = nullptr;
    auto const status = bdb_tx_alloc(&transaction, relation, field, &id,
        &error);
    return value_outcome(status, error, id);
}

/// Committed-state membership of one dynamic fact.
auto snapshot_contains(bdb_snapshot_ref const& snapshot,
    std::uint32_t relation, std::span<bdb_value const> values)
    -> std::expected<bool, error_handle> {
    auto contains = false;
    bdb_error* error = nullptr;
    auto const status = bdb_snapshot_contains(
        &snapshot, relation, values.data(), values.size(), &contains,
        &error);
    return value_outcome(status, error, contains);
}

/// Full-relation export in row_id order: one owned row-set crossing.
auto snapshot_scan(bdb_snapshot_ref const& snapshot, std::uint32_t relation)
    -> std::expected<row_set_handle, error_handle> {
    bdb_row_set* rows = nullptr;
    bdb_error* error = nullptr;
    auto const status = bdb_snapshot_scan(&snapshot, relation, &rows, &error);
    return value_outcome(status, error, rows)
        .transform([](bdb_row_set* owned) { return row_set_handle{owned}; });
}

/// The relation names of a spec view, copied out in declaration order —
/// declaration index IS the minted RelationId (lowering.md §1.1), which is
/// how the pre-schema lane resolves coordinates to wire ids.
auto relation_names_of(bdb_schema_spec const& spec)
    -> std::vector<std::string> {
    auto names = std::vector<std::string>{};
    names.reserve(spec.relation_count);
    for (auto const& relation : std::span{spec.relations,
             spec.relation_count}) {
        names.emplace_back(text_of(relation.name));
    }
    return names;
}

/// A scalar structural value type (no payload beyond the tag).
auto scalar_type(bdb_value_type_kind kind) -> bdb_value_type {
    return bdb_value_type{
        .kind = kind,
        .fixed_len = 0,
        .element = bdb_interval_element::BDB_INTERVAL_ELEMENT_U64,
        .has_width = false,
        .width = 0,
    };
}

/// The FixedBytes structural type (the length IS the type).
auto fixed_bytes_type(std::uint16_t len) -> bdb_value_type {
    return bdb_value_type{
        .kind = bdb_value_type_kind::BDB_VALUE_TYPE_KIND_FIXED_BYTES,
        .fixed_len = len,
        .element = bdb_interval_element::BDB_INTERVAL_ELEMENT_U64,
        .has_width = false,
        .width = 0,
    };
}

/// The general (widthless) interval structural type.
auto interval_type(bdb_interval_element element) -> bdb_value_type {
    return bdb_value_type{
        .kind = bdb_value_type_kind::BDB_VALUE_TYPE_KIND_INTERVAL,
        .fixed_len = 0,
        .element = element,
        .has_width = false,
        .width = 0,
    };
}

// --- the pre-schema spec lane (TODO_CPP §13's placeholder) -------------------
// Owned, ownership-closed descriptions that lower to borrowed spec views.
// This is scaffolding for the phase BEFORE bdb::schema<> exists: the
// runtime tests (and nothing else) spell specs through it, and the schema
// elaborator replaces it wholesale. Ordinary relations and the fd /
// containment statement forms only — exactly what the §39 slice needs.

/// One field description (name, structural type, law-class newtype label,
/// fresh mark) — the owned twin of bdb_field_spec.
struct owned_field {
    std::string name;
    bdb_value_type value_type;
    std::optional<std::string> newtype;
    bool fresh;
};

/// One ordinary relation description.
struct owned_relation {
    std::string name;
    std::vector<owned_field> fields;
};

/// key(R, [f...]) — the fd statement form.
struct owned_fd {
    std::string relation;
    std::vector<std::string> projection;
};

/// One bare side (no σ selection — the §39 slice needs none).
struct owned_side {
    std::string relation;
    std::vector<std::string> projection;
};

/// contained(source, target) / mirrors via the bidirectional flag.
struct owned_containment {
    owned_side source;
    owned_side target;
    bool bidirectional;
};

using owned_statement = std::variant<owned_fd, owned_containment>;

/// Owns every byte of a schema spec and materializes the borrowed ABI
/// view once at construction. Non-copyable and non-movable so the interior
/// view pointers stay valid for exactly this object's lifetime; view() is
/// valid while *this is alive.
class owned_schema_spec {
    std::vector<owned_relation> relations_;
    std::vector<owned_statement> statements_;
    std::vector<std::vector<bdb_field_spec>> field_views_;
    std::vector<bdb_relation_spec> relation_views_;
    std::vector<std::vector<bdb_string_view>> projection_views_;
    std::vector<bdb_statement_spec> statement_views_;
    bdb_schema_spec view_{};

    static auto view_of_owned(std::string const& text) -> bdb_string_view {
        return view_of(std::string_view{text});
    }

    static auto projection_view(std::vector<std::string> const& names)
        -> std::vector<bdb_string_view> {
        auto views = std::vector<bdb_string_view>{};
        views.reserve(names.size());
        for (auto const& name : names) {
            views.push_back(view_of_owned(name));
        }
        return views;
    }

    static auto side_view(owned_side const& side,
        std::vector<bdb_string_view> const& projection) -> bdb_side {
        return bdb_side{
            .relation = view_of_owned(side.relation),
            .projection = projection.data(),
            .projection_count = projection.size(),
            .selection = nullptr,
            .selection_count = 0,
        };
    }

public:
    owned_schema_spec(std::vector<owned_relation> relations,
        std::vector<owned_statement> statements)
        : relations_{std::move(relations)},
          statements_{std::move(statements)} {
        field_views_.reserve(relations_.size());
        relation_views_.reserve(relations_.size());
        for (auto const& relation : relations_) {
            auto& fields = field_views_.emplace_back();
            fields.reserve(relation.fields.size());
            for (auto const& field : relation.fields) {
                fields.push_back(bdb_field_spec{
                    .name = view_of_owned(field.name),
                    .value_type = field.value_type,
                    .newtype = field.newtype.has_value()
                        ? view_of_owned(*field.newtype)
                        : absent_view(),
                    .fresh = field.fresh,
                });
            }
            relation_views_.push_back(bdb_relation_spec{
                .name = view_of_owned(relation.name),
                .fields = fields.data(),
                .field_count = fields.size(),
                .closed = nullptr,
            });
        }

        statement_views_.reserve(statements_.size());
        // Reserve the projection-list slots up front: side_view holds
        // REFERENCES to the emplaced inner vectors, and an outer
        // reallocation between the two emplace_backs of one containment
        // would invalidate the first reference (the inner BUFFERS survive
        // a reallocation; the vector objects do not stay put).
        auto projection_lists = std::size_t{0};
        for (auto const& statement : statements_) {
            projection_lists +=
                std::holds_alternative<owned_fd>(statement) ? 1U : 2U;
        }
        projection_views_.reserve(projection_lists);
        for (auto const& statement : statements_) {
            // Unread union-shaped fields stay value-initialized (zeroed);
            // the bridge reads only what the kind names.
            auto view = bdb_statement_spec{};
            if (auto const* fd = std::get_if<owned_fd>(&statement)) {
                auto const& projection = projection_views_.emplace_back(
                    projection_view(fd->projection));
                view.kind =
                    bdb_statement_spec_kind::BDB_STATEMENT_SPEC_KIND_FD;
                view.fd_relation = view_of_owned(fd->relation);
                view.fd_projection = projection.data();
                view.fd_projection_count = projection.size();
            } else {
                auto const& containment =
                    std::get<owned_containment>(statement);
                auto const& source_projection =
                    projection_views_.emplace_back(
                        projection_view(containment.source.projection));
                auto const& target_projection =
                    projection_views_.emplace_back(
                        projection_view(containment.target.projection));
                view.kind = bdb_statement_spec_kind::
                    BDB_STATEMENT_SPEC_KIND_CONTAINMENT;
                view.source =
                    side_view(containment.source, source_projection);
                view.target =
                    side_view(containment.target, target_projection);
                view.bidirectional = containment.bidirectional;
            }
            statement_views_.push_back(view);
        }

        view_ = bdb_schema_spec{
            .relations = relation_views_.data(),
            .relation_count = relation_views_.size(),
            .statements = statement_views_.empty()
                ? nullptr
                : statement_views_.data(),
            .statement_count = statement_views_.size(),
        };
    }

    owned_schema_spec(owned_schema_spec const&) = delete;
    auto operator=(owned_schema_spec const&) -> owned_schema_spec& = delete;
    owned_schema_spec(owned_schema_spec&&) = delete;
    auto operator=(owned_schema_spec&&) -> owned_schema_spec& = delete;
    ~owned_schema_spec() = default;

    /// The borrowed ABI view; valid while *this is alive.
    [[nodiscard]] auto view() const -> bdb_schema_spec const& {
        return view_;
    }
};

} // namespace bdb::foreign
