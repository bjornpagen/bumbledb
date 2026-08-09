/**
 * Host-side answer ordering: answers are sets and the engine never
 * orders — the host owns the sort and the limit (ts/src/order.ts is the
 * twin).
 */
export module bumbledb:order;

import std;
import :interval;
import :bytes;
import :allen;
import :fresh;

export namespace bdb {

template<class Key>
struct Desc {
	Key key;
};

/**
 * The one descending spelling: `bdb::desc(&Row::total)`. A bare member
 * pointer is ascending; no `asc` wrapper exists.
 */
template<class Key>
[[nodiscard]] constexpr auto desc(Key key) -> Desc<Key> {
	return {key};
}

/**
 * The comparator `bdb::by(...)` builds: strict-weak "less" over rows
 * (or, with zero keys, over bare cells) — the first non-equal key
 * decides.
 */
template<class... Keys>
struct Ordering {
	std::tuple<Keys...> keys;

	template<class Row>
	[[nodiscard]] constexpr auto operator()(Row const& left, Row const& right) const -> bool {
		if constexpr (sizeof...(Keys) == 0) {
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
	/**
	 * Total over the cell vocabulary, mirroring the engine's own
	 * orderability: bool false < true, u64/i64 numeric, strings/bytes
	 * lexicographic, intervals by (start, end).
	 */
	template<class T>
	[[nodiscard]] static constexpr auto cell_order(T const& left, T const& right) -> std::strong_ordering {
		if constexpr (requires {
			              left.lo();
			              left.hi();
		              }) {
			if (auto const starts = left.lo() <=> right.lo(); starts != 0) {
				return starts;
			}
			return left.hi() <=> right.hi();
		} else if constexpr (std::convertible_to<T, std::string_view>) {
			return std::string_view{left} <=> std::string_view{right};
		} else if constexpr (std::ranges::range<T>) {
			return std::lexicographical_compare_three_way(std::ranges::begin(left), std::ranges::end(left), std::ranges::begin(right),
			                                              std::ranges::end(right));
		} else {
			return left <=> right;
		}
	}

	template<class Row, class Key>
	[[nodiscard]] static constexpr auto key_order(Row const& left, Row const& right, Key const& key) -> std::strong_ordering {
		if constexpr (requires { key.key; }) {
			return cell_order(right.*(key.key), left.*(key.key));
		} else {
			return cell_order(left.*key, right.*key);
		}
	}
};

/**
 * Folds sort keys into one row comparator usable with std::ranges::sort:
 * `bdb::by(&Row::pool, bdb::desc(&Row::total))`. Zero keys orders bare
 * cells by the value itself.
 */
template<class... Keys>
[[nodiscard]] constexpr auto by(Keys... keys) -> Ordering<Keys...> {
	return {std::tuple{keys...}};
}

}
