// :order — host-side answer ordering (ts/src/order.ts). Answers
// are SETS and the ENGINE NEVER ORDERS; the host owns the sort and the
// limit. What the SDK ships is exactly the row-typed comparator over its
// own cell vocabulary: sort keys are DATA — a bare member pointer is
// ascending (the punning spelling; no `asc` wrapper exists — one spelling
// per meaning) and `bdb::desc(&Row::col)` is the one descending spelling
// — folded by `bdb::by(...)` into one comparator usable with
// std::ranges::sort. Cell order mirrors the engine's own orderability:
// bool false < true, u64/i64 numeric, strings/bytes lexicographic,
// intervals by (start, end).
//
// ZERO keys is the identity comparator (the TS ruling): `bdb::by()`
// orders bare scalar sequences by the value itself.
//
export module bumbledb:order;

import std;
import :interval;
import :bytes;
import :allen;
import :fresh;

export namespace bdb {

/// One DESCENDING sort key, plain data — built by bdb::desc.
template<class Key>
struct Desc {
	Key key;
};

/// The one descending spelling: `bdb::desc(&Row::total)`.
template<class Key>
[[nodiscard]] constexpr auto desc(Key key) -> Desc<Key> {
	return {key};
}

/// The folded comparator value `bdb::by(...)` builds: strict-weak "less"
/// over rows (or, with zero keys, over bare cells) — the first non-equal
/// key decides.
template<class... Keys>
struct Ordering {
	std::tuple<Keys...> keys;

	template<class Row>
	[[nodiscard]] constexpr auto operator()(Row const& left, Row const& right) const -> bool {
		if constexpr (sizeof...(Keys) == 0) {
			// The identity comparator: the value itself is the key.
			return std::is_lt(cell_order(left, right));
		} else {
			auto verdict = std::strong_ordering::equal;
			std::apply(
			    [&](Keys const&... each) -> void {
				    static_cast<void>(((verdict = key_order(left, right, each), verdict != 0) || ...));
			    },
			    keys);
			return std::is_lt(verdict);
		}
	}

private:
	/// One cell against one cell — total over the SDK's cell vocabulary.
	template<class T>
	static constexpr auto cell_order(T const& left, T const& right) -> std::strong_ordering {
		if constexpr (requires {
			              left.lo();
			              left.hi();
		              }) {
			// An interval orders by start, then end (ts/src/order.ts).
			if (auto const starts = left.lo() <=> right.lo(); starts != 0) {
				return starts;
			}
			return left.hi() <=> right.hi();
		} else if constexpr (std::convertible_to<T, std::string_view>) {
			return std::string_view{left} <=> std::string_view{right};
		} else if constexpr (std::ranges::range<T>) {
			// Bytes (and any other cell sequence): lexicographic over
			// the shared prefix, then by length.
			return std::lexicographical_compare_three_way(std::ranges::begin(left), std::ranges::end(left), std::ranges::begin(right),
			                                              std::ranges::end(right));
		} else {
			// bool (false < true), u64, i64.
			return left <=> right;
		}
	}

	/// One key's verdict over one row pair (descending keys flip the
	/// sides). A Desc wrapper is recognized structurally (it carries the
	/// wrapped member pointer as `.key`).
	template<class Row, class Key>
	static constexpr auto key_order(Row const& left, Row const& right, Key const& key) -> std::strong_ordering {
		if constexpr (requires { key.key; }) {
			return cell_order(right.*(key.key), left.*(key.key));
		} else {
			return cell_order(left.*key, right.*key);
		}
	}
};

/// Folds sort keys into one row comparator — keys as data, ascending by
/// default: `std::ranges::sort(rows, bdb::by(&Row::pool,
/// bdb::desc(&Row::total)))`. Zero keys is the identity comparator over
/// bare cells.
template<class... Keys>
[[nodiscard]] constexpr auto by(Keys... keys) -> Ordering<Keys...> {
	return {std::tuple{keys...}};
}

} // namespace bdb
