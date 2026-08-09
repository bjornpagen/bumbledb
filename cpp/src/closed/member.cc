// :member — a declared closed-vocabulary member (TODO_CPP §8; the payload
// tier's argument): the handle name rides the TYPE, the axiom row the
// value.
export module bumbledb:member;

import std;
import :name;

export namespace bdb {

/// The bare tier's empty payload row (no declared columns; the axioms
/// readback rows are this empty product).
struct no_payload {
	[[nodiscard]] constexpr auto operator==(no_payload const&) const -> bool = default;
};

/// One declared vocabulary member (the payload tier's argument):
/// `bdb::member<"DirectPass">(KindPayload{...})` — the handle name rides
/// the TYPE (it becomes a facade member name), the axiom row the value.
template<fixed_string Handle, class Payload>
struct member_value {
	static constexpr name_text handle = detail::to_name_text(Handle.view());

	Payload payload;
};

/// Mints one vocabulary member for the payload tier.
template<fixed_string Handle, class Payload>
[[nodiscard]] consteval auto member(Payload payload) -> member_value<Handle, Payload> {
	return {payload};
}

} // namespace bdb

namespace bdb::detail {

/// Whether T is a `bdb::member<...>` of exactly this payload type.
template<class T, class Payload>
inline constexpr bool is_member_of_v = false;

template<fixed_string Handle, class Payload>
inline constexpr bool is_member_of_v<member_value<Handle, Payload>, Payload> = true;

} // namespace bdb::detail
