import std;
import bumbledb;

struct RatioRow {
	std::uint64_t id;
	float ratio;
};

inline constexpr auto Ratio = bdb::relation<"Ratio", RatioRow>;
