export module bumbledb:face;

import std;
import :name;
import :spec;
import :schema_member;
import :where;

export namespace bdb {

/**
 * A statement face value: `on(Outage.service)`,
 * `on(Device.model, Device.watts)`. Positional pairing reads the
 * projection in written order (lowering.md §2). The σ/ψ selection is the
 * face's one VALUE payload — `on(bdb::where(Task, {...}), Task.id)`
 * carries its resolved bindings here (empty on a bare face).
 */
template<class First, class... Rest>
struct face {
	static constexpr std::size_t width = 1 + sizeof...(Rest);
	static constexpr name_text relation_name = First::relation_name;
	static constexpr std::array<name_text, width> projection{First::field_name, Rest::field_name...};

	std::size_t selection_count{};
	std::array<selection_data, max_face_selections> selections{};
};

/**
 * Projects one or more columns of ONE relation as a statement face.
 */
template<class First, class... Rest>
[[nodiscard]] consteval auto on(First, Rest...) -> face<First, Rest...> {
	static_assert(detail::is_coordinate_v<First> && (detail::is_coordinate_v<Rest> && ...),
	              "bumbledb on(): every argument must be a relation coordinate "
	              "(Relation.field)");
	static_assert(detail::same_relation<First, Rest...>(),
	              detail::span_message<First, Rest...>("on", "a face projects one relation's columns"));
	static_assert(1 + sizeof...(Rest) <= max_projection_width, "bumbledb on(): the projection exceeds max_projection_width");
	return {};
}

/**
 * Projects columns of a ψ/σ-selected relation as a statement face.
 */
template<class Facade, class First, class... Rest>
[[nodiscard]] consteval auto on(selected<Facade> const& source, First, Rest...) -> face<First, Rest...> {
	static_assert(detail::is_coordinate_v<First> && (detail::is_coordinate_v<Rest> && ...),
	              "bumbledb on(): every projected argument must be a relation "
	              "coordinate (Relation.field)");
	static_assert(detail::same_relation<First, Rest...>(),
	              detail::span_message<First, Rest...>("on", "a face projects one relation's columns"));
	static_assert(detail::member_relation_of<Facade>() == First::relation_name, detail::selected_projection_message<Facade, First>());
	static_assert(1 + sizeof...(Rest) <= max_projection_width, "bumbledb on(): the projection exceeds max_projection_width");
	auto out = face<First, Rest...>{};
	out.selection_count = source.selection_count;
	out.selections = source.selections;
	return out;
}

}

namespace bdb::detail {

template<class T>
inline constexpr bool is_face_v = false;

template<class First, class... Rest>
inline constexpr bool is_face_v<face<First, Rest...>> = true;

}
