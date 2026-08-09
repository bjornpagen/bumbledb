// :write — the write-outcome algebra (TODO_CPP §19, §27–§28): the write
// callback's decision vocabulary (Commit/Abandon as DATA on the success
// path), the WriteOutcome, and the witnessed loop's typed livelock
// refusal. Engine failure is std::unexpected (bdb::Error); domain
// abandonment is data on the success path; checked-value construction
// failure never reaches this layer (bdb::TypeError, :interval).
export module bumbledb:write;

import std;
import :error;

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

/// The witnessed loop's honesty bound (the TS WITNESSED_ATTEMPT_CAP):
/// contention alone converges — each rerun reads a FRESHER snapshot; a
/// workload that moves the generation on EVERY one of this many
/// consecutive attempts is not converging and never will.
inline constexpr std::uint64_t witnessed_attempt_cap = 64;

/// The typed livelock refusal `Db::write_witnessed` answers past the cap:
/// every attempt found the generation moved, which is only sustainable
/// when the callback ITSELF (even indirectly) commits an interleaved
/// write each try. Host-policy pathology, not engine judgment — the
/// remedy is to move the interleaved write out of the callback. Carries
/// the final attempt's GenerationMoved error.
struct WitnessedLivelock {
	std::uint64_t attempts;
	Error last;
};

/// What a witnessed write can fail with: an engine failure (commit
/// rejection included), or the typed livelock refusal.
using WitnessedFailure = std::variant<Error, WitnessedLivelock>;

} // namespace bdb
