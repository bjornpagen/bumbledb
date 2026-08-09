// compile-fail (TODO_CPP §34): a relation row with a float field must be
// rejected — the value vocabulary is closed — with a diagnostic naming the
// relation, the field, and the offending type.
import std;
import bumbledb;

struct RatioRow {
    std::uint64_t id;
    float ratio;
};

inline constexpr auto Ratio = bdb::relation<"Ratio", RatioRow>;
