export module bumbledb:write;

import std;
import :error;

export namespace bdb {

/**
 * The write callback's positive decision: commit the delta, carrying a
 * result value out of the callback.
 */
template<class T>
struct Commit {
	using value_type = T;
	T value;
};

/**
 * The write callback's negative decision AS DATA: drop the delta — the
 * engine never saw a fact — carrying the abandonment's own payload out.
 * Not an error and never the unexpected path.
 */
template<class A>
struct Abandon {
	using value_type = A;
	A value;
};

template<class T, class A>
using WriteDecision = std::variant<Commit<T>, Abandon<A>>;

/**
 * The valueless commit decision (`return bdb::commit();`).
 */
[[nodiscard]] constexpr auto commit() -> Commit<std::monostate> {
	return Commit<std::monostate>{std::monostate{}};
}

template<class T>
[[nodiscard]] constexpr auto commit(T value) -> Commit<T> {
	return Commit<T>{std::move(value)};
}

/**
 * The valueless abandon decision.
 */
[[nodiscard]] constexpr auto abandon() -> Abandon<std::monostate> {
	return Abandon<std::monostate>{std::monostate{}};
}

template<class A>
[[nodiscard]] constexpr auto abandon(A value) -> Abandon<A> {
	return Abandon<A>{std::move(value)};
}

template<class T>
struct Committed {
	T value;
};

template<class A>
struct Abandoned {
	A value;
};

/**
 * What Db::write returns on the SUCCESS path: the write either committed
 * or was abandoned by its own callback. Engine failure — commit
 * rejection included — is the expected's error path, never an
 * alternative here.
 */
template<class T, class A>
using WriteOutcome = std::variant<Committed<T>, Abandoned<A>>;

/**
 * The witnessed loop's honesty bound (the TS WITNESSED_ATTEMPT_CAP):
 * contention alone converges — each rerun reads a fresher snapshot; a
 * workload that moves the generation on every one of this many
 * consecutive attempts is not converging and never will.
 */
inline constexpr std::uint64_t witnessed_attempt_cap = 64;

/**
 * The typed livelock refusal Db::write_witnessed answers past the cap:
 * every attempt found the generation moved, which is only sustainable
 * when the callback itself (even indirectly) commits an interleaved
 * write each try — the remedy is to move that write out of the callback.
 * Carries the final attempt's GenerationMoved error.
 */
struct WitnessedLivelock {
	std::uint64_t attempts;
	Error last;
};

/**
 * What a witnessed write can fail with: an engine failure (commit
 * rejection included), or the typed livelock refusal.
 */
using WitnessedFailure = std::variant<Error, WitnessedLivelock>;

}
