// compile-fail (TODO_CPP §34): [[=bdb::fresh]] is legal on std::uint64_t
// fields only (the TS SDK twin rule; engine validation re-judges). A fresh
// mark on a std::string field must be rejected with a diagnostic naming
// the relation, the field, and the offending type.
import std;
import bumbledb;

struct NamingRow {
	[[= bdb::fresh]] std::string name;
};

inline constexpr auto Naming = bdb::relation<"Naming", NamingRow>;
