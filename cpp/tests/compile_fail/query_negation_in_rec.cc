import std;
import bumbledb;

struct NodeRow {
	[[= bdb::fresh]] std::uint64_t id;
};

struct EdgeRow {
	std::uint64_t child;
	std::uint64_t parent;
};

inline constexpr auto Node = bdb::relation<"Node", NodeRow>;
inline constexpr auto Edge = bdb::relation<"Edge", EdgeRow>;
inline constexpr auto S = bdb::schema<"S">(Node, Edge, bdb::contained(bdb::on(Edge.child), bdb::on(Node.id)),
                                           bdb::contained(bdb::on(Edge.parent), bdb::on(Node.id)));

inline constexpr auto Broken = bdb::query(S)
                                   .reach<"reach">(
                                       bdb::base{[](auto r) consteval {
	                                       auto vars = r.vars(Node);
	                                       return r.match(Node, {.id = vars.id}).find({}, bdb::as<"c">(vars.id));
                                       }},
                                       bdb::rec{[](auto r) consteval {
	                                       auto vars = r.vars(Edge);
	                                       return r.match(Edge, {.child = vars.child, .parent = vars.parent})
	                                           .template not_interior<"reach">(bdb::bind<"c">(vars.parent))
	                                           .find({}, bdb::as<"c">(vars.child));
                                       }})
                                   .rule([](auto r) consteval {
	                                   auto vars = r.vars(Node);
	                                   return r.template interior<"reach">(bdb::bind<"c">(vars.id)).find({.id = vars.id});
                                   });
