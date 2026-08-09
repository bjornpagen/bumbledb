import std;
import bumbledb;

struct LabelRow {
	std::uint64_t id;
	char const* label;
};

inline constexpr auto Labels = bdb::relation<"Labels", LabelRow>;
