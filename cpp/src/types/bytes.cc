export module bumbledb:bytes;

import std;

export namespace bdb {

/**
 * Fixed-width raw bytes: `bdb::bytes<N>` IS `std::array<std::byte, N>` —
 * one type, two spellings. The engine admits 1 <= N <= 64; the relation
 * reflector enforces that bound at classification.
 */
template<std::size_t N>
using bytes = std::array<std::byte, N>;

}
