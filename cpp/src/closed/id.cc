export module bumbledb:id;

import std;
import :name;
import :classify;
import :handle;

export namespace bdb {

/**
 * The synthetic `id` coordinate of a closed relation (`Kind.id`):
 * coordinate-shaped (the statement algebra consumes it exactly like a
 * `bdb::coord`) at sealed ordinal 0, physically the u64 handle row id.
 * Deliberately NOT fresh — closedness itself is the generator judgment
 * (lowering.md §3.2), made by the schema elaborator off this type.
 */
template<name_text Relation, std::size_t HandleCount>
struct closed_id {
	using value_type = closed_ref<Relation>;

	static constexpr name_text relation_name = Relation;
	static constexpr name_text field_name = detail::to_name_text("id");
	static constexpr std::size_t ordinal = 0;
	static constexpr field_class cls{value_kind::u64, 0};
	static constexpr value_kind kind = value_kind::u64;
	static constexpr std::uint16_t fixed_len = 0;
	static constexpr bool fresh = false;
	static constexpr std::size_t handle_count = HandleCount;

	[[nodiscard]] constexpr auto relation() const -> std::string_view {
		return relation_name.view();
	}

	[[nodiscard]] constexpr auto field() const -> std::string_view {
		return field_name.view();
	}
};

template<class T>
inline constexpr bool is_closed_id_v = false;

template<name_text Relation, std::size_t HandleCount>
inline constexpr bool is_closed_id_v<closed_id<Relation, HandleCount>> = true;

}

namespace bdb::detail {

/**
 * `bdb::ref<Kind.id>` — the closed-reference field spelling. Constrained
 * through a struct because alias templates cannot carry requirements.
 */
template<auto Id>
struct ref_of {
	static_assert(is_closed_id_v<std::remove_cvref_t<decltype(Id)>>, "bumbledb ref<>: the argument must be a closed relation's id "
	                                                                 "coordinate (bdb::ref<Kind.id>)");
	using type = closed_ref<std::remove_cvref_t<decltype(Id)>::relation_name>;
};

}

export namespace bdb {

/**
 * The closed-reference field spelling: `bdb::ref_to<Kind.id> kind;` in a
 * row struct references the vocabulary (the C++ image of TS `kind:
 * Kind.id` — physically the engine's u64 handle row id; the vocabulary
 * rides the type, and the wire's newtype label stays law-computed,
 * lowering.md §3). Named `ref_to` because `bdb::ref` is the capacity
 * vocabulary's dependent bound.
 */
template<auto Id>
using ref_to = typename detail::ref_of<Id>::type;

}
