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

}
