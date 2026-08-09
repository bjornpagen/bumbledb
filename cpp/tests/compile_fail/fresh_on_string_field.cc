import std;
import bumbledb;

struct NamingRow {
	[[= bdb::fresh]] std::string name;
};

inline constexpr auto Naming = bdb::relation<"Naming", NamingRow>;
