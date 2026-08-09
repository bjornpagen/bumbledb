// :key — the functional-dependency law (pairs NOTHING in the class laws;
// TODO_CPP §9, §26). The law itself is the selector — get() resolves the
// key statement by this value's structural identity.
export module bumbledb:key;

import std;
import :name;
import :spec;
import :schema_member;

namespace bdb::detail {

// The key pattern injection (TODO_CPP §26): one member per projected
// coordinate, named by the field, typed by the field's physical type —
// so `db.get(Outage, outage_key, {.service = s, .window = w})` marshals
// its key values in the key statement's projection order by reflection.
template<class... Coords>
struct key_pattern_types {
	struct Pattern;
	consteval {
		std::meta::define_aggregate(^^Pattern, {
		                                           std::meta::data_member_spec(^^typename Coords::value_type,
		                                                                       {
		                                                                           .name = spec_name(Coords::field_name.view())})...});
	}
};

} // namespace bdb::detail

export namespace bdb {

// ————————————————————————————————————————————————————————————————————
// key: the functional-dependency law (pairs NOTHING in the class laws).
// ————————————————————————————————————————————————————————————————————

/// A stored key law value (§26: the law itself is the selector — get()
/// resolves the key statement by this value's structural identity).
template<class First, class... Rest>
struct key_law {
	static constexpr std::size_t width = 1 + sizeof...(Rest);
	static constexpr name_text relation_name = First::relation_name;
	static constexpr std::array<name_text, width> projection{First::field_name, Rest::field_name...};

	/// The keyed-read pattern product: members named by the projected
	/// fields in projection order.
	using pattern = typename detail::key_pattern_types<First, Rest...>::Pattern;
};

/// `key(Outage.service, Outage.window)` — R(X) -> R over one relation.
template<class First, class... Rest>
consteval auto key(First, Rest...) -> key_law<First, Rest...> {
	static_assert(detail::is_coordinate_v<First> && (detail::is_coordinate_v<Rest> && ...),
	              "bumbledb key(): every argument must be a relation coordinate "
	              "(Relation.field)");
	static_assert(detail::same_relation<First, Rest...>(),
	              detail::span_message<First, Rest...>("key", "a key constrains one relation's own rows"));
	static_assert(1 + sizeof...(Rest) <= max_projection_width, "bumbledb key(): the projection exceeds max_projection_width");
	return {};
}

} // namespace bdb

namespace bdb::detail {

template<class T>
inline constexpr bool is_key_v = false;

template<class First, class... Rest>
inline constexpr bool is_key_v<key_law<First, Rest...>> = true;

} // namespace bdb::detail
