export module bumbledb:fresh;

import std;

export namespace bdb {

/**
 * The fresh mark's tag type: the relation reflector matches annotations
 * by this type; `fresh` is the one annotation object.
 */
struct FreshTag {};

/**
 * The fresh mark: `[[=bdb::fresh]]` on a `std::uint64_t` row field marks
 * the engine-minted identity column. u64-only — the reflector enforces
 * it and engine validation re-judges.
 */
inline constexpr auto fresh = FreshTag{};

}
