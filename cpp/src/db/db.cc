// :db — the owning database capability (TODO_CPP §15–§19, §24–§25).
//
// Two admission lanes: the SCHEMA lane (Db::create/open/ephemeral over a
// bdb::schema<> value, TODO_CPP §13) lowers the schema's flattened tables
// to the owned spec builder — declared statements only, newtype slots fed
// from the law-computed class map (lowering.md §2/§7) — and captures the
// manifest (relation ids + materialized statement ids for §26 keyed
// reads) from the same tables. The PRE-SCHEMA lane (raw
// bdb::foreign::bdb_schema_spec views) remains for spec-level tests.
//
// Failure taxonomy (§19, §27–§28): engine failure is std::unexpected
// (bdb::Error); domain abandonment is DATA on the success path
// (WriteOutcome::Abandoned); checked-value construction failure never
// reaches this partition (bdb::TypeError, :interval).
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

// The witnessed twin: the body takes the WITNESSING snapshot and the tx
// (premise reads on snap, the delta on tx — TODO_CPP §18).
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

} // namespace bdb::detail

export namespace bdb {

/// A read body: Snapshot& -> std::expected<R, Error>.
template<class Body>
concept ReadBody = std::invocable<Body&, Snapshot&> && detail::is_error_expected<std::invoke_result_t<Body&, Snapshot&>>;

/// A write body: WriteTx& -> std::expected<WriteDecision<T, A>, Error>.
template<class Body>
concept WriteBody = std::invocable<Body&, WriteTx&> && requires { typename detail::WriteShape<Body>::Result; };

/// A witnessed-write body: (Snapshot&, WriteTx&) ->
/// std::expected<WriteDecision<T, A>, Error> — premise reads on the
/// snapshot, the delta on the tx (TODO_CPP §18).
template<class Body>
concept WitnessedBody = std::invocable<Body&, Snapshot&, WriteTx&> && requires { typename detail::WitnessedShape<Body>::Result; };

/// The owning database capability (§15): move-only RAII; no shared
/// ownership exists at this API. The moved-from Db is inert
/// (alive() == false); RAII owns cleanup — there is no close().
class Db {
	// Pinned GCC 16.1 quirk: a NON-template member function DEFINITION
	// whose body instantiates the foreign std::expected API (admit, the
	// pre-schema create/open/ephemeral lanes, fingerprint) corrupts this
	// partition's BMI for re-export — the primary interface's
	// `export import :db;` then dies with "failed to read compiled module
	// cluster N: Bad file data". Template members are unaffected. Those
	// bodies therefore live in db_impl.cc (a module IMPLEMENTATION unit,
	// which produces no BMI) — the one interface/impl split in the
	// module, forced by the toolchain, not by design. Re-test on any GCC
	// bump.
	foreign::db_handle handle_;
	detail::Manifest manifest_;

	Db(foreign::db_handle handle, detail::Manifest manifest) : handle_{std::move(handle)}, manifest_{std::move(manifest)} {}

	static auto admit(std::expected<foreign::db_handle, foreign::error_handle> opened, foreign::bdb_schema_spec const& spec)
	    -> std::expected<Db, Error>;

	// The §19 algebra, shared by write and write_from. The optional slot
	// smuggles the C++ body's full result through the C trampoline;
	// OK/ABORT is derived from it — Commit is the ONLY OK — so
	// user-abandon and user-error both abort the delta but stay
	// distinguishable on the way out.
	template<WriteBody Body, class Runner>
	auto write_through(Body& body, Runner runner) -> typename detail::WriteShape<Body>::Result {
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
			// Engine failure — commit rejection included (§19's
			// unexpected path).
			return Result{std::unexpect, detail::lift(std::move(outcome).error())};
		}
		contract_assert(slot.has_value());
		if (*outcome == foreign::callback_done::completed) {
			contract_assert(slot->has_value());
			return Result{typename Shape::Outcome{Committed{std::move(std::get<typename Shape::CommitCase>(**slot).value)}}};
		}
		if (!slot->has_value()) {
			// The body's own typed failure aborted the delta (§36:
			// callback-local failure commits nothing).
			return Result{std::unexpect, std::move(*slot).error()};
		}
		// Abandonment-as-data: the delta dropped, the payload survives.
		return Result{typename Shape::Outcome{Abandoned{std::move(std::get<typename Shape::AbandonCase>(**slot).value)}}};
	}

	// The schema lane's admission: the spec views live exactly for the
	// create/open call (the bridge marshals them before returning); the
	// manifest is rebuilt from the theory's own tables.
	template<Theory S>
	static auto admit_theory(std::expected<foreign::db_handle, foreign::error_handle> opened, S const& theory) -> std::expected<Db, Error> {
		return std::move(opened)
		    .transform([&theory](foreign::db_handle handle) {
			    return Db{std::move(handle), detail::manifest_of(theory)};
		    })
		    .transform_error([](foreign::error_handle handle) {
			    return detail::lift(std::move(handle));
		    });
	}

public:
	/// Creates a fresh DURABLE store (pre-schema lane — module comment).
	static auto create(std::string_view path, foreign::bdb_schema_spec const& spec) -> std::expected<Db, Error>;

	/// Opens an existing durable store, fingerprint-verified.
	static auto open(std::string_view path, foreign::bdb_schema_spec const& spec) -> std::expected<Db, Error>;

	/// Opens or initializes an EPHEMERAL store.
	static auto ephemeral(std::string_view path, foreign::bdb_schema_spec const& spec) -> std::expected<Db, Error>;

	/// Creates a fresh DURABLE store from a bdb::schema<> value (the
	/// schema lane, TODO_CPP §13): the spec views are built from the
	/// schema's flattened tables — DECLARED statements only, newtype
	/// slots fed from the law-computed class map — and handed to the
	/// engine's SchemaSpec::descriptor(), which stays authoritative.
	template<Theory S>
	static auto create(std::string_view path, S const& theory) -> std::expected<Db, Error> {
		auto const spec = foreign::owned_schema_spec{detail::owned_relations_of(theory), detail::owned_statements_of(theory)};
		return admit_theory(foreign::db_handle::create(path, spec.view()), theory);
	}

	/// Opens an existing durable store against a schema value,
	/// fingerprint-verified by the engine.
	template<Theory S>
	static auto open(std::string_view path, S const& theory) -> std::expected<Db, Error> {
		auto const spec = foreign::owned_schema_spec{detail::owned_relations_of(theory), detail::owned_statements_of(theory)};
		return admit_theory(foreign::db_handle::open(path, spec.view()), theory);
	}

	/// Opens or initializes an EPHEMERAL store from a schema value.
	template<Theory S>
	static auto ephemeral(std::string_view path, S const& theory) -> std::expected<Db, Error> {
		auto const spec = foreign::owned_schema_spec{detail::owned_relations_of(theory), detail::owned_statements_of(theory)};
		return admit_theory(foreign::db_handle::ephemeral(path, spec.view()), theory);
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
	[[nodiscard]] auto fingerprint() const -> std::expected<std::string, Error>;

	/// Runs the body over one consistent read snapshot (§16),
	/// synchronously on this thread. The body's own typed failure comes
	/// back out through the expected; the Snapshot dies with the callback.
	template<ReadBody Body>
	auto read(Body&& body) const -> std::invoke_result_t<Body&, Snapshot&> {
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

	/// Runs the body as the single writer (§17/§19). Returns the §19
	/// outcome algebra: Committed | Abandoned on success, engine failure
	/// (commit rejection included) as the error. Re-entrant writes are
	/// refused with a typed EnvironmentLocked error.
	template<WriteBody Body>
	auto write(Body&& body) -> typename detail::WriteShape<Body>::Result {
		return write_through(body, [this](auto& shim) {
			return handle_.write(shim);
		});
	}

	/// write conditional on a still-live snapshot (§18) — legal from
	/// inside the read callback that owns it. A state-changing commit
	/// since the snapshot is the typed GenerationMoved error; retry is
	/// host policy.
	template<WriteBody Body>
	auto write_from(Snapshot& snapshot, Body&& body) -> typename detail::WriteShape<Body>::Result {
		return write_through(body, [this, &snapshot](auto& shim) {
			return handle_.write_from(snapshot.raw_, shim);
		});
	}

	/// The witnessed write loop (§18; the TS db.writeWitnessed): one
	/// callback receives a consistent snapshot AND the write tx; the
	/// commit lands only if the generation the snapshot witnessed is
	/// still current. On GenerationMoved the STALE diff is dropped —
	/// never replayed — and the callback reruns against a FRESH snapshot,
	/// up to `witnessed_attempt_cap` attempts; past the cap the typed
	/// WitnessedLivelock refusal comes back (the callback itself moves
	/// the generation each try — host pathology, not engine judgment).
	/// Every other engine failure (commit rejection included) surfaces
	/// unchanged on the first occurrence.
	template<WitnessedBody Body>
	auto write_witnessed(Body&& body) -> typename detail::WitnessedShape<Body>::Result {
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
			// Rebuild on a fresh snapshot (the loop's next read).
		}
	}

	/// Full-relation export, one call (the TS db.scan symmetry): opens a
	/// read snapshot for exactly this scan; the RowSet is owned and
	/// outlives it.
	template<class Facade>
	[[nodiscard]] auto scan(Facade const& relation) const -> std::expected<RowSet, Error> {
		return read([&](Snapshot& snapshot) -> std::expected<RowSet, Error> {
			return snapshot.scan(relation);
		});
	}

	/// Executes a prepared query, one call (the TS db.execute symmetry):
	/// opens a read snapshot for exactly this execution.
	template<auto Query>
	[[nodiscard]] auto execute(Prepared<Query>& prepared, params_of<Query> const& params) const -> std::expected<Answers<Query>, Error> {
		return read([&](Snapshot& snapshot) -> std::expected<Answers<Query>, Error> {
			return snapshot.execute(prepared, params);
		});
	}

	/// Committed-state keyed point read (§26), one call: opens a read
	/// snapshot for exactly this lookup. The stored key law value is the
	/// selector; the RowSet is owned and outlives the snapshot.
	template<class Facade, class First, class... Rest>
	[[nodiscard]] auto get(Facade const& relation, key_law<First, Rest...> const& law,
	                       typename key_law<First, Rest...>::pattern const& key) const -> std::expected<std::optional<RowSet>, Error> {
		return read([&](Snapshot& snapshot) -> std::expected<std::optional<RowSet>, Error> {
			return snapshot.get(relation, law, key);
		});
	}

	/// The fresh-field primary read (§26): `db.get(Service, {.id = id})`.
	template<class Facade>
	    requires(fresh_field_count<Facade>() >= 1)
	[[nodiscard]] auto get(Facade const& relation, fresh_pattern_of<Facade> const& key) const
	    -> std::expected<std::optional<RowSet>, Error> {
		return read([&](Snapshot& snapshot) -> std::expected<std::optional<RowSet>, Error> {
			return snapshot.get(relation, key);
		});
	}

	/// Prepares one compile-time query value against this store
	/// (TODO_CPP §20, §43: `db.prepare<DownAt>()`). The query already
	/// lowered to a static program-IR view graph during constant
	/// evaluation; the engine's IR validator remains the trust boundary
	/// here — compile-time validation supplements it, never replaces it
	/// (§11).
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

} // namespace bdb
