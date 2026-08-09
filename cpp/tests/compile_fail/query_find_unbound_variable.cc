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

inline constexpr auto Uptime = bdb::schema<"Uptime">(Service, Outage,

                                                     bdb::contained(bdb::on(Outage.service), bdb::on(Service.id)));

inline constexpr auto Broken = bdb::query(Uptime).rule([](auto r) consteval {
	auto vars = r.vars(Outage);
	return r
	    .match(Outage,
	           {
	               .service = vars.service,
	           })
	    .find({
	        .window = vars.window,
	    });
});
