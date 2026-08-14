import std;
import bumbledb;

struct NodeRow {
	[[= bdb::fresh]] std::uint64_t id;
};

inline constexpr auto Node = bdb::relation<"Node", NodeRow>;
inline constexpr auto S = bdb::schema<"S">(Node);

inline constexpr auto InteriorsOnly = bdb::query(S).interior<"mid">([](auto r) consteval {
	auto vars = r.vars(Node);
	return r.match(Node, {.id = vars.id}).find({.id = vars.id});
});

auto probe(bdb::Db const& db) -> void {
	(void)db.prepare<InteriorsOnly>();
}
