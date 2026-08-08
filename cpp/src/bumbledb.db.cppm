// bumbledb.db — the runtime resource layer (TODO_CPP §15–§19, §24–§25).
//
// Zoning note (pinned): this module is dialect code under src/ but it is
// GCC-ONLY, because it imports the reflection-backed meta marshaller
// (bumbledb.meta.row) for tx.insert(Relation, Row). That is ACCEPTED
// (TODO_CPP §32: the reflective core's enforcement ladder is GCC
// diagnostics + compile-fail + review); the module itself contains no
// reflection syntax — everything reflective stays in meta/. The rest of
// the runtime layer (types, error, answers) remains Clang-visible.
//
// The pre-schema lane (pinned): bdb::schema<> arrives next phase. Until
// then Db::create/open/ephemeral accept the RAW foreign spec view
// (bdb::foreign::bdb_schema_spec) built through the owned pre-schema
// builder in bumbledb.foreign.raii, and coordinate → wire-id resolution
// works by NAME against the manifest captured ONCE at construction from
// that spec: relation id = the relation's declaration index (the order IS
// the id mint, lowering.md §1.1), field id = the coordinate's reflected
// ordinal (ordinary relations: FieldId = declaration index, lowering.md
// §1.11). The schema phase replaces both with compile-time ids
// (TODO_CPP §25, §37).
//
// Failure taxonomy (§19, §27–§28): engine failure is std::unexpected
// (bdb::Error); domain abandonment is DATA on the success path
// (WriteOutcome::Abandoned); checked-value construction failure never
// reaches this module (bdb::TypeError, bumbledb.types).
export module bumbledb.db;

import std;
import bumbledb.error;
import bumbledb.answers;
import bumbledb.foreign;
import bumbledb.foreign.raii;
import bumbledb.meta.relation;
import bumbledb.meta.row;

export namespace bdb {

/// The write callback's positive decision: commit the delta, carrying a
/// result value out of the callback.
template<class T>
struct Commit {
    using value_type = T;
    T value;
};

/// The write callback's negative decision AS DATA (§19): drop the delta —
/// LMDB never saw a fact — carrying the abandonment's own payload out.
/// Not an error and never the unexpected path.
template<class A>
struct Abandon {
    using value_type = A;
    A value;
};

/// What a write callback decides (§19).
template<class T, class A>
using WriteDecision = std::variant<Commit<T>, Abandon<A>>;

/// The valueless commit decision (`return bdb::commit();`).
constexpr auto commit() -> Commit<std::monostate> {
    return Commit<std::monostate>{std::monostate{}};
}

/// A value-carrying commit decision.
template<class T>
constexpr auto commit(T value) -> Commit<T> {
    return Commit<T>{std::move(value)};
}

/// The valueless abandon decision.
constexpr auto abandon() -> Abandon<std::monostate> {
    return Abandon<std::monostate>{std::monostate{}};
}

/// A value-carrying abandon decision (abandonment-as-data).
template<class A>
constexpr auto abandon(A value) -> Abandon<A> {
    return Abandon<A>{std::move(value)};
}

/// A committed write's outcome, carrying the Commit value.
template<class T>
struct Committed {
    T value;
};

/// An abandoned write's outcome, carrying the Abandon value.
template<class A>
struct Abandoned {
    A value;
};

/// What Db::write returns on the SUCCESS path (§19): the write either
/// committed or was abandoned by its own callback. Engine failure — commit
/// rejection included — is the expected's error path, never an alternative
/// here.
template<class T, class A>
using WriteOutcome = std::variant<Committed<T>, Abandoned<A>>;

} // namespace bdb

namespace bdb::detail {

/// The pre-schema resolution table (see the module comment): relation
/// names copied from the admitted spec at construction, declaration order
/// = wire id.
struct Manifest {
    std::vector<std::string> relation_names;

    [[nodiscard]] auto resolve(std::string_view relation) const
        -> std::optional<std::uint32_t> {
        for (auto const& [index, name] :
            std::views::enumerate(relation_names)) {
            if (name == relation) {
                return static_cast<std::uint32_t>(index);
            }
        }
        return std::nullopt;
    }
};

/// Resolves or dies: a coordinate/facade naming a relation outside the
/// admitted spec is an impossible programmer state (the facade and the
/// spec are both compile-time artifacts of the same declaration set), not
/// a recoverable input.
auto resolved_relation(Manifest const& manifest, std::string_view relation)
    -> std::uint32_t {
    auto const id = manifest.resolve(relation);
    contract_assert(id.has_value());
    return *id;
}

/// The facade's relation name, read off its first coordinate (every
/// coordinate of one facade carries the same relation name). C++26
/// structured-binding packs (P1061) — no reflection syntax, which is what
/// keeps this module out of meta/.
template<class Facade>
constexpr auto facade_relation_name(Facade const& facade)
    -> std::string_view {
    auto const& [...coords] = facade;
    static_assert(sizeof...(coords) > 0);
    return [](auto const& first, auto const&...) {
        return first.relation();
    }(coords...);
}

auto lift(foreign::error_handle handle) -> Error {
    return Error{std::move(handle)};
}

} // namespace bdb::detail

export namespace bdb {

/// A lexical borrowed read capability (§16): alive exactly for the
/// Db::read callback. Non-copyable, non-movable, constructible only by
/// Db's trampoline; it never owns and never outlives the callback frame.
class Snapshot {
    foreign::bdb_snapshot_ref const& raw_;
    detail::Manifest const& manifest_;

    Snapshot(foreign::bdb_snapshot_ref const& raw,
        detail::Manifest const& manifest)
        : raw_{raw}, manifest_{manifest} {}

    friend class Db;

public:
    Snapshot(Snapshot const&) = delete;
    auto operator=(Snapshot const&) -> Snapshot& = delete;
    ~Snapshot() = default;

    /// Committed-state membership of one row (marshalled by reflection in
    /// declaration order, §24).
    template<class Facade, class Row>
    [[nodiscard]] auto contains(Facade const& relation, Row const& row) const
        -> std::expected<bool, Error> {
        auto const cells = marshal_row(row);
        return foreign::snapshot_contains(raw_,
            detail::resolved_relation(
                manifest_, detail::facade_relation_name(relation)),
            cells)
            .transform_error([](foreign::error_handle handle) {
                return detail::lift(std::move(handle));
            });
    }

    /// Full-relation export in row_id order: ONE owned crossing, iterated
    /// host-side (§37) — cells decode to bdb::Value; typed row decode
    /// arrives with the schema phase.
    template<class Facade>
    [[nodiscard]] auto scan(Facade const& relation) const
        -> std::expected<RowSet, Error> {
        return foreign::snapshot_scan(raw_,
            detail::resolved_relation(
                manifest_, detail::facade_relation_name(relation)))
            .transform([](foreign::row_set_handle handle) {
                return RowSet{std::move(handle)};
            })
            .transform_error([](foreign::error_handle handle) {
                return detail::lift(std::move(handle));
            });
    }
};

/// A lexical borrowed write capability (§17): alive exactly for the
/// Db::write / Db::write_from callback. Non-copyable, non-movable,
/// constructible only by Db's trampoline. Nothing is judged until commit;
/// the callback's decision (§19) is the commit/abandon switch.
class WriteTx {
    foreign::bdb_tx_ref& raw_;
    detail::Manifest const& manifest_;

    WriteTx(foreign::bdb_tx_ref& raw, detail::Manifest const& manifest)
        : raw_{raw}, manifest_{manifest} {}

    friend class Db;

    [[nodiscard]] auto relation_id(std::string_view relation) const
        -> std::uint32_t {
        return detail::resolved_relation(manifest_, relation);
    }

public:
    WriteTx(WriteTx const&) = delete;
    auto operator=(WriteTx const&) -> WriteTx& = delete;
    ~WriteTx() = default;

    /// Records an insert into the delta (reflection-marshalled, §24);
    /// true = the final state changed. Shape violations are the engine's
    /// typed FactShape error. This phase does not type-check Row against
    /// Facade — that theorem belongs to the schema phase (§28).
    template<class Facade, class Row>
    [[nodiscard]] auto insert(Facade const& relation, Row const& row)
        -> std::expected<bool, Error> {
        auto const cells = marshal_row(row);
        return foreign::tx_insert(raw_,
            relation_id(detail::facade_relation_name(relation)), cells)
            .transform_error([](foreign::error_handle handle) {
                return detail::lift(std::move(handle));
            });
    }

    /// Records a delete into the delta; true = the final state changed.
    template<class Facade, class Row>
    [[nodiscard]] auto remove(Facade const& relation, Row const& row)
        -> std::expected<bool, Error> {
        auto const cells = marshal_row(row);
        return foreign::tx_remove(raw_,
            relation_id(detail::facade_relation_name(relation)), cells)
            .transform_error([](foreign::error_handle handle) {
                return detail::lift(std::move(handle));
            });
    }

    /// Final-state membership (base + pending delta — what the commit
    /// judgment judges; check-then-act is race-free under the single
    /// writer).
    template<class Facade, class Row>
    [[nodiscard]] auto contains(Facade const& relation, Row const& row) const
        -> std::expected<bool, Error> {
        auto const cells = marshal_row(row);
        return foreign::tx_contains(raw_,
            relation_id(detail::facade_relation_name(relation)), cells)
            .transform_error([](foreign::error_handle handle) {
                return detail::lift(std::move(handle));
            });
    }

    /// Mints the next fresh id for the coordinate's field (§25):
    /// `tx.alloc(Service.id)`. The coordinate carries relation name and
    /// ordinal; resolution is the pre-schema name lane (module comment).
    /// Fresh fields are u64 by construction, so only u64 coordinates
    /// allocate.
    [[nodiscard]] auto alloc(coord<std::uint64_t> const& field)
        -> std::expected<std::uint64_t, Error> {
        return foreign::tx_alloc(raw_, relation_id(field.relation()),
            static_cast<std::uint16_t>(field.ordinal))
            .transform_error([](foreign::error_handle handle) {
                return detail::lift(std::move(handle));
            });
    }
};

} // namespace bdb

namespace bdb::detail {

// Pattern-match of a write body's required result shape:
// std::expected<WriteDecision<T, A>, Error>. The primary stays undefined
// so a mis-shaped body fails the WriteBody concept, not an instantiation
// deep inside Db::write.
template<class BodyResult>
struct WriteShapeOf;

template<class T, class A>
struct WriteShapeOf<std::expected<std::variant<Commit<T>, Abandon<A>>, Error>> {
    using CommitCase = Commit<T>;
    using AbandonCase = Abandon<A>;
    using Outcome = WriteOutcome<T, A>;
    using Result = std::expected<Outcome, Error>;
};

template<class Body>
using WriteShape = WriteShapeOf<std::invoke_result_t<Body&, WriteTx&>>;

template<class Result>
inline constexpr bool is_error_expected = false;

template<class T>
inline constexpr bool is_error_expected<std::expected<T, Error>> = true;

} // namespace bdb::detail

export namespace bdb {

/// A read body: Snapshot& -> std::expected<R, Error>.
template<class Body>
concept ReadBody = std::invocable<Body&, Snapshot&>
    && detail::is_error_expected<std::invoke_result_t<Body&, Snapshot&>>;

/// A write body: WriteTx& -> std::expected<WriteDecision<T, A>, Error>.
template<class Body>
concept WriteBody = std::invocable<Body&, WriteTx&>
    && requires { typename detail::WriteShape<Body>::Result; };

/// The owning database capability (§15): move-only RAII; no shared
/// ownership exists at this API. The moved-from Db is inert
/// (alive() == false); RAII owns cleanup — there is no close().
class Db {
    foreign::db_handle handle_;
    detail::Manifest manifest_;

    Db(foreign::db_handle handle, detail::Manifest manifest)
        : handle_{std::move(handle)}, manifest_{std::move(manifest)} {}

    static auto admit(
        std::expected<foreign::db_handle, foreign::error_handle> opened,
        foreign::bdb_schema_spec const& spec) -> std::expected<Db, Error> {
        return std::move(opened)
            .transform([&spec](foreign::db_handle handle) {
                return Db{std::move(handle),
                    detail::Manifest{foreign::relation_names_of(spec)}};
            })
            .transform_error([](foreign::error_handle handle) {
                return detail::lift(std::move(handle));
            });
    }

    // The §19 algebra, shared by write and write_from. The optional slot
    // smuggles the C++ body's full result through the C trampoline;
    // OK/ABORT is derived from it — Commit is the ONLY OK — so
    // user-abandon and user-error both abort the delta but stay
    // distinguishable on the way out.
    template<WriteBody Body, class Runner>
    auto write_through(Body& body, Runner runner) ->
        typename detail::WriteShape<Body>::Result {
        using Shape = detail::WriteShape<Body>;
        using Result = typename Shape::Result;
        using BodyResult = std::invoke_result_t<Body&, WriteTx&>;

        auto slot = std::optional<BodyResult>{};
        auto shim = [&](foreign::bdb_tx_ref& transaction)
            -> foreign::bdb_callback_control {
            auto tx = WriteTx{transaction, manifest_};
            slot.emplace(body(tx));
            auto const wants_commit = slot->has_value()
                && std::holds_alternative<typename Shape::CommitCase>(
                    **slot);
            return wants_commit
                ? foreign::bdb_callback_control::BDB_CALLBACK_CONTROL_OK
                : foreign::bdb_callback_control::BDB_CALLBACK_CONTROL_ABORT;
        };
        auto outcome = runner(shim);
        if (!outcome.has_value()) {
            // Engine failure — commit rejection included (§19's
            // unexpected path).
            return Result{
                std::unexpect, detail::lift(std::move(outcome).error())};
        }
        contract_assert(slot.has_value());
        if (*outcome == foreign::callback_done::completed) {
            contract_assert(slot->has_value());
            return Result{typename Shape::Outcome{Committed{std::move(
                std::get<typename Shape::CommitCase>(**slot).value)}}};
        }
        if (!slot->has_value()) {
            // The body's own typed failure aborted the delta (§36:
            // callback-local failure commits nothing).
            return Result{std::unexpect, std::move(*slot).error()};
        }
        // Abandonment-as-data: the delta dropped, the payload survives.
        return Result{typename Shape::Outcome{Abandoned{std::move(
            std::get<typename Shape::AbandonCase>(**slot).value)}}};
    }

public:
    /// Creates a fresh DURABLE store (pre-schema lane — module comment).
    static auto create(std::string_view path,
        foreign::bdb_schema_spec const& spec) -> std::expected<Db, Error> {
        return admit(foreign::db_handle::create(path, spec), spec);
    }

    /// Opens an existing durable store, fingerprint-verified.
    static auto open(std::string_view path,
        foreign::bdb_schema_spec const& spec) -> std::expected<Db, Error> {
        return admit(foreign::db_handle::open(path, spec), spec);
    }

    /// Opens or initializes an EPHEMERAL store.
    static auto ephemeral(std::string_view path,
        foreign::bdb_schema_spec const& spec) -> std::expected<Db, Error> {
        return admit(foreign::db_handle::ephemeral(path, spec), spec);
    }

    Db(Db const&) = delete;
    auto operator=(Db const&) -> Db& = delete;
    Db(Db&&) noexcept = default;
    auto operator=(Db&&) noexcept -> Db& = default;
    ~Db() = default;

    /// Whether this handle still owns a store (false after move-out —
    /// the §36 inert-source witness).
    [[nodiscard]] auto alive() const -> bool {
        return handle_.alive();
    }

    /// The admitted store's schema fingerprint: 64 lowercase hex chars
    /// (§33's parity readback).
    [[nodiscard]] auto fingerprint() const
        -> std::expected<std::string, Error> {
        return handle_.fingerprint().transform_error(
            [](foreign::error_handle handle) {
                return detail::lift(std::move(handle));
            });
    }

    /// Runs the body over one consistent read snapshot (§16),
    /// synchronously on this thread. The body's own typed failure comes
    /// back out through the expected; the Snapshot dies with the callback.
    template<ReadBody Body>
    auto read(Body&& body) const -> std::invoke_result_t<Body&, Snapshot&> {
        using Result = std::invoke_result_t<Body&, Snapshot&>;
        auto slot = std::optional<Result>{};
        auto outcome = handle_.read(
            [&](foreign::bdb_snapshot_ref const& raw)
                -> foreign::bdb_callback_control {
                auto snapshot = Snapshot{raw, manifest_};
                slot.emplace(body(snapshot));
                return slot->has_value()
                    ? foreign::bdb_callback_control::BDB_CALLBACK_CONTROL_OK
                    : foreign::bdb_callback_control::
                          BDB_CALLBACK_CONTROL_ABORT;
            });
        if (!outcome.has_value()) {
            return Result{
                std::unexpect, detail::lift(std::move(outcome).error())};
        }
        contract_assert(slot.has_value());
        return std::move(*slot);
    }

    /// Runs the body as the single writer (§17/§19). Returns the §19
    /// outcome algebra: Committed | Abandoned on success, engine failure
    /// (commit rejection included) as the error. Re-entrant writes are
    /// refused with a typed EnvironmentLocked error.
    template<WriteBody Body>
    auto write(Body&& body) -> typename detail::WriteShape<Body>::Result {
        return write_through(body,
            [this](auto& shim) { return handle_.write(shim); });
    }

    /// write conditional on a still-live snapshot (§18) — legal from
    /// inside the read callback that owns it. A state-changing commit
    /// since the snapshot is the typed GenerationMoved error; retry is
    /// host policy.
    template<WriteBody Body>
    auto write_from(Snapshot& snapshot, Body&& body) ->
        typename detail::WriteShape<Body>::Result {
        return write_through(body, [this, &snapshot](auto& shim) {
            return handle_.write_from(snapshot.raw_, shim);
        });
    }
};

} // namespace bdb
