// §34 — a statement referencing a coordinate of a relation that is not a
// member of the schema is a construction compile error naming the
// coordinate (semantic coordinates, never template internals).
import std;
import bumbledb;

struct ServiceRow {
	[[= bdb::fresh]] std::uint64_t id;

	std::string name;
};

struct OutageRow {
	std::uint64_t service;
	bdb::interval<std::int64_t> window;
};

inline constexpr auto Service = bdb::relation<"Service", ServiceRow>;
inline constexpr auto Outage = bdb::relation<"Outage", OutageRow>;

// Outage is declared above but NOT a member of this schema.
inline constexpr auto Solo = bdb::schema<"Solo">(Service,

                                                 bdb::contained(bdb::on(Outage.service), bdb::on(Service.id)));
