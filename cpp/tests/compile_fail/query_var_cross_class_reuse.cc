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

struct AuditRow {
	std::uint64_t actor;
};

inline constexpr auto Service = bdb::relation<"Service", ServiceRow>;
inline constexpr auto Outage = bdb::relation<"Outage", OutageRow>;
inline constexpr auto Audit = bdb::relation<"Audit", AuditRow>;

inline constexpr auto Uptime = bdb::schema<"Uptime">(Service, Outage, Audit,

                                                     bdb::contained(bdb::on(Outage.service), bdb::on(Service.id)));

inline constexpr auto Broken = bdb::query(Uptime).rule([](auto r) consteval {
	auto outage = r.vars(Outage);
	return r
	    .match(Outage,
	           {
	               .service = outage.service,
	               .window = outage.window,
	           })
	    .match(Audit,
	           {
	               .actor = outage.service,
	           })
	    .find({
	        .service = outage.service,
	    });
});
