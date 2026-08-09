// compile-fail (TODO_CPP §34): a relation row with a raw pointer field
// must be rejected — the SDK never serializes pointers — with a diagnostic
// naming the relation and the field. (The pointer declaration below is the
// invalid program under test, not dialect code.)
import std;
import bumbledb;

struct LabelRow {
	std::uint64_t id;
	char const* label;
};

inline constexpr auto Labels = bdb::relation<"Labels", LabelRow>;
