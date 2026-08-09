export module bumbledb:db;

import std;
import :error;
import :answers;
import :key;
import :schema;
import :query;
import :manifest;
import :wire;
import :write;
import :prepared;
import :snapshot;
import :tx;
import :foreign_program;
import bumbledb_foreign;

namespace bdb::detail {

/**
 * Pattern-match of a write body's required result shape. The primary
 * stays undefined so a mis-shaped body fails the WriteBody concept, not
 * an instantiation deep inside Db::write.
 */
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

template<class BodyResult>
struct WitnessedShapeOf;

template<class T, class A>
struct WitnessedShapeOf<std::expected<std::variant<Commit<T>, Abandon<A>>, Error>> {
	using Outcome = WriteOutcome<T, A>;
	using Result = std::expected<Outcome, WitnessedFailure>;
};

template<class Body>
using WitnessedShape = WitnessedShapeOf<std::invoke_result_t<Body&, Snapshot&, WriteTx&>>;

template<class Result>
inline constexpr bool is_error_expected = false;

template<class T>
inline constexpr bool is_error_expected<std::expected<T, Error>> = true;

}

export namespace bdb {

template<class Body>
concept ReadBody = std::invocable<Body&, Snapshot&> && detail::is_error_expected<std::invoke_result_t<Body&, Snapshot&>>;

template<class Body>
concept WriteBody = std::invocable<Body&, WriteTx&> && requires { typename detail::WriteShape<Body>::Result; };

/**
 * A witnessed-write body: premise reads belong on the Snapshot, the
 * delta on the WriteTx.
 */
template<class Body>
concept WitnessedBody = std::invocable<Body&, Snapshot&, WriteTx&> && requires { typename detail::WitnessedShape<Body>::Result; };

/**
 * The owning database capability: move-only RAII; no shared ownership
 * exists at this API. The moved-from Db is inert (alive() == false);
 * RAII owns cleanup — there is no close().
 */
class [[nodiscard]] Db {
	/* PIN(gcc-partition-bmi-expected): admit, the pre-schema create/open/ephemeral lanes, and fingerprint have bodies in db_impl.cc */
	foreign::db_handle handle_;
	detail::Manifest manifest_;

	Db(foreign::db_handle handle, detail::Manifest manifest) : handle_{std::move(handle)}, manifest_{std::move(manifest)} {}

	[[nodiscard]] static auto admit(std::expected<foreign::db_handle, foreign::error_handle> opened, foreign::bdb_schema_spec const& spec)
	    -> std::expected<Db, Error>;

	template<WriteBody Body, class Runner>
	[[nodiscard]] auto write_through(Body& body, Runner runner) -> typename detail::WriteShape<Body>::Result {
		using Shape = detail::WriteShape<Body>;
		using Result = typename Shape::Result;
		using BodyResult = std::invoke_result_t<Body&, WriteTx&>;

		auto slot = std::optional<BodyResult>{};
		auto shim = [&](foreign::bdb_tx_ref& transaction) -> foreign::bdb_callback_control {
			auto tx = WriteTx{transaction, manifest_};
			slot.emplace(body(tx));
			auto const wants_commit = slot->has_value() && std::holds_alternative<typename Shape::CommitCase>(**slot);
			return wants_commit ? foreign::bdb_callback_control::BDB_CALLBACK_CONTROL_OK
			                    : foreign::bdb_callback_control::BDB_CALLBACK_CONTROL_ABORT;
		};
		auto outcome = runner(shim);
		if (!outcome.has_value()) {
			return Result{std::unexpect, detail::lift(std::move(outcome).error())};
		}
		contract_assert(slot.has_value());
		if (*outcome == foreign::callback_done::completed) {
			contract_assert(slot->has_value());
			return Result{typename Shape::Outcome{Committed{std::move(std::get<typename Shape::CommitCase>(**slot).value)}}};
		}
		if (!slot->has_value()) {
			return Result{std::unexpect, std::move(*slot).error()};
		}
		return Result{typename Shape::Outcome{Abandoned{std::move(std::get<typename Shape::AbandonCase>(**slot).value)}}};
	}

	template<Theory S>
	[[nodiscard]] static auto admit_theory(std::expected<foreign::db_handle, foreign::error_handle> opened, S const& theory) -> std::expected<Db, Error> {
		return std::move(opened)
		    .transform([&theory](foreign::db_handle handle) {
			    return Db{std::move(handle), detail::manifest_of(theory)};
		    })
		    .transform_error([](foreign::error_handle handle) {
			    return detail::lift(std::move(handle));
		    });
	}

public:
	/**
	 * The pre-schema raw-spec lane (spec-level tests only): creates a
	 * fresh durable store. Keyed reads do not resolve on this lane.
	 */
	[[nodiscard]] static auto create(std::string_view path, foreign::bdb_schema_spec const& spec) -> std::expected<Db, Error>;

	/**
	 * The pre-schema raw-spec lane: opens an existing durable store,
	 * fingerprint-verified by the engine.
	 */
	[[nodiscard]] static auto open(std::string_view path, foreign::bdb_schema_spec const& spec) -> std::expected<Db, Error>;

	/**
	 * The pre-schema raw-spec lane: opens or initializes an ephemeral
	 * store.
	 */
	[[nodiscard]] static auto ephemeral(std::string_view path, foreign::bdb_schema_spec const& spec) -> std::expected<Db, Error>;

	/**
	 * Creates a fresh durable store from a bdb::schema<> value. The
	 * engine's SchemaSpec::descriptor() stays authoritative over the
	 * lowered spec (lowering.md §2/§7).
	 */
	template<Theory S>
	[[nodiscard]] static auto create(std::string_view path, S const& theory) -> std::expected<Db, Error> {
		auto const spec = foreign::owned_schema_spec{detail::owned_relations_of(theory), detail::owned_statements_of(theory)};
		return admit_theory(foreign::db_handle::create(path, spec.view()), theory);
	}

	/**
	 * Opens an existing durable store against a schema value,
	 * fingerprint-verified by the engine.
	 */
	template<Theory S>
	[[nodiscard]] static auto open(std::string_view path, S const& theory) -> std::expected<Db, Error> {
		auto const spec = foreign::owned_schema_spec{detail::owned_relations_of(theory), detail::owned_statements_of(theory)};
		return admit_theory(foreign::db_handle::open(path, spec.view()), theory);
	}

	/**
	 * Opens or initializes an ephemeral store from a schema value.
	 */
	template<Theory S>
	[[nodiscard]] static auto ephemeral(std::string_view path, S const& theory) -> std::expected<Db, Error> {
		auto const spec = foreign::owned_schema_spec{detail::owned_relations_of(theory), detail::owned_statements_of(theory)};
		return admit_theory(foreign::db_handle::ephemeral(path, spec.view()), theory);
	}

	Db(Db const&) = delete;
	auto operator=(Db const&) -> Db& = delete;
	Db(Db&&) noexcept = default;
	auto operator=(Db&&) noexcept -> Db& = default;
	~Db() = default;

	/**
	 * Whether this handle still owns a store (false after move-out).
	 */
	[[nodiscard]] auto alive() const -> bool {
		return handle_.alive();
	}

	/**
	 * The admitted store's schema fingerprint: 64 lowercase hex chars.
	 */
	[[nodiscard]] auto fingerprint() const -> std::expected<std::string, Error>;

	/**
	 * Runs the body over one consistent read snapshot, synchronously on
	 * this thread. The body's own typed failure comes back out through
	 * the expected; the Snapshot dies with the callback.
	 */
	template<ReadBody Body>
	[[nodiscard]] auto read(Body&& body) const -> std::invoke_result_t<Body&, Snapshot&> {
		using Result = std::invoke_result_t<Body&, Snapshot&>;
		auto slot = std::optional<Result>{};
		auto outcome = handle_.read([&](foreign::bdb_snapshot_ref const& raw) -> foreign::bdb_callback_control {
			auto snapshot = Snapshot{raw, manifest_};
			slot.emplace(body(snapshot));
			return slot->has_value() ? foreign::bdb_callback_control::BDB_CALLBACK_CONTROL_OK
			                         : foreign::bdb_callback_control::BDB_CALLBACK_CONTROL_ABORT;
		});
		if (!outcome.has_value()) {
			return Result{std::unexpect, detail::lift(std::move(outcome).error())};
		}
		contract_assert(slot.has_value());
		return std::move(*slot);
	}

	/**
	 * Runs the body as the single writer. Returns the outcome algebra:
	 * Committed | Abandoned on success; engine failure (commit rejection
	 * included) as the error. A re-entrant write is refused with the
	 * typed EnvironmentLocked error.
	 */
	template<WriteBody Body>
	[[nodiscard]] auto write(Body&& body) -> typename detail::WriteShape<Body>::Result {
		return write_through(body, [this](auto& shim) {
			return handle_.write(shim);
		});
	}

	/**
	 * write conditional on a still-live snapshot — legal only from
	 * inside the read callback that owns it. A state-changing commit
	 * since the snapshot is the typed GenerationMoved error; retry is
	 * host policy.
	 */
	template<WriteBody Body>
	[[nodiscard]] auto write_from(Snapshot& snapshot, Body&& body) -> typename detail::WriteShape<Body>::Result {
		return write_through(body, [this, &snapshot](auto& shim) {
			return handle_.write_from(snapshot.raw_, shim);
		});
	}

	/**
	 * The derived-fact maintenance protocol spelled once, host-side
	 * (normative: docs/architecture/70-api.md, "Derived-fact maintenance
	 * protocol"): retries exactly GenerationMoved — stale diff dropped,
	 * the body rerun against a fresh snapshot — and refuses past
	 * witnessed_attempt_cap attempts with the typed WitnessedLivelock.
	 * Every other failure surfaces unchanged on first occurrence.
	 */
	template<WitnessedBody Body>
	[[nodiscard]] auto write_witnessed(Body&& body) -> typename detail::WitnessedShape<Body>::Result {
		using Shape = detail::WitnessedShape<Body>;
		using Result = typename Shape::Result;
		using Outcome = typename Shape::Outcome;
		for (auto attempt = std::uint64_t{1};; ++attempt) {
			auto tried = read([&](Snapshot& snapshot) -> std::expected<Outcome, Error> {
				return write_from(snapshot, [&](WriteTx& tx) {
					return body(snapshot, tx);
				});
			});
			if (tried.has_value()) {
				return Result{std::move(*tried)};
			}
			auto error = std::move(tried).error();
			if (error.kind() != ErrorKind::GenerationMoved) {
				return Result{std::unexpect, WitnessedFailure{std::in_place_type<Error>, std::move(error)}};
			}
			if (attempt == witnessed_attempt_cap) {
				return Result{std::unexpect, WitnessedFailure{std::in_place_type<WitnessedLivelock>, WitnessedLivelock{
				                                                                                         .attempts = attempt,
				                                                                                         .last = std::move(error),
				                                                                                     }}};
			}
		}
	}

	/**
	 * Full-relation export, one call: opens a read snapshot for exactly
	 * this scan; the RowSet is owned and outlives it.
	 */
	template<class Facade>
	[[nodiscard]] auto scan(Facade const& relation) const -> std::expected<RowSet, Error> {
		return read([&](Snapshot& snapshot) -> std::expected<RowSet, Error> {
			return snapshot.scan(relation);
		});
	}

	/**
	 * Executes a prepared query, one call: opens a read snapshot for
	 * exactly this execution.
	 */
	template<auto Query>
	[[nodiscard]] auto execute(Prepared<Query>& prepared, params_of<Query> const& params) const -> std::expected<Answers<Query>, Error> {
		return read([&](Snapshot& snapshot) -> std::expected<Answers<Query>, Error> {
			return snapshot.execute(prepared, params);
		});
	}

	/**
	 * Committed-state keyed point read, one call: opens a read snapshot
	 * for exactly this lookup. The stored key law value is the selector;
	 * the RowSet is owned and outlives the snapshot.
	 */
	template<class Facade, class First, class... Rest>
	[[nodiscard]] auto get(Facade const& relation, key_law<First, Rest...> const& law,
	                       typename key_law<First, Rest...>::pattern const& key) const -> std::expected<std::optional<RowSet>, Error> {
		return read([&](Snapshot& snapshot) -> std::expected<std::optional<RowSet>, Error> {
			return snapshot.get(relation, law, key);
		});
	}

	/**
	 * The fresh-field primary read: `db.get(Service, {.id = id})`.
	 */
	template<class Facade>
	    requires(fresh_field_count<Facade>() >= 1)
	[[nodiscard]] auto get(Facade const& relation, fresh_pattern_of<Facade> const& key) const
	    -> std::expected<std::optional<RowSet>, Error> {
		return read([&](Snapshot& snapshot) -> std::expected<std::optional<RowSet>, Error> {
			return snapshot.get(relation, key);
		});
	}

	/**
	 * Prepares one compile-time query value against this store. The
	 * query already lowered to a static program-IR view graph during
	 * constant evaluation; the engine's IR validator remains the trust
	 * boundary — compile-time validation supplements it, never replaces
	 * it.
	 */
	template<auto Query>
	[[nodiscard]] auto prepare() const -> std::expected<Prepared<Query>, Error> {
		return handle_.prepare(foreign::program_of<Query>)
		    .transform([](foreign::prepared_handle handle) {
			    return Prepared<Query>{std::move(handle)};
		    })
		    .transform_error([](foreign::error_handle handle) {
			    return detail::lift(std::move(handle));
		    });
	}
};

}
