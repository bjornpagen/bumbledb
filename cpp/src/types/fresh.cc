// :fresh — the fresh-mark annotation (TODO_CPP §6).
export module bumbledb:fresh;

import std;

export namespace bdb {

/// The fresh-mark annotation's tag type. The relation reflector matches
/// annotations of this type; `fresh` below is the one annotation object.
struct FreshTag {};

/// The fresh mark: `[[=bdb::fresh]]` on a `std::uint64_t` row field marks
/// the engine-minted identity column (TODO_CPP §6; u64-only, enforced by
/// the reflector and re-judged by engine validation).
inline constexpr auto fresh = FreshTag{};

} // namespace bdb
